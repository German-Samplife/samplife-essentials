using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Text;
using System.Web.Script.Serialization;

namespace SamplifeEssentials
{
    /// <summary>
    /// Was sich der Launcher fuer den Windows-Benutzer merkt: das bestaetigte GTA-Verzeichnis und die gewaehlten optionalen
    /// Komponenten. Der Spielername steht dort, wo SA-MP ihn liest (Spielstart). Liegt unter
    /// %LOCALAPPDATA%\German Samplife\Launcher\einstellungen.json.
    /// </summary>
    public class Einstellungen
    {
        public string Gta = "";
        public List<string> Optional = new List<string>();

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
            }
            catch (Exception) { }
            return einstellungen;
        }

        public void Speichern()
        {
            try
            {
                Directory.CreateDirectory(Path.GetDirectoryName(Datei));
                var json = new JavaScriptSerializer().Serialize(new Dictionary<string, object> { { "gta", Gta }, { "optional", Optional } });
                File.WriteAllText(Datei, json, new UTF8Encoding(false));
            }
            catch (Exception) { }
        }
    }
}
