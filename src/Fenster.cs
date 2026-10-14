using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Net;
using System.Reflection;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Markup;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Navigation;

namespace SamplifeEssentials
{
    /// <summary>Eine Komponente in der Auswahlliste.</summary>
    public class KomponenteZeile : INotifyPropertyChanged
    {
        public Komponente Komponente;
        bool gewaehlt;
        bool verfuegbar = true;
        string info = "";

        public string Name { get { return Komponente.Name; } }
        public string Beschreibung { get { return Komponente.Beschreibung; } }
        public string Quelle { get { return Komponente.Quelle; } }
        public string QuelleVor { get { return Komponente.Quelle == "" ? "" : "Quelle: "; } }
        public string QuelleNach { get { return Komponente.Lizenz == "" ? "" : (Komponente.Quelle == "" ? "Lizenz: " : " · Lizenz: ") + Komponente.Lizenz; } }

        public bool Gewaehlt
        {
            get { return gewaehlt; }
            set { gewaehlt = value; Geaendert("Gewaehlt"); if (AuswahlGeaendert != null) AuswahlGeaendert(); }
        }
        public bool Verfuegbar
        {
            get { return verfuegbar; }
            set { verfuegbar = value; Geaendert("Verfuegbar"); Geaendert("Symbol"); Geaendert("SymbolFarbe"); Geaendert("Deckkraft"); Geaendert("InfoFarbe"); }
        }
        public string Symbol { get { return verfuegbar ? "" : ""; } }
        public Brush SymbolFarbe { get { return verfuegbar ? Farben.Akzent : Farben.Warnung; } }
        public double Deckkraft { get { return verfuegbar ? 1.0 : 0.5; } }
        public string Info { get { return info; } set { info = value; Geaendert("Info"); } }
        public Brush InfoFarbe { get { return verfuegbar ? Farben.Gedaempft : Farben.Warnung; } }

        public Action AuswahlGeaendert;
        public event PropertyChangedEventHandler PropertyChanged;
        void Geaendert(string name)
        {
            if (PropertyChanged != null) PropertyChanged(this, new PropertyChangedEventArgs(name));
        }
    }

    /// <summary>Eine Zeile im Feld „Was gerade passiert“.</summary>
    public class ProtokollZeile : INotifyPropertyChanged
    {
        public int Nummer;
        string vor = "", url = "", nach = "", symbol = "";
        Brush symbolFarbe = Farben.Gedaempft, textFarbe = Farben.Text;

        public string Zeit { get; set; }
        public string Vor { get { return vor; } set { vor = value; Geaendert("Vor"); } }
        public string Url { get { return url; } set { url = value; Geaendert("Url"); } }
        public string Nach { get { return nach; } set { nach = value; Geaendert("Nach"); } }
        public string Symbol { get { return symbol; } set { symbol = value; Geaendert("Symbol"); } }
        public Brush SymbolFarbe { get { return symbolFarbe; } set { symbolFarbe = value; Geaendert("SymbolFarbe"); } }
        public Brush TextFarbe { get { return textFarbe; } set { textFarbe = value; Geaendert("TextFarbe"); } }

        public event PropertyChangedEventHandler PropertyChanged;
        void Geaendert(string name)
        {
            if (PropertyChanged != null) PropertyChanged(this, new PropertyChangedEventArgs(name));
        }

        public void Uebernehmen(Meldung m)
        {
            if (m.Art == MeldungArt.Download)
            {
                // "Lade X von <Adresse> herunter (10,0 MB von 310,0 MB)"
                Vor = m.Text.Replace(" herunter", "") + " von ";
                Url = m.Url;
                if (m.Fertig) Nach = " herunter (" + Installation.Groesse(m.Erledigt) + ", fertig)";
                else if (m.Gesamt > 0) Nach = " herunter (" + Installation.Groesse(m.Erledigt) + " von " + Installation.Groesse(m.Gesamt) + ")";
                else Nach = " herunter (" + Installation.Groesse(m.Erledigt) + ")";
            }
            else
            {
                Vor = m.Text;
                Url = "";
                Nach = "";
            }
            switch (m.Art)
            {
                case MeldungArt.Download: Symbol = ""; SymbolFarbe = Farben.Akzent; break;
                case MeldungArt.Datei: Symbol = ""; SymbolFarbe = Farben.Leise; break;
                case MeldungArt.Erfolg: Symbol = ""; SymbolFarbe = Farben.Erfolg; break;
                case MeldungArt.Warnung: Symbol = ""; SymbolFarbe = Farben.Warnung; break;
                case MeldungArt.Fehler: Symbol = ""; SymbolFarbe = Farben.Fehler; break;
                default: Symbol = ""; SymbolFarbe = Farben.Gedaempft; break;
            }
            if (m.Art == MeldungArt.Fehler) TextFarbe = Farben.Fehler;
            else if (m.Art == MeldungArt.Erfolg) TextFarbe = Farben.Erfolg;
            else if (m.Art == MeldungArt.Datei) TextFarbe = Farben.Gedaempft;
            else TextFarbe = Farben.Text;
        }
    }

    public static class Farben
    {
        public static readonly Brush Text = Pinsel("#F1F3F6");
        public static readonly Brush Gedaempft = Pinsel("#A3A9B6");
        public static readonly Brush Leise = Pinsel("#6F7685");
        public static readonly Brush Akzent = Pinsel("#1A8BFF");
        public static readonly Brush Erfolg = Pinsel("#39D874");
        public static readonly Brush Warnung = Pinsel("#F0B24A");
        public static readonly Brush Fehler = Pinsel("#FF6B72");

        static Brush Pinsel(string farbe)
        {
            var pinsel = new SolidColorBrush((Color)ColorConverter.ConvertFromString(farbe));
            pinsel.Freeze();
            return pinsel;
        }
    }

    public class Fenster
    {
        public readonly Window Wurzel;
        readonly Optionen optionen;
        readonly TextBox pfad;
        readonly TextBlock pfadSymbol, pfadStatus, listeStatus, prozent, fusszeile, pfadTitel, sucheText;
        readonly Button knopfInstallieren, knopfAbbrechen, knopfProtokoll, knopfErneut, knopfDurchsuchen, knopfSuchen, knopfAnderer;
        readonly FrameworkElement bereichAuswahl, bereichProtokoll, listeLaedt, listeRollen, bereichSuche, bereichPfad;
        readonly ItemsControl fundListe;
        readonly FrameworkElement bereichVersion;
        readonly StackPanel versionen;
        readonly TextBlock versionPflicht, versionHinweis, versionTitel;
        bool nurEineVersion;
        string vonHandGewaehlt = "";
        readonly ObservableCollection<Fund> funde = new ObservableCollection<Fund>();
        readonly ScrollViewer protokollRollen;
        readonly Border balken;
        readonly ObservableCollection<KomponenteZeile> komponenten = new ObservableCollection<KomponenteZeile>();
        readonly ObservableCollection<ProtokollZeile> zeilen = new ObservableCollection<ProtokollZeile>();

        Manifest manifest;
        string sampVersion = "";
        bool verzeichnisGueltig, brauchtAdmin, laeuft, fertig;
        // Erst wenn der Spieler ein gefundenes Verzeichnis bestaetigt oder selbst eines waehlt, darf installiert werden
        bool bestaetigt;
        CancellationTokenSource suchAbbruch;
        CancellationTokenSource abbruch;
        string protokollDatei = "";

        public Fenster(Optionen optionen)
        {
            this.optionen = optionen;
            using (var strom = Assembly.GetExecutingAssembly().GetManifestResourceStream("Fenster.xaml"))
            {
                Wurzel = (Window)XamlReader.Load(strom);
            }
            using (var symbol = Assembly.GetExecutingAssembly().GetManifestResourceStream("symbol.ico"))
            {
                if (symbol != null) Wurzel.Icon = BitmapFrame.Create(symbol, BitmapCreateOptions.None, BitmapCacheOption.OnLoad);
            }

            pfad = Finde<TextBox>("Pfad");
            pfadSymbol = Finde<TextBlock>("PfadSymbol");
            pfadStatus = Finde<TextBlock>("PfadStatus");
            listeStatus = Finde<TextBlock>("ListeStatus");
            prozent = Finde<TextBlock>("Prozent");
            fusszeile = Finde<TextBlock>("Fusszeile");
            knopfInstallieren = Finde<Button>("KnopfInstallieren");
            knopfAbbrechen = Finde<Button>("KnopfAbbrechen");
            knopfProtokoll = Finde<Button>("KnopfProtokoll");
            knopfErneut = Finde<Button>("KnopfErneut");
            knopfDurchsuchen = Finde<Button>("KnopfDurchsuchen");
            bereichAuswahl = Finde<FrameworkElement>("BereichAuswahl");
            bereichProtokoll = Finde<FrameworkElement>("BereichProtokoll");
            listeLaedt = Finde<FrameworkElement>("ListeLaedt");
            listeRollen = Finde<FrameworkElement>("ListeRollen");
            protokollRollen = Finde<ScrollViewer>("ProtokollRollen");
            balken = Finde<Border>("Balken");
            pfadTitel = Finde<TextBlock>("PfadTitel");
            sucheText = Finde<TextBlock>("SucheText");
            knopfSuchen = Finde<Button>("KnopfSuchen");
            knopfAnderer = Finde<Button>("KnopfAnderer");
            bereichSuche = Finde<FrameworkElement>("BereichSuche");
            bereichPfad = Finde<FrameworkElement>("BereichPfad");
            fundListe = Finde<ItemsControl>("Funde");
            bereichVersion = Finde<FrameworkElement>("BereichVersion");
            versionen = Finde<StackPanel>("Versionen");
            versionPflicht = Finde<TextBlock>("VersionPflicht");
            versionHinweis = Finde<TextBlock>("VersionHinweis");
            versionTitel = Finde<TextBlock>("VersionTitel");
            fundListe.ItemsSource = funde;
            Finde<ItemsControl>("Liste").ItemsSource = komponenten;
            Finde<ItemsControl>("Protokoll").ItemsSource = zeilen;

            Finde<FrameworkElement>("Kopf").MouseLeftButtonDown += delegate(object s, MouseButtonEventArgs e)
            {
                if (e.ButtonState == MouseButtonState.Pressed) Wurzel.DragMove();
            };
            Finde<Button>("KnopfMinimieren").Click += delegate { Wurzel.WindowState = WindowState.Minimized; };
            Finde<Button>("KnopfSchliessenX").Click += delegate { Wurzel.Close(); };
            knopfDurchsuchen.Click += delegate { Durchsuchen(); };
            knopfSuchen.Click += delegate { VerzeichnisSuchen(); };
            knopfAnderer.Click += delegate { SucheBeenden(); Durchsuchen(); };
            // Die Knoepfe "Ja, verwenden" liegen in den Zeilen der Fundliste und tragen den Pfad im Tag
            fundListe.AddHandler(Button.ClickEvent, new RoutedEventHandler(delegate(object s, RoutedEventArgs e)
            {
                var knopf = e.OriginalSource as Button;
                if (knopf != null && knopf.Tag is string) Uebernehmen((string)knopf.Tag);
            }));
            knopfErneut.Click += delegate { ListeLaden(); };
            knopfInstallieren.Click += delegate { HauptknopfGedrueckt(); };
            knopfAbbrechen.Click += delegate { if (abbruch != null) abbruch.Cancel(); };
            knopfProtokoll.Click += delegate { Oeffnen(protokollDatei); };
            pfad.TextChanged += delegate { if (pfad.IsKeyboardFocusWithin) bestaetigt = true; VerzeichnisPruefen(); };
            Wurzel.AddHandler(Hyperlink.RequestNavigateEvent, new RequestNavigateEventHandler(LinkGeklickt));
            Wurzel.Closing += delegate(object s, CancelEventArgs e)
            {
                if (!laeuft) return;
                var antwort = MessageBox.Show(Wurzel, "Die Installation läuft noch. Wirklich abbrechen?", Wurzel.Title,
                    MessageBoxButton.YesNo, MessageBoxImage.Question);
                if (antwort != MessageBoxResult.Yes) { e.Cancel = true; return; }
                if (abbruch != null) abbruch.Cancel();
            };

            if (optionen.Gta != "") Uebernehmen(optionen.Gta);
            else VerzeichnisPruefen();
            // Erst laden, wenn das Fenster laeuft: vorher gibt es keinen Kontext, der nach dem Warten
            // in den Thread der Oberflaeche zurueckfuehrt
            Wurzel.Loaded += delegate
            {
                if (!optionen.OhneNetz) ListeLaden();
                if (optionen.Gta == "" && (optionen.Bildschirmfoto == "" || optionen.Zustand == "ablauf")) VerzeichnisSuchen();
            };
        }

        T Finde<T>(string name) where T : class
        {
            var element = Wurzel.FindName(name) as T;
            if (element == null) throw new InvalidOperationException("Element " + name + " fehlt in Fenster.xaml");
            return element;
        }

        // ---------------------------------------------------------------- Verzeichnis

        void Durchsuchen()
        {
            using (var dialog = new System.Windows.Forms.FolderBrowserDialog())
            {
                dialog.Description = "Wähle den Ordner, in dem gta_sa.exe liegt.";
                dialog.ShowNewFolderButton = false;
                if (Directory.Exists(pfad.Text)) dialog.SelectedPath = pfad.Text;
                if (dialog.ShowDialog() == System.Windows.Forms.DialogResult.OK) Uebernehmen(dialog.SelectedPath);
            }
        }

        /// <summary>Sucht im Hintergrund nach GTA-Verzeichnissen und legt das Ergebnis zur Bestaetigung vor.</summary>
        async void VerzeichnisSuchen()
        {
            if (laeuft || fertig) return;
            if (suchAbbruch != null) suchAbbruch.Cancel();
            suchAbbruch = new CancellationTokenSource();
            var token = suchAbbruch.Token;
            bestaetigt = false;
            funde.Clear();
            pfadTitel.Text = "GTA-Verzeichnis";
            sucheText.Text = "Suche nach GTA San Andreas auf diesem Rechner …";
            bereichPfad.Visibility = Visibility.Collapsed;
            bereichSuche.Visibility = Visibility.Visible;
            knopfAnderer.Content = "Selbst einen Ordner wählen …";
            KnoepfeAktualisieren();

            List<Fund> gefunden;
            try { gefunden = await Task.Run(() => GtaVerzeichnis.Suchen(token)); }
            catch (Exception) { gefunden = new List<Fund>(); }
            if (token.IsCancellationRequested) return;
            FundeZeigen(gefunden);
        }

        public void FundeZeigen(IList<Fund> gefunden)
        {
            funde.Clear();
            foreach (var fund in gefunden) funde.Add(fund);
            if (funde.Count == 0)
            {
                // Nichts gefunden: zurueck zur Eingabe von Hand
                SucheBeenden();
                PfadMeldung("\uE8B7", Farben.Warnung, "Kein GTA-Verzeichnis gefunden. Wähle den Ordner, in dem gta_sa.exe liegt.");
                return;
            }
            bereichPfad.Visibility = Visibility.Collapsed;
            bereichSuche.Visibility = Visibility.Visible;
            pfadTitel.Text = funde.Count == 1 ? "Ist das dein GTA-Verzeichnis?" : "Welches ist dein GTA-Verzeichnis?";
            sucheText.Text = funde.Count == 1
                ? "Die Suche hat ein Verzeichnis gefunden. Bestätige es, oder wähle einen anderen Ordner."
                : "Die Suche hat " + funde.Count + " Verzeichnisse gefunden. Wähle das, mit dem du auf dem Server spielst.";
            knopfAnderer.Content = "Anderen Ordner wählen …";
            KnoepfeAktualisieren();
        }

        void SucheBeenden()
        {
            if (suchAbbruch != null) suchAbbruch.Cancel();
            pfadTitel.Text = "GTA-Verzeichnis";
            bereichSuche.Visibility = Visibility.Collapsed;
            bereichPfad.Visibility = Visibility.Visible;
        }

        /// <summary>Wie ein Klick auf „Ja, verwenden“ beim ersten Fund (nur fuer das Bildschirmfoto des ganzen Ablaufs).</summary>
        public void ErstenFundBestaetigen()
        {
            if (funde.Count > 0) Uebernehmen(funde[0].Pfad);
        }

        /// <summary>Waehlt die letzte angebotene SA-MP-Version aus (nur fuer das Bildschirmfoto des ganzen Ablaufs).</summary>
        public void VersionWaehlen()
        {
            if (versionen.Children.Count > 0) ((RadioButton)versionen.Children[versionen.Children.Count - 1]).IsChecked = true;
        }

        /// <summary>Der Spieler hat sich fuer ein Verzeichnis entschieden.</summary>
        void Uebernehmen(string verzeichnis)
        {
            SucheBeenden();
            bestaetigt = true;
            if (pfad.Text == verzeichnis) VerzeichnisPruefen();
            else pfad.Text = verzeichnis;
        }

        void VerzeichnisPruefen()
        {
            var verzeichnis = pfad.Text.Trim().Trim('"');
            verzeichnisGueltig = false;
            brauchtAdmin = false;
            sampVersion = "";
            if (verzeichnis == "")
            {
                PfadMeldung("", Farben.Gedaempft, "Wähle den Ordner, in dem gta_sa.exe liegt.");
            }
            else if (!Directory.Exists(verzeichnis))
            {
                PfadMeldung("", Farben.Fehler, "Diesen Ordner gibt es nicht.");
            }
            else if (!GtaVerzeichnis.IstGtaVerzeichnis(verzeichnis))
            {
                PfadMeldung("", Farben.Fehler, "In diesem Ordner liegt keine gta_sa.exe.");
            }
            else
            {
                verzeichnisGueltig = true;
                brauchtAdmin = !GtaVerzeichnis.IstBeschreibbar(verzeichnis);
                var text = "gta_sa.exe gefunden";
                var farbe = Farben.Erfolg;
                var symbol = "";
                if (manifest != null) sampVersion = GtaVerzeichnis.SampVersion(verzeichnis, manifest);
                if (sampVersion != "")
                {
                    text += " · SA-MP " + sampVersion;
                    bereichVersion.Visibility = Visibility.Collapsed;
                }
                else if (manifest != null && manifest.SampVersionen.Count > 0)
                {
                    // Nicht erkannt (keine samp.dll im Ordner oder unbekannte Fassung): der Spieler waehlt selbst
                    sampVersion = vonHandGewaehlt;
                    if (sampVersion == "")
                    {
                        text += " · SA-MP-Version bitte unten wählen";
                        farbe = Farben.Warnung; symbol = "";
                    }
                    else if (nurEineVersion) text += " · benötigt SA-MP " + sampVersion + ", siehe unten";
                    else text += " · SA-MP " + sampVersion + " (von dir gewählt)";
                    bereichVersion.Visibility = Visibility.Visible;
                }
                else bereichVersion.Visibility = Visibility.Collapsed;
                if (brauchtAdmin) text += " · Schreiben ist hier nur als Administrator möglich";
                PfadMeldung(symbol, farbe, text);
            }
            if (!verzeichnisGueltig) bereichVersion.Visibility = Visibility.Collapsed;
            KomponentenAbgleichen();
            KnoepfeAktualisieren();
        }

        void PfadMeldung(string symbol, Brush farbe, string text)
        {
            pfadSymbol.Text = symbol;
            pfadSymbol.Foreground = farbe;
            pfadStatus.Text = text;
        }

        // ---------------------------------------------------------------- Liste der Komponenten

        async void ListeLaden()
        {
            manifest = null;
            komponenten.Clear();
            listeRollen.Visibility = Visibility.Collapsed;
            listeLaedt.Visibility = Visibility.Visible;
            knopfErneut.Visibility = Visibility.Collapsed;
            listeStatus.Foreground = Farben.Gedaempft;
            listeStatus.Text = "Lade die Liste der Komponenten von " + optionen.ManifestQuelle + " …";
            KnoepfeAktualisieren();
            try
            {
                var json = await Task.Run(() => Programm.ListeHolen(optionen));
                ManifestSetzen(Manifest.Lesen(json));
            }
            catch (Exception fehler)
            {
                listeStatus.Foreground = Farben.Fehler;
                listeStatus.Text = "Die Liste der Komponenten konnte nicht geladen werden.\n" + fehler.Message;
                knopfErneut.Visibility = Visibility.Visible;
            }
            KnoepfeAktualisieren();
        }

        public void ManifestSetzen(Manifest neu)
        {
            manifest = neu;
            komponenten.Clear();
            foreach (var komponente in manifest.Komponenten)
            {
                var zeile = new KomponenteZeile { Komponente = komponente };
                zeile.AuswahlGeaendert = KnoepfeAktualisieren;
                komponenten.Add(zeile);
            }
            listeLaedt.Visibility = Visibility.Collapsed;
            listeRollen.Visibility = Visibility.Visible;
            if (manifest.Hinweis != "") fusszeile.Text = manifest.Hinweis;
            versionen.Children.Clear();
            vonHandGewaehlt = "";
            var namen = new List<string>();
            foreach (var version in manifest.SampVersionen) namen.Add(version.Name);
            var unterstuetzt = string.Join(" oder ", namen.ToArray());
            nurEineVersion = namen.Count == 1;
            if (nurEineVersion)
            {
                // Nichts zu waehlen: die eine Version gilt, der Spieler muss sie nur im Launcher einstellen
                vonHandGewaehlt = namen[0];
                versionTitel.Text = "Wichtig: Du brauchst SA-MP " + unterstuetzt;
                versionTitel.Foreground = Farben.Warnung;
                versionPflicht.Text = "Die Erweiterungen und damit der Server laufen ausschließlich mit SA-MP " + unterstuetzt + ".";
                versionHinweis.Text = "Spielst du über den open.mp-Launcher? Dann stelle dort in den Einstellungen als SA-MP-Version " + unterstuetzt
                    + " ein, bevor du verbindest. Mit jeder anderen Version, auch 0.3.7-R3 oder 0.3.DL, stürzt das Spiel beim Start ab "
                    + "oder du kommst nicht auf den Server.";
                versionen.Visibility = Visibility.Collapsed;
            }
            else
            {
                versionTitel.Text = "Mit welcher SA-MP-Version spielst du?";
                versionTitel.Foreground = Farben.Text;
                versionPflicht.Text = "Die Erweiterungen laufen nur mit SA-MP " + unterstuetzt + ".";
                versionHinweis.Text = "Spielst du über den open.mp-Launcher? Dann stelle dort in den Einstellungen als SA-MP-Version " + unterstuetzt
                    + " ein und wähle hier dieselbe. Mit einer anderen Version, zum Beispiel 0.3.DL, funktionieren die Erweiterungen nicht, "
                    + "und wenn Launcher und Auswahl nicht übereinstimmen, stürzt das Spiel beim Start ab.";
                versionen.Visibility = Visibility.Visible;
            }
            foreach (var version in manifest.SampVersionen)
            {
                var knopf = new RadioButton { Content = version.Name, GroupName = "SampVersion", Style = (Style)Wurzel.FindResource("Wahlknopf") };
                var name = version.Name;
                knopf.Checked += delegate { vonHandGewaehlt = name; VerzeichnisPruefen(); };
                versionen.Children.Add(knopf);
            }
            VerzeichnisPruefen();
        }

        /// <summary>Welche Komponenten gibt es fuer die erkannte SA-MP-Version, und wie gross sind sie?</summary>
        void KomponentenAbgleichen()
        {
            foreach (var zeile in komponenten)
            {
                var pakete = Manifest.PassendePakete(zeile.Komponente, sampVersion);
                if (pakete.Count == 0)
                {
                    zeile.Verfuegbar = false;
                    // Den Schritt nennen, der wirklich als naechster dran ist: erst das Verzeichnis, dann die Version
                    if (!bestaetigt || !verzeichnisGueltig) zeile.Info = "bestätige zuerst oben dein GTA-Verzeichnis";
                    else if (sampVersion == "") zeile.Info = "wähle oben deine SA-MP-Version";
                    else zeile.Info = "nicht für SA-MP " + sampVersion;
                    zeile.Gewaehlt = false;
                }
                else
                {
                    long groesse = 0;
                    foreach (var paket in pakete) groesse += paket.Groesse;
                    zeile.Verfuegbar = true;
                    zeile.Info = groesse > 0 ? Installation.Groesse(groesse) : "";
                    // Alles, was es fuer die SA-MP-Version des Spielers gibt, wird installiert
                    zeile.Gewaehlt = true;
                }
            }
        }

        // ---------------------------------------------------------------- Installation

        void KnoepfeAktualisieren()
        {
            if (laeuft || fertig) return;
            var gewaehlt = 0;
            foreach (var zeile in komponenten) if (zeile.Gewaehlt && zeile.Verfuegbar) gewaehlt++;
            knopfInstallieren.Content = brauchtAdmin ? "Als Administrator starten" : "Installieren";
            knopfInstallieren.IsEnabled = bestaetigt && verzeichnisGueltig && (brauchtAdmin || (manifest != null && gewaehlt > 0));
        }

        void HauptknopfGedrueckt()
        {
            if (fertig) { Wurzel.Close(); return; }
            if (brauchtAdmin) { AlsAdministratorStarten(); return; }
            Installieren();
        }

        void AlsAdministratorStarten()
        {
            try
            {
                var start = new ProcessStartInfo(Assembly.GetExecutingAssembly().Location);
                start.Verb = "runas";
                start.Arguments = "--gta \"" + pfad.Text.Trim().Trim('"') + "\"" + optionen.AlsArgumente();
                Process.Start(start);
                Wurzel.Close();
            }
            catch (Win32Exception)
            {
                // Abfrage der Benutzerkontensteuerung abgelehnt
            }
        }

        async void Installieren()
        {
            var auswahl = new List<Komponente>();
            foreach (var zeile in komponenten) if (zeile.Gewaehlt && zeile.Verfuegbar) auswahl.Add(zeile.Komponente);
            if (auswahl.Count == 0) return;

            laeuft = true;
            abbruch = new CancellationTokenSource();
            zeilen.Clear();
            FortschrittZeigen(0);
            bereichAuswahl.Visibility = Visibility.Collapsed;
            bereichProtokoll.Visibility = Visibility.Visible;
            pfad.IsEnabled = false;
            knopfDurchsuchen.IsEnabled = false;
            knopfSuchen.IsEnabled = false;
            bereichVersion.IsEnabled = false;
            knopfInstallieren.IsEnabled = false;
            knopfInstallieren.Content = "Installiere …";
            knopfAbbrechen.Visibility = Visibility.Visible;

            var installation = new Installation(pfad.Text.Trim().Trim('"'), sampVersion, auswahl, optionen.Testmodus);
            // Progress leitet die Meldungen aus dem Hintergrund-Thread in den Thread der Oberflaeche
            var meldungen = new Progress<Meldung>(MeldungZeigen);
            var fortschritt = new Progress<double>(FortschrittZeigen);
            installation.Melden = m => ((IProgress<Meldung>)meldungen).Report(m);
            installation.Fortschritt = f => ((IProgress<double>)fortschritt).Report(f);

            var token = abbruch.Token;
            var ergebnis = await Task.Run(() => installation.Ausfuehren(token));
            // Die letzten Meldungen laufen ueber dieselbe Warteschlange; kurz abwarten, bis sie angezeigt sind
            await Task.Delay(150);

            laeuft = false;
            fertig = true;
            protokollDatei = ergebnis.Protokoll;
            knopfAbbrechen.Visibility = Visibility.Collapsed;
            knopfProtokoll.Visibility = protokollDatei == "" ? Visibility.Collapsed : Visibility.Visible;
            knopfInstallieren.Content = "Schließen";
            knopfInstallieren.IsEnabled = true;
            if (ergebnis.Erfolg)
            {
                FortschrittZeigen(1);
                fusszeile.Text = "Alles eingerichtet. Starte SA-MP und verbinde dich mit dem Server."
                    + (ergebnis.Sicherung == "" ? "" : " Ersetzte Dateien liegen in " + ergebnis.Sicherung + ".");
            }
            else
            {
                fusszeile.Text = "Die Installation wurde nicht abgeschlossen. Details stehen oben und im Protokoll.";
            }
        }

        public void MeldungZeigen(Meldung m)
        {
            ProtokollZeile zeile = null;
            for (var i = zeilen.Count - 1; i >= 0 && i >= zeilen.Count - 6; i--)
            {
                if (zeilen[i].Nummer == m.Nummer) { zeile = zeilen[i]; break; }
            }
            if (zeile == null)
            {
                zeile = new ProtokollZeile { Nummer = m.Nummer, Zeit = DateTime.Now.ToString("HH:mm", CultureInfo.InvariantCulture) };
                zeile.Uebernehmen(m);
                zeilen.Add(zeile);
                protokollRollen.ScrollToEnd();
            }
            else zeile.Uebernehmen(m);
        }

        public void FortschrittZeigen(double anteil)
        {
            var breite = ((FrameworkElement)balken.Parent).ActualWidth;
            if (breite <= 0) breite = 894;
            balken.Width = Math.Max(0, Math.Min(breite, breite * anteil));
            prozent.Text = ((int)Math.Round(anteil * 100)) + " %";
        }

        // ---------------------------------------------------------------- Sonstiges

        void LinkGeklickt(object sender, RequestNavigateEventArgs e)
        {
            if (e.Uri != null && e.Uri.IsAbsoluteUri && (e.Uri.Scheme == Uri.UriSchemeHttps || e.Uri.Scheme == Uri.UriSchemeHttp))
                Oeffnen(e.Uri.AbsoluteUri);
            e.Handled = true;
        }

        static void Oeffnen(string ziel)
        {
            if (string.IsNullOrEmpty(ziel)) return;
            try { Process.Start(new ProcessStartInfo(ziel) { UseShellExecute = true }); }
            catch (Exception) { }
        }

        // ---------------------------------------------------------------- Bildschirmfoto (nur zum Pruefen der Oberflaeche)

        /// <summary>Stellt einen Zustand mit Beispieldaten dar, ohne etwas herunterzuladen oder zu schreiben.</summary>
        public void ZustandVorfuehren(string zustand)
        {
            if (zustand == "start") return;
            if (zustand == "gefunden")
            {
                bestaetigt = false;
                FundeZeigen(new List<Fund>
                {
                    new Fund { Pfad = @"D:\Spiele\GTA San Andreas", Herkunft = "aus den Einstellungen von SA-MP", HatSamp = true },
                    new Fund { Pfad = @"C:\Program Files (x86)\Steam\steamapps\common\Grand Theft Auto San Andreas", Herkunft = "Steam", HatSamp = false },
                });
                return;
            }
            bereichAuswahl.Visibility = Visibility.Collapsed;
            bereichProtokoll.Visibility = Visibility.Visible;
            knopfInstallieren.Content = "Installiere …";
            knopfInstallieren.IsEnabled = false;
            knopfAbbrechen.Visibility = Visibility.Visible;
            var n = 1;
            MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Schritt, Text = "Prüfe das GTA-Verzeichnis " + pfad.Text, Fertig = true });
            MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Download, Text = "Lade ASI Loader herunter", Url = "https://example.com/essentials/asi-loader.zip", Erledigt = 412000, Gesamt = 412000, Fertig = true });
            MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Schritt, Text = "Prüfsumme von ASI Loader stimmt (402 KB)", Fertig = true });
            MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Schritt, Text = "Entpacke 1 Datei von ASI Loader nach " + pfad.Text, Fertig = true });
            MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Datei, Text = "Sichere vorhandene vorbisFile.dll nach samplife-essentials-sicherung\\2026-10-13_19-04-11\\vorbisFile.dll", Fertig = true });
            MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Datei, Text = "Schreibe vorbisFile.dll", Fertig = true });
            if (zustand == "fertig")
            {
                MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Download, Text = "Lade Browser-Oberfläche (CEF) herunter", Url = "https://example.com/essentials/cef-client.zip", Erledigt = 325058560, Gesamt = 325058560, Fertig = true });
                MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Schritt, Text = "Prüfsumme von Browser-Oberfläche (CEF) stimmt (310,0 MB)", Fertig = true });
                MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Erfolg, Text = "Fertig. 148 Dateien geschrieben, 0 waren schon aktuell, 1 vorhandene Datei gesichert.", Fertig = true });
                FortschrittZeigen(1);
                knopfAbbrechen.Visibility = Visibility.Collapsed;
                knopfProtokoll.Visibility = Visibility.Visible;
                knopfInstallieren.Content = "Schließen";
                knopfInstallieren.IsEnabled = true;
                fusszeile.Text = "Alles eingerichtet. Starte SA-MP und verbinde dich mit dem Server.";
            }
            else
            {
                MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Download, Text = "Lade Browser-Oberfläche (CEF) herunter", Url = "https://example.com/essentials/cef-client.zip", Erledigt = 10485760, Gesamt = 325058560 });
                FortschrittZeigen(0.04);
            }
        }

        public void Speichern(string datei)
        {
            var inhalt = (FrameworkElement)Wurzel.Content;
            var bild = new RenderTargetBitmap((int)inhalt.ActualWidth, (int)inhalt.ActualHeight, 96, 96, PixelFormats.Pbgra32);
            var hintergrund = new DrawingVisual();
            using (var kontext = hintergrund.RenderOpen())
            {
                kontext.DrawRectangle(Wurzel.Background, null, new Rect(0, 0, inhalt.ActualWidth, inhalt.ActualHeight));
            }
            bild.Render(hintergrund);
            bild.Render(inhalt);
            var kodierer = new PngBitmapEncoder();
            kodierer.Frames.Add(BitmapFrame.Create(bild));
            using (var strom = File.Create(datei)) kodierer.Save(strom);
        }
    }
}
