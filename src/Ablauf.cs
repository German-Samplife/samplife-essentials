using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;

namespace SamplifeEssentials
{
    public class AblaufErgebnis
    {
        public bool Erfolg;
        public bool Abgebrochen;
        public bool Gestartet;
        public int Code;                   // Rueckgabe fuer --still, siehe Programm
        public string Fehler = "";
        public string Warnung = "";        // z. B. Anmeldung fehlgeschlagen, Spiel trotzdem gestartet
        public string Protokoll = "";
        public string Sicherung = "";
        public int Nachgeladen;            // Komponenten, die geladen wurden
    }

    /// <summary>
    /// Was beim Klick auf „Spielen“ passiert: Dateien mit der Liste abgleichen, Fehlendes und Veraltetes laden, den
    /// geprueften Stand sichern, beim Server anmelden und samp.exe starten. Laeuft synchron im Hintergrund-Thread.
    /// </summary>
    public class Ablauf
    {
        public string Gta;
        public string SampVersion = "";
        public string Name = "";
        public Manifest Manifest;
        public string ManifestJson = "";
        public bool AusSicherung;          // Liste kam aus der Sicherung, Nachladen ist nicht moeglich
        public ICollection<string> Optional = new List<string>();
        public bool Gruendlich;
        public bool MitStart;
        public bool Testmodus;

        public Action<Meldung> Melden = delegate { };
        public Action<double> Fortschritt = delegate { };
        public Action<List<Pruefung>> Geprueft = delegate { };

        // Eigene Nummern weit weg von denen der Installation, damit keine Zeile die andere ueberschreibt
        int naechsteNummer = 100000;

        public AblaufErgebnis Ausfuehren(CancellationToken abbruch)
        {
            var ergebnis = new AblaufErgebnis();
            try
            {
                if (MitStart)
                {
                    // Vor dem Laden pruefen, was ohnehin den Start verhindern wuerde
                    var name = Spielstart.NamePruefen(Name);
                    if (name != "") return Fehlschlag(ergebnis, name, 5);
                    var grund = StartVoraussetzung();
                    if (grund != "") return Fehlschlag(ergebnis, grund, 4);
                }

                var verwaltet = Manifest.Verwaltet(Optional);
                Schritt(Gruendlich ? "Prüfe alle Dateien gründlich (SHA-256)" : "Prüfe die Dateien im GTA-Verzeichnis");
                var stand = Stand.Lesen(Gta);
                var pruefungen = Stand.Abgleichen(Gta, stand, verwaltet, SampVersion, Gruendlich, abbruch);
                Geprueft(pruefungen);

                var noetig = new List<Komponente>();
                var neuerStand = new Stand { ManifestJson = ManifestJson, ManifestStand = Manifest.Stand, Geprueft = DateTime.UtcNow };
                foreach (var pruefung in pruefungen)
                {
                    if (pruefung.Zustand == Zustand.Aktuell) neuerStand.Komponenten.Add(pruefung.Gesichert);
                    else
                    {
                        noetig.Add(pruefung.Komponente);
                        Schritt(pruefung.Komponente.Name + ": " + ZustandText(pruefung));
                    }
                }

                if (noetig.Count > 0)
                {
                    if (AusSicherung)
                        return Fehlschlag(ergebnis, "Die Server-Liste ist nicht erreichbar, deshalb kann nichts nachgeladen werden. "
                            + noetig[0].Name + " ist nicht vollständig. Versuche es später noch einmal.", 3);
                    var installation = new Installation(Gta, SampVersion, noetig, Testmodus);
                    installation.Melden = Melden;
                    installation.Fortschritt = Fortschritt;
                    var installiert = installation.Ausfuehren(abbruch);
                    ergebnis.Protokoll = installiert.Protokoll;
                    ergebnis.Sicherung = installiert.Sicherung;
                    if (!installiert.Erfolg)
                    {
                        ergebnis.Abgebrochen = installiert.Abgebrochen;
                        ergebnis.Fehler = installiert.Fehler;
                        ergebnis.Code = 1;
                        return ergebnis;
                    }
                    neuerStand.Komponenten.AddRange(installation.Staende);
                    ergebnis.Nachgeladen = noetig.Count;
                }
                else
                {
                    Fortschritt(1);
                    Erfolg(AusSicherung
                        ? "Alle Dateien stimmen mit dem gesicherten Stand vom " + stand.Geprueft.ToLocalTime().ToString("dd.MM.yyyy, HH:mm") + " überein."
                        : "Alle Dateien sind aktuell.");
                }

                // Mit Liste vom Server: geprueften Stand sichern. Aus der Sicherung bleibt sie, wie sie ist.
                if (!AusSicherung)
                {
                    neuerStand.Schreiben(Gta);
                    Datei("Stand " + Manifest.Stand + " gesichert in " + Path.Combine(Stand.Ordner, Stand.StandDatei));
                }

                if (MitStart)
                {
                    abbruch.ThrowIfCancellationRequested();
                    Spielstart.NameSetzen(Name, Testmodus);
                    if (Manifest.Anmeldung != "")
                    {
                        Schritt("Melde " + Name + " beim Server an");
                        var fehler = Spielstart.Anmelden(Manifest.Anmeldung, Name, Manifest.Stand, Testmodus);
                        if (fehler != "")
                        {
                            // Das Spiel startet trotzdem; ohne Anmeldung erklaert der Server selbst, was fehlt
                            ergebnis.Warnung = fehler;
                            Melden(new Meldung { Nummer = naechsteNummer++, Art = MeldungArt.Warnung, Text = fehler, Fertig = true });
                        }
                    }
                    Schritt("Starte SA-MP und verbinde mit " + Manifest.ServerAdresse + ":" + Manifest.ServerPort);
                    try { Spielstart.Starten(Gta, Manifest.ServerAdresse, Manifest.ServerPort); }
                    catch (Exception fehler)
                    {
                        return Fehlschlag(ergebnis, "samp.exe konnte nicht gestartet werden: " + fehler.Message, 6);
                    }
                    ergebnis.Gestartet = true;
                }
                ergebnis.Erfolg = true;
            }
            catch (OperationCanceledException)
            {
                ergebnis.Abgebrochen = true;
                return Fehlschlag(ergebnis, "Abgebrochen.", 1);
            }
            catch (Exception fehler)
            {
                return Fehlschlag(ergebnis, "Unerwarteter Fehler: " + fehler.Message, 1);
            }
            return ergebnis;
        }

        /// <summary>Leer, wenn gestartet werden kann; sonst der Grund (Name, SA-MP-Version, samp.exe, Server).</summary>
        public string StartVoraussetzung()
        {
            var name = Spielstart.NamePruefen(Name);
            if (name != "") return name;
            if (Manifest.ServerAdresse == "" || Manifest.ServerPort <= 0)
                return "Die Server-Liste nennt keinen Server. Lade den aktuellen Launcher von der Website.";
            if (!File.Exists(Path.Combine(Gta, Spielstart.SampExe)))
                return "Im GTA-Verzeichnis fehlt samp.exe. Installiere SA-MP " + Benoetigt() + " in diesen Ordner.";
            if (SampVersion == "" || GtaVerzeichnis.SampVersion(Gta, Manifest) == "")
                return "Im GTA-Verzeichnis liegt nicht SA-MP " + Benoetigt() + ". Der Server läuft nur mit dieser Version.";
            return "";
        }

        string Benoetigt()
        {
            var namen = new List<string>();
            foreach (var version in Manifest.SampVersionen) namen.Add(version.Name);
            return namen.Count == 0 ? "0.3.7-R1" : string.Join(" oder ", namen.ToArray());
        }

        public static string ZustandText(Pruefung pruefung)
        {
            switch (pruefung.Zustand)
            {
                case Zustand.Aktuell: return "aktuell";
                case Zustand.Fehlt: return "noch nicht eingerichtet";
                case Zustand.Veraltet: return "neue Fassung";
                default: return "Datei fehlt oder ist verändert" + (pruefung.Grund == "" ? "" : " (" + pruefung.Grund + ")");
            }
        }

        AblaufErgebnis Fehlschlag(AblaufErgebnis ergebnis, string text, int code)
        {
            ergebnis.Erfolg = false;
            ergebnis.Fehler = text;
            ergebnis.Code = code;
            Melden(new Meldung { Nummer = naechsteNummer++, Art = MeldungArt.Fehler, Text = text, Fertig = true });
            return ergebnis;
        }

        void Schritt(string text)
        {
            Melden(new Meldung { Nummer = naechsteNummer++, Art = MeldungArt.Schritt, Text = text, Fertig = true });
        }

        void Datei(string text)
        {
            Melden(new Meldung { Nummer = naechsteNummer++, Art = MeldungArt.Datei, Text = text, Fertig = true });
        }

        void Erfolg(string text)
        {
            Melden(new Meldung { Nummer = naechsteNummer++, Art = MeldungArt.Erfolg, Text = text, Fertig = true });
        }
    }
}
