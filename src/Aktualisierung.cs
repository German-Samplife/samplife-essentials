using System;
using System.Diagnostics;
using System.IO;
using System.Net;
using System.Reflection;
using System.Security.Cryptography;
using System.Threading;

namespace SamplifeEssentials
{
    /// <summary>
    /// Selbstaktualisierung: Nennt die Liste eine hoehere Launcher-Version, wird die neue .exe geladen, geprueft und an die
    /// Stelle der laufenden gesetzt. Die laufende Datei wird dafuer in .alt umbenannt (das erlaubt Windows auch bei einem
    /// laufenden Programm) und beim naechsten Start entfernt.
    /// </summary>
    public static class Aktualisierung
    {
        public static string EigeneDatei
        {
            get { return Assembly.GetExecutingAssembly().Location; }
        }

        /// <summary>
        /// Braucht es eine neue Fassung? Nach einer Aktualisierung (nachAktualisierung = die Version, auf die aktualisiert
        /// wurde) nie ein zweites Mal, damit eine falsch beschriftete Datei keinen endlosen Kreislauf ausloest.
        /// </summary>
        public static bool Noetig(Manifest manifest, string nachAktualisierung)
        {
            if (manifest == null || manifest.Launcher == null || nachAktualisierung != "") return false;
            Version neu, eigen;
            if (!Version.TryParse(manifest.Launcher.Version, out neu) || !Version.TryParse(Programm.Version, out eigen)) return false;
            return neu > eigen;
        }

        /// <summary>Laedt die neue Fassung, prueft sie und ersetzt die laufende Datei. Wirft bei jedem Fehler; dann bleibt die alte Datei.</summary>
        public static void Ausfuehren(LauncherFassung fassung, bool testmodus, Action<long, long> fortschritt, CancellationToken abbruch)
        {
            Uri adresse;
            if (!Uri.TryCreate(fassung.Url, UriKind.Absolute, out adresse)) throw new InvalidOperationException("Ungültige Adresse: " + fassung.Url);
            Pruefen(adresse, testmodus);

            var eigen = EigeneDatei;
            var neu = eigen + ".neu";
            var alt = eigen + ".alt";
            ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
            var anfrage = (HttpWebRequest)WebRequest.Create(adresse);
            anfrage.UserAgent = "SamplifeLauncher/" + Programm.Version;
            anfrage.Timeout = 30000;
            anfrage.ReadWriteTimeout = 30000;
            string pruefsumme;
            using (var antwort = (HttpWebResponse)anfrage.GetResponse())
            {
                // Auch nach Weiterleitungen (GitHub) verschluesselt
                Pruefen(antwort.ResponseUri, testmodus);
                using (var quelle = antwort.GetResponseStream())
                using (var ausgabe = File.Create(neu))
                using (var sha = SHA256.Create())
                {
                    var puffer = new byte[81920];
                    long gelesen = 0;
                    int anzahl;
                    while ((anzahl = quelle.Read(puffer, 0, puffer.Length)) > 0)
                    {
                        abbruch.ThrowIfCancellationRequested();
                        ausgabe.Write(puffer, 0, anzahl);
                        sha.TransformBlock(puffer, 0, anzahl, null, 0);
                        gelesen += anzahl;
                        fortschritt(gelesen, fassung.Groesse > 0 ? fassung.Groesse : antwort.ContentLength);
                    }
                    sha.TransformFinalBlock(new byte[0], 0, 0);
                    pruefsumme = BitConverter.ToString(sha.Hash).Replace("-", "").ToLowerInvariant();
                }
            }
            if (pruefsumme != fassung.Sha256)
            {
                TryDelete(neu);
                throw new InvalidOperationException("Die neue Fassung des Launchers stimmt nicht mit der erwarteten Prüfsumme überein.");
            }
            TryDelete(alt);
            File.Move(eigen, alt);
            try { File.Move(neu, eigen); }
            catch (Exception)
            {
                // Zurueck auf die alte Fassung, damit der Spieler nicht ohne Launcher dasteht
                File.Move(alt, eigen);
                throw;
            }
        }

        /// <summary>Startet die neue Fassung mit denselben Parametern.</summary>
        public static void NeuStarten(string argumente, string version)
        {
            var start = new ProcessStartInfo(EigeneDatei, (argumente + " --nach-aktualisierung " + version).Trim());
            start.UseShellExecute = false;
            Process.Start(start);
        }

        /// <summary>Entfernt die Datei der vorigen Fassung. Die laeuft vielleicht noch kurz, deshalb einige Versuche.</summary>
        public static void AlteEntfernen()
        {
            var alt = EigeneDatei + ".alt";
            if (!File.Exists(alt)) return;
            ThreadPool.QueueUserWorkItem(delegate
            {
                for (var versuch = 0; versuch < 20 && File.Exists(alt); versuch++)
                {
                    TryDelete(alt);
                    if (File.Exists(alt)) Thread.Sleep(500);
                }
            });
        }

        static void Pruefen(Uri adresse, bool testmodus)
        {
            if (adresse.Scheme == Uri.UriSchemeHttps) return;
            if (testmodus && adresse.Scheme == Uri.UriSchemeHttp && adresse.IsLoopback) return;
            throw new InvalidOperationException("Die neue Fassung darf nur über eine verschlüsselte Verbindung (https) geladen werden.");
        }

        static void TryDelete(string datei)
        {
            try { if (File.Exists(datei)) File.Delete(datei); }
            catch (Exception) { }
        }
    }
}
