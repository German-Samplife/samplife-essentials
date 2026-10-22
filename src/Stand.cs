using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using System.Web.Script.Serialization;

namespace SamplifeEssentials
{
    /// <summary>Eine Datei, die aus einem Paket im GTA-Verzeichnis liegt.</summary>
    public class DateiStand
    {
        public string Pfad;            // relativ zum GTA-Verzeichnis
        public long Groesse;
        public long Geaendert;         // Aenderungszeit (UTC, Ticks) beim letzten Pruefen
        public string Sha256;
    }

    public class PaketStand
    {
        public string Url;
        public string Sha256;          // Pruefsumme des Downloads laut Liste
        public List<DateiStand> Dateien = new List<DateiStand>();
    }

    public class KomponenteStand
    {
        public string Id;
        public List<PaketStand> Pakete = new List<PaketStand>();
    }

    public enum Zustand { Aktuell, Fehlt, Veraltet, Beschaedigt }

    /// <summary>Ergebnis des Abgleichs fuer eine Komponente.</summary>
    public class Pruefung
    {
        public Komponente Komponente;
        public Zustand Zustand;
        public string Grund = "";              // z. B. die erste fehlende Datei
        public KomponenteStand Gesichert;      // bei Aktuell: der (ggf. aufgefrischte) Eintrag fuer die Sicherung
    }

    /// <summary>
    /// Sicherung des letzten geprueften Stands im GTA-Verzeichnis: das Manifest, wie es geladen wurde, und je Paket die
    /// Dateien mit Groesse, Aenderungszeit und SHA-256. Damit erkennt der Launcher bei jedem Start Fehlendes und Veraltetes
    /// und kann ohne Manifest-Server mit dem zuletzt geprueften Stand starten.
    /// </summary>
    public class Stand
    {
        public const string Ordner = "samplife-launcher";
        public const string ManifestDatei = "manifest.json";
        public const string StandDatei = "stand.json";
        public const int Format = 1;

        public DateTime Geprueft;
        public long ManifestStand;
        public string ManifestSha256 = "";
        public string ManifestJson = "";
        public List<KomponenteStand> Komponenten = new List<KomponenteStand>();

        public KomponenteStand Suchen(string id)
        {
            foreach (var komponente in Komponenten)
            {
                if (string.Equals(komponente.Id, id, StringComparison.OrdinalIgnoreCase)) return komponente;
            }
            return null;
        }

        // ---------------------------------------------------------------- Lesen und Schreiben

        /// <summary>
        /// Liest die Sicherung. Passt das gespeicherte Manifest nicht zu seiner Pruefsumme, gilt die vorige Fassung (.alt).
        /// Rueckgabe null, wenn keine brauchbare Sicherung da ist.
        /// </summary>
        public static Stand Lesen(string gta)
        {
            var ordner = Path.Combine(gta, Ordner);
            var stand = LesenAus(Path.Combine(ordner, StandDatei), Path.Combine(ordner, ManifestDatei));
            if (stand == null) stand = LesenAus(Path.Combine(ordner, StandDatei + ".alt"), Path.Combine(ordner, ManifestDatei + ".alt"));
            return stand;
        }

        static Stand LesenAus(string standDatei, string manifestDatei)
        {
            try
            {
                if (!File.Exists(standDatei) || !File.Exists(manifestDatei)) return null;
                var leser = new JavaScriptSerializer();
                leser.MaxJsonLength = int.MaxValue;
                var daten = leser.DeserializeObject(File.ReadAllText(standDatei, Encoding.UTF8)) as Dictionary<string, object>;
                if (daten == null || Zahl(daten, "format") != Format) return null;
                var manifestBytes = File.ReadAllBytes(manifestDatei);
                var stand = new Stand();
                stand.ManifestSha256 = Text(daten, "manifest_sha256");
                if (stand.ManifestSha256 == "" || stand.ManifestSha256 != Sha256(manifestBytes)) return null;
                stand.ManifestJson = new UTF8Encoding(false).GetString(manifestBytes);
                stand.ManifestStand = Zahl(daten, "stand");
                stand.Geprueft = DateTime.Parse(Text(daten, "geprueft"), CultureInfo.InvariantCulture, DateTimeStyles.RoundtripKind);
                foreach (var k in Liste(daten, "komponenten"))
                {
                    var komponente = new KomponenteStand { Id = Text(k, "id") };
                    foreach (var p in Liste(k, "pakete"))
                    {
                        var paket = new PaketStand { Url = Text(p, "url"), Sha256 = Text(p, "sha256") };
                        foreach (var d in Liste(p, "dateien"))
                        {
                            paket.Dateien.Add(new DateiStand
                            {
                                Pfad = Text(d, "pfad"), Groesse = Zahl(d, "groesse"),
                                Geaendert = long.Parse(Text(d, "geaendert"), CultureInfo.InvariantCulture), Sha256 = Text(d, "sha256")
                            });
                        }
                        komponente.Pakete.Add(paket);
                    }
                    stand.Komponenten.Add(komponente);
                }
                return stand;
            }
            catch (Exception) { return null; }
        }

        /// <summary>
        /// Schreibt Manifest und Stand erst in temporaere Dateien und ersetzt dann die alten. Die vorige Fassung bleibt als
        /// .alt liegen, so hinterlaesst ein Absturz nie eine halbe Sicherung.
        /// </summary>
        public void Schreiben(string gta)
        {
            var ordner = Path.Combine(gta, Ordner);
            Directory.CreateDirectory(ordner);
            var manifestBytes = new UTF8Encoding(false).GetBytes(ManifestJson);
            ManifestSha256 = Sha256(manifestBytes);

            var komponenten = new List<object>();
            foreach (var komponente in Komponenten)
            {
                var pakete = new List<object>();
                foreach (var paket in komponente.Pakete)
                {
                    var dateien = new List<object>();
                    foreach (var datei in paket.Dateien)
                    {
                        dateien.Add(new Dictionary<string, object>
                        {
                            { "pfad", datei.Pfad }, { "groesse", datei.Groesse },
                            { "geaendert", datei.Geaendert.ToString(CultureInfo.InvariantCulture) }, { "sha256", datei.Sha256 }
                        });
                    }
                    pakete.Add(new Dictionary<string, object> { { "url", paket.Url }, { "sha256", paket.Sha256 }, { "dateien", dateien } });
                }
                komponenten.Add(new Dictionary<string, object> { { "id", komponente.Id }, { "pakete", pakete } });
            }
            var daten = new Dictionary<string, object>
            {
                { "format", Format },
                { "geprueft", Geprueft.ToUniversalTime().ToString("o", CultureInfo.InvariantCulture) },
                { "stand", ManifestStand },
                { "manifest_sha256", ManifestSha256 },
                { "komponenten", komponenten }
            };
            var serialisierer = new JavaScriptSerializer();
            serialisierer.MaxJsonLength = int.MaxValue;
            var json = serialisierer.Serialize(daten);

            // Erst das Manifest, dann den Stand: Der Stand verweist per Pruefsumme auf das Manifest
            Ersetzen(Path.Combine(ordner, ManifestDatei), manifestBytes);
            Ersetzen(Path.Combine(ordner, StandDatei), new UTF8Encoding(false).GetBytes(json));
        }

        static void Ersetzen(string ziel, byte[] inhalt)
        {
            var neu = ziel + ".neu";
            File.WriteAllBytes(neu, inhalt);
            using (var strom = new FileStream(neu, FileMode.Open, FileAccess.ReadWrite)) strom.Flush(true);
            if (File.Exists(ziel)) File.Replace(neu, ziel, ziel + ".alt", true);
            else File.Move(neu, ziel);
        }

        // ---------------------------------------------------------------- Abgleich

        /// <summary>
        /// Vergleicht jede verwaltete Komponente mit der Sicherung und den Dateien auf der Platte. Schnell ueber Groesse und
        /// Aenderungszeit; weicht eins ab (oder gruendlich = true), wird die SHA-256 der Datei nachgerechnet.
        /// </summary>
        public static List<Pruefung> Abgleichen(string gta, Stand stand, IList<Komponente> verwaltet, string sampVersion,
            bool gruendlich, CancellationToken abbruch)
        {
            var ergebnis = new List<Pruefung>();
            foreach (var komponente in verwaltet)
            {
                abbruch.ThrowIfCancellationRequested();
                var pruefung = new Pruefung { Komponente = komponente };
                ergebnis.Add(pruefung);
                var gesichert = stand == null ? null : stand.Suchen(komponente.Id);
                if (gesichert == null) { pruefung.Zustand = Zustand.Fehlt; continue; }

                // Veraltet, wenn die Pakete laut Liste andere sind als die installierten
                var soll = new List<string>();
                foreach (var paket in Manifest.PassendePakete(komponente, sampVersion)) soll.Add(paket.Sha256);
                var ist = new List<string>();
                foreach (var paket in gesichert.Pakete) ist.Add(paket.Sha256);
                soll.Sort(StringComparer.Ordinal);
                ist.Sort(StringComparer.Ordinal);
                if (string.Join(",", soll.ToArray()) != string.Join(",", ist.ToArray()))
                {
                    pruefung.Zustand = Zustand.Veraltet;
                    continue;
                }

                var aufgefrischt = new KomponenteStand { Id = gesichert.Id };
                pruefung.Zustand = Zustand.Aktuell;
                foreach (var paket in gesichert.Pakete)
                {
                    var neuesPaket = new PaketStand { Url = paket.Url, Sha256 = paket.Sha256 };
                    foreach (var datei in paket.Dateien)
                    {
                        abbruch.ThrowIfCancellationRequested();
                        var geprueft = DateiPruefen(gta, datei, gruendlich);
                        if (geprueft == null)
                        {
                            pruefung.Zustand = Zustand.Beschaedigt;
                            pruefung.Grund = datei.Pfad;
                            break;
                        }
                        neuesPaket.Dateien.Add(geprueft);
                    }
                    if (pruefung.Zustand != Zustand.Aktuell) break;
                    aufgefrischt.Pakete.Add(neuesPaket);
                }
                if (pruefung.Zustand == Zustand.Aktuell) pruefung.Gesichert = aufgefrischt;
            }
            return ergebnis;
        }

        /// <summary>Rueckgabe: der Eintrag mit aktueller Aenderungszeit, oder null, wenn die Datei fehlt oder anders ist.</summary>
        static DateiStand DateiPruefen(string gta, DateiStand datei, bool gruendlich)
        {
            try
            {
                var pfad = Path.GetFullPath(Path.Combine(gta, datei.Pfad));
                if (!pfad.StartsWith(Path.GetFullPath(gta).TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
                    return null;
                var info = new FileInfo(pfad);
                if (!info.Exists || info.Length != datei.Groesse) return null;
                var geaendert = info.LastWriteTimeUtc.Ticks;
                if (!gruendlich && geaendert == datei.Geaendert) return datei;
                if (GtaVerzeichnis.Sha256(pfad) != datei.Sha256) return null;
                return new DateiStand { Pfad = datei.Pfad, Groesse = datei.Groesse, Geaendert = geaendert, Sha256 = datei.Sha256 };
            }
            catch (Exception) { return null; }
        }

        // ---------------------------------------------------------------- Hilfen

        public static string Sha256(byte[] daten)
        {
            using (var sha = SHA256.Create()) return BitConverter.ToString(sha.ComputeHash(daten)).Replace("-", "").ToLowerInvariant();
        }

        static string Text(Dictionary<string, object> daten, string name)
        {
            object wert;
            if (!daten.TryGetValue(name, out wert) || wert == null) return "";
            return Convert.ToString(wert, CultureInfo.InvariantCulture);
        }

        static long Zahl(Dictionary<string, object> daten, string name)
        {
            object wert;
            if (!daten.TryGetValue(name, out wert) || wert == null) return 0;
            return Convert.ToInt64(wert, CultureInfo.InvariantCulture);
        }

        static List<Dictionary<string, object>> Liste(Dictionary<string, object> daten, string name)
        {
            var ergebnis = new List<Dictionary<string, object>>();
            object wert;
            if (!daten.TryGetValue(name, out wert) || wert == null) return ergebnis;
            var eintraege = wert as IEnumerable;
            if (eintraege == null) return ergebnis;
            foreach (var eintrag in eintraege)
            {
                var objekt = eintrag as Dictionary<string, object>;
                if (objekt != null) ergebnis.Add(objekt);
            }
            return ergebnis;
        }
    }
}
