using System;
using System.Collections;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;
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
        public int Regeln;          // so viele Regeln hat die Firewall gelesen (Diagnose)
        public List<string> BlockNamen = new List<string>();     // Namen der Sperr-Regeln fuer gta_sa.exe
        public List<string> Diagnose = new List<string>();       // Regeln, deren Programm gta_sa.exe heisst (fuer --firewall-pruefen)

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
    /// der Sprache von Windows, und zwar in einem eigenen Aufruf des Launchers (--firewall-pruefen, PruefenExtern), nicht im
    /// Fenster. Geschrieben wird nur in einem eigenen, erhoehten Aufruf (--firewall-freigeben bzw. --firewall-entfernen) mit
    /// netsh: Er schaltet die Sperr-Regeln fuer genau diese gta_sa.exe ab und legt eine eigene Freigabe an (RegelName).
    /// Andere Regeln fasst er nicht an.
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
                var alle = Regeln(politik);
                stand.Regeln = alle.Count;
                foreach (var regel in alle)
                {
                    var app = Text(Holen(regel, "ApplicationName"));
                    if (app.EndsWith(GtaVerzeichnis.SpielDatei, StringComparison.OrdinalIgnoreCase))
                        stand.Diagnose.Add(Text(Holen(regel, "Name")) + " | " + app + " | dir " + Zahl(Holen(regel, "Direction"))
                            + " | action " + Zahl(Holen(regel, "Action")) + " | enabled " + Wahr(Holen(regel, "Enabled"))
                            + " | profiles " + Zahl(Holen(regel, "Profiles")) + " | proto " + Zahl(Holen(regel, "Protocol")));
                    if (!Passt(regel, ziel)) continue;
                    if ((Zahl(Holen(regel, "Profiles")) & profile) == 0) continue;
                    var aktion = Zahl(Holen(regel, "Action"));
                    var protokoll = Zahl(Holen(regel, "Protocol"));
                    if (aktion == AktionSperren)
                    {
                        stand.Blockiert = true;
                        var name = Text(Holen(regel, "Name"));
                        if (name != "" && !stand.BlockNamen.Contains(name)) stand.BlockNamen.Add(name);
                    }
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
        /// eigene Freigabe anlegen bzw. wieder einschalten. Geschrieben wird mit netsh: Es filtert beim Abschalten nach Name und
        /// Programm, Regeln anderer Programme mit demselben Namen bleiben also. Rueckgabe 0 bei Erfolg.
        /// </summary>
        public static int FreigabeSchreiben(string programm, StringBuilder protokoll)
        {
            programm = Normalisieren(programm);
            var stand = Pruefen(programm);
            foreach (var name in stand.BlockNamen)
                Netsh("advfirewall firewall set rule name=\"" + name + "\" dir=in program=\"" + programm + "\" new enable=no", protokoll);
            if (stand.EigeneRegel)
                Netsh("advfirewall firewall set rule name=\"" + RegelName + "\" new enable=yes action=allow program=\"" + programm
                    + "\" profile=private,public", protokoll);
            else
                Netsh("advfirewall firewall add rule name=\"" + RegelName + "\" dir=in action=allow program=\"" + programm
                    + "\" profile=private,public enable=yes description=\"" + RegelBeschreibung + "\"", protokoll);
            var danach = Pruefen(programm);
            protokoll.AppendLine("danach: " + danach.Grund + ", eigene Regel " + (danach.EigeneRegel ? "ja" : "nein"));
            if (!danach.EigeneRegel) return 1;
            return danach.Noetig ? 1 : 0;
        }

        /// <summary>Laeuft im erhoehten Launcher (--firewall-entfernen): die eigene Freigabe loeschen. Rueckgabe 0 bei Erfolg.</summary>
        public static int FreigabeLoeschen(StringBuilder protokoll)
        {
            if (!RegelVorhanden()) return 0;
            Netsh("advfirewall firewall delete rule name=\"" + RegelName + "\"", protokoll);
            return RegelVorhanden() ? 1 : 0;
        }

        static int Netsh(string argumente, StringBuilder protokoll)
        {
            var start = new ProcessStartInfo(Path.Combine(Environment.SystemDirectory, "netsh.exe"), argumente);
            start.UseShellExecute = false;
            start.CreateNoWindow = true;
            start.RedirectStandardOutput = true;
            start.RedirectStandardError = true;
            using (var prozess = Process.Start(start))
            {
                var ausgabe = prozess.StandardOutput.ReadToEnd() + prozess.StandardError.ReadToEnd();
                prozess.WaitForExit(30000);
                protokoll.AppendLine("netsh " + argumente + " -> " + prozess.ExitCode + " " + ausgabe.Trim());
                return prozess.ExitCode;
            }
        }

        /// <summary>
        /// Prueft in einem eigenen Aufruf des Launchers (--firewall-pruefen): Die COM-Schnittstelle der Firewall laeuft so im
        /// Hauptthread eines eigenen Prozesses und nicht im Fenster. Faellt der Aufruf aus, gilt: nicht geprueft.
        /// </summary>
        public static FirewallStand PruefenExtern(string gta)
        {
            var stand = new FirewallStand();
            var datei = Path.Combine(Path.GetTempPath(), "samplife-firewall-" + Guid.NewGuid().ToString("N") + ".txt");
            try
            {
                var start = new ProcessStartInfo(Aktualisierung.EigeneDatei, "--firewall-pruefen \"" + datei + "\" --gta \"" + gta.TrimEnd('\\') + "\"");
                start.UseShellExecute = false;
                start.CreateNoWindow = true;
                using (var prozess = Process.Start(start))
                {
                    if (!prozess.WaitForExit(20000)) { try { prozess.Kill(); } catch (Exception) { } stand.Grund = "Prüfung dauert zu lange"; return stand; }
                    if (prozess.ExitCode != 0 || !File.Exists(datei)) { stand.Grund = "Prüfung nicht möglich (" + prozess.ExitCode + ")"; return stand; }
                }
                foreach (var zeile in File.ReadAllLines(datei, Encoding.UTF8))
                {
                    var teile = zeile.Split(new[] { '=' }, 2);
                    if (teile.Length != 2) continue;
                    switch (teile[0])
                    {
                        case "geprueft": stand.Geprueft = teile[1] == "1"; break;
                        case "erlaubt": stand.Erlaubt = teile[1] == "1"; break;
                        case "blockiert": stand.Blockiert = teile[1] == "1"; break;
                        case "eigene_regel": stand.EigeneRegel = teile[1] == "1"; break;
                        case "grund": stand.Grund = teile[1]; break;
                    }
                }
            }
            catch (Exception fehler)
            {
                stand = new FirewallStand { Grund = "Prüfung nicht möglich: " + fehler.Message };
            }
            finally
            {
                try { File.Delete(datei); } catch (Exception) { }
            }
            return stand;
        }

        /// <summary>Stand als Text fuer --firewall-pruefen (eine Angabe je Zeile, dazu Diagnose).</summary>
        public static string AlsText(FirewallStand stand)
        {
            var text = new StringBuilder();
            text.Append("geprueft=").Append(stand.Geprueft ? 1 : 0).Append('\n');
            text.Append("erlaubt=").Append(stand.Erlaubt ? 1 : 0).Append('\n');
            text.Append("blockiert=").Append(stand.Blockiert ? 1 : 0).Append('\n');
            text.Append("eigene_regel=").Append(stand.EigeneRegel ? 1 : 0).Append('\n');
            text.Append("grund=").Append(stand.Grund).Append('\n');
            text.Append("regeln=").Append(stand.Regeln).Append('\n');
            foreach (var zeile in stand.Diagnose) text.Append("regel=").Append(zeile).Append('\n');
            return text.ToString();
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

        /// <summary>
        /// Pfad vergleichbar machen: Umgebungsvariablen aufloesen, Anfuehrungszeichen weg, voller und langer Pfad (kurze 8.3-Namen
        /// wie RUNNER~1 kennt netsh nicht, und die Firewall speichert den langen).
        /// </summary>
        public static string Normalisieren(string pfad)
        {
            if (string.IsNullOrEmpty(pfad)) return "";
            var text = Environment.ExpandEnvironmentVariables(pfad.Trim().Trim('"'));
            try { text = Path.GetFullPath(text); }
            catch (Exception) { }
            return LangerPfad(text).TrimEnd('\\');
        }

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
        static extern uint GetLongPathName(string kurz, StringBuilder lang, uint groesse);

        static string LangerPfad(string pfad)
        {
            try
            {
                var lang = new StringBuilder(1024);
                var laenge = GetLongPathName(pfad, lang, (uint)lang.Capacity);
                return laenge > 0 && laenge < lang.Capacity ? lang.ToString() : pfad;
            }
            catch (Exception) { return pfad; }
        }

        static object Holen(object com, string name, params object[] argumente)
        {
            return com.GetType().InvokeMember(name, BindingFlags.GetProperty, null, com, argumente.Length == 0 ? null : argumente);
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
