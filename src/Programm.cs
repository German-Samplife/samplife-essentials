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
        // Wird gefragt, wenn die Liste unter ManifestQuelle mit einem HTTP-Fehler antwortet; leer = kein Ersatz
        public string AusweichQuelle = Programm.AusweichManifest;
        // Neuestes Release auf GitHub, beim Start nach einer neueren Fassung des Launchers gefragt; leer = nicht fragen
        public string ReleaseQuelle = Aktualisierung.StandardReleaseQuelle;
        public string Gta = "";
        public string Samp = "";     // SA-MP-Version vorgeben, falls sie nicht erkannt wird
        public bool Testmodus;      // erlaubt eine lokale Liste und unverschluesselte Downloads vom eigenen Rechner
        public bool Still;          // ohne Fenster abgleichen und laden (fuer automatische Tests)
        public bool Spielen;        // mit --still: danach anmelden und samp.exe starten
        public bool Gruendlich;     // mit --still: jede Datei per SHA-256 pruefen
        public bool KeinNeustart;   // mit --still: nach einer Selbstaktualisierung nicht neu starten (Rueckgabe 10)
        public string Name = "";
        public string Passwort = "";    // Serverpasswort fuer --spielen
        public string Kontopasswort = "";   // Passwort des Accounts fuer --spielen
        public List<string> Optional = new List<string>();
        public string NachAktualisierung = "";  // Version, auf die gerade aktualisiert wurde; verhindert eine zweite Runde
        public bool Direkt;         // ueber samplife://verbinden gestartet: pruefen und ohne Klick verbinden
        public bool AdresseEintragen;   // nur samplife:// eintragen und beenden (fuer Tests)
        public string Titelbild = "";   // nur fuer Bildschirmfotos: Bilddatei fuer den Kopf
        public bool OhneNetz;       // Fenster ohne Laden der Liste (fuer Bildschirmfotos)
        public string Bildschirmfoto = "";
        public string Zustand = "start";
        public string Suchergebnis = "";    // nur suchen und die Funde in diese Datei schreiben
        public bool Deinstallieren;     // mit --still: alles entfernen, was der Launcher angelegt hat
        public string Fremde = "";      // mit --still: fremde ASI Loader und Konflikte in diese Datei schreiben
        public bool Abschalten;         // mit --fremde: die gefundenen, die sich ausschalten lassen, ausschalten
        public bool Selbsttest;         // Fenster starten, 6 Sekunden laufen lassen, beenden; Rueckgabe 98/99 bei einem Absturz
        public bool SampErsetzen;       // mit --spielen: eine andere SA-MP-Version sichern und ersetzen (wie „Einrichten“ im Fenster)

        public string AlsArgumente()
        {
            var text = "";
            if (ManifestQuelle != Programm.StandardManifest) text += " --manifest \"" + ManifestQuelle + "\"";
            if (AusweichQuelle != Programm.AusweichManifest && AusweichQuelle != "") text += " --ausweich \"" + AusweichQuelle + "\"";
            if (ReleaseQuelle != Aktualisierung.StandardReleaseQuelle && ReleaseQuelle != "") text += " --release \"" + ReleaseQuelle + "\"";
            if (Testmodus) text += " --test";
            foreach (var id in Optional) text += " --optional \"" + id + "\"";
            return text;
        }
    }

    public static class Programm
    {
        public const string Version = "2.7.1";
        public const string StandardManifest = "https://www.german-samplife.de/essentials/manifest.json";
        // Fruehere Adresse der Website; dient als Ersatz, falls die neue nicht antwortet
        public const string AusweichManifest = "https://samplife.isjustfucking.pro/essentials/manifest.json";

        // Die Parameter dieses Starts, fuer den Neustart nach einer Selbstaktualisierung
        static string Argumente = "";

        [STAThread]
        public static int Main(string[] args)
        {
            var teile = new List<string>();
            foreach (var arg in args) teile.Add("\"" + arg.Replace("\"", "") + "\"");
            Argumente = string.Join(" ", teile.ToArray());
            var optionen = new Optionen();
            var ausweichGesetzt = false;
            var releaseGesetzt = false;
            var appDaten = "";
            var systemOrdner = "";
            for (var i = 0; i < args.Length; i++)
            {
                var naechstes = i + 1 < args.Length ? args[i + 1] : "";
                switch (args[i])
                {
                    case "--manifest": optionen.ManifestQuelle = naechstes; i++; break;
                    case "--ausweich": optionen.AusweichQuelle = naechstes; ausweichGesetzt = true; i++; break;
                    case "--release": optionen.ReleaseQuelle = naechstes; releaseGesetzt = true; i++; break;
                    case "--gta": optionen.Gta = naechstes; i++; break;
                    case "--samp": optionen.Samp = naechstes; i++; break;
                    case "--test": optionen.Testmodus = true; break;
                    case "--still": optionen.Still = true; break;
                    case "--spielen": optionen.Spielen = true; break;
                    case "--gruendlich": optionen.Gruendlich = true; break;
                    case "--kein-neustart": optionen.KeinNeustart = true; break;
                    case "--name": optionen.Name = naechstes; i++; break;
                    case "--passwort": optionen.Passwort = naechstes; i++; break;
                    case "--kontopasswort": optionen.Kontopasswort = naechstes; i++; break;
                    case "--optional": optionen.Optional.Add(naechstes); i++; break;
                    case "--nach-aktualisierung": optionen.NachAktualisierung = naechstes; i++; break;
                    case "--link": optionen.Direkt = Spielstart.IstVerbindenLink(naechstes); i++; break;
                    case "--adresse-eintragen": optionen.AdresseEintragen = true; break;
                    case "--titelbild": optionen.Titelbild = naechstes; i++; break;
                    case "--appdaten": appDaten = naechstes; i++; break;
                    case "--bildschirmfoto": optionen.Bildschirmfoto = naechstes; i++; break;
                    case "--zustand": optionen.Zustand = naechstes; i++; break;
                    case "--suche": optionen.Suchergebnis = naechstes; i++; break;
                    case "--deinstallieren": optionen.Deinstallieren = true; break;
                    case "--fremde": optionen.Fremde = naechstes; i++; break;
                    case "--abschalten": optionen.Abschalten = true; break;
                    case "--selbsttest": optionen.Selbsttest = true; break;
                    case "--samp-ersetzen": optionen.SampErsetzen = true; break;
                    case "--systemordner": systemOrdner = naechstes; i++; break;
                }
            }

            // Eigener Ordner statt %LOCALAPPDATA% nur fuer Tests
            if (optionen.Testmodus && appDaten != "") Pfade.Lokal = appDaten;
            // Eigener Systemordner fuer die Pruefung auf DirectX nur fuer Tests
            if (optionen.Testmodus && systemOrdner != "") Systemvoraussetzung.Ordner = systemOrdner;

            // Eine eigene Liste hat keinen Ersatz, ausser er wird ausdruecklich angegeben
            if (optionen.ManifestQuelle != StandardManifest && !ausweichGesetzt) optionen.AusweichQuelle = "";
            // Tests und eigene Listen fragen GitHub nur, wenn ein Release ausdruecklich angegeben ist: Sonst wuerde sich eine
            // Vorab-Fassung beim Start selbst durch das letzte Release ersetzen
            if ((optionen.Testmodus || optionen.ManifestQuelle != StandardManifest) && !releaseGesetzt) optionen.ReleaseQuelle = "";

            Aktualisierung.AlteEntfernen();

            if (optionen.Suchergebnis != "")
            {
                var zeilen = new List<string>();
                foreach (var fund in GtaVerzeichnis.Suchen(CancellationToken.None)) zeilen.Add(fund.Pfad + " | " + fund.Detail);
                File.WriteAllLines(optionen.Suchergebnis, zeilen, Encoding.UTF8);
                return 0;
            }
            if (optionen.AdresseEintragen)
            {
                try { Spielstart.AdresseEintragen(optionen.Testmodus); return 0; }
                catch (Exception) { return 1; }
            }
            if (optionen.Still) return StillAusfuehren(optionen);

            var anwendung = new Application();
            anwendung.ShutdownMode = ShutdownMode.OnMainWindowClose;
            anwendung.DispatcherUnhandledException += delegate(object s, DispatcherUnhandledExceptionEventArgs e)
            {
                if (optionen.Selbsttest) Environment.Exit(98);
                MessageBox.Show("Unerwarteter Fehler: " + e.Exception.Message, "Samplife Launcher", MessageBoxButton.OK, MessageBoxImage.Error);
                e.Handled = true;
            };
            // Fehler ausserhalb des Fensters (z. B. in einem Hintergrund-Thread) beenden das Programm. Dann wenigstens sagen,
            // was passiert ist, statt dass sich der Launcher wortlos schliesst.
            AppDomain.CurrentDomain.UnhandledException += delegate(object s, UnhandledExceptionEventArgs e)
            {
                if (optionen.Selbsttest) Environment.Exit(99);
                var fehler = e.ExceptionObject as Exception;
                MessageBox.Show("Der Launcher muss wegen eines unerwarteten Fehlers beendet werden:\n\n"
                    + (fehler != null ? fehler.GetType().Name + ": " + fehler.Message : "unbekannt")
                    + "\n\nBitte melde das in der Community. Die aktuelle Fassung gibt es unter "
                    + "https://github.com/German-Samplife/samplife-essentials/releases/latest",
                    "Samplife Launcher", MessageBoxButton.OK, MessageBoxImage.Error);
            };

            if (optionen.Bildschirmfoto != "") return Bildschirmfoto(anwendung, optionen);

            var fenster = new Fenster(optionen);
            if (optionen.Selbsttest)
            {
                // Fuer Tests: das echte Fenster starten, einige Sekunden laufen lassen und sauber beenden (Rueckgabe 0)
                fenster.Wurzel.ShowInTaskbar = false;
                fenster.Wurzel.WindowStartupLocation = WindowStartupLocation.Manual;
                fenster.Wurzel.Left = -20000;
                fenster.Wurzel.Top = -20000;
                fenster.Wurzel.Loaded += delegate
                {
                    var ende = new DispatcherTimer { Interval = TimeSpan.FromSeconds(6) };
                    ende.Tick += delegate { ende.Stop(); anwendung.Shutdown(0); };
                    ende.Start();
                };
            }
            anwendung.Run(fenster.Wurzel);
            return 0;
        }

        /// <summary>
        /// Holt die Liste von der eingestellten Adresse. Antwortet sie mit einem HTTP-Fehler oder gar nicht,
        /// wird die Ersatzadresse gefragt.
        /// </summary>
        public static string ListeHolen(Optionen optionen)
        {
            try
            {
                return ManifestHolen(optionen.ManifestQuelle, optionen.Testmodus);
            }
            catch (WebException fehler)
            {
                if (optionen.AusweichQuelle == "") throw;
                try
                {
                    return ManifestHolen(optionen.AusweichQuelle, optionen.Testmodus);
                }
                catch (WebException ersatzFehler)
                {
                    throw new InvalidOperationException(fehler.Message + " Auch die Ersatzadresse " + optionen.AusweichQuelle
                        + " antwortet nicht: " + ersatzFehler.Message);
                }
            }
        }

        /// <summary>Holt die Liste der Komponenten. Aus dem Netz nur verschluesselt; eine lokale Datei nur im Testmodus.</summary>
        public static string ManifestHolen(string quelle, bool testmodus, int wartezeit = 20000)
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
            anfrage.UserAgent = "SamplifeLauncher/" + Version;
            anfrage.Timeout = wartezeit;
            anfrage.ReadWriteTimeout = wartezeit;
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

        /// <summary>
        /// Der Ablauf von „Spielen“ ohne Fenster, fuer automatische Tests. Rueckgabe:
        /// 0 Erfolg, 1 Laden fehlgeschlagen, 2 kein GTA-Verzeichnis, 3 weder Liste noch brauchbare Sicherung,
        /// 4 SA-MP fehlt oder falsche Version, 5 Name ungueltig, 6 samp.exe startet nicht, 7 Selbstaktualisierung
        /// fehlgeschlagen, 8 Passwort des Accounts falsch, 10 selbst aktualisiert (mit --kein-neustart).
        /// </summary>
        static int StillAusfuehren(Optionen optionen)
        {
            try
            {
                if (optionen.Deinstallieren)
                {
                    // Nur mit ausdruecklich genanntem Verzeichnis, nie mit einem Fund der Suche
                    var deinstallation = new Deinstallation { Gta = optionen.Gta, Testmodus = optionen.Testmodus };
                    return deinstallation.Ausfuehren().Erfolg ? 0 : 1;
                }

                var gta = optionen.Gta != "" ? optionen.Gta : GtaVerzeichnis.Finden();
                if (!GtaVerzeichnis.IstGtaVerzeichnis(gta)) return 2;

                // Neuestes Release auf GitHub; aktualisiert auch dann, wenn die Liste fehlt oder zu neu ist
                var release = Aktualisierung.NeuesteHolen(optionen.ReleaseQuelle, optionen.Testmodus);

                Manifest manifest = null;
                string json = null;
                var ausSicherung = false;
                var ohneListe = false;
                try
                {
                    json = ListeHolen(optionen);
                    manifest = Manifest.Lesen(json);
                }
                catch (ManifestFehler) { ohneListe = true; }
                catch (Exception)
                {
                    var stand = Stand.Lesen(gta);
                    if (stand == null) ohneListe = true;
                    else
                    {
                        json = stand.ManifestJson;
                        manifest = Manifest.Lesen(json);
                        ausSicherung = true;
                    }
                }

                // Was die Liste verlangt, ist Pflicht; ein neueres Release allein nicht: Klappt es nicht, geht es mit dieser Fassung weiter
                var ausListe = manifest != null && !ausSicherung ? manifest.Launcher : null;
                var fassung = Aktualisierung.Neuere(ausListe, release, optionen.NachAktualisierung);
                if (fassung != null)
                {
                    var geklappt = true;
                    try { Aktualisierung.Ausfuehren(fassung, optionen.Testmodus, delegate { }, CancellationToken.None); }
                    catch (Exception)
                    {
                        if (Aktualisierung.Neuere(ausListe, null, optionen.NachAktualisierung) != null) return 7;
                        geklappt = false;
                    }
                    if (geklappt)
                    {
                        if (optionen.KeinNeustart) return 10;
                        Aktualisierung.NeuStarten(Argumente, fassung.Version);
                        return 0;
                    }
                }
                if (ohneListe) return 3;

                if (optionen.Fremde != "")
                {
                    var zeilen = new List<string>();
                    foreach (var fremd in FremdeDateien.Suchen(gta, manifest))
                    {
                        var zeile = fremd.Pfad + "|" + fremd.Art;
                        if (optionen.Abschalten && fremd.Abschaltbar) zeile += "|" + FremdeDateien.Abschalten(gta, fremd);
                        zeilen.Add(zeile);
                    }
                    File.WriteAllLines(optionen.Fremde, zeilen, Encoding.UTF8);
                    return 0;
                }

                var sampVersion = GtaVerzeichnis.SampVersion(gta, manifest);
                if (sampVersion == "") sampVersion = optionen.Samp;
                // Liegt eine andere Version im Verzeichnis, die die Liste selbst einrichten kann (samp_client), nicht einfach die
                // eine Version der Liste annehmen: Ohne --samp-ersetzen hielte der Ablauf sonst die fremde Version für passend
                var andere = manifest.SampClient != null && Ablauf.Planen(gta, manifest).AndereVersion;
                if (sampVersion == "" && manifest.SampVersionen.Count == 1 && !optionen.Spielen && !andere) sampVersion = manifest.SampVersionen[0].Name;

                var ablauf = new Ablauf
                {
                    Gta = gta, SampVersion = sampVersion, Name = optionen.Name != "" ? optionen.Name : Spielstart.GespeicherterName(optionen.Testmodus),
                    Passwort = optionen.Passwort,
                    Kontopasswort = optionen.Kontopasswort,
                    Manifest = manifest, ManifestJson = json, AusSicherung = ausSicherung, Optional = optionen.Optional,
                    Gruendlich = optionen.Gruendlich, MitStart = optionen.Spielen, Testmodus = optionen.Testmodus,
                    SampErsetzen = optionen.SampErsetzen
                };
                var ergebnis = ablauf.Ausfuehren(CancellationToken.None);
                return ergebnis.Erfolg ? 0 : ergebnis.Code;
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
            {
                var json = ManifestHolen(optionen.ManifestQuelle, true);
                fenster.ManifestSetzen(Manifest.Lesen(json), json);
            }
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
