using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Threading;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Documents;
using System.Windows.Input;
using System.Windows.Markup;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Imaging;
using System.Windows.Navigation;
using System.Windows.Threading;

namespace SamplifeEssentials
{
    /// <summary>Eine Komponente in der Liste „Dateien“.</summary>
    public class KomponenteZeile : INotifyPropertyChanged
    {
        public Komponente Komponente;
        bool gewaehlt;
        bool gesperrt;
        bool verfuegbar = true;
        string info = "";
        Brush infoFarbe = Farben.Gedaempft;

        public string Name { get { return Komponente.Name; } }
        public string Art { get { return Komponente.Optional ? "  ·  optional" : "  ·  Pflicht"; } }
        public string Beschreibung { get { return Komponente.Beschreibung; } }
        public string Quelle { get { return Komponente.Quelle; } }
        public string QuelleVor { get { return Komponente.Quelle == "" ? "" : "Quelle: "; } }
        public string QuelleNach { get { return Komponente.Lizenz == "" ? "" : (Komponente.Quelle == "" ? "Lizenz: " : " · Lizenz: ") + Komponente.Lizenz; } }
        public string HakenHinweis
        {
            get { return Komponente.Optional ? "Optional: Du entscheidest, ob der Launcher das einrichtet." : "Gehört zum Spielen auf dem Server und lässt sich nicht abwählen."; }
        }

        public bool Gewaehlt
        {
            get { return gewaehlt; }
            set { gewaehlt = value; Geaendert("Gewaehlt"); if (AuswahlGeaendert != null) AuswahlGeaendert(); }
        }
        /// <summary>Nur optionale Komponenten lassen sich abwaehlen, und nicht, waehrend der Launcher arbeitet.</summary>
        public bool Waehlbar { get { return Komponente.Optional && !gesperrt && verfuegbar; } }
        public bool Gesperrt { set { gesperrt = value; Geaendert("Waehlbar"); } }
        public bool Verfuegbar
        {
            get { return verfuegbar; }
            set { verfuegbar = value; Geaendert("Verfuegbar"); Geaendert("Deckkraft"); Geaendert("Waehlbar"); }
        }
        public double Deckkraft { get { return verfuegbar ? 1.0 : 0.5; } }
        public string Info { get { return info; } set { info = value; Geaendert("Info"); } }
        public Brush InfoFarbe { get { return infoFarbe; } set { infoFarbe = value; Geaendert("InfoFarbe"); } }

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
                case MeldungArt.Download: Symbol = "\uE896"; SymbolFarbe = Farben.Akzent; break;
                case MeldungArt.Datei: Symbol = "\uE8A5"; SymbolFarbe = Farben.Leise; break;
                case MeldungArt.Erfolg: Symbol = "\uE73E"; SymbolFarbe = Farben.Erfolg; break;
                case MeldungArt.Warnung: Symbol = "\uE7BA"; SymbolFarbe = Farben.Warnung; break;
                case MeldungArt.Fehler: Symbol = "\uE711"; SymbolFarbe = Farben.Fehler; break;
                default: Symbol = "\uE76C"; SymbolFarbe = Farben.Gedaempft; break;
            }
            if (m.Art == MeldungArt.Fehler) TextFarbe = Farben.Fehler;
            else if (m.Art == MeldungArt.Erfolg) TextFarbe = Farben.Erfolg;
            else if (m.Art == MeldungArt.Warnung) TextFarbe = Farben.Warnung;
            else if (m.Art == MeldungArt.Datei) TextFarbe = Farben.Gedaempft;
            else TextFarbe = Farben.Text;
        }
    }

    /// <summary>Farben aus DESIGN.md (Repo samplife).</summary>
    public static class Farben
    {
        public static readonly Brush Text = Pinsel("#F1F3F6");
        public static readonly Brush Gedaempft = Pinsel("#A3A9B6");
        public static readonly Brush Leise = Pinsel("#6F7685");
        public static readonly Brush Akzent = Pinsel("#1A8BFF");
        public static readonly Brush Erfolg = Pinsel("#39D874");
        public static readonly Brush Warnung = Pinsel("#FFB547");
        public static readonly Brush Fehler = Pinsel("#FF8A8F");

        static Brush Pinsel(string farbe)
        {
            var pinsel = new SolidColorBrush((Color)ColorConverter.ConvertFromString(farbe));
            pinsel.Freeze();
            return pinsel;
        }
    }

    public class Fenster
    {
        // Was der Hauptknopf gerade tut
        enum Modus { Spielen, Laeuft, Erneut, Schliessen, Download }

        const string DownloadSeite = "https://github.com/German-Samplife/samplife-essentials/releases/latest";
        const string UpdatesSeite = "https://www.german-samplife.de/community/updates";

        public readonly Window Wurzel;
        readonly Optionen optionen;
        readonly Einstellungen einstellungen;
        readonly TextBox pfad, name;
        readonly TextBlock pfadSymbol, pfadStatus, listeStatus, prozent, fusszeile, pfadTitel, sucheText, titel, untertitel,
            hinweisText, nameStatus, neuStatus;
        readonly Button knopfSpielen, knopfAbbrechen, knopfProtokoll, knopfErneut, knopfDurchsuchen, knopfSuchen, knopfAnderer, knopfGruendlich;
        readonly FrameworkElement bereichAuswahl, bereichProtokoll, listeLaedt, listeRollen, bereichSuche, bereichPfad, hinweis;
        readonly ItemsControl fundListe;
        readonly FrameworkElement bereichVersion;
        readonly StackPanel versionen;
        readonly TextBlock versionPflicht, versionHinweis, versionTitel;
        bool nurEineVersion;
        readonly ObservableCollection<Fund> funde = new ObservableCollection<Fund>();
        readonly ScrollViewer protokollRollen;
        readonly Border balken;
        readonly Image titelbild;
        readonly PasswordBox passwort, kontopasswort;
        readonly CheckBox serverpasswortHaken;
        readonly FrameworkElement bereichPasswort, protokollFenster;
        readonly Button knopfDateien;
        // Die letzten drei Zeilen von „Was gerade passiert“; alles steht im Fenster hinter „Mehr anzeigen“
        readonly ObservableCollection<ProtokollZeile> kurzZeilen = new ObservableCollection<ProtokollZeile>();
        bool titelbildGezeigt;
        bool direktVersucht;       // samplife://verbinden: nur einmal von selbst starten
        readonly ObservableCollection<KomponenteZeile> komponenten = new ObservableCollection<KomponenteZeile>();
        readonly ObservableCollection<ProtokollZeile> zeilen = new ObservableCollection<ProtokollZeile>();
        readonly ObservableCollection<Neuigkeit> neuigkeiten = new ObservableCollection<Neuigkeit>();

        Manifest manifest;
        string manifestJson = "";
        bool ausSicherung;          // Liste kam aus der lokalen Sicherung, weil der Server nicht antwortete
        bool listeLaedtGerade;
        string sampVersion = "";
        bool verzeichnisGueltig, brauchtAdmin;
        // Erst wenn der Spieler ein gefundenes Verzeichnis bestaetigt oder selbst eines waehlt, darf geladen werden
        bool bestaetigt;
        Modus modus = Modus.Spielen;
        int pruefRunde;             // verwirft Ergebnisse einer ueberholten Vorschau
        CancellationTokenSource suchAbbruch;
        CancellationTokenSource abbruch;
        string protokollDatei = "";

        public Fenster(Optionen optionen)
        {
            this.optionen = optionen;
            // Fuer Bildschirmfotos nichts vom Rechner uebernehmen
            einstellungen = optionen.Bildschirmfoto == "" ? Einstellungen.Laden() : new Einstellungen();
            using (var strom = Assembly.GetExecutingAssembly().GetManifestResourceStream("Fenster.xaml"))
            {
                Wurzel = (Window)XamlReader.Load(strom);
            }
            using (var symbol = Assembly.GetExecutingAssembly().GetManifestResourceStream("symbol.ico"))
            {
                if (symbol != null) Wurzel.Icon = BitmapFrame.Create(symbol, BitmapCreateOptions.None, BitmapCacheOption.OnLoad);
            }
            // Mitgebrachte Schriften wie auf der Website; ohne sie bleibt es bei Segoe UI
            var schriften = Schriften.Bereitstellen();
            if (schriften != null)
            {
                Wurzel.FontFamily = new FontFamily(schriften, "./#Inter, Segoe UI");
                Wurzel.Resources["Anzeige"] = new FontFamily(schriften, "./#Inter Display, ./#Inter, Segoe UI");
            }

            pfad = Finde<TextBox>("Pfad");
            name = Finde<TextBox>("Name");
            pfadSymbol = Finde<TextBlock>("PfadSymbol");
            pfadStatus = Finde<TextBlock>("PfadStatus");
            listeStatus = Finde<TextBlock>("ListeStatus");
            prozent = Finde<TextBlock>("Prozent");
            fusszeile = Finde<TextBlock>("Fusszeile");
            titel = Finde<TextBlock>("Titel");
            untertitel = Finde<TextBlock>("Untertitel");
            hinweis = Finde<FrameworkElement>("Hinweis");
            hinweisText = Finde<TextBlock>("HinweisText");
            nameStatus = Finde<TextBlock>("NameStatus");
            neuStatus = Finde<TextBlock>("NeuStatus");
            knopfSpielen = Finde<Button>("KnopfSpielen");
            knopfAbbrechen = Finde<Button>("KnopfAbbrechen");
            knopfProtokoll = Finde<Button>("KnopfProtokoll");
            knopfErneut = Finde<Button>("KnopfErneut");
            knopfDurchsuchen = Finde<Button>("KnopfDurchsuchen");
            knopfGruendlich = Finde<Button>("KnopfGruendlich");
            bereichAuswahl = Finde<FrameworkElement>("BereichAuswahl");
            bereichProtokoll = Finde<FrameworkElement>("BereichProtokoll");
            listeLaedt = Finde<FrameworkElement>("ListeLaedt");
            listeRollen = Finde<FrameworkElement>("ListeRollen");
            protokollRollen = Finde<ScrollViewer>("ProtokollRollen");
            balken = Finde<Border>("Balken");
            titelbild = Finde<Image>("Titelbild");
            passwort = Finde<PasswordBox>("Passwort");
            kontopasswort = Finde<PasswordBox>("Kontopasswort");
            serverpasswortHaken = Finde<CheckBox>("ServerpasswortHaken");
            bereichPasswort = Finde<FrameworkElement>("BereichPasswort");
            protokollFenster = Finde<FrameworkElement>("ProtokollFenster");
            knopfDateien = Finde<Button>("KnopfDateien");
            Finde<ItemsControl>("ProtokollKurz").ItemsSource = kurzZeilen;
            serverpasswortHaken.Checked += delegate { bereichPasswort.Visibility = Visibility.Visible; passwort.Focus(); };
            serverpasswortHaken.Unchecked += delegate { bereichPasswort.Visibility = Visibility.Collapsed; };
            knopfDateien.Click += delegate { DateienUmschalten(); };
            Finde<Button>("KnopfMehr").Click += delegate { protokollFenster.Visibility = Visibility.Visible; protokollRollen.ScrollToEnd(); };
            Finde<Button>("KnopfFensterZu").Click += delegate { protokollFenster.Visibility = Visibility.Collapsed; };
            protokollFenster.MouseLeftButtonDown += delegate(object s, MouseButtonEventArgs e)
            {
                // Klick neben das Fenster schliesst es, wie auf der Website
                if (e.OriginalSource == protokollFenster) protokollFenster.Visibility = Visibility.Collapsed;
            };
            Wurzel.KeyDown += delegate(object s, KeyEventArgs e)
            {
                if (e.Key == Key.Escape && protokollFenster.Visibility == Visibility.Visible) protokollFenster.Visibility = Visibility.Collapsed;
            };
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
            var neuListe = Finde<ItemsControl>("NeuListe");
            neuListe.ItemsSource = neuigkeiten;
            Finde<TextBlock>("VersionText").Text = "Launcher " + Programm.Version;

            Finde<FrameworkElement>("Kopf").MouseLeftButtonDown += delegate(object s, MouseButtonEventArgs e)
            {
                if (e.ButtonState == MouseButtonState.Pressed) Wurzel.DragMove();
            };
            Finde<Button>("KnopfMinimieren").Click += delegate { Wurzel.WindowState = WindowState.Minimized; };
            Finde<Button>("KnopfSchliessenX").Click += delegate { Wurzel.Close(); };
            Finde<Button>("KnopfAlleUpdates").Click += delegate { Oeffnen(UpdatesSeite); };
            knopfDurchsuchen.Click += delegate { Durchsuchen(); };
            knopfSuchen.Click += delegate { VerzeichnisSuchen(); };
            knopfAnderer.Click += delegate { SucheBeenden(); Durchsuchen(); };
            // Die Knoepfe "Ja, verwenden" liegen in den Zeilen der Fundliste und tragen den Pfad im Tag
            fundListe.AddHandler(Button.ClickEvent, new RoutedEventHandler(delegate(object s, RoutedEventArgs e)
            {
                var knopf = e.OriginalSource as Button;
                if (knopf != null && knopf.Tag is string) Uebernehmen((string)knopf.Tag);
            }));
            // Updates rechts: Klick oeffnet die Seite des Updates im Browser
            neuListe.AddHandler(Button.ClickEvent, new RoutedEventHandler(delegate(object s, RoutedEventArgs e)
            {
                var knopf = e.OriginalSource as Button;
                if (knopf != null && knopf.Tag is string && (string)knopf.Tag != "") Oeffnen((string)knopf.Tag);
            }));
            knopfErneut.Click += delegate { ListeLaden(); };
            knopfSpielen.Click += delegate { HauptknopfGedrueckt(); };
            knopfGruendlich.Click += delegate { Starten(true); };
            knopfAbbrechen.Click += delegate { if (abbruch != null) abbruch.Cancel(); };
            knopfProtokoll.Click += delegate { Oeffnen(protokollDatei); };
            pfad.TextChanged += delegate { if (pfad.IsKeyboardFocusWithin) bestaetigt = true; VerzeichnisPruefen(); };
            name.TextChanged += delegate { NamePruefen(false); KnoepfeAktualisieren(); };
            Wurzel.AddHandler(Hyperlink.RequestNavigateEvent, new RequestNavigateEventHandler(LinkGeklickt));
            Wurzel.Closing += delegate(object s, CancelEventArgs e)
            {
                if (modus != Modus.Laeuft) return;
                var antwort = MessageBox.Show(Wurzel, "Der Launcher lädt gerade. Wirklich abbrechen?", Wurzel.Title,
                    MessageBoxButton.YesNo, MessageBoxImage.Question);
                if (antwort != MessageBoxResult.Yes) { e.Cancel = true; return; }
                if (abbruch != null) abbruch.Cancel();
            };

            if (optionen.Bildschirmfoto == "")
            {
                name.Text = Spielstart.GespeicherterName(optionen.Testmodus);
                // samplife:// bei jedem Start auf diese .exe zeigen lassen, damit „Verbinden“ auf der Website sie findet
                try { Spielstart.AdresseEintragen(optionen.Testmodus); }
                catch (Exception) { }
            }
            var gemerkt = optionen.Gta != "" ? optionen.Gta : einstellungen.Gta;
            if (gemerkt != "" && GtaVerzeichnis.IstGtaVerzeichnis(gemerkt)) Uebernehmen(gemerkt);
            else VerzeichnisPruefen();
            // Erst laden, wenn das Fenster laeuft: vorher gibt es keinen Kontext, der nach dem Warten
            // in den Thread der Oberflaeche zurueckfuehrt
            Wurzel.Loaded += delegate
            {
                // Sofort ein schon geladenes Bild zeigen; die Liste bringt spaeter weitere
                if (optionen.Bildschirmfoto == "") TitelbildZeigen(Titelbild.Gespeichertes());
                else if (optionen.Titelbild != "" && File.Exists(optionen.Titelbild)) TitelbildZeigen(File.ReadAllBytes(optionen.Titelbild));
                if (!optionen.OhneNetz) ListeLaden();
                if (!bestaetigt && (optionen.Bildschirmfoto == "" || optionen.Zustand == "ablauf")) VerzeichnisSuchen();
            };
        }

        T Finde<T>(string elementName) where T : class
        {
            var element = Wurzel.FindName(elementName) as T;
            if (element == null) throw new InvalidOperationException("Element " + elementName + " fehlt in Fenster.xaml");
            return element;
        }

        string Gta { get { return pfad.Text.Trim().Trim('"'); } }

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
            if (modus == Modus.Laeuft) return;
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
                PfadMeldung("\uE7BA", Farben.Warnung, "Kein GTA-Verzeichnis gefunden. Wähle den Ordner, in dem gta_sa.exe liegt.");
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
            // Kam die Liste nicht vom Server, hilft vielleicht die Sicherung in diesem Verzeichnis
            if (manifest == null && !listeLaedtGerade && !optionen.OhneNetz && Wurzel.IsLoaded) ListeLaden();
        }

        void VerzeichnisPruefen()
        {
            var verzeichnis = Gta;
            verzeichnisGueltig = false;
            brauchtAdmin = false;
            sampVersion = "";
            if (verzeichnis == "")
            {
                PfadMeldung("\uE946", Farben.Gedaempft, "Wähle den Ordner, in dem gta_sa.exe liegt.");
            }
            else if (!Directory.Exists(verzeichnis))
            {
                PfadMeldung("\uE711", Farben.Fehler, "Diesen Ordner gibt es nicht.");
            }
            else if (!GtaVerzeichnis.IstGtaVerzeichnis(verzeichnis))
            {
                PfadMeldung("\uE711", Farben.Fehler, "In diesem Ordner liegt keine gta_sa.exe.");
            }
            else
            {
                verzeichnisGueltig = true;
                brauchtAdmin = !GtaVerzeichnis.IstBeschreibbar(verzeichnis);
                var text = "gta_sa.exe gefunden";
                var farbe = Farben.Erfolg;
                var symbol = "\uE73E";
                bereichVersion.Visibility = Visibility.Collapsed;
                if (manifest != null)
                {
                    // Wie SA-MP startet, entscheidet derselbe Plan wie beim Klick auf „Spielen“ (Ablauf.Planen)
                    var plan = Ablauf.Planen(verzeichnis, manifest);
                    sampVersion = plan.Version;
                    switch (plan.Art)
                    {
                        case "exe":
                            text += " · SA-MP " + plan.Version;
                            break;
                        case "dll":
                            text += " · SA-MP " + plan.Version + (plan.VonOpenMp ? " aus dem open.mp-Launcher" : "");
                            break;
                        case "einrichten":
                            var groesse = Groesse(Manifest.PassendePakete(manifest.SampClient, plan.Version));
                            text += " · SA-MP fehlt, der Launcher richtet " + manifest.SampClient.Name + " mit ein"
                                + (groesse == "" ? "" : " (" + groesse + ")");
                            farbe = Farben.Akzent; symbol = "\uE946";
                            break;
                        default:
                            text += " · " + plan.Grund;
                            farbe = Farben.Warnung; symbol = "\uE7BA";
                            break;
                    }
                }
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

        bool NamePruefen(bool laut)
        {
            var fehler = Spielstart.NamePruefen(name.Text.Trim());
            if (fehler == "")
            {
                nameStatus.Text = "Passwort freiwillig: Damit entfällt der Login im Spiel.";
                nameStatus.Foreground = Farben.Leise;
                return true;
            }
            // Beim Tippen nicht gleich meckern, erst beim Klick auf „Spielen“
            nameStatus.Text = laut || name.Text.Length > 0 ? fehler : "Dein Name so, wie du auf dem Server heißt, z. B. Max_Mustermann";
            nameStatus.Foreground = laut ? Farben.Fehler : Farben.Leise;
            return false;
        }

        // ---------------------------------------------------------------- Liste der Komponenten

        async void ListeLaden()
        {
            if (listeLaedtGerade) return;
            listeLaedtGerade = true;
            manifest = null;
            komponenten.Clear();
            listeRollen.Visibility = Visibility.Collapsed;
            listeLaedt.Visibility = Visibility.Visible;
            knopfErneut.Visibility = Visibility.Collapsed;
            listeStatus.Foreground = Farben.Gedaempft;
            listeStatus.Text = "Lade die Liste der Dateien von " + optionen.ManifestQuelle + " …";
            KnoepfeAktualisieren();
            string fehlerText = "";
            Stand sicherung = null;
            var sicherungVersuchen = false;
            string json = null;
            try
            {
                json = await Task.Run(() => Programm.ListeHolen(optionen));
                var neu = Manifest.Lesen(json);
                if (Aktualisierung.Noetig(neu, optionen.NachAktualisierung))
                {
                    listeLaedtGerade = false;
                    SelbstAktualisieren(neu.Launcher);
                    return;
                }
                ausSicherung = false;
                hinweis.Visibility = Visibility.Collapsed;
                ManifestSetzen(neu, json);
            }
            catch (ManifestFehler fehler)
            {
                // Die Liste kam an, passt aber nicht: hier hilft keine Sicherung, sondern nur ein neuer Launcher
                fehlerText = fehler.Message;
            }
            catch (Exception fehler)
            {
                // Server nicht erreichbar: mit dem zuletzt geprueften Stand weiter (await geht im catch unter C# 5 nicht)
                fehlerText = fehler.Message;
                sicherungVersuchen = verzeichnisGueltig;
            }
            if (sicherungVersuchen)
            {
                var gta = Gta;
                sicherung = await Task.Run(() => Stand.Lesen(gta));
            }
            listeLaedtGerade = false;

            if (manifest == null && sicherung != null)
            {
                try
                {
                    ausSicherung = true;
                    ManifestSetzen(Manifest.Lesen(sicherung.ManifestJson), sicherung.ManifestJson);
                    HinweisZeigen("Die Server-Liste ist gerade nicht erreichbar. Du spielst mit dem geprüften Stand vom "
                        + sicherung.Geprueft.ToLocalTime().ToString("dd.MM.yyyy, HH:mm", CultureInfo.InvariantCulture)
                        + ". Fehlt eine Datei, kann der Launcher sie erst wieder laden, wenn der Server antwortet.");
                }
                catch (Exception) { manifest = null; ausSicherung = false; }
            }
            if (manifest != null) DirektVersuchen();
            if (manifest == null)
            {
                // Den Fehler zeigen, auch wenn die Liste sonst hinter „Was wird geladen?“ versteckt ist
                if (bereichAuswahl.Visibility != Visibility.Visible) DateienUmschalten();
                listeStatus.Foreground = Farben.Fehler;
                listeStatus.Text = "Die Liste der Dateien konnte nicht geladen werden.\n" + fehlerText
                    + (verzeichnisGueltig ? "" : "\nBestätige oben dein GTA-Verzeichnis, dann versucht der Launcher es mit dem zuletzt geprüften Stand.");
                knopfErneut.Visibility = Visibility.Visible;
            }
            KnoepfeAktualisieren();
        }

        void HinweisZeigen(string text)
        {
            hinweisText.Text = text;
            hinweis.Visibility = Visibility.Visible;
        }

        public void ManifestSetzen(Manifest neu, string json)
        {
            manifest = neu;
            manifestJson = json;
            komponenten.Clear();
            foreach (var komponente in manifest.Alle())
            {
                var zeile = new KomponenteZeile { Komponente = komponente };
                // Pflicht immer, optional nur, was der Spieler einmal gewaehlt hat
                zeile.Gewaehlt = !komponente.Optional || einstellungen.Optional.Contains(komponente.Id);
                zeile.AuswahlGeaendert = delegate { KomponentenAbgleichen(); KnoepfeAktualisieren(); };
                komponenten.Add(zeile);
            }
            listeLaedt.Visibility = Visibility.Collapsed;
            listeRollen.Visibility = Visibility.Visible;
            if (manifest.Hinweis != "") fusszeile.Text = manifest.Hinweis;
            versionen.Children.Clear();
            var namen = new List<string>();
            foreach (var version in manifest.SampVersionen) namen.Add(version.Name);
            var unterstuetzt = string.Join(" oder ", namen.ToArray());
            nurEineVersion = namen.Count == 1;
            if (nurEineVersion)
            {
                // Nichts zu waehlen: die eine Version gilt
                versionTitel.Text = "Wichtig: Du brauchst SA-MP " + unterstuetzt;
                versionTitel.Foreground = Farben.Warnung;
                versionPflicht.Text = "Der Server läuft ausschließlich mit SA-MP " + unterstuetzt + ".";
                versionHinweis.Text = "Der Launcher erkennt die Version an der samp.dll in deinem GTA-Verzeichnis. Liegt dort eine andere Version, "
                    + "zum Beispiel 0.3.7-R3 oder 0.3.DL, installiere SA-MP " + unterstuetzt + " in diesen Ordner.";
                versionen.Visibility = Visibility.Collapsed;
            }
            else
            {
                versionTitel.Text = "Mit welcher SA-MP-Version spielst du?";
                versionTitel.Foreground = Farben.Text;
                versionPflicht.Text = "Die Erweiterungen laufen nur mit SA-MP " + unterstuetzt + ".";
                versionHinweis.Text = "Wähle die Version, die in deinem GTA-Verzeichnis liegt.";
                versionen.Visibility = Visibility.Visible;
            }
            foreach (var version in manifest.SampVersionen)
            {
                var knopf = new RadioButton { Content = version.Name, GroupName = "SampVersion", Style = (Style)Wurzel.FindResource("Wahlknopf") };
                knopf.Checked += delegate { VerzeichnisPruefen(); };
                versionen.Children.Add(knopf);
            }
            VerzeichnisPruefen();
            TitelbildLaden(manifest.Bilder);
            PasswortAbfragen(manifest.ServerAdresse, manifest.ServerPort);
            if (manifest.Updates != "" && neuigkeiten.Count == 0) NeuigkeitenLaden(manifest.Updates);
            else if (manifest.Updates == "") neuStatus.Text = "Die Updates findest du in der Community auf unserer Website.";
        }

        /// <summary>Groesse, Verfuegbarkeit und – wenn schon moeglich – der Stand jeder Komponente auf der Platte.</summary>
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
                    zeile.InfoFarbe = Farben.Warnung;
                }
                else
                {
                    zeile.Verfuegbar = true;
                    if (!zeile.Gewaehlt) { zeile.Info = "nicht gewählt"; zeile.InfoFarbe = Farben.Leise; }
                    else { zeile.Info = Groesse(pakete); zeile.InfoFarbe = Farben.Gedaempft; }
                }
            }
            VorschauPruefen();
        }

        static string Groesse(List<Paket> pakete)
        {
            long groesse = 0;
            foreach (var paket in pakete) groesse += paket.Groesse;
            return groesse > 0 ? Installation.Groesse(groesse) : "";
        }

        List<string> GewaehlteOptionale()
        {
            var liste = new List<string>();
            foreach (var zeile in komponenten) if (zeile.Komponente.Optional && zeile.Gewaehlt) liste.Add(zeile.Komponente.Id);
            return liste;
        }

        /// <summary>Prueft im Hintergrund schnell, was aktuell ist und was der Launcher vor dem Start laden muss.</summary>
        async void VorschauPruefen()
        {
            var runde = ++pruefRunde;
            if (manifest == null || !bestaetigt || !verzeichnisGueltig || sampVersion == "" || modus == Modus.Laeuft) return;
            var gta = Gta;
            var version = sampVersion;
            var verwaltet = manifest.Verwaltet(GewaehlteOptionale());
            List<Pruefung> ergebnis;
            try { ergebnis = await Task.Run(() => Stand.Abgleichen(gta, Stand.Lesen(gta), verwaltet, version, false, CancellationToken.None)); }
            catch (Exception) { return; }
            if (runde != pruefRunde) return;
            PruefungZeigen(ergebnis);
        }

        void PruefungZeigen(List<Pruefung> ergebnis)
        {
            long nachladen = 0;
            var alleAktuell = true;
            foreach (var pruefung in ergebnis)
            {
                foreach (var zeile in komponenten)
                {
                    if (zeile.Komponente != pruefung.Komponente) continue;
                    var pakete = Manifest.PassendePakete(zeile.Komponente, sampVersion);
                    if (pruefung.Zustand == Zustand.Aktuell)
                    {
                        zeile.Info = "aktuell";
                        zeile.InfoFarbe = ausSicherung ? Farben.Gedaempft : Farben.Erfolg;
                    }
                    else
                    {
                        alleAktuell = false;
                        foreach (var paket in pakete) nachladen += paket.Groesse;
                        zeile.Info = Ablauf.ZustandText(pruefung) + (Groesse(pakete) == "" ? "" : " · " + Groesse(pakete));
                        zeile.InfoFarbe = pruefung.Zustand == Zustand.Fehlt ? Farben.Gedaempft : Farben.Warnung;
                    }
                }
            }
            if (modus != Modus.Spielen) return;
            if (alleAktuell)
            {
                titel.Text = "Bereit zum Spielen";
                untertitel.Text = "Alle Dateien sind auf dem Stand des Servers. Mit „Spielen“ startet SA-MP und verbindet dich mit "
                    + manifest.ServerAdresse + ".";
            }
            else
            {
                titel.Text = "Bereit machen";
                untertitel.Text = ausSicherung
                    ? "Es fehlen Dateien, und die Server-Liste ist gerade nicht erreichbar. Versuche es gleich noch einmal."
                    : "Vor dem Start lädt der Launcher " + (nachladen > 0 ? Installation.Groesse(nachladen) : "die fehlenden Dateien") + " nach.";
            }
        }

        /// <summary>
        /// Laedt ein Bild aus der Liste, bevorzugt eins, das noch nicht gespeichert ist. Zeigt es nur, wenn noch keins zu sehen
        /// ist; sonst liegt es fuer den naechsten Start bereit.
        /// </summary>
        async void TitelbildLaden(List<string> adressen)
        {
            if (optionen.Bildschirmfoto != "") return;
            var url = Titelbild.Auswaehlen(adressen);
            if (url == "") return;
            var testmodus = optionen.Testmodus;
            byte[] daten;
            try { daten = await Task.Run(() => Titelbild.Laden(url, testmodus)); }
            catch (Exception) { return; }
            if (!titelbildGezeigt) TitelbildZeigen(daten);
        }

        void TitelbildZeigen(byte[] daten)
        {
            if (daten == null) return;
            try
            {
                var bild = new BitmapImage();
                bild.BeginInit();
                bild.CacheOption = BitmapCacheOption.OnLoad;
                bild.DecodePixelWidth = 1600;
                bild.StreamSource = new MemoryStream(daten);
                bild.EndInit();
                bild.Freeze();
                titelbild.Source = bild;
                titelbildGezeigt = true;
                // Fuer Bildschirmfotos sofort sichtbar, sonst sanft einblenden
                if (optionen.Bildschirmfoto != "") titelbild.Opacity = 1;
                else titelbild.BeginAnimation(UIElement.OpacityProperty, new DoubleAnimation(0, 1, TimeSpan.FromMilliseconds(450)));
            }
            catch (Exception) { }
        }

        /// <summary>„Was wird geladen?“: die Liste der Dateien ein- und ausblenden.</summary>
        void DateienUmschalten()
        {
            var zeigen = bereichAuswahl.Visibility != Visibility.Visible;
            bereichAuswahl.Visibility = zeigen ? Visibility.Visible : Visibility.Collapsed;
            knopfDateien.Content = zeigen ? "Liste ausblenden" : "Was wird geladen?";
        }

        /// <summary>Verlangt der Server ein Passwort, ist das Kaestchen „Serverpasswort“ schon angehakt.</summary>
        async void PasswortAbfragen(string adresse, int port)
        {
            if (optionen.Bildschirmfoto != "" || adresse == "" || port <= 0) return;
            bool? verlangt;
            try { verlangt = await Task.Run(() => SampStart.VerlangtPasswort(adresse, port)); }
            catch (Exception) { return; }
            if (verlangt == true && modus == Modus.Spielen) serverpasswortHaken.IsChecked = true;
        }

        /// <summary>Ueber samplife://verbinden gestartet: Ist alles bereit, startet der Launcher ohne Klick.</summary>
        void DirektVersuchen()
        {
            if (!optionen.Direkt || direktVersucht || manifest == null) return;
            direktVersucht = true;
            if (modus == Modus.Spielen && bestaetigt && verzeichnisGueltig && !brauchtAdmin && NamePruefen(false))
            {
                Starten(false);
                return;
            }
            untertitel.Text = "Bevor der Launcher dich direkt verbinden kann, bestätige dein GTA-Verzeichnis und gib deinen Namen ein. "
                + "Dann auf „Spielen“.";
        }

        async void NeuigkeitenLaden(string url)
        {
            neuStatus.Text = "Lade die neuesten Updates …";
            neuStatus.Visibility = Visibility.Visible;
            try
            {
                var testmodus = optionen.Testmodus;
                var liste = await Task.Run(() => Neuigkeiten.Laden(url, testmodus, 6));
                neuigkeiten.Clear();
                foreach (var neuigkeit in liste) neuigkeiten.Add(neuigkeit);
                neuStatus.Visibility = neuigkeiten.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
                if (neuigkeiten.Count == 0) neuStatus.Text = "Noch keine Updates.";
            }
            catch (Exception)
            {
                neuStatus.Text = "Die Updates sind gerade nicht erreichbar. Du findest sie in der Community auf unserer Website.";
            }
        }

        // ---------------------------------------------------------------- Spielen

        void KnoepfeAktualisieren()
        {
            if (modus != Modus.Spielen) return;
            knopfSpielen.Content = brauchtAdmin ? "Als Administrator starten" : "Spielen";
            var bereit = bestaetigt && verzeichnisGueltig && manifest != null;
            knopfSpielen.IsEnabled = bereit;
            knopfGruendlich.IsEnabled = bereit && !brauchtAdmin;
        }

        void HauptknopfGedrueckt()
        {
            switch (modus)
            {
                case Modus.Schliessen: Wurzel.Close(); return;
                case Modus.Download: Oeffnen(DownloadSeite); return;
                case Modus.Erneut: ZurueckZurListe(); return;
                case Modus.Laeuft: return;
            }
            if (brauchtAdmin) { AlsAdministratorStarten(); return; }
            Starten(false);
        }

        void AlsAdministratorStarten()
        {
            try
            {
                var start = new ProcessStartInfo(Aktualisierung.EigeneDatei);
                start.Verb = "runas";
                start.Arguments = "--gta \"" + Gta + "\"" + optionen.AlsArgumente();
                Process.Start(start);
                Wurzel.Close();
            }
            catch (Win32Exception)
            {
                // Abfrage der Benutzerkontensteuerung abgelehnt
            }
        }

        /// <summary>„Spielen“ (abgleichen, laden, anmelden, starten) oder „Gründlich prüfen“ (nur abgleichen und reparieren).</summary>
        async void Starten(bool nurPruefen)
        {
            if (manifest == null || modus != Modus.Spielen) return;
            if (!nurPruefen && !NamePruefen(true)) { name.Focus(); return; }

            einstellungen.Gta = Gta;
            einstellungen.Optional = GewaehlteOptionale();
            einstellungen.Speichern();

            modus = Modus.Laeuft;
            ++pruefRunde;
            abbruch = new CancellationTokenSource();
            zeilen.Clear();
            kurzZeilen.Clear();
            FortschrittZeigen(0);
            bereichProtokoll.Visibility = Visibility.Visible;
            Sperren(true);
            knopfSpielen.IsEnabled = false;
            knopfSpielen.Content = nurPruefen ? "Prüfe …" : "Einen Moment …";
            knopfAbbrechen.Visibility = Visibility.Visible;
            knopfProtokoll.Visibility = Visibility.Collapsed;
            titel.Text = nurPruefen ? "Gründlich prüfen" : "Bereit machen";
            untertitel.Text = nurPruefen
                ? "Der Launcher rechnet die Prüfsumme jeder Datei nach und lädt neu, was nicht stimmt."
                : "Der Launcher gleicht deine Dateien mit dem Server ab, lädt nach, was fehlt, und startet dann SA-MP.";

            var ablauf = new Ablauf
            {
                Gta = Gta, SampVersion = sampVersion, Name = name.Text.Trim(), Manifest = manifest, ManifestJson = manifestJson,
                Passwort = serverpasswortHaken.IsChecked == true ? passwort.Password : "",
                Kontopasswort = kontopasswort.Password,
                AusSicherung = ausSicherung, Optional = einstellungen.Optional, Gruendlich = nurPruefen, MitStart = !nurPruefen,
                Testmodus = optionen.Testmodus
            };
            // Progress leitet die Meldungen aus dem Hintergrund-Thread in den Thread der Oberflaeche
            var meldungen = new Progress<Meldung>(MeldungZeigen);
            var fortschritt = new Progress<double>(FortschrittZeigen);
            var geprueft = new Progress<List<Pruefung>>(PruefungZeigen);
            ablauf.Melden = m => ((IProgress<Meldung>)meldungen).Report(m);
            ablauf.Fortschritt = f => ((IProgress<double>)fortschritt).Report(f);
            ablauf.Geprueft = p => ((IProgress<List<Pruefung>>)geprueft).Report(p);

            var token = abbruch.Token;
            var ergebnis = await Task.Run(() => ablauf.Ausfuehren(token));
            // Die letzten Meldungen laufen ueber dieselbe Warteschlange; kurz abwarten, bis sie angezeigt sind
            await Task.Delay(150);

            protokollDatei = ergebnis.Protokoll;
            knopfAbbrechen.Visibility = Visibility.Collapsed;
            knopfProtokoll.Visibility = protokollDatei == "" ? Visibility.Collapsed : Visibility.Visible;
            knopfSpielen.IsEnabled = true;
            if (ergebnis.Erfolg && ergebnis.Gestartet)
            {
                FortschrittZeigen(1);
                modus = Modus.Schliessen;
                knopfSpielen.Content = "Schließen";
                titel.Text = "Viel Spaß auf German Samplife";
                if (ergebnis.Warnung != "")
                {
                    // Spiel laeuft, aber die Anmeldung fehlt: Fenster offen lassen, damit der Grund lesbar bleibt
                    untertitel.Text = "SA-MP startet. Die Anmeldung beim Server hat aber nicht geklappt, deshalb lässt dich der Server "
                        + "vielleicht nicht herein. Der Grund steht unten.";
                    return;
                }
                untertitel.Text = "SA-MP startet und verbindet dich mit " + manifest.ServerAdresse + ". Der Launcher schließt sich gleich.";
                var zu = new DispatcherTimer { Interval = TimeSpan.FromSeconds(3) };
                zu.Tick += delegate { zu.Stop(); Wurzel.Close(); };
                zu.Start();
                return;
            }
            if (ergebnis.Erfolg)
            {
                // Nur geprueft: zurueck zur Liste mit dem frischen Stand
                FortschrittZeigen(1);
                modus = Modus.Erneut;
                knopfSpielen.Content = "Weiter";
                titel.Text = "Geprüft";
                untertitel.Text = ergebnis.Nachgeladen > 0
                    ? "Der Launcher hat " + ergebnis.Nachgeladen + (ergebnis.Nachgeladen == 1 ? " Komponente" : " Komponenten") + " neu geladen."
                    : "Alle Dateien stimmen.";
                return;
            }
            modus = Modus.Erneut;
            knopfSpielen.Content = "Zurück";
            if (ergebnis.Code == 8) kontopasswort.Clear();
            titel.Text = ergebnis.Abgebrochen ? "Abgebrochen" : ergebnis.Code == 8 ? "Anmeldung nicht möglich" : "Das hat nicht geklappt";
            untertitel.Text = ergebnis.Abgebrochen ? "Bereits geladene Dateien bleiben im GTA-Verzeichnis." : ergebnis.Fehler;
        }

        void ZurueckZurListe()
        {
            modus = Modus.Spielen;
            bereichProtokoll.Visibility = Visibility.Collapsed;
            protokollFenster.Visibility = Visibility.Collapsed;
            knopfProtokoll.Visibility = Visibility.Collapsed;
            Sperren(false);
            titel.Text = "Bereit machen";
            untertitel.Text = "Der Launcher hält dein GTA auf dem Stand des Servers und verbindet dich direkt.";
            KomponentenAbgleichen();
            KnoepfeAktualisieren();
        }

        void Sperren(bool gesperrt)
        {
            pfad.IsEnabled = !gesperrt;
            name.IsEnabled = !gesperrt;
            passwort.IsEnabled = !gesperrt;
            kontopasswort.IsEnabled = !gesperrt;
            serverpasswortHaken.IsEnabled = !gesperrt;
            knopfDurchsuchen.IsEnabled = !gesperrt;
            knopfSuchen.IsEnabled = !gesperrt;
            knopfGruendlich.IsEnabled = !gesperrt;
            bereichVersion.IsEnabled = !gesperrt;
            foreach (var zeile in komponenten) zeile.Gesperrt = gesperrt;
        }

        /// <summary>Die Liste nennt eine neuere Fassung: laden, pruefen, ersetzen und neu starten.</summary>
        async void SelbstAktualisieren(LauncherFassung fassung)
        {
            modus = Modus.Laeuft;
            zeilen.Clear();
            kurzZeilen.Clear();
            bereichProtokoll.Visibility = Visibility.Visible;
            Sperren(true);
            knopfSpielen.IsEnabled = false;
            knopfSpielen.Content = "Aktualisiere …";
            titel.Text = "Neue Fassung des Launchers";
            untertitel.Text = "Der Launcher aktualisiert sich auf " + fassung.Version + " und startet danach von selbst neu.";
            var zeile = new Meldung { Nummer = 1, Art = MeldungArt.Download, Text = "Lade Samplife Launcher " + fassung.Version + " herunter", Url = fassung.Url, Gesamt = fassung.Groesse };
            MeldungZeigen(zeile);
            var meldungen = new Progress<Meldung>(MeldungZeigen);
            var fortschritt = new Progress<double>(FortschrittZeigen);
            var testmodus = optionen.Testmodus;
            try
            {
                await Task.Run(() => Aktualisierung.Ausfuehren(fassung, testmodus, delegate(long erledigt, long gesamt)
                {
                    ((IProgress<Meldung>)meldungen).Report(new Meldung { Nummer = 1, Art = MeldungArt.Download, Text = zeile.Text, Url = zeile.Url, Erledigt = erledigt, Gesamt = gesamt });
                    if (gesamt > 0) ((IProgress<double>)fortschritt).Report((double)erledigt / gesamt);
                }, CancellationToken.None));
                MeldungZeigen(new Meldung { Nummer = 2, Art = MeldungArt.Erfolg, Text = "Prüfsumme stimmt. Starte die neue Fassung …", Fertig = true });
                await Task.Delay(600);
                Aktualisierung.NeuStarten(optionen.AlsArgumente() + (Gta != "" && bestaetigt ? " --gta \"" + Gta + "\"" : ""), fassung.Version);
                modus = Modus.Schliessen;
                Wurzel.Close();
            }
            catch (Exception fehler)
            {
                MeldungZeigen(new Meldung { Nummer = 3, Art = MeldungArt.Fehler, Text = "Die Aktualisierung hat nicht geklappt: " + fehler.Message, Fertig = true });
                modus = Modus.Download;
                knopfSpielen.IsEnabled = true;
                knopfSpielen.Content = "Download öffnen";
                titel.Text = "Bitte den neuen Launcher laden";
                untertitel.Text = "Diese Fassung ist zu alt für den Server, und die Aktualisierung hat nicht geklappt. Lade den aktuellen Launcher "
                    + "von GitHub und starte ihn. Deine Dateien bleiben, wie sie sind.";
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
                // Auf der Seite nur die letzten drei Zeilen
                kurzZeilen.Add(zeile);
                while (kurzZeilen.Count > 3) kurzZeilen.RemoveAt(0);
            }
            else zeile.Uebernehmen(m);
        }

        public void FortschrittZeigen(double anteil)
        {
            var breite = ((FrameworkElement)balken.Parent).ActualWidth;
            if (breite <= 0) breite = 680;
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

            // Beispieldaten fuer die uebrigen Zustaende
            name.Text = "Max_Mustermann";
            pfad.Text = @"D:\Spiele\GTA San Andreas";
            PfadMeldung("\uE73E", Farben.Erfolg, "gta_sa.exe gefunden · SA-MP 0.3.7-R1");
            neuStatus.Visibility = Visibility.Collapsed;
            neuigkeiten.Clear();
            neuigkeiten.Add(new Neuigkeit { Titel = "Eigenes Profilbild, Avatar mit Initial", Kurz = "Lade auf deinem Profil ein eigenes Profilbild hoch.", Datum = new DateTime(2026, 10, 21), Url = UpdatesSeite });
            neuigkeiten.Add(new Neuigkeit { Titel = "Bewusstlose im Fahrzeug mitnehmen", Kurz = "Wen du mit K trägst, nimmst du jetzt mit ins Fahrzeug.", Datum = new DateTime(2026, 10, 21), Url = UpdatesSeite });
            neuigkeiten.Add(new Neuigkeit { Titel = "Fahrzeugmenü und F3-Menü mit der Maus", Kurz = "Bedienen mit der Maus, weiterfahren mit der Tastatur.", Datum = new DateTime(2026, 10, 20), Url = UpdatesSeite });
            komponenten.Clear();
            var beispiel = new[]
            {
                new Komponente { Id = "sampvoice", Name = "SampVoice 3.1", Beschreibung = "Sprach-Chat im Spiel: Wer in deiner Nähe steht, hört dich.", Quelle = "https://github.com/German-Samplife/samplife-essentials/tree/main/sampvoice", Lizenz = "MIT" },
                new Komponente { Id = "cef", Name = "Browser-Oberfläche (CEF)", Beschreibung = "Menüs und Anzeigen des Servers als Oberfläche im Spiel.", Quelle = "https://github.com/German-Samplife/samplife-essentials/tree/main/cef", Lizenz = "GPL-3.0" },
                new Komponente { Id = "kleidung", Name = "CJ-Kleidung", Beschreibung = "Zeigt die Kleidung von CJ so, wie der Server sie vorgibt.", Quelle = "", Lizenz = "" },
            };
            foreach (var komponente in beispiel)
            {
                komponente.Pakete.Add(new Paket { Samp = "", Groesse = komponente.Id == "cef" ? 152101349 : 893058 });
                komponenten.Add(new KomponenteZeile { Komponente = komponente, Gewaehlt = true });
            }
            listeLaedt.Visibility = Visibility.Collapsed;
            listeRollen.Visibility = Visibility.Visible;
            knopfSpielen.IsEnabled = true;
            knopfGruendlich.IsEnabled = true;

            if (zustand == "passwort") serverpasswortHaken.IsChecked = true;
            if (zustand == "dateien") DateienUmschalten();
            if (zustand == "bereit" || zustand == "offline" || zustand == "passwort" || zustand == "dateien")
            {
                foreach (var zeile in komponenten) { zeile.Info = "aktuell"; zeile.InfoFarbe = zustand == "offline" ? Farben.Gedaempft : Farben.Erfolg; }
                titel.Text = "Bereit zum Spielen";
                untertitel.Text = "Alle Dateien sind auf dem Stand des Servers. Mit „Spielen“ startet SA-MP und verbindet dich mit german-samplife.de.";
                if (zustand == "offline")
                    HinweisZeigen("Die Server-Liste ist gerade nicht erreichbar. Du spielst mit dem geprüften Stand vom 21.10.2026, 13:40. "
                        + "Fehlt eine Datei, kann der Launcher sie erst wieder laden, wenn der Server antwortet.");
                return;
            }

            bereichProtokoll.Visibility = Visibility.Visible;
            knopfSpielen.Content = "Einen Moment …";
            knopfSpielen.IsEnabled = false;
            knopfAbbrechen.Visibility = Visibility.Visible;
            titel.Text = "Bereit machen";
            untertitel.Text = "Der Launcher gleicht deine Dateien mit dem Server ab, lädt nach, was fehlt, und startet dann SA-MP.";
            var n = 1;
            MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Schritt, Text = "Prüfe die Dateien im GTA-Verzeichnis", Fertig = true });
            MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Schritt, Text = "Browser-Oberfläche (CEF): neue Fassung", Fertig = true });
            if (zustand == "fertig")
            {
                MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Download, Text = "Lade Browser-Oberfläche (CEF) herunter", Url = "https://github.com/German-Samplife/samplife-essentials/releases/download/cef-f14fe4f-sl4/samp-cef-client-cef-f14fe4f-sl4.zip", Erledigt = 152101349, Gesamt = 152101349, Fertig = true });
                MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Schritt, Text = "Prüfsumme von Browser-Oberfläche (CEF) stimmt (145,1 MB)", Fertig = true });
                MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Erfolg, Text = "Fertig. 3 Dateien geschrieben, 212 waren schon aktuell, 3 vorhandene Dateien gesichert.", Fertig = true });
                MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Schritt, Text = "Melde Max_Mustermann beim Server an", Fertig = true });
                MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Schritt, Text = "Starte SA-MP und verbinde mit german-samplife.de:7777", Fertig = true });
                FortschrittZeigen(1);
                knopfAbbrechen.Visibility = Visibility.Collapsed;
                knopfProtokoll.Visibility = Visibility.Visible;
                knopfSpielen.Content = "Schließen";
                knopfSpielen.IsEnabled = true;
                titel.Text = "Viel Spaß auf German Samplife";
                untertitel.Text = "SA-MP startet und verbindet dich mit german-samplife.de. Der Launcher schließt sich gleich.";
            }
            else
            {
                MeldungZeigen(new Meldung { Nummer = n++, Art = MeldungArt.Download, Text = "Lade Browser-Oberfläche (CEF) herunter", Url = "https://github.com/German-Samplife/samplife-essentials/releases/download/cef-f14fe4f-sl4/samp-cef-client-cef-f14fe4f-sl4.zip", Erledigt = 63963136, Gesamt = 152101349 });
                FortschrittZeigen(0.42);
                if (zustand == "mehr") protokollFenster.Visibility = Visibility.Visible;
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
