using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using System.Web.Script.Serialization;

namespace SamplifeEssentials
{
    /// <summary>
    /// Was sich der Launcher fuer den Windows-Benutzer merkt: das bestaetigte GTA-Verzeichnis, die gewaehlten optionalen
    /// Komponenten, die zuletzt benutzten Passwoerter (Konto und Server), den zuletzt gemeldeten Absturz und wann der Spieler die
    /// Freigabe in der Firewall auf spaeter verschoben hat. Der Spielername steht dort, wo SA-MP ihn liest
    /// (Spielstart). Liegt unter %LOCALAPPDATA%\German Samplife\Launcher\einstellungen.json; die Passwoerter darin
    /// verschluesselt Windows fuer den angemeldeten Benutzer (DPAPI), ein anderer Benutzer oder Rechner kann sie nicht lesen.
    /// </summary>
    public class Einstellungen
    {
        public string Gta = "";
        public List<string> Optional = new List<string>();
        public string Kontopasswort = "";
        public string Serverpasswort = "";
        // Zeitpunkt (UTC, ISO) des zuletzt gemeldeten oder abgelehnten Absturzes (Absturzbericht)
        public string AbsturzGemeldet = "";
        // Zeitpunkt (UTC, ISO) von „Später“ bei der Freigabe in der Firewall (Firewall.FragenErlaubt)
        public string FirewallSpaeter = "";

        static readonly byte[] Zusatz = Encoding.UTF8.GetBytes("German Samplife Launcher");

        static string Datei
        {
            get
            {
                return Path.Combine(Pfade.Launcher, "einstellungen.json");
            }
        }

        public static Einstellungen Laden()
        {
            var einstellungen = new Einstellungen();
            try
            {
                if (!File.Exists(Datei)) return einstellungen;
                var daten = new JavaScriptSerializer().DeserializeObject(File.ReadAllText(Datei, Encoding.UTF8)) as Dictionary<string, object>;
                if (daten == null) return einstellungen;
                object wert;
                if (daten.TryGetValue("gta", out wert) && wert is string) einstellungen.Gta = (string)wert;
                if (daten.TryGetValue("optional", out wert) && wert is IEnumerable && !(wert is string))
                {
                    foreach (var id in (IEnumerable)wert) if (id is string) einstellungen.Optional.Add((string)id);
                }
                if (daten.TryGetValue("kontopasswort", out wert) && wert is string) einstellungen.Kontopasswort = Entschluesseln((string)wert);
                if (daten.TryGetValue("serverpasswort", out wert) && wert is string) einstellungen.Serverpasswort = Entschluesseln((string)wert);
                if (daten.TryGetValue("absturz_gemeldet", out wert) && wert is string) einstellungen.AbsturzGemeldet = (string)wert;
                if (daten.TryGetValue("firewall_spaeter", out wert) && wert is string) einstellungen.FirewallSpaeter = (string)wert;
            }
            catch (Exception) { }
            return einstellungen;
        }

        public void Speichern()
        {
            try
            {
                Directory.CreateDirectory(Path.GetDirectoryName(Datei));
                var json = new JavaScriptSerializer().Serialize(new Dictionary<string, object>
                {
                    { "gta", Gta }, { "optional", Optional },
                    { "kontopasswort", Verschluesseln(Kontopasswort) }, { "serverpasswort", Verschluesseln(Serverpasswort) },
                    { "absturz_gemeldet", AbsturzGemeldet }, { "firewall_spaeter", FirewallSpaeter }
                });
                File.WriteAllText(Datei, json, new UTF8Encoding(false));
            }
            catch (Exception) { }
        }

        static string Verschluesseln(string klartext)
        {
            if (string.IsNullOrEmpty(klartext)) return "";
            try
            {
                return Convert.ToBase64String(ProtectedData.Protect(Encoding.UTF8.GetBytes(klartext), Zusatz, DataProtectionScope.CurrentUser));
            }
            catch (Exception) { return ""; }
        }

        /// <summary>Leer, wenn nichts gespeichert ist oder Windows es nicht entschluesseln kann (anderer Benutzer, neues Profil).</summary>
        static string Entschluesseln(string gespeichert)
        {
            if (string.IsNullOrEmpty(gespeichert)) return "";
            try
            {
                return Encoding.UTF8.GetString(ProtectedData.Unprotect(Convert.FromBase64String(gespeichert), Zusatz, DataProtectionScope.CurrentUser));
            }
            catch (Exception) { return ""; }
        }
    }
}
