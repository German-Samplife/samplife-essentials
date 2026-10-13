using System;
using System.Collections;
using System.Collections.Generic;
using System.Web.Script.Serialization;

namespace SamplifeEssentials
{
    /// <summary>Ein Download: eine einzelne Datei oder ein Zip-Archiv, das ins GTA-Verzeichnis entpackt wird.</summary>
    public class Paket
    {
        public string Samp;            // leer = fuer jede SA-MP-Version
        public string Url;
        public string Sha256;
        public long Groesse;
        public string Typ;             // "zip" oder "datei"
        public string Ziel;            // Unterordner im GTA-Verzeichnis, leer = Wurzel
        public string ZipUnterordner;  // nur diesen Ordner aus dem Archiv entpacken, leer = alles
        public string Dateiname;       // Name der Zieldatei bei Typ "datei"
    }

    public class Komponente
    {
        public string Id;
        public string Name;
        public string Beschreibung;
        public string Quelle;          // Projektseite, wird in der Oberflaeche verlinkt
        public string Lizenz;
        public bool Pflicht;
        public List<Paket> Pakete = new List<Paket>();
    }

    public class SampVersion
    {
        public string Name;
        public string Sha256;          // Pruefsumme der samp.dll dieser Version, leer = nur von Hand waehlbar
    }

    public class Manifest
    {
        public const int UnterstuetzteVersion = 1;

        public int Version;
        public string Hinweis;
        public List<SampVersion> SampVersionen = new List<SampVersion>();
        public List<Komponente> Komponenten = new List<Komponente>();

        public static Manifest Lesen(string json)
        {
            object wurzel;
            try
            {
                wurzel = new JavaScriptSerializer().DeserializeObject(json);
            }
            catch (Exception fehler)
            {
                throw new ManifestFehler("Die Liste der Komponenten ist kein gültiges JSON: " + fehler.Message);
            }
            var daten = wurzel as Dictionary<string, object>;
            if (daten == null) throw new ManifestFehler("Die Liste der Komponenten hat ein unerwartetes Format.");

            var manifest = new Manifest();
            manifest.Version = (int)Zahl(daten, "version");
            if (manifest.Version != UnterstuetzteVersion)
            {
                throw new ManifestFehler("Diese Version des Installers ist veraltet (Liste Version " + manifest.Version
                    + ", unterstützt wird " + UnterstuetzteVersion + "). Bitte lade den aktuellen Installer von der Website.");
            }
            manifest.Hinweis = Text(daten, "hinweis");

            foreach (var eintrag in Liste(daten, "samp_versionen"))
            {
                var version = new SampVersion();
                version.Name = Pflichttext(eintrag, "name", "samp_versionen");
                // Ohne Pruefsumme wird die Version nicht erkannt, der Spieler kann sie aber selbst auswaehlen
                version.Sha256 = Text(eintrag, "sha256").ToLowerInvariant();
                manifest.SampVersionen.Add(version);
            }

            foreach (var eintrag in Liste(daten, "komponenten"))
            {
                var komponente = new Komponente();
                komponente.Id = Pflichttext(eintrag, "id", "komponenten");
                komponente.Name = Pflichttext(eintrag, "name", "komponenten");
                komponente.Beschreibung = Text(eintrag, "beschreibung");
                komponente.Quelle = Text(eintrag, "quelle");
                komponente.Lizenz = Text(eintrag, "lizenz");
                komponente.Pflicht = Wahr(eintrag, "pflicht");
                foreach (var p in Liste(eintrag, "pakete"))
                {
                    var paket = new Paket();
                    paket.Samp = Text(p, "samp");
                    paket.Url = Pflichttext(p, "url", komponente.Id);
                    paket.Sha256 = Pflichttext(p, "sha256", komponente.Id).ToLowerInvariant();
                    paket.Groesse = Zahl(p, "groesse");
                    paket.Typ = Text(p, "typ");
                    if (paket.Typ == "") paket.Typ = "zip";
                    paket.Ziel = Text(p, "ziel");
                    paket.ZipUnterordner = Text(p, "zip_unterordner");
                    paket.Dateiname = Text(p, "dateiname");
                    if (paket.Typ != "zip" && paket.Typ != "datei")
                        throw new ManifestFehler("Unbekannter Pakettyp \"" + paket.Typ + "\" bei " + komponente.Name + ".");
                    if (paket.Sha256.Length != 64)
                        throw new ManifestFehler("Die Prüfsumme eines Pakets von " + komponente.Name + " ist unvollständig.");
                    komponente.Pakete.Add(paket);
                }
                if (komponente.Pakete.Count == 0)
                    throw new ManifestFehler("Die Komponente " + komponente.Name + " enthält keine Pakete.");
                manifest.Komponenten.Add(komponente);
            }
            if (manifest.Komponenten.Count == 0) throw new ManifestFehler("Die Liste enthält keine Komponenten.");
            return manifest;
        }

        /// <summary>Pakete einer Komponente, die zur erkannten SA-MP-Version passen.</summary>
        public static List<Paket> PassendePakete(Komponente komponente, string sampVersion)
        {
            var passend = new List<Paket>();
            foreach (var paket in komponente.Pakete)
            {
                if (paket.Samp == "" || string.Equals(paket.Samp, sampVersion, StringComparison.OrdinalIgnoreCase))
                    passend.Add(paket);
            }
            return passend;
        }

        static string Text(Dictionary<string, object> daten, string name)
        {
            object wert;
            if (!daten.TryGetValue(name, out wert) || wert == null) return "";
            return Convert.ToString(wert).Trim();
        }

        static string Pflichttext(Dictionary<string, object> daten, string name, string wo)
        {
            var text = Text(daten, name);
            if (text == "") throw new ManifestFehler("In der Liste fehlt die Angabe \"" + name + "\" (" + wo + ").");
            return text;
        }

        static long Zahl(Dictionary<string, object> daten, string name)
        {
            object wert;
            if (!daten.TryGetValue(name, out wert) || wert == null) return 0;
            try { return Convert.ToInt64(wert); }
            catch (Exception) { throw new ManifestFehler("Die Angabe \"" + name + "\" ist keine Zahl."); }
        }

        static bool Wahr(Dictionary<string, object> daten, string name)
        {
            object wert;
            if (!daten.TryGetValue(name, out wert) || wert == null) return false;
            return wert is bool && (bool)wert;
        }

        static List<Dictionary<string, object>> Liste(Dictionary<string, object> daten, string name)
        {
            var ergebnis = new List<Dictionary<string, object>>();
            object wert;
            if (!daten.TryGetValue(name, out wert) || wert == null) return ergebnis;
            var eintraege = wert as IEnumerable;
            if (eintraege == null || wert is string) throw new ManifestFehler("Die Angabe \"" + name + "\" ist keine Liste.");
            foreach (var eintrag in eintraege)
            {
                var objekt = eintrag as Dictionary<string, object>;
                if (objekt == null) throw new ManifestFehler("Ein Eintrag in \"" + name + "\" hat ein unerwartetes Format.");
                ergebnis.Add(objekt);
            }
            return ergebnis;
        }
    }

    public class ManifestFehler : Exception
    {
        public ManifestFehler(string meldung) : base(meldung) { }
    }
}
