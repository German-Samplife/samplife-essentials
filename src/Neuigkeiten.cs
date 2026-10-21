using System;
using System.Collections;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Net;
using System.Text;
using System.Web.Script.Serialization;

namespace SamplifeEssentials
{
    /// <summary>Ein Update aus der Community (/community/updates), wie die Website es unter /api/updates liefert.</summary>
    public class Neuigkeit
    {
        public string Titel { get; set; }
        public string Kurz { get; set; }
        public string Url { get; set; }
        public DateTime Datum;
        public string DatumText { get { return Datum == DateTime.MinValue ? "" : Datum.ToString("dd.MM.yyyy", CultureInfo.InvariantCulture); } }
    }

    public static class Neuigkeiten
    {
        /// <summary>Laedt die neuesten Updates. Fehler werden geworfen; die Oberflaeche zeigt dann einen Satz statt der Liste.</summary>
        public static List<Neuigkeit> Laden(string url, bool testmodus, int hoechstens)
        {
            Uri adresse;
            if (!Uri.TryCreate(url, UriKind.Absolute, out adresse)) throw new InvalidOperationException("Ungültige Adresse.");
            if (adresse.Scheme != Uri.UriSchemeHttps && !(testmodus && adresse.Scheme == Uri.UriSchemeHttp && adresse.IsLoopback))
                throw new InvalidOperationException("Updates nur über https.");
            ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
            var anfrage = (HttpWebRequest)WebRequest.Create(adresse);
            anfrage.UserAgent = "SamplifeLauncher/" + Programm.Version;
            anfrage.Timeout = 10000;
            string text;
            using (var antwort = (HttpWebResponse)anfrage.GetResponse())
            using (var leser = new StreamReader(antwort.GetResponseStream(), Encoding.UTF8))
            {
                text = leser.ReadToEnd();
            }

            var liste = new List<Neuigkeit>();
            var daten = new JavaScriptSerializer().DeserializeObject(text) as Dictionary<string, object>;
            object wert;
            if (daten == null || !daten.TryGetValue("updates", out wert) || !(wert is IEnumerable)) return liste;
            foreach (var eintrag in (IEnumerable)wert)
            {
                var feld = eintrag as Dictionary<string, object>;
                if (feld == null) continue;
                var neuigkeit = new Neuigkeit { Titel = Text(feld, "titel"), Kurz = Text(feld, "kurz"), Url = Text(feld, "url") };
                if (neuigkeit.Titel == "") continue;
                // Nur Links auf eine Website mit https oeffnen, nichts anderes, was in der Antwort stehen koennte
                Uri ziel;
                if (!Uri.TryCreate(neuigkeit.Url, UriKind.Absolute, out ziel) || ziel.Scheme != Uri.UriSchemeHttps) neuigkeit.Url = "";
                DateTime datum;
                if (DateTime.TryParseExact(Text(feld, "datum"), "yyyy-MM-dd", CultureInfo.InvariantCulture, DateTimeStyles.None, out datum))
                    neuigkeit.Datum = datum;
                liste.Add(neuigkeit);
                if (liste.Count >= hoechstens) break;
            }
            return liste;
        }

        static string Text(Dictionary<string, object> daten, string name)
        {
            object wert;
            if (!daten.TryGetValue(name, out wert) || wert == null) return "";
            return Convert.ToString(wert, CultureInfo.InvariantCulture).Trim();
        }
    }
}
