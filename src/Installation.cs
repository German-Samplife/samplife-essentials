using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.IO.Compression;
using System.Net;
using System.Security.Cryptography;
using System.Text;
using System.Threading;

namespace SamplifeEssentials
{
    public enum MeldungArt { Schritt, Download, Datei, Erfolg, Warnung, Fehler }

    /// <summary>Eine Zeile im Protokollfeld. Meldungen mit derselben Nummer aktualisieren dieselbe Zeile.</summary>
    public class Meldung
    {
        public int Nummer;
        public MeldungArt Art;
        public string Text;
        public string Url;
        public long Erledigt;
        public long Gesamt;
        public bool Fertig;
    }

    public class Ergebnis
    {
        public bool Erfolg;
        public bool Abgebrochen;
        public string Fehler = "";
        public int Geschrieben;
        public int Unveraendert;
        public int Gesichert;
        public string Sicherung = "";
        public string Protokoll = "";
    }

    public class InstallationsFehler : Exception
    {
        public InstallationsFehler(string meldung) : base(meldung) { }
    }

    /// <summary>
    /// Laedt die gewaehlten Komponenten herunter, prueft sie und schreibt sie ins GTA-Verzeichnis.
    /// Laeuft synchron; die Oberflaeche ruft Ausfuehren in einem Hintergrund-Thread auf.
    /// </summary>
    public class Installation
    {
        public const string ProtokollDatei = "samplife-essentials-protokoll.txt";
        public const string SicherungsOrdner = "samplife-essentials-sicherung";

        // Diese Dateien gehoeren zum Spiel bzw. zu SA-MP und werden nie ersetzt. SA-MP darf nur das Paket samp_client
        // ablegen, und nur dort, wo noch keine Datei liegt.
        static readonly string[] Geschuetzt = { "gta_sa.exe", "samp.dll", "samp.exe", "samp.saa" };

        readonly string gta;
        readonly string sampVersion;
        readonly IList<Komponente> auswahl;
        readonly bool testmodus;
        readonly string temp;
        readonly StringBuilder protokoll = new StringBuilder();
        readonly List<string> installiert = new List<string>();
        int naechsteNummer = 1;
        long bytesGesamt;
        long bytesErledigt;
        string sicherung = "";
        Ergebnis ergebnis = new Ergebnis();
        // Was der Launcher angelegt hat und wie es vorher aussah (fuer „Deinstallieren“)
        Installiert verzeichnis;

        // Je installierter Komponente die Pakete und ihre Dateien, fuer die Sicherung des Stands
        public readonly List<KomponenteStand> Staende = new List<KomponenteStand>();

        /// <summary>
        /// Andere SA-MP-Version im GTA-Verzeichnis, der Spieler hat dem Ersetzen zugestimmt (Ablauf.SampErsetzen): Vor dem
        /// ersten Paket von samp_client wandern samp.dll und samp.exe in die Sicherung, und samp_client ersetzt seine
        /// Dateien wie jede andere Komponente (mit Sicherung), statt Vorhandenes liegen zu lassen.
        /// </summary>
        public bool SampErsetzen;
        bool sampGesichert;

        public Action<Meldung> Melden = delegate { };
        public Action<double> Fortschritt = delegate { };

        public Installation(string gtaVerzeichnis, string sampVersion, IList<Komponente> auswahl, bool testmodus)
        {
            gta = Path.GetFullPath(gtaVerzeichnis).TrimEnd(Path.DirectorySeparatorChar);
            this.sampVersion = sampVersion;
            this.auswahl = auswahl;
            this.testmodus = testmodus;
            temp = Path.Combine(Path.GetTempPath(), "samplife-essentials-" + Guid.NewGuid().ToString("N"));
        }

        public Ergebnis Ausfuehren(CancellationToken abbruch)
        {
            ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
            try
            {
                Schritt("Prüfe das GTA-Verzeichnis " + gta);
                if (!GtaVerzeichnis.IstGtaVerzeichnis(gta))
                    throw new InstallationsFehler("In diesem Verzeichnis liegt keine gta_sa.exe.");
                if (!GtaVerzeichnis.IstBeschreibbar(gta))
                    throw new InstallationsFehler("In dieses Verzeichnis darf nicht geschrieben werden. Starte den Launcher als Administrator.");

                verzeichnis = Installiert.Laden(gta);

                var auftraege = new List<KeyValuePair<Komponente, Paket>>();
                // Systemvoraussetzungen zuerst: Lehnt der Spieler die Abfrage von Windows ab, ist noch nichts Grosses geladen
                var reihenfolge = new List<Komponente>();
                foreach (var komponente in auswahl) if (komponente.IstSystem) reihenfolge.Add(komponente);
                foreach (var komponente in auswahl) if (!komponente.IstSystem) reihenfolge.Add(komponente);
                foreach (var komponente in reihenfolge)
                {
                    var pakete = Manifest.PassendePakete(komponente, sampVersion);
                    if (pakete.Count == 0)
                        throw new InstallationsFehler(komponente.Name + " gibt es nicht für deine SA-MP-Version"
                            + (sampVersion == "" ? "." : " (" + sampVersion + ")."));
                    foreach (var paket in pakete)
                    {
                        auftraege.Add(new KeyValuePair<Komponente, Paket>(komponente, paket));
                        bytesGesamt += paket.Groesse;
                    }
                }
                if (auftraege.Count == 0) throw new InstallationsFehler("Es ist keine Komponente ausgewählt.");

                Directory.CreateDirectory(temp);
                foreach (var auftrag in auftraege)
                {
                    abbruch.ThrowIfCancellationRequested();
                    var datei = Herunterladen(auftrag.Key, auftrag.Value, abbruch);
                    if (auftrag.Key.IstSystem)
                    {
                        // Gehoert zu Windows: nicht ins GTA-Verzeichnis, nicht in den Stand
                        Systemvoraussetzung.Einrichten(auftrag.Key, datei, gta, testmodus, Schritt);
                        Schritt(auftrag.Key.Name + " ist eingerichtet");
                        continue;
                    }
                    // Erst jetzt, wo das Paket geladen und geprueft ist: das bisherige SA-MP aus dem Weg raeumen
                    if (auftrag.Key.IstSampClient && SampErsetzen && !sampGesichert)
                    {
                        SampAltesSichern();
                        sampGesichert = true;
                    }
                    var paketStand = new PaketStand { Url = auftrag.Value.Url, Sha256 = auftrag.Value.Sha256 };
                    if (auftrag.Value.Typ == "zip") Entpacken(auftrag.Key, auftrag.Value, datei, paketStand, abbruch);
                    else DateiAblegen(auftrag.Key, auftrag.Value, datei, paketStand);
                    StandVon(auftrag.Key).Pakete.Add(paketStand);
                }

                ergebnis.Erfolg = true;
                Melden(new Meldung
                {
                    Nummer = naechsteNummer++, Art = MeldungArt.Erfolg, Fertig = true,
                    Text = "Fertig. " + Anzahl(ergebnis.Geschrieben, "Datei", "Dateien") + " geschrieben, "
                        + ergebnis.Unveraendert + (ergebnis.Unveraendert == 1 ? " war" : " waren") + " schon aktuell"
                        + (ergebnis.Gesichert > 0 ? ", " + Anzahl(ergebnis.Gesichert, "vorhandene Datei", "vorhandene Dateien") + " gesichert." : ".")
                });
                Notieren("Fertig: " + ergebnis.Geschrieben + " geschrieben, " + ergebnis.Unveraendert + " unverändert, " + ergebnis.Gesichert + " gesichert");
            }
            catch (OperationCanceledException)
            {
                ergebnis.Abgebrochen = true;
                ergebnis.Fehler = "Die Installation wurde abgebrochen.";
                Fehlermeldung(ergebnis.Fehler + " Bereits geschriebene Dateien bleiben im GTA-Verzeichnis.");
            }
            catch (InstallationsFehler fehler)
            {
                ergebnis.Fehler = fehler.Message;
                Fehlermeldung(fehler.Message);
            }
            catch (UnauthorizedAccessException)
            {
                ergebnis.Fehler = "Zugriff verweigert. Starte den Launcher als Administrator.";
                Fehlermeldung(ergebnis.Fehler);
            }
            catch (IOException fehler)
            {
                ergebnis.Fehler = "Eine Datei konnte nicht geschrieben werden: " + fehler.Message
                    + " Schließe GTA San Andreas und SA-MP und versuche es erneut.";
                Fehlermeldung(ergebnis.Fehler);
            }
            catch (Exception fehler)
            {
                ergebnis.Fehler = "Unerwarteter Fehler: " + fehler.Message;
                Fehlermeldung(ergebnis.Fehler);
            }
            finally
            {
                try { if (Directory.Exists(temp)) Directory.Delete(temp, true); }
                catch (Exception) { }
                // Auch nach einem Fehler: Was schon geschrieben ist, muss die Deinstallation kennen
                try { if (verzeichnis != null) verzeichnis.Speichern(gta); }
                catch (Exception fehler) { Notieren("Die Liste der installierten Dateien konnte nicht gespeichert werden: " + fehler.Message); }
                ProtokollSchreiben();
            }
            ergebnis.Sicherung = sicherung;
            return ergebnis;
        }

        /// <summary>
        /// samp.dll und samp.exe der anderen SA-MP-Version in die Sicherung verschieben (nicht kopieren: Liegt im
        /// GTA-Verzeichnis keine samp.dll mehr, laedt der Launcher die aus samp_client). Klappt eine Datei nicht, kommen die
        /// schon verschobenen zurueck, und nichts ist veraendert.
        /// </summary>
        void SampAltesSichern()
        {
            var verschoben = new List<KeyValuePair<string, string>>();
            try
            {
                foreach (var name in new[] { GtaVerzeichnis.SampDatei, Spielstart.SampExe })
                {
                    var pfad = Path.Combine(gta, name);
                    if (!File.Exists(pfad)) continue;
                    if (sicherung == "")
                        sicherung = Path.Combine(gta, SicherungsOrdner, DateTime.Now.ToString("yyyy-MM-dd_HH-mm-ss", CultureInfo.InvariantCulture));
                    Directory.CreateDirectory(sicherung);
                    var kopie = Path.Combine(sicherung, name);
                    File.Move(pfad, kopie);
                    verschoben.Add(new KeyValuePair<string, string>(pfad, kopie));
                }
            }
            catch (Exception fehler)
            {
                foreach (var paar in verschoben)
                {
                    try { File.Move(paar.Value, paar.Key); }
                    catch (Exception) { }
                }
                throw new InstallationsFehler("Dein bisheriges SA-MP ließ sich nicht sichern: " + fehler.Message
                    + " Schließe GTA San Andreas und SA-MP und versuche es erneut. Es wurde nichts verändert.");
            }
            foreach (var paar in verschoben)
            {
                var name = paar.Key.Substring(gta.Length + 1);
                var ablage = paar.Value.Substring(gta.Length + 1);
                ergebnis.Gesichert++;
                Datei("Verschiebe " + name + " der bisherigen SA-MP-Version nach " + ablage);
                verzeichnis.Sichern(name, ablage);
            }
        }

        // ---------------------------------------------------------------- Download

        string Herunterladen(Komponente komponente, Paket paket, CancellationToken abbruch)
        {
            var adresse = AdressePruefen(paket.Url);
            var ziel = Path.Combine(temp, Guid.NewGuid().ToString("N") + ".download");
            var zeile = new Meldung
            {
                Nummer = naechsteNummer++, Art = MeldungArt.Download,
                Text = "Lade " + komponente.Name + " herunter", Url = paket.Url, Gesamt = paket.Groesse
            };
            Melden(Kopie(zeile));
            Notieren("Lade " + komponente.Name + " von " + paket.Url + " herunter");

            var anfrage = (HttpWebRequest)WebRequest.Create(adresse);
            anfrage.UserAgent = "SamplifeLauncher/" + Programm.Version;
            anfrage.AllowAutoRedirect = true;
            anfrage.Timeout = 30000;
            anfrage.ReadWriteTimeout = 30000;

            string pruefsumme;
            long gelesenGesamt = 0;
            HttpWebResponse antwort;
            try
            {
                antwort = (HttpWebResponse)anfrage.GetResponse();
            }
            catch (WebException fehler)
            {
                throw new InstallationsFehler("Der Download von " + paket.Url + " ist fehlgeschlagen: " + fehler.Message);
            }
            using (antwort)
            {
                // Auch nach Weiterleitungen (z. B. bei GitHub) muss die Verbindung verschluesselt bleiben
                AdressePruefen(antwort.ResponseUri.ToString());
                if (antwort.ContentLength > 0) zeile.Gesamt = antwort.ContentLength;
                long vorab = paket.Groesse;

                using (var quelle = antwort.GetResponseStream())
                using (var ausgabe = File.Create(ziel))
                using (var sha = SHA256.Create())
                {
                    var puffer = new byte[81920];
                    var letzteMeldung = DateTime.MinValue;
                    int gelesen;
                    while ((gelesen = quelle.Read(puffer, 0, puffer.Length)) > 0)
                    {
                        abbruch.ThrowIfCancellationRequested();
                        ausgabe.Write(puffer, 0, gelesen);
                        sha.TransformBlock(puffer, 0, gelesen, null, 0);
                        gelesenGesamt += gelesen;
                        if ((DateTime.UtcNow - letzteMeldung).TotalMilliseconds >= 120)
                        {
                            letzteMeldung = DateTime.UtcNow;
                            zeile.Erledigt = gelesenGesamt;
                            Melden(Kopie(zeile));
                            FortschrittMelden(bytesErledigt + Math.Min(gelesenGesamt, vorab > 0 ? vorab : gelesenGesamt));
                        }
                    }
                    sha.TransformFinalBlock(new byte[0], 0, 0);
                    pruefsumme = BitConverter.ToString(sha.Hash).Replace("-", "").ToLowerInvariant();
                }
            }
            bytesErledigt += paket.Groesse;
            FortschrittMelden(bytesErledigt);
            zeile.Erledigt = gelesenGesamt;
            zeile.Gesamt = gelesenGesamt;
            zeile.Fertig = true;
            Melden(Kopie(zeile));

            var pruefung = new Meldung
            {
                Nummer = naechsteNummer++, Art = MeldungArt.Schritt,
                Text = "Prüfe die Prüfsumme (SHA-256) von " + komponente.Name
            };
            Melden(Kopie(pruefung));
            if (pruefsumme != paket.Sha256)
            {
                Notieren("Prüfsumme falsch: erwartet " + paket.Sha256 + ", erhalten " + pruefsumme);
                throw new InstallationsFehler("Die heruntergeladene Datei von " + komponente.Name
                    + " stimmt nicht mit der erwarteten Prüfsumme überein. Es wurde nichts davon installiert.");
            }
            pruefung.Fertig = true;
            pruefung.Text = "Prüfsumme von " + komponente.Name + " stimmt (" + Groesse(gelesenGesamt) + ")";
            Melden(Kopie(pruefung));
            Notieren("Prüfsumme stimmt: " + pruefsumme);
            return ziel;
        }

        Uri AdressePruefen(string url)
        {
            Uri adresse;
            if (!Uri.TryCreate(url, UriKind.Absolute, out adresse))
                throw new InstallationsFehler("Ungültige Adresse in der Liste: " + url);
            if (adresse.Scheme == Uri.UriSchemeHttps) return adresse;
            // Unverschluesselt nur im Testmodus und nur vom eigenen Rechner
            if (testmodus && adresse.Scheme == Uri.UriSchemeHttp && adresse.IsLoopback) return adresse;
            throw new InstallationsFehler("Downloads sind nur über eine verschlüsselte Verbindung (https) erlaubt: " + url);
        }

        // ---------------------------------------------------------------- Dateien

        KomponenteStand StandVon(Komponente komponente)
        {
            foreach (var stand in Staende) if (stand.Id == komponente.Id) return stand;
            var neu = new KomponenteStand { Id = komponente.Id };
            Staende.Add(neu);
            return neu;
        }

        void Entpacken(Komponente komponente, Paket paket, string archivDatei, PaketStand paketStand, CancellationToken abbruch)
        {
            var praefix = paket.ZipUnterordner.Replace('\\', '/').Trim('/');
            if (praefix != "") praefix += "/";
            using (var archiv = ZipFile.OpenRead(archivDatei))
            {
                var eintraege = new List<ZipArchiveEntry>();
                foreach (var eintrag in archiv.Entries)
                {
                    var name = eintrag.FullName.Replace('\\', '/');
                    if (name.EndsWith("/")) continue;
                    if (praefix != "" && !name.StartsWith(praefix, StringComparison.OrdinalIgnoreCase)) continue;
                    eintraege.Add(eintrag);
                }
                if (eintraege.Count == 0)
                    throw new InstallationsFehler("Das Archiv von " + komponente.Name + " enthält keine passenden Dateien.");
                Schritt("Entpacke " + Anzahl(eintraege.Count, "Datei", "Dateien") + " von " + komponente.Name + " nach " + ZielOrdner(paket));

                foreach (var eintrag in eintraege)
                {
                    abbruch.ThrowIfCancellationRequested();
                    var relativ = eintrag.FullName.Replace('\\', '/').Substring(praefix.Length);
                    var zwischen = Path.Combine(temp, Guid.NewGuid().ToString("N") + ".datei");
                    eintrag.ExtractToFile(zwischen, true);
                    Ablegen(zwischen, Path.Combine(paket.Ziel, relativ.Replace('/', Path.DirectorySeparatorChar)), paketStand, komponente.IstSampClient);
                }
            }
        }

        void DateiAblegen(Komponente komponente, Paket paket, string datei, PaketStand paketStand)
        {
            var name = paket.Dateiname;
            if (name == "")
            {
                Uri adresse;
                if (Uri.TryCreate(paket.Url, UriKind.Absolute, out adresse)) name = Path.GetFileName(adresse.LocalPath);
            }
            if (string.IsNullOrEmpty(name)) throw new InstallationsFehler("Für " + komponente.Name + " fehlt der Dateiname in der Liste.");
            if (paket.Ablage == "launcher")
            {
                LauncherAblegen(komponente, paket, datei, name);
                return;
            }
            Schritt("Lege " + name + " von " + komponente.Name + " in " + ZielOrdner(paket) + " ab");
            Ablegen(datei, Path.Combine(paket.Ziel, name), paketStand, komponente.IstSampClient);
        }

        /// <summary>
        /// samp.dll aus samp_client in den Ordner des Launchers, nicht ins GTA-Verzeichnis: Von dort laedt der Launcher sie
        /// ins Spiel (SampStart). Die Datei gehoert dem Launcher und darf ersetzt werden.
        /// </summary>
        void LauncherAblegen(Komponente komponente, Paket paket, string datei, string name)
        {
            if (!komponente.IstSampClient) throw new InstallationsFehler("Nur SA-MP darf außerhalb des GTA-Verzeichnisses abgelegt werden.");
            var basis = Path.GetFullPath(Pfade.Launcher).TrimEnd(Path.DirectorySeparatorChar);
            var ziel = Path.GetFullPath(Path.Combine(basis, paket.Ziel, name));
            if (!ziel.StartsWith(basis + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
                throw new InstallationsFehler("Ein Paket wollte außerhalb des Launcher-Ordners schreiben (" + paket.Ziel + "). Die Installation wurde gestoppt.");
            Directory.CreateDirectory(Path.GetDirectoryName(ziel));
            File.Copy(datei, ziel, true);
            ergebnis.Geschrieben++;
            Datei("Lege " + name + " von " + komponente.Name + " in " + ziel + " ab");
        }

        string ZielOrdner(Paket paket)
        {
            return paket.Ziel == "" ? gta : Path.Combine(gta, paket.Ziel);
        }

        /// <summary>
        /// Schreibt eine Datei ins GTA-Verzeichnis. Vorhandene, abweichende Dateien werden vorher gesichert.
        /// Die Datei kommt mit Groesse, Aenderungszeit und Pruefsumme in die Liste des Pakets.
        /// </summary>
        void Ablegen(string quelle, string relativ, PaketStand paketStand, bool sampErlaubt)
        {
            var ziel = Path.GetFullPath(Path.Combine(gta, relativ));
            // Ein Archiv darf nicht mit "..\" aus dem GTA-Verzeichnis herausschreiben
            if (!ziel.StartsWith(gta + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
                throw new InstallationsFehler("Ein Paket wollte außerhalb des GTA-Verzeichnisses schreiben (" + relativ + "). Die Installation wurde gestoppt.");
            var anzeige = ziel.Substring(gta.Length + 1);
            // SA-MP legt nur ab, was fehlt: Vorhandenes (auch vom open.mp-Launcher kopiert) bleibt, wie es ist. Ausnahme:
            // Der Spieler hat dem Ersetzen einer anderen SA-MP-Version zugestimmt, dann wird gesichert und ersetzt.
            if (sampErlaubt && File.Exists(ziel) && !SampErsetzen)
            {
                ergebnis.Unveraendert++;
                Datei(anzeige + " liegt schon da und bleibt, wie es ist");
                return;
            }
            foreach (var name in Geschuetzt)
            {
                if (!string.Equals(anzeige, name, StringComparison.OrdinalIgnoreCase)) continue;
                if (!sampErlaubt || name == "gta_sa.exe")
                    throw new InstallationsFehler("Ein Paket wollte " + name + " ersetzen. Das lässt der Launcher nicht zu.");
            }

            var pruefsumme = GtaVerzeichnis.Sha256(quelle);
            if (File.Exists(ziel))
            {
                if (GtaVerzeichnis.Sha256(ziel) == pruefsumme)
                {
                    ergebnis.Unveraendert++;
                    Datei(anzeige + " ist bereits aktuell");
                    Merken(paketStand, anzeige, ziel, pruefsumme);
                    // Lag schon genau so da: gehoert nicht dem Launcher (ausser er hat sie frueher selbst angelegt)
                    verzeichnis.Merken(anzeige, Installiert.Vorhanden, "");
                    return;
                }
                if (sicherung == "")
                    sicherung = Path.Combine(gta, SicherungsOrdner, DateTime.Now.ToString("yyyy-MM-dd_HH-mm-ss", CultureInfo.InvariantCulture));
                var kopie = Path.Combine(sicherung, anzeige);
                Directory.CreateDirectory(Path.GetDirectoryName(kopie));
                File.Copy(ziel, kopie, true);
                ergebnis.Gesichert++;
                Datei("Sichere vorhandene " + anzeige + " nach " + kopie.Substring(gta.Length + 1));
                if (sampErlaubt && SampErsetzen) verzeichnis.Sichern(anzeige, kopie.Substring(gta.Length + 1));
                else verzeichnis.Merken(anzeige, Installiert.Gesichert, kopie.Substring(gta.Length + 1));
            }
            else verzeichnis.Merken(anzeige, Installiert.Neu, "");
            Directory.CreateDirectory(Path.GetDirectoryName(ziel));
            File.Copy(quelle, ziel, true);
            ergebnis.Geschrieben++;
            installiert.Add(anzeige);
            Datei("Schreibe " + anzeige);
            Merken(paketStand, anzeige, ziel, pruefsumme);
        }

        static void Merken(PaketStand paketStand, string relativ, string ziel, string pruefsumme)
        {
            var info = new FileInfo(ziel);
            paketStand.Dateien.Add(new DateiStand { Pfad = relativ, Groesse = info.Length, Geaendert = info.LastWriteTimeUtc.Ticks, Sha256 = pruefsumme });
        }

        // ---------------------------------------------------------------- Meldungen

        void Schritt(string text)
        {
            Melden(new Meldung { Nummer = naechsteNummer++, Art = MeldungArt.Schritt, Text = text, Fertig = true });
            Notieren(text);
        }

        void Datei(string text)
        {
            Melden(new Meldung { Nummer = naechsteNummer++, Art = MeldungArt.Datei, Text = text, Fertig = true });
            Notieren(text);
        }

        void Fehlermeldung(string text)
        {
            Melden(new Meldung { Nummer = naechsteNummer++, Art = MeldungArt.Fehler, Text = text, Fertig = true });
            Notieren("FEHLER: " + text);
        }

        void FortschrittMelden(long erledigt)
        {
            if (bytesGesamt <= 0) return;
            Fortschritt(Math.Max(0.0, Math.Min(1.0, (double)erledigt / bytesGesamt)));
        }

        static Meldung Kopie(Meldung m)
        {
            return new Meldung { Nummer = m.Nummer, Art = m.Art, Text = m.Text, Url = m.Url, Erledigt = m.Erledigt, Gesamt = m.Gesamt, Fertig = m.Fertig };
        }

        void Notieren(string text)
        {
            protokoll.Append(DateTime.Now.ToString("HH:mm:ss", CultureInfo.InvariantCulture)).Append("  ").AppendLine(text);
        }

        void ProtokollSchreiben()
        {
            try
            {
                if (!Directory.Exists(gta)) return;
                var datei = Path.Combine(gta, ProtokollDatei);
                var kopf = new StringBuilder();
                kopf.AppendLine("==== Samplife Launcher " + Programm.Version + " – "
                    + DateTime.Now.ToString("dd.MM.yyyy HH:mm", CultureInfo.InvariantCulture) + " ====");
                kopf.AppendLine("SA-MP-Version: " + (sampVersion == "" ? "nicht erkannt" : sampVersion));
                kopf.Append(protokoll);
                if (installiert.Count > 0)
                {
                    kopf.AppendLine("Geschriebene Dateien:");
                    foreach (var name in installiert) kopf.AppendLine("  " + name);
                }
                kopf.AppendLine();
                File.AppendAllText(datei, kopf.ToString(), Encoding.UTF8);
                ergebnis.Protokoll = datei;
            }
            catch (Exception) { }
        }

        static string Anzahl(int anzahl, string einzahl, string mehrzahl)
        {
            return anzahl + " " + (anzahl == 1 ? einzahl : mehrzahl);
        }

        public static string Groesse(long bytes)
        {
            var kultur = CultureInfo.GetCultureInfo("de-DE");
            if (bytes >= 1024 * 1024) return (bytes / 1048576.0).ToString("0.0", kultur) + " MB";
            if (bytes >= 1024) return (bytes / 1024.0).ToString("0", kultur) + " KB";
            return bytes + " Bytes";
        }
    }
}
