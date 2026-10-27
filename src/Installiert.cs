using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using System.Text.RegularExpressions;
using System.Web.Script.Serialization;

namespace SamplifeEssentials
{
    /// <summary>Eine Datei im GTA-Verzeichnis, die der Launcher angelegt, ersetzt oder unveraendert vorgefunden hat.</summary>
    public class InstallierteDatei
    {
        public string Pfad;            // relativ zum GTA-Verzeichnis
        public string Vorher;          // "neu", "gesichert" oder "vorhanden" (lag schon genau so da)
        public string Sicherung = "";  // bei "gesichert": die Kopie der Fassung davor, relativ zum GTA-Verzeichnis
    }

    /// <summary>Eine Datei des Spielers, die der Launcher auf Wunsch ausgeschaltet (umbenannt) hat.</summary>
    public class AbgeschalteteDatei
    {
        public string Pfad;            // Name vorher, relativ zum GTA-Verzeichnis
        public string Ablage;          // Name jetzt
    }

    /// <summary>
    /// Was der Launcher im GTA-Verzeichnis angelegt hat und wie es dort vorher aussah, fuer die Deinstallation:
    /// samplife-launcher\installiert.json. Je Datei zaehlt nur die erste Beruehrung; spaetere Aktualisierungen derselben Datei
    /// aendern den Eintrag nicht, sonst wuerde die Deinstallation eine alte eigene Fassung statt der des Spielers zurueckholen.
    /// </summary>
    public class Installiert
    {
        public const string Datei = "installiert.json";
        public const string Neu = "neu";
        public const string Gesichert = "gesichert";
        public const string Vorhanden = "vorhanden";
        const int Format = 1;

        public readonly List<InstallierteDatei> Dateien = new List<InstallierteDatei>();
        public readonly List<AbgeschalteteDatei> Abgeschaltet = new List<AbgeschalteteDatei>();
        /// <summary>Aus Stand, Protokoll und Sicherungen rekonstruiert (Launcher bis 2.0 schrieb keine Liste).</summary>
        public bool Geschaetzt;

        public InstallierteDatei Suchen(string pfad)
        {
            foreach (var datei in Dateien)
            {
                if (string.Equals(datei.Pfad, pfad, StringComparison.OrdinalIgnoreCase)) return datei;
            }
            return null;
        }

        /// <summary>Haelt die erste Beruehrung einer Datei fest; ist sie schon bekannt, bleibt der Eintrag, wie er ist.</summary>
        public void Merken(string pfad, string vorher, string sicherung)
        {
            if (Suchen(pfad) != null) return;
            Dateien.Add(new InstallierteDatei { Pfad = pfad, Vorher = vorher, Sicherung = sicherung ?? "" });
        }

        /// <summary>
        /// Eine Datei des Spielers wurde gesichert und ersetzt (andere SA-MP-Version, Ablauf.SampErsetzen). Stand sie schon als
        /// „neu“ oder „vorhanden“ in der Liste, gehoert die jetzt gesicherte Fassung dem Spieler: Sie kommt bei der
        /// Deinstallation zurueck. Eine aeltere Sicherung derselben Datei bleibt, denn sie ist die erste Fassung des Spielers.
        /// </summary>
        public void Sichern(string pfad, string sicherung)
        {
            var datei = Suchen(pfad);
            if (datei == null) { Merken(pfad, Gesichert, sicherung); return; }
            if (datei.Vorher == Gesichert) return;
            datei.Vorher = Gesichert;
            datei.Sicherung = sicherung ?? "";
        }

        // ---------------------------------------------------------------- Lesen und Schreiben

        /// <summary>Liest die Liste. Fehlt sie, wird sie aus dem, was aeltere Fassungen hinterlassen haben, rekonstruiert.</summary>
        public static Installiert Laden(string gta)
        {
            var datei = Path.Combine(gta, Stand.Ordner, Datei);
            try
            {
                if (File.Exists(datei))
                {
                    var leser = new JavaScriptSerializer();
                    leser.MaxJsonLength = int.MaxValue;
                    var daten = leser.DeserializeObject(File.ReadAllText(datei, Encoding.UTF8)) as Dictionary<string, object>;
                    if (daten != null)
                    {
                        var liste = new Installiert();
                        liste.Geschaetzt = Wahr(daten, "geschaetzt");
                        foreach (var d in Liste(daten, "dateien"))
                            liste.Dateien.Add(new InstallierteDatei { Pfad = Text(d, "pfad"), Vorher = Text(d, "vorher"), Sicherung = Text(d, "sicherung") });
                        foreach (var a in Liste(daten, "abgeschaltet"))
                            liste.Abgeschaltet.Add(new AbgeschalteteDatei { Pfad = Text(a, "pfad"), Ablage = Text(a, "ablage") });
                        return liste;
                    }
                }
            }
            catch (Exception) { }
            return Rekonstruieren(gta);
        }

        public void Speichern(string gta)
        {
            var ordner = Path.Combine(gta, Stand.Ordner);
            Directory.CreateDirectory(ordner);
            var dateien = new List<object>();
            foreach (var datei in Dateien)
            {
                var eintrag = new Dictionary<string, object> { { "pfad", datei.Pfad }, { "vorher", datei.Vorher } };
                if (datei.Sicherung != "") eintrag["sicherung"] = datei.Sicherung;
                dateien.Add(eintrag);
            }
            var abgeschaltet = new List<object>();
            foreach (var a in Abgeschaltet) abgeschaltet.Add(new Dictionary<string, object> { { "pfad", a.Pfad }, { "ablage", a.Ablage } });
            var daten = new Dictionary<string, object>
            {
                { "format", Format }, { "geschaetzt", Geschaetzt }, { "dateien", dateien }, { "abgeschaltet", abgeschaltet }
            };
            var schreiber = new JavaScriptSerializer();
            schreiber.MaxJsonLength = int.MaxValue;
            var ziel = Path.Combine(ordner, Datei);
            var neu = ziel + ".neu";
            File.WriteAllText(neu, schreiber.Serialize(daten), new UTF8Encoding(false));
            if (File.Exists(ziel)) File.Replace(neu, ziel, null, true);
            else File.Move(neu, ziel);
        }

        // ---------------------------------------------------------------- Rekonstruktion fuer Launcher bis 2.0

        static readonly Regex SitzungMuster = new Regex(@"^\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2}$");
        static readonly Regex KopfMuster = new Regex(@"^==== Samplife .* ====");
        static readonly Regex SchreibeMuster = new Regex(@"^\d{2}:\d{2}:\d{2}  Schreibe (.+)$");
        static readonly Regex SichereMuster = new Regex(@"^\d{2}:\d{2}:\d{2}  Sichere vorhandene (.+) nach (.+)$");

        /// <summary>
        /// Launcher bis 2.0 haben keine Liste geschrieben. Was sie angelegt haben, ergibt sich aus:
        /// dem Protokoll ("Sichere vorhandene X nach Y", "Schreibe X"), wenn es bis zur ersten Sicherung zurueckreicht;
        /// sonst aus dem Stand (Dateien der Pakete) und der aeltesten Sicherung: Was dort liegt, war vor dem Launcher da.
        /// </summary>
        static Installiert Rekonstruieren(string gta)
        {
            var liste = new Installiert();
            var stand = Stand.Lesen(gta);
            var standDateien = new List<string>();
            if (stand != null)
            {
                foreach (var k in stand.Komponenten)
                    foreach (var p in k.Pakete)
                        foreach (var d in p.Dateien) standDateien.Add(d.Pfad);
            }

            // Sitzungen der Sicherung, aelteste zuerst
            var sitzungen = new List<string>();
            var sicherungsOrdner = Path.Combine(gta, Installation.SicherungsOrdner);
            try
            {
                if (Directory.Exists(sicherungsOrdner))
                {
                    foreach (var ordner in Directory.GetDirectories(sicherungsOrdner))
                    {
                        var name = Path.GetFileName(ordner);
                        if (SitzungMuster.IsMatch(name)) sitzungen.Add(name);
                    }
                }
            }
            catch (Exception) { }
            sitzungen.Sort(StringComparer.Ordinal);

            var protokoll = new List<string>();
            try
            {
                var datei = Path.Combine(gta, Installation.ProtokollDatei);
                if (File.Exists(datei)) protokoll.AddRange(File.ReadAllLines(datei, Encoding.UTF8));
            }
            catch (Exception) { }

            if (liste.AusProtokoll(protokoll, sitzungen))
            {
                // Dateien im Stand, die das Protokoll nie geschrieben hat, lagen schon genau so da
                foreach (var pfad in standDateien) liste.Merken(pfad, Vorhanden, "");
            }
            else
            {
                var aelteste = sitzungen.Count > 0 ? Path.Combine(Installation.SicherungsOrdner, sitzungen[0]) : "";
                var pfade = new List<string>(standDateien);
                // Was das (gekuerzte) Protokoll noch nennt, z. B. Dateien von SA-MP, die nicht im Stand stehen
                foreach (var zeile in protokoll)
                {
                    var treffer = SchreibeMuster.Match(zeile.TrimStart('﻿'));
                    if (treffer.Success) pfade.Add(treffer.Groups[1].Value.Trim());
                }
                foreach (var pfad in pfade)
                {
                    var kopie = aelteste == "" ? "" : Path.Combine(aelteste, pfad);
                    if (kopie != "" && File.Exists(Path.Combine(gta, kopie))) liste.Merken(pfad, Gesichert, kopie);
                    else liste.Merken(pfad, Neu, "");
                }
            }
            // Ohne Spuren einer frueheren Fassung (frisches Verzeichnis) gibt es nichts zu schaetzen
            liste.Geschaetzt = liste.Dateien.Count > 0;
            return liste;
        }

        /// <summary>
        /// Wertet das Protokoll aus, wenn es vollstaendig ist: Es muss die aelteste Sicherung selbst nennen, also aus dem Lauf
        /// stammen, in dem der Launcher zum ersten Mal etwas ersetzt hat. Rueckgabe false, wenn es fehlt oder gekuerzt ist.
        /// </summary>
        bool AusProtokoll(List<string> protokoll, List<string> sitzungen)
        {
            var hatKopf = false;
            var nenntAelteste = sitzungen.Count == 0;
            var aelteste = sitzungen.Count == 0 ? "" : " nach " + Installation.SicherungsOrdner + "\\" + sitzungen[0] + "\\";
            foreach (var zeile in protokoll)
            {
                if (KopfMuster.IsMatch(zeile.TrimStart('﻿'))) hatKopf = true;
                if (!nenntAelteste && zeile.IndexOf(aelteste, StringComparison.OrdinalIgnoreCase) >= 0) nenntAelteste = true;
            }
            if (!hatKopf || !nenntAelteste) return false;

            var gesichert = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            foreach (var roh in protokoll)
            {
                var zeile = roh.TrimStart('﻿');
                var sichere = SichereMuster.Match(zeile);
                if (sichere.Success)
                {
                    gesichert[sichere.Groups[1].Value.Trim()] = sichere.Groups[2].Value.Trim();
                    continue;
                }
                var schreibe = SchreibeMuster.Match(zeile);
                if (!schreibe.Success) continue;
                var pfad = schreibe.Groups[1].Value.Trim();
                string kopie;
                // Nur die erste Beruehrung zaehlt (Merken uebergeht bekannte Pfade)
                if (gesichert.TryGetValue(pfad, out kopie)) Merken(pfad, Gesichert, kopie);
                else Merken(pfad, Neu, "");
                gesichert.Remove(pfad);
            }
            return true;
        }

        // ---------------------------------------------------------------- Hilfen

        static string Text(Dictionary<string, object> daten, string name)
        {
            object wert;
            if (!daten.TryGetValue(name, out wert) || wert == null) return "";
            return Convert.ToString(wert, CultureInfo.InvariantCulture);
        }

        static bool Wahr(Dictionary<string, object> daten, string name)
        {
            object wert;
            return daten.TryGetValue(name, out wert) && wert is bool && (bool)wert;
        }

        static List<Dictionary<string, object>> Liste(Dictionary<string, object> daten, string name)
        {
            var ergebnis = new List<Dictionary<string, object>>();
            object wert;
            if (!daten.TryGetValue(name, out wert) || wert == null) return ergebnis;
            var eintraege = wert as IEnumerable;
            if (eintraege == null || wert is string) return ergebnis;
            foreach (var eintrag in eintraege)
            {
                var objekt = eintrag as Dictionary<string, object>;
                if (objekt != null) ergebnis.Add(objekt);
            }
            return ergebnis;
        }
    }
}
