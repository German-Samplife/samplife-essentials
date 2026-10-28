using System;
using System.Collections;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices.ComTypes;
using System.Text;

namespace SamplifeEssentials
{
    /// <summary>Was die Windows-Firewall zu gta_sa.exe sagt (Firewall.Pruefen).</summary>
    public class FirewallStand
    {
        public bool Geprueft;       // false: Firewall aus, eingehend ohnehin erlaubt oder Pruefung nicht moeglich: nichts tun
        public bool Erlaubt;        // aktive Regel, die gta_sa.exe eingehend erlaubt (UDP oder alle Protokolle)
        public bool Blockiert;      // aktive Regel, die gta_sa.exe eingehend sperrt (gewinnt gegen jede Freigabe)
        public bool EigeneRegel;    // die Regel des Launchers (Firewall.RegelName) gibt es
        public string Grund = "";   // fuer das Protokoll

        /// <summary>Der Sprach-Chat kommt vermutlich nicht durch: fehlende Freigabe oder eine Sperre.</summary>
        public bool Noetig
        {
            get { return Geprueft && (!Erlaubt || Blockiert); }
        }
    }

    /// <summary>
    /// Freigabe von gta_sa.exe in der Windows-Firewall fuer den Sprach-Chat. SampVoice spricht per UDP mit dem Voice-Server;
    /// startet GTA zum ersten Mal, fragt Windows, ob es ins Netz darf. Im Vollbild liegt diese Frage hinter dem Spiel und
    /// wird abgelehnt, dann legt Windows Sperr-Regeln fuer gta_sa.exe an. Der Spieler hoert dann niemanden, und die Taste
    /// zum Sprechen tut nichts.
    ///
    /// Gelesen wird ohne Administratorrechte ueber die COM-Schnittstelle der Firewall (HNetCfg.FwPolicy2), unabhaengig von
    /// der Sprache von Windows. Geschrieben wird nur in einem eigenen, erhoehten Aufruf des Launchers
    /// (--firewall-freigeben bzw. --firewall-entfernen): Er schaltet die Sperr-Regeln fuer genau diese gta_sa.exe ab und legt
    /// eine eigene Freigabe an (RegelName). Andere Regeln fasst er nicht an.
    /// </summary>
    public static class Firewall
    {
        public const string RegelName = "German Samplife – GTA San Andreas";
        public const string RegelBeschreibung = "Vom Samplife Launcher angelegt: Sprach-Chat (SampVoice) auf German Samplife. Entfernt sich beim Deinstallieren des Launchers.";

        // Werte der Firewall-Schnittstelle (NET_FW_*)
        const int RichtungEin = 1;
        const int AktionSperren = 0;
        const int AktionErlauben = 1;
        const int ProfilDomaene = 1;
        const int ProfilPrivat = 2;
        const int ProfilOeffentlich = 4;
        const int ProtokollAlle = 256;
        const int ProtokollUdp = 17;

        /// <summary>Nach „Später“ fragt der Launcher erst nach so vielen Tagen wieder.</summary>
        public const int SpaeterTage = 7;

        /// <summary>Darf der Launcher vor dem Start fragen? Nicht, solange „Später“ weniger als SpaeterTage her ist.</summary>
        public static bool FragenErlaubt(Einstellungen einstellungen)
        {
            if (einstellungen == null || string.IsNullOrEmpty(einstellungen.FirewallSpaeter)) return true;
            DateTime wann;
            if (!DateTime.TryParse(einstellungen.FirewallSpaeter, CultureInfo.InvariantCulture,
                    DateTimeStyles.AdjustToUniversal | DateTimeStyles.AssumeUniversal, out wann)) return true;
            return DateTime.UtcNow - wann >= TimeSpan.FromDays(SpaeterTage);
        }

        public static void SpaeterMerken(Einstellungen einstellungen)
        {
            einstellungen.FirewallSpaeter = DateTime.UtcNow.ToString("o", CultureInfo.InvariantCulture);
            einstellungen.Speichern();
        }

        /// <summary>Liest die Regeln ohne Administratorrechte. Wirft nie; im Zweifel ist Geprueft false.</summary>
        public static FirewallStand Pruefen(string programm)
        {
            var stand = new FirewallStand();
            try
            {
                var politik = Politik();
                if (politik == null) { stand.Grund = "keine Firewall-Schnittstelle"; return stand; }
                var profile = Zahl(Holen(politik, "CurrentProfileTypes"));
                var aktiv = false;
                var gesperrt = false;
                foreach (var profil in new[] { ProfilDomaene, ProfilPrivat, ProfilOeffentlich })
                {
                    if ((profile & profil) == 0 || !Wahr(Holen(politik, "FirewallEnabled", profil))) continue;
                    aktiv = true;
                    if (Zahl(Holen(politik, "DefaultInboundAction", profil)) != AktionErlauben) gesperrt = true;
                }
                foreach (var regel in Regeln(politik))
                {
                    if (Text(Holen(regel, "Name")) == RegelName) stand.EigeneRegel = true;
                }
                if (!aktiv) { stand.Grund = "die Windows-Firewall ist aus"; return stand; }
                if (!gesperrt) { stand.Grund = "eingehende Verbindungen sind ohnehin erlaubt"; return stand; }

                var ziel = Normalisieren(programm);
                foreach (var regel in Regeln(politik))
                {
                    if (!Passt(regel, ziel)) continue;
                    if ((Zahl(Holen(regel, "Profiles")) & profile) == 0) continue;
                    var aktion = Zahl(Holen(regel, "Action"));
                    var protokoll = Zahl(Holen(regel, "Protocol"));
                    if (aktion == AktionSperren) stand.Blockiert = true;
                    else if (aktion == AktionErlauben && (protokoll == ProtokollAlle || protokoll == ProtokollUdp)) stand.Erlaubt = true;
                }
                stand.Geprueft = true;
                stand.Grund = stand.Blockiert ? "Windows sperrt gta_sa.exe" : stand.Erlaubt ? "gta_sa.exe ist freigegeben" : "keine Freigabe für gta_sa.exe";
            }
            catch (Exception fehler)
            {
                stand.Geprueft = false;
                stand.Grund = "Prüfung nicht möglich: " + fehler.Message;
            }
            return stand;
        }

        /// <summary>Gibt es die Regel des Launchers? Ohne Administratorrechte; false, wenn sich das nicht lesen laesst.</summary>
        public static bool RegelVorhanden()
        {
            try
            {
                var politik = Politik();
                if (politik == null) return false;
                foreach (var regel in Regeln(politik))
                {
                    if (Text(Holen(regel, "Name")) == RegelName) return true;
                }
            }
            catch (Exception) { }
            return false;
        }

        // ---------------------------------------------------------------- Schreiben (nur im erhoehten Aufruf)

        /// <summary>
        /// Laeuft im erhoehten Launcher (--firewall-freigeben): Sperr-Regeln fuer genau diese gta_sa.exe abschalten und die
        /// eigene Freigabe anlegen bzw. wieder einschalten. Rueckgabe 0 bei Erfolg.
        /// </summary>
        public static int FreigabeSchreiben(string programm)
        {
            var politik = Politik();
            if (politik == null) return 2;
            var ziel = Normalisieren(programm);
            var regeln = Holen(politik, "Rules");
            object eigene = null;
            foreach (var regel in Regeln(politik))
            {
                if (Text(Holen(regel, "Name")) == RegelName) { eigene = regel; continue; }
                if (Passt(regel, ziel) && Zahl(Holen(regel, "Action")) == AktionSperren) Setzen(regel, "Enabled", false);
            }
            if (eigene == null)
            {
                var typ = Type.GetTypeFromProgID("HNetCfg.FWRule");
                if (typ == null) return 2;
                eigene = Activator.CreateInstance(typ);
                RegelFuellen(eigene, programm);
                regeln.GetType().InvokeMember("Add", BindingFlags.InvokeMethod, null, regeln, new[] { eigene });
            }
            else
            {
                RegelFuellen(eigene, programm);
            }
            return 0;
        }

        /// <summary>Laeuft im erhoehten Launcher (--firewall-entfernen): die eigene Freigabe loeschen. Rueckgabe 0 bei Erfolg.</summary>
        public static int FreigabeLoeschen()
        {
            var politik = Politik();
            if (politik == null) return 2;
            var regeln = Holen(politik, "Rules");
            // Remove loescht je Aufruf eine Regel dieses Namens; mehr als eine sollte es nie geben
            for (var i = 0; i < 10 && RegelVorhanden(); i++)
                regeln.GetType().InvokeMember("Remove", BindingFlags.InvokeMethod, null, regeln, new object[] { RegelName });
            return RegelVorhanden() ? 1 : 0;
        }

        static void RegelFuellen(object regel, string programm)
        {
            Setzen(regel, "Name", RegelName);
            Setzen(regel, "Description", RegelBeschreibung);
            Setzen(regel, "ApplicationName", programm);
            Setzen(regel, "Protocol", ProtokollAlle);
            Setzen(regel, "Direction", RichtungEin);
            Setzen(regel, "Action", AktionErlauben);
            Setzen(regel, "Profiles", ProfilPrivat | ProfilOeffentlich);
            Setzen(regel, "Enabled", true);
        }

        // ---------------------------------------------------------------- Erhoehter Aufruf aus dem Fenster

        /// <summary>
        /// Startet den Launcher erhoeht mit --firewall-freigeben (Windows fragt nach Administratorrechten) und wartet auf ihn.
        /// Wirft Win32Exception mit NativeErrorCode 1223, wenn der Spieler die Abfrage ablehnt.
        /// </summary>
        public static int Freigeben(string gta)
        {
            return Erhoeht("--firewall-freigeben --gta \"" + gta.TrimEnd('\\') + "\"");
        }

        public static int Entfernen()
        {
            return Erhoeht("--firewall-entfernen");
        }

        static int Erhoeht(string argumente)
        {
            var start = new ProcessStartInfo(Aktualisierung.EigeneDatei, argumente);
            start.Verb = "runas";
            start.UseShellExecute = true;
            start.WindowStyle = ProcessWindowStyle.Hidden;
            using (var prozess = Process.Start(start))
            {
                if (prozess == null) return -1;
                if (!prozess.WaitForExit(60000)) return -1;
                return prozess.ExitCode;
            }
        }

        public static bool Abgelehnt(Exception fehler)
        {
            var win32 = fehler as Win32Exception;
            return win32 != null && win32.NativeErrorCode == 1223;
        }

        /// <summary>Eine Zeile ins Protokoll des Launchers im GTA-Verzeichnis (samplife-essentials-protokoll.txt).</summary>
        public static void Notieren(string gta, string text)
        {
            try
            {
                if (string.IsNullOrEmpty(gta) || !Directory.Exists(gta)) return;
                File.AppendAllText(Path.Combine(gta, Installation.ProtokollDatei),
                    DateTime.Now.ToString("dd.MM.yyyy HH:mm:ss", CultureInfo.InvariantCulture) + "  Firewall: " + text + Environment.NewLine,
                    Encoding.UTF8);
            }
            catch (Exception) { }
        }

        // ---------------------------------------------------------------- Hilfen fuer die COM-Schnittstelle (spaet gebunden)

        static object Politik()
        {
            var typ = Type.GetTypeFromProgID("HNetCfg.FwPolicy2");
            return typ == null ? null : Activator.CreateInstance(typ);
        }

        /// <summary>Eingehende, eingeschaltete Regel fuer genau dieses Programm.</summary>
        static bool Passt(object regel, string ziel)
        {
            var programm = Text(Holen(regel, "ApplicationName"));
            if (programm == "" || !string.Equals(Normalisieren(programm), ziel, StringComparison.OrdinalIgnoreCase)) return false;
            return Zahl(Holen(regel, "Direction")) == RichtungEin && Wahr(Holen(regel, "Enabled"));
        }

        static List<object> Regeln(object politik)
        {
            var liste = new List<object>();
            var sammlung = Holen(politik, "Rules");
            var aufzaehlbar = sammlung as IEnumerable;
            if (aufzaehlbar != null)
            {
                foreach (var regel in aufzaehlbar) liste.Add(regel);
                return liste;
            }
            var aufzaehlung = (IEnumVARIANT)sammlung.GetType().InvokeMember("_NewEnum", BindingFlags.GetProperty, null, sammlung, null);
            var puffer = new object[1];
            while (aufzaehlung.Next(1, puffer, IntPtr.Zero) == 0) liste.Add(puffer[0]);
            return liste;
        }

        /// <summary>Pfad vergleichbar machen: Umgebungsvariablen aufloesen, Anfuehrungszeichen weg, voller Pfad.</summary>
        public static string Normalisieren(string pfad)
        {
            if (string.IsNullOrEmpty(pfad)) return "";
            var text = Environment.ExpandEnvironmentVariables(pfad.Trim().Trim('"'));
            try { text = Path.GetFullPath(text); }
            catch (Exception) { }
            return text.TrimEnd('\\');
        }

        static object Holen(object com, string name, params object[] argumente)
        {
            return com.GetType().InvokeMember(name, BindingFlags.GetProperty, null, com, argumente.Length == 0 ? null : argumente);
        }

        static void Setzen(object com, string name, object wert)
        {
            com.GetType().InvokeMember(name, BindingFlags.SetProperty, null, com, new[] { wert });
        }

        static int Zahl(object wert)
        {
            try { return wert == null ? 0 : Convert.ToInt32(wert, CultureInfo.InvariantCulture); }
            catch (Exception) { return 0; }
        }

        static bool Wahr(object wert)
        {
            try { return wert != null && Convert.ToBoolean(wert, CultureInfo.InvariantCulture); }
            catch (Exception) { return false; }
        }

        static string Text(object wert)
        {
            return wert as string ?? "";
        }
    }
}
