using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Net;
using System.Reflection;
using System.Security.Cryptography;
using System.Threading;
using System.Web.Script.Serialization;

namespace SamplifeEssentials
{
    /// <summary>
    /// Selbstaktualisierung: Bei jedem Start fragt der Launcher GitHub nach dem neuesten Release und liest die Liste. Nennt
    /// eines von beiden eine hoehere Launcher-Version, wird die neue .exe geladen, geprueft und an die Stelle der laufenden
    /// gesetzt. Die laufende Datei wird dafuer in .alt umbenannt (das erlaubt Windows auch bei einem laufenden Programm) und
    /// beim naechsten Start entfernt.
    /// </summary>
    public static class Aktualisierung
    {
        /// <summary>Neuestes Release auf GitHub. Nur Launcher-Releases sind dort „Latest“, alle anderen Pakete nicht.</summary>
        public const string StandardReleaseQuelle = "https://api.github.com/repos/German-Samplife/samplife-essentials/releases/latest";
        const string ReleaseDatei = "Samplife.Launcher.exe";
        const string ReleaseSummen = "SHA256SUMS.txt";
        // Die Abfrage darf den Start nicht lange aufhalten; antwortet GitHub nicht, reicht die Liste
        const int ReleaseWartezeit = 8000;

        public static string EigeneDatei
        {
            get { return Assembly.GetExecutingAssembly().Location; }
        }

        /// <summary>
        /// Die Fassung, auf die aktualisiert wird: die hoehere aus Liste und Release, wenn sie neuer ist als die laufende, sonst
        /// null. Bei gleicher Version gilt die Liste. Nach einer Aktualisierung (nachAktualisierung = die Version, auf die
        /// aktualisiert wurde) nie ein zweites Mal, damit eine falsch beschriftete Datei keinen endlosen Kreislauf ausloest.
        /// </summary>
        public static LauncherFassung Neuere(LauncherFassung ausListe, LauncherFassung ausRelease, string nachAktualisierung)
        {
            Version eigen;
            if (nachAktualisierung != "" || !Version.TryParse(Programm.Version, out eigen)) return null;
            LauncherFassung beste = null;
            Version besteVersion = eigen;
            foreach (var fassung in new[] { ausListe, ausRelease })
            {
                Version version;
                if (fassung == null || !Version.TryParse(fassung.Version, out version)) continue;
                if (version > besteVersion)
                {
                    beste = fassung;
                    besteVersion = version;
                }
            }
            return beste;
        }

        /// <summary>
        /// Fragt GitHub nach dem neuesten Release des Launchers. Null, wenn die Quelle leer ist, GitHub nicht antwortet oder das
        /// Release nicht passt: Die Abfrage ergaenzt die Liste nur und haelt den Start nie mit einem Fehler auf.
        /// </summary>
        public static LauncherFassung NeuesteHolen(string quelle, bool testmodus)
        {
            if (string.IsNullOrEmpty(quelle)) return null;
            try
            {
                string summenAdresse;
                var fassung = ReleaseLesen(Programm.ManifestHolen(quelle, testmodus, ReleaseWartezeit), out summenAdresse);
                if (fassung == null) return null;
                // Aeltere Releases ohne Pruefsumme in der Antwort: die Datei SHA256SUMS.txt aus dem Release lesen
                if (fassung.Sha256 == "" && summenAdresse != "")
                    fassung.Sha256 = AusSummen(Programm.ManifestHolen(summenAdresse, testmodus, ReleaseWartezeit), ReleaseDatei);
                return fassung.Sha256.Length == 64 ? fassung : null;
            }
            catch (Exception)
            {
                return null;
            }
        }

        /// <summary>
        /// Liest die Antwort von GitHub (ein Release). Nur ein veroeffentlichtes Release mit Tag v&lt;Version&gt; und der Datei
        /// Samplife.Launcher.exe zaehlt; Pakete wie Kleidung oder samp-cef haben eigene Tags.
        /// </summary>
        static LauncherFassung ReleaseLesen(string json, out string summenAdresse)
        {
            summenAdresse = "";
            var release = new JavaScriptSerializer().DeserializeObject(json) as Dictionary<string, object>;
            if (release == null || Wahr(release, "draft") || Wahr(release, "prerelease")) return null;
            var tag = Text(release, "tag_name");
            Version version;
            if (!tag.StartsWith("v", StringComparison.Ordinal) || !Version.TryParse(tag.Substring(1), out version)) return null;
            object dateien;
            if (!release.TryGetValue("assets", out dateien) || !(dateien is object[])) return null;
            LauncherFassung fassung = null;
            foreach (var eintrag in (object[])dateien)
            {
                var datei = eintrag as Dictionary<string, object>;
                if (datei == null) continue;
                var name = Text(datei, "name");
                if (name == ReleaseSummen) summenAdresse = Text(datei, "browser_download_url");
                if (name != ReleaseDatei) continue;
                fassung = new LauncherFassung { Version = tag.Substring(1), Url = Text(datei, "browser_download_url"), Sha256 = "" };
                object groesse;
                if (datei.TryGetValue("size", out groesse) && groesse != null) fassung.Groesse = Convert.ToInt64(groesse);
                // GitHub nennt die Pruefsumme jeder Datei als "sha256:…"
                var digest = Text(datei, "digest").ToLowerInvariant();
                if (digest.StartsWith("sha256:", StringComparison.Ordinal)) fassung.Sha256 = digest.Substring(7);
            }
            if (fassung == null || fassung.Url == "") return null;
            return fassung;
        }

        /// <summary>Pruefsumme einer Datei aus SHA256SUMS.txt (Zeilen „&lt;sha256&gt;  &lt;Datei&gt;“), leer, wenn sie fehlt.</summary>
        static string AusSummen(string text, string dateiname)
        {
            foreach (var zeile in text.Split('\n'))
            {
                var teile = zeile.Trim().Split(new[] { ' ', '\t' }, StringSplitOptions.RemoveEmptyEntries);
                if (teile.Length == 2 && teile[1].TrimStart('*') == dateiname && teile[0].Length == 64) return teile[0].ToLowerInvariant();
            }
            return "";
        }

        static string Text(Dictionary<string, object> daten, string schluessel)
        {
            object wert;
            return daten.TryGetValue(schluessel, out wert) && wert is string ? (string)wert : "";
        }

        static bool Wahr(Dictionary<string, object> daten, string schluessel)
        {
            object wert;
            return daten.TryGetValue(schluessel, out wert) && wert is bool && (bool)wert;
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
