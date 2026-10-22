using System;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Security.Cryptography;
using System.Text;

namespace SamplifeEssentials
{
    /// <summary>
    /// Bilder aus dem Spiel fuer den Kopf des Fensters. Die Adressen stehen im Manifest unter "bilder" und zeigen auf die
    /// Website; die Bilder selbst liegen nicht in diesem Repository. Einmal geladen, liegen sie unter
    /// %LOCALAPPDATA%\German Samplife\Launcher\bilder, damit der Kopf auch ohne Netz und sofort beim Start ein Bild hat.
    /// </summary>
    public static class Titelbild
    {
        const long HoechsteGroesse = 8 * 1024 * 1024;
        static readonly Random Zufall = new Random();

        static string Ordner
        {
            get
            {
                return Path.Combine(Pfade.Launcher, "bilder");
            }
        }

        /// <summary>Ein zufaelliges, schon geladenes Bild, sonst null.</summary>
        public static byte[] Gespeichertes()
        {
            try
            {
                if (!Directory.Exists(Ordner)) return null;
                var dateien = Directory.GetFiles(Ordner, "*.bild");
                if (dateien.Length == 0) return null;
                return File.ReadAllBytes(dateien[Zufall.Next(dateien.Length)]);
            }
            catch (Exception) { return null; }
        }

        /// <summary>Eine Adresse aus der Liste, bevorzugt eine, die noch nicht geladen ist.</summary>
        public static string Auswaehlen(IList<string> adressen)
        {
            if (adressen == null || adressen.Count == 0) return "";
            var fehlend = new List<string>();
            foreach (var adresse in adressen) if (!File.Exists(Datei(adresse))) fehlend.Add(adresse);
            var auswahl = fehlend.Count > 0 ? fehlend : new List<string>(adressen);
            return auswahl[Zufall.Next(auswahl.Count)];
        }

        /// <summary>Laedt ein Bild (nur https, im Testmodus auch vom eigenen Rechner) und legt es ab. Wirft bei Fehlern.</summary>
        public static byte[] Laden(string url, bool testmodus)
        {
            var gespeichert = Datei(url);
            if (File.Exists(gespeichert)) return File.ReadAllBytes(gespeichert);

            Uri adresse;
            if (!Uri.TryCreate(url, UriKind.Absolute, out adresse)) throw new InvalidOperationException("Ungültige Adresse.");
            if (adresse.Scheme != Uri.UriSchemeHttps && !(testmodus && adresse.Scheme == Uri.UriSchemeHttp && adresse.IsLoopback))
                throw new InvalidOperationException("Bilder nur über https.");
            ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
            var anfrage = (HttpWebRequest)WebRequest.Create(adresse);
            anfrage.UserAgent = "SamplifeLauncher/" + Programm.Version;
            anfrage.Timeout = 15000;
            byte[] daten;
            using (var antwort = (HttpWebResponse)anfrage.GetResponse())
            using (var quelle = antwort.GetResponseStream())
            using (var puffer = new MemoryStream())
            {
                var block = new byte[81920];
                int gelesen;
                while ((gelesen = quelle.Read(block, 0, block.Length)) > 0)
                {
                    puffer.Write(block, 0, gelesen);
                    if (puffer.Length > HoechsteGroesse) throw new InvalidOperationException("Das Bild ist zu groß.");
                }
                daten = puffer.ToArray();
            }
            try
            {
                Directory.CreateDirectory(Ordner);
                File.WriteAllBytes(gespeichert, daten);
            }
            catch (Exception) { }
            return daten;
        }

        static string Datei(string url)
        {
            using (var sha = SHA256.Create())
            {
                var name = BitConverter.ToString(sha.ComputeHash(Encoding.UTF8.GetBytes(url))).Replace("-", "").Substring(0, 24).ToLowerInvariant();
                return Path.Combine(Ordner, name + ".bild");
            }
        }
    }
}
