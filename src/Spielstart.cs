using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Net;
using System.Text;
using System.Text.RegularExpressions;
using System.Web.Script.Serialization;
using Microsoft.Win32;

namespace SamplifeEssentials
{
    /// <summary>Name fuer SA-MP, Anmeldung beim Server und Start von samp.exe.</summary>
    public static class Spielstart
    {
        public const string SampExe = "samp.exe";
        // Hier liest SA-MP den Spielernamen. Im Testmodus ein eigener Schluessel, damit Tests den echten Namen nicht ueberschreiben.
        const string SampSchluessel = @"Software\SAMP";
        const string TestSchluessel = @"Software\German Samplife\Launcher-Test\SAMP";
        // Regeln von SA-MP fuer Spielernamen
        static readonly Regex NameMuster = new Regex(@"^[A-Za-z0-9\[\]()$@._=]{3,20}$");

        // Eigene Adresse fuer den Knopf „Verbinden“ auf der Website: samplife://verbinden oeffnet den Launcher, der prueft und
        // direkt verbindet. Im Testmodus ein eigenes Schema, damit Tests die echte Verknuepfung nicht ueberschreiben.
        public const string Schema = "samplife";
        const string TestSchema = "samplife-test";

        /// <summary>Traegt samplife:// fuer den Windows-Benutzer ein (ohne Administratorrechte) und zeigt auf diese .exe.</summary>
        public static void AdresseEintragen(bool testmodus)
        {
            var exe = Aktualisierung.EigeneDatei;
            using (var schluessel = Registry.CurrentUser.CreateSubKey(@"Software\Classes\" + (testmodus ? TestSchema : Schema)))
            {
                schluessel.SetValue("", "URL:Samplife Launcher");
                schluessel.SetValue("URL Protocol", "");
                using (var symbol = schluessel.CreateSubKey("DefaultIcon")) symbol.SetValue("", "\"" + exe + "\",0");
                using (var befehl = schluessel.CreateSubKey(@"shell\open\command")) befehl.SetValue("", "\"" + exe + "\" --link \"%1\"");
            }
        }

        /// <summary>
        /// Nur genau „samplife://verbinden“ startet direkt. Alles andere, was eine Seite in den Link schreiben koennte,
        /// oeffnet nur das Fenster.
        /// </summary>
        public static bool IstVerbindenLink(string link)
        {
            if (string.IsNullOrEmpty(link)) return false;
            var ohneEnde = link.Trim().TrimEnd('/').ToLowerInvariant();
            return ohneEnde == Schema + "://verbinden" || ohneEnde == TestSchema + "://verbinden";
        }

        /// <summary>Leer, wenn der Name gueltig ist, sonst der Grund.</summary>
        public static string NamePruefen(string name)
        {
            if (string.IsNullOrEmpty(name)) return "Gib deinen Spielernamen ein.";
            if (name.Length < 3 || name.Length > 20) return "Der Name muss 3 bis 20 Zeichen lang sein.";
            if (!NameMuster.IsMatch(name)) return "Erlaubt sind nur Buchstaben ohne Umlaute, Ziffern und [ ] ( ) $ @ . _ =";
            return "";
        }

        public static string GespeicherterName(bool testmodus)
        {
            try
            {
                using (var schluessel = Registry.CurrentUser.OpenSubKey(testmodus ? TestSchluessel : SampSchluessel))
                {
                    var wert = schluessel == null ? null : schluessel.GetValue("PlayerName") as string;
                    return wert ?? "";
                }
            }
            catch (Exception) { return ""; }
        }

        public static void NameSetzen(string name, bool testmodus)
        {
            using (var schluessel = Registry.CurrentUser.CreateSubKey(testmodus ? TestSchluessel : SampSchluessel))
            {
                schluessel.SetValue("PlayerName", name, RegistryValueKind.String);
            }
        }

        /// <summary>Wo SA-MP das Spiel sucht (wie „Einstellungen“ im SA-MP-Browser): das bestaetigte GTA-Verzeichnis.</summary>
        public static void GtaEintragen(string gta, bool testmodus)
        {
            using (var schluessel = Registry.CurrentUser.CreateSubKey(testmodus ? TestSchluessel : SampSchluessel))
            {
                schluessel.SetValue("gta_sa_exe", Path.Combine(gta, GtaVerzeichnis.SpielDatei), RegistryValueKind.String);
            }
        }

        /// <summary>
        /// SA-MP-Version an der Dateiversion der samp.dll, so wie samp-cef sie erkennt: 0.3.7.0 ist R1, 0.3.7.2 ist R3,
        /// 0.3.8.0 ist 0.3.DL. Leer, wenn die Datei fehlt oder unbekannt ist.
        /// </summary>
        public static string SampDateiVersion(string gta)
        {
            try
            {
                var datei = Path.Combine(gta, GtaVerzeichnis.SampDatei);
                if (!File.Exists(datei)) return "";
                var info = FileVersionInfo.GetVersionInfo(datei);
                var haupt = info.FileMinorPart & 0xFF;
                var neben = info.FileBuildPart & 0xFF;
                var stufe = info.FilePrivatePart & 0xFF;
                if (haupt == 3 && neben == 7 && stufe == 0) return "0.3.7-R1";
                if (haupt == 3 && neben == 7 && stufe == 2) return "0.3.7-R3";
                if (haupt == 3 && neben == 8 && stufe == 0) return "0.3.DL";
                return "";
            }
            catch (Exception) { return ""; }
        }

        /// <summary>Startet samp.exe aus dem GTA-Verzeichnis und verbindet direkt mit dem Server.</summary>
        public static Process Starten(string gta, string adresse, int port)
        {
            var start = new ProcessStartInfo(Path.Combine(gta, SampExe), adresse + ":" + port);
            start.WorkingDirectory = gta;
            start.UseShellExecute = false;
            return Process.Start(start);
        }

        /// <summary>
        /// Meldet den Start beim Server an (Name, Stand, Launcher-Version). Der Gamemode laesst nur Spieler herein, die sich so
        /// angemeldet haben. Rueckgabe leer bei Erfolg, sonst der Grund.
        /// </summary>
        public static string Anmelden(string url, string name, long stand, bool testmodus)
        {
            Uri adresse;
            if (!Uri.TryCreate(url, UriKind.Absolute, out adresse)) return "Ungültige Adresse für die Anmeldung: " + url;
            if (adresse.Scheme != Uri.UriSchemeHttps && !(testmodus && adresse.Scheme == Uri.UriSchemeHttp && adresse.IsLoopback))
                return "Die Anmeldung muss über eine verschlüsselte Verbindung (https) laufen.";

            var serialisierer = new JavaScriptSerializer();
            var daten = Encoding.UTF8.GetBytes(serialisierer.Serialize(new Dictionary<string, object>
            {
                { "name", name }, { "stand", stand }, { "launcher", Programm.Version }
            }));
            ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
            var anfrage = (HttpWebRequest)WebRequest.Create(adresse);
            anfrage.Method = "POST";
            anfrage.ContentType = "application/json";
            anfrage.UserAgent = "SamplifeLauncher/" + Programm.Version;
            anfrage.Timeout = 15000;
            anfrage.AllowAutoRedirect = false;
            try
            {
                using (var strom = anfrage.GetRequestStream()) strom.Write(daten, 0, daten.Length);
                using (var antwort = (HttpWebResponse)anfrage.GetResponse())
                {
                    var text = Lesen(antwort);
                    var json = Auswerten(text);
                    if (json != null && json.ContainsKey("ok") && json["ok"] is bool && (bool)json["ok"]) return "";
                    return Fehlertext(json, "Der Server hat die Anmeldung nicht bestätigt.");
                }
            }
            catch (WebException fehler)
            {
                var antwort = fehler.Response as HttpWebResponse;
                if (antwort == null) return "Der Server ist für die Anmeldung nicht erreichbar: " + fehler.Message;
                using (antwort) return Fehlertext(Auswerten(Lesen(antwort)), "Die Anmeldung wurde abgelehnt (HTTP " + (int)antwort.StatusCode + ").");
            }
            catch (Exception fehler)
            {
                return "Die Anmeldung ist fehlgeschlagen: " + fehler.Message;
            }
        }

        static string Lesen(HttpWebResponse antwort)
        {
            try
            {
                using (var leser = new StreamReader(antwort.GetResponseStream(), Encoding.UTF8)) return leser.ReadToEnd();
            }
            catch (Exception) { return ""; }
        }

        static Dictionary<string, object> Auswerten(string text)
        {
            try { return new JavaScriptSerializer().DeserializeObject(text) as Dictionary<string, object>; }
            catch (Exception) { return null; }
        }

        static string Fehlertext(Dictionary<string, object> json, string ersatz)
        {
            if (json == null) return ersatz;
            foreach (var feld in new[] { "fehler", "error", "message" })
            {
                object wert;
                if (json.TryGetValue(feld, out wert) && wert is string && (string)wert != "") return (string)wert;
            }
            return ersatz;
        }
    }
}
