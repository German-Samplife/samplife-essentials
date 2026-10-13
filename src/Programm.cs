using System;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Text;
using System.Threading;
using System.Windows;
using System.Windows.Threading;

namespace SamplifeEssentials
{
    public class Optionen
    {
        // Liste der Komponenten: was installiert wird, woher es kommt und welche Pruefsumme es haben muss
        public string ManifestQuelle = Programm.StandardManifest;
        public string Gta = "";
        public string Samp = "";     // SA-MP-Version vorgeben, falls sie nicht erkannt wird
        public bool Testmodus;      // erlaubt eine lokale Liste und unverschluesselte Downloads vom eigenen Rechner
        public bool Still;          // ohne Fenster installieren (fuer automatische Tests)
        public bool OhneNetz;       // Fenster ohne Laden der Liste (fuer Bildschirmfotos)
        public string Bildschirmfoto = "";
        public string Zustand = "start";
        public string Suchergebnis = "";    // nur suchen und die Funde in diese Datei schreiben

        public string AlsArgumente()
        {
            var text = "";
            if (ManifestQuelle != Programm.StandardManifest) text += " --manifest \"" + ManifestQuelle + "\"";
            if (Testmodus) text += " --test";
            return text;
        }
    }

    public static class Programm
    {
        public const string Version = "1.3.0";
        public const string StandardManifest = "https://samplife.isjustfucking.pro/essentials/manifest.json";

        [STAThread]
        public static int Main(string[] args)
        {
            var optionen = new Optionen();
            for (var i = 0; i < args.Length; i++)
            {
                var naechstes = i + 1 < args.Length ? args[i + 1] : "";
                switch (args[i])
                {
                    case "--manifest": optionen.ManifestQuelle = naechstes; i++; break;
                    case "--gta": optionen.Gta = naechstes; i++; break;
                    case "--samp": optionen.Samp = naechstes; i++; break;
                    case "--test": optionen.Testmodus = true; break;
                    case "--still": optionen.Still = true; break;
                    case "--bildschirmfoto": optionen.Bildschirmfoto = naechstes; i++; break;
                    case "--zustand": optionen.Zustand = naechstes; i++; break;
                    case "--suche": optionen.Suchergebnis = naechstes; i++; break;
                }
            }

            if (optionen.Suchergebnis != "")
            {
                var zeilen = new List<string>();
                foreach (var fund in GtaVerzeichnis.Suchen(CancellationToken.None)) zeilen.Add(fund.Pfad + " | " + fund.Detail);
                File.WriteAllLines(optionen.Suchergebnis, zeilen, Encoding.UTF8);
                return 0;
            }
            if (optionen.Still) return StillInstallieren(optionen);

            var anwendung = new Application();
            anwendung.ShutdownMode = ShutdownMode.OnMainWindowClose;
            anwendung.DispatcherUnhandledException += delegate(object s, DispatcherUnhandledExceptionEventArgs e)
            {
                MessageBox.Show("Unerwarteter Fehler: " + e.Exception.Message, "Samplife Essentials Installer", MessageBoxButton.OK, MessageBoxImage.Error);
                e.Handled = true;
            };

            if (optionen.Bildschirmfoto != "") return Bildschirmfoto(anwendung, optionen);

            var fenster = new Fenster(optionen);
            anwendung.Run(fenster.Wurzel);
            return 0;
        }

        /// <summary>Holt die Liste der Komponenten. Aus dem Netz nur verschluesselt; eine lokale Datei nur im Testmodus.</summary>
        public static string ManifestHolen(string quelle, bool testmodus)
        {
            Uri adresse;
            var istAdresse = Uri.TryCreate(quelle, UriKind.Absolute, out adresse) && (adresse.Scheme == Uri.UriSchemeHttps || adresse.Scheme == Uri.UriSchemeHttp);
            if (!istAdresse)
            {
                if (!testmodus) throw new InvalidOperationException("Eine lokale Liste ist nur im Testmodus erlaubt.");
                return File.ReadAllText(quelle, Encoding.UTF8);
            }
            if (adresse.Scheme != Uri.UriSchemeHttps && !(testmodus && adresse.IsLoopback))
                throw new InvalidOperationException("Die Liste muss über eine verschlüsselte Verbindung (https) geladen werden.");

            ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
            var anfrage = (HttpWebRequest)WebRequest.Create(adresse);
            anfrage.UserAgent = "SamplifeEssentialsInstaller/" + Version;
            anfrage.Timeout = 20000;
            using (var antwort = (HttpWebResponse)anfrage.GetResponse())
            {
                if (antwort.ResponseUri.Scheme != Uri.UriSchemeHttps && !(testmodus && antwort.ResponseUri.IsLoopback))
                    throw new InvalidOperationException("Die Liste wurde auf eine unverschlüsselte Adresse umgeleitet.");
                using (var leser = new StreamReader(antwort.GetResponseStream(), Encoding.UTF8))
                {
                    return leser.ReadToEnd();
                }
            }
        }

        /// <summary>Installiert alle verfuegbaren Komponenten ohne Fenster. Rueckgabe 0 bei Erfolg.</summary>
        static int StillInstallieren(Optionen optionen)
        {
            try
            {
                var manifest = Manifest.Lesen(ManifestHolen(optionen.ManifestQuelle, optionen.Testmodus));
                var gta = optionen.Gta != "" ? optionen.Gta : GtaVerzeichnis.Finden();
                if (!GtaVerzeichnis.IstGtaVerzeichnis(gta)) return 2;
                var sampVersion = GtaVerzeichnis.SampVersion(gta, manifest);
                if (sampVersion == "") sampVersion = optionen.Samp;
                if (sampVersion == "" && manifest.SampVersionen.Count == 1) sampVersion = manifest.SampVersionen[0].Name;
                var auswahl = new List<Komponente>();
                foreach (var komponente in manifest.Komponenten)
                {
                    if (Manifest.PassendePakete(komponente, sampVersion).Count > 0) auswahl.Add(komponente);
                }
                var ergebnis = new Installation(gta, sampVersion, auswahl, optionen.Testmodus).Ausfuehren(CancellationToken.None);
                return ergebnis.Erfolg ? 0 : 1;
            }
            catch (Exception)
            {
                return 3;
            }
        }

        static int Bildschirmfoto(Application anwendung, Optionen optionen)
        {
            // Zustand "netz" laedt die Liste auf dem normalen Weg und haelt fest, was das Fenster danach zeigt
            // "ablauf" startet wie beim Spieler (Suche und Liste), haelt den Stand fest, bestaetigt den ersten Fund und haelt ihn noch einmal fest
            var ablauf = optionen.Zustand == "ablauf";
            var mitNetz = optionen.Zustand == "netz" || ablauf;
            optionen.OhneNetz = !mitNetz;
            var fenster = new Fenster(optionen);
            if (!mitNetz && optionen.ManifestQuelle != StandardManifest)
                fenster.ManifestSetzen(Manifest.Lesen(ManifestHolen(optionen.ManifestQuelle, true)));
            // ausserhalb des sichtbaren Bildschirms, damit beim Erzeugen nichts aufblitzt
            fenster.Wurzel.WindowStartupLocation = WindowStartupLocation.Manual;
            fenster.Wurzel.Left = -20000;
            fenster.Wurzel.Top = -20000;
            fenster.Wurzel.ShowInTaskbar = false;
            fenster.Wurzel.ContentRendered += delegate
            {
                if (!mitNetz) fenster.ZustandVorfuehren(optionen.Zustand);
                var warten = new DispatcherTimer();
                warten.Interval = TimeSpan.FromSeconds(mitNetz ? 8 : 0.2);
                var schritt = 0;
                warten.Tick += delegate
                {
                    if (ablauf && schritt == 0)
                    {
                        schritt = 1;
                        fenster.Speichern(optionen.Bildschirmfoto.Replace(".png", "-vorher.png"));
                        fenster.ErstenFundBestaetigen();
                        warten.Interval = TimeSpan.FromSeconds(2);
                        return;
                    }
                    if (ablauf && schritt == 1)
                    {
                        schritt = 2;
                        fenster.Speichern(optionen.Bildschirmfoto.Replace(".png", "-bestaetigt.png"));
                        fenster.VersionWaehlen();
                        warten.Interval = TimeSpan.FromSeconds(1);
                        return;
                    }
                    warten.Stop();
                    fenster.Speichern(optionen.Bildschirmfoto);
                    anwendung.Shutdown();
                };
                warten.Start();
            };
            anwendung.Run(fenster.Wurzel);
            return 0;
        }
    }
}
