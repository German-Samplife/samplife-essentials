using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Net;
using System.Text;
using System.Web.Script.Serialization;

namespace SamplifeEssentials
{
    /// <summary>
    /// Absturzberichte: Stuerzt GTA San Andreas mit SA-MP ab, schreibt SA-MP die Datei crashinfo.txt nach
    /// „Eigene Dokumente\GTA San Andreas User Files\SAMP“. Ist der Launcher noch offen, fragt er gleich
    /// nach dem Ende des Spiels (Fenster.SpielBeendet), sonst beim naechsten Start, und bietet an, sie an die Adresse server.absturzbericht aus der Liste zu schicken (Website: /api/launcher/crash).
    /// Mitgeschickt werden nur Name im Spiel, Version des Launchers, Windows-Version, Zeitpunkt, der Inhalt der Datei und
    /// ein freiwilliger Kommentar. Ob ein Absturz schon gemeldet oder abgelehnt ist, merkt sich Einstellungen.AbsturzGemeldet.
    /// </summary>
    public static class Absturzbericht
    {
        public const int MaxInhalt = 60000;
        public const int MaxKommentar = 1000;
        // Aeltere Abstuerze bietet der Launcher nicht mehr an
        static readonly TimeSpan Hoechstalter = TimeSpan.FromDays(7);

        public static string Datei
        {
            get
            {
                return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments),
                    "GTA San Andreas User Files", "SAMP", "crashinfo.txt");
            }
        }

        /// <summary>Zeitpunkt (UTC) eines neuen, noch nicht gemeldeten Absturzes, sonst null.</summary>
        public static DateTime? Neuer(Einstellungen einstellungen)
        {
            try
            {
                if (!File.Exists(Datei)) return null;
                var zeit = File.GetLastWriteTimeUtc(Datei);
                if (DateTime.UtcNow - zeit > Hoechstalter) return null;
                var gemeldet = Gemeldet(einstellungen);
                if (gemeldet.HasValue && zeit <= gemeldet.Value) return null;
                return zeit;
            }
            catch (Exception) { return null; }
        }

        static DateTime? Gemeldet(Einstellungen einstellungen)
        {
            DateTime zeit;
            if (DateTime.TryParse(einstellungen.AbsturzGemeldet, CultureInfo.InvariantCulture,
                DateTimeStyles.AdjustToUniversal | DateTimeStyles.AssumeUniversal, out zeit)) return zeit;
            return null;
        }

        /// <summary>Gemeldet oder abgelehnt: Der Hinweis kommt fuer diesen Absturz nicht wieder.</summary>
        public static void Merken(Einstellungen einstellungen, DateTime zeitUtc)
        {
            einstellungen.AbsturzGemeldet = Iso(zeitUtc);
            einstellungen.Speichern();
        }

        public static string Iso(DateTime zeitUtc)
        {
            return zeitUtc.ToUniversalTime().ToString("yyyy-MM-dd'T'HH:mm:ss'Z'", CultureInfo.InvariantCulture);
        }

        /// <summary>Inhalt der crashinfo.txt: UTF-8, sonst Windows-1252, gekuerzt auf MaxInhalt Zeichen.</summary>
        public static string Lesen()
        {
            var daten = File.ReadAllBytes(Datei);
            string text;
            try { text = new UTF8Encoding(false, true).GetString(daten); }
            catch (DecoderFallbackException) { text = Encoding.GetEncoding(1252).GetString(daten); }
            text = text.Replace("\0", "");
            if (text.Length > MaxInhalt) text = text.Substring(0, MaxInhalt);
            return text;
        }

        public static string Windows()
        {
            var version = Environment.OSVersion.Version;
            return "Windows " + version.Major + "." + version.Minor + "." + version.Build + ", "
                + (Environment.Is64BitOperatingSystem ? "64 Bit" : "32 Bit");
        }

        /// <summary>Schickt den Bericht. Rueckgabe leer bei Erfolg (nummer = Nummer des Berichts), sonst der Grund.</summary>
        public static string Senden(string url, string name, string kommentar, DateTime zeitUtc, string inhalt, bool testmodus, out int nummer)
        {
            nummer = 0;
            Uri adresse;
            if (!Uri.TryCreate(url, UriKind.Absolute, out adresse)) return "Ungültige Adresse für den Absturzbericht: " + url;
            if (adresse.Scheme != Uri.UriSchemeHttps && !(testmodus && adresse.Scheme == Uri.UriSchemeHttp && adresse.IsLoopback))
                return "Der Bericht muss über eine verschlüsselte Verbindung (https) laufen.";

            if (name.Length > 24) name = name.Substring(0, 24);
            kommentar = (kommentar ?? "").Trim();
            if (kommentar.Length > MaxKommentar) kommentar = kommentar.Substring(0, MaxKommentar);
            var serialisierer = new JavaScriptSerializer { MaxJsonLength = 1024 * 1024 };
            var felder = new Dictionary<string, object>
            {
                { "name", name }, { "launcher", Programm.Version }, { "windows", Windows() }, { "zeit", Iso(zeitUtc) },
                { "crashinfo", inhalt }, { "kommentar", kommentar }
            };
            var daten = Encoding.UTF8.GetBytes(serialisierer.Serialize(felder));
            ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
            var anfrage = (HttpWebRequest)WebRequest.Create(adresse);
            anfrage.Method = "POST";
            anfrage.ContentType = "application/json; charset=utf-8";
            anfrage.UserAgent = "SamplifeLauncher/" + Programm.Version;
            anfrage.Timeout = 20000;
            anfrage.AllowAutoRedirect = false;
            try
            {
                using (var strom = anfrage.GetRequestStream()) strom.Write(daten, 0, daten.Length);
                using (var antwort = (HttpWebResponse)anfrage.GetResponse())
                {
                    var json = Auswerten(Lesen(antwort));
                    object wert;
                    if (json != null && json.TryGetValue("ok", out wert) && wert is bool && (bool)wert)
                    {
                        if (json.TryGetValue("nummer", out wert) && wert is int) nummer = (int)wert;
                        return "";
                    }
                    return Fehlertext(json, "Der Server hat den Bericht nicht bestätigt.");
                }
            }
            catch (WebException fehler)
            {
                var antwort = fehler.Response as HttpWebResponse;
                if (antwort == null) return "Der Server ist gerade nicht erreichbar: " + fehler.Message;
                using (antwort)
                {
                    if ((int)antwort.StatusCode == 429)
                        return Fehlertext(Auswerten(Lesen(antwort)), "Du hast gerade schon Berichte geschickt. Versuch es später noch einmal.");
                    return Fehlertext(Auswerten(Lesen(antwort)), "Der Bericht wurde abgelehnt (HTTP " + (int)antwort.StatusCode + ").");
                }
            }
            catch (Exception fehler)
            {
                return "Der Bericht ließ sich nicht senden: " + fehler.Message;
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
