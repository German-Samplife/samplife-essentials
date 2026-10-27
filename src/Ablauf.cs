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

    /// <summary>Wie SA-MP gestartet wird (Ablauf.SampPlanen).</summary>
    public class SampPlan
    {
        public string Art = "";        // "exe": samp.exe im GTA-Verzeichnis, "dll": gta_sa.exe starten und samp.dll laden,
                                       // "einrichten": erst samp_client laden, "": geht nicht (Grund)
        public string Dll = "";        // bei "dll": die samp.dll, die geladen wird
        public string Version = "";
        public bool VonOpenMp;         // die samp.dll stammt aus dem Ordner des open.mp-Launchers
        public string Grund = "";
        public bool AndereVersion;     // im GTA-Verzeichnis liegt SA-MP in einer Version, die der Server nicht kennt
        public string Andere = "";     // deren Version, soweit erkennbar ("" = unbekannt)
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
        public string Passwort = "";       // Serverpasswort, nur fuer diesen Start, wird nirgends gespeichert
        public string Kontopasswort = "";  // Passwort des Accounts: die Website meldet an, der Login im Spiel entfaellt
        public Manifest Manifest;
        public string ManifestJson = "";
        public bool AusSicherung;          // Liste kam aus der Sicherung, Nachladen ist nicht moeglich
        public ICollection<string> Optional = new List<string>();
        public bool Gruendlich;
        public bool MitStart;
        public bool Testmodus;
        /// <summary>
        /// Der Spieler hat zugestimmt (Fenster „Andere SA-MP-Version gefunden“, still: --samp-ersetzen): Liegt im
        /// GTA-Verzeichnis eine andere SA-MP-Version, sichert der Launcher sie und richtet samp_client ein. Gilt nur fuer
        /// diesen Ablauf und wird nicht gemerkt.
        /// </summary>
        public bool SampErsetzen;

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
                // Wie SA-MP startet: samp.exe, samp.dll laden oder erst einrichten. Eine vorhandene Installation im
                // GTA-Verzeichnis ersetzt der Launcher nur, wenn es eine andere Version ist und der Spieler zugestimmt hat
                // (SampErsetzen); sie kommt dann in die Sicherung.
                var plan = SampPlanen();
                var sampErsetzen = SampErsetzen && plan.AndereVersion && Manifest.SampClient != null && Manifest.SampVersionen.Count > 0;
                var andereVersion = plan.Andere;
                if (sampErsetzen)
                {
                    plan.Art = "einrichten";
                    plan.Version = Manifest.SampVersionen[0].Name;
                    plan.Grund = "";
                    SampVersion = plan.Version;
                }
                var sampEinrichten = plan.Art == "einrichten";
                if (SampVersion == "") SampVersion = plan.Version;

                if (MitStart)
                {
                    // Vor dem Laden pruefen, was ohnehin den Start verhindern wuerde
                    var name = Spielstart.NamePruefen(Name);
                    if (name != "") return Fehlschlag(ergebnis, name, 5);
                    var grund = StartVoraussetzung(plan);
                    if (grund != "") return Fehlschlag(ergebnis, grund, 4);
                }

                var verwaltet = Manifest.Verwaltet(Optional);
                Schritt(Gruendlich ? "Prüfe alle Dateien gründlich (SHA-256)" : "Prüfe die Dateien im GTA-Verzeichnis");
                var stand = Stand.Lesen(Gta);
                var pruefungen = Stand.Abgleichen(Gta, stand, verwaltet, SampVersion, Gruendlich, abbruch);
                Geprueft(pruefungen);

                var noetig = new List<Komponente>();
                if (sampEinrichten)
                {
                    noetig.Add(Manifest.SampClient);
                    if (sampErsetzen)
                        Schritt("Im GTA-Verzeichnis liegt SA-MP " + (andereVersion == "" ? "in einer anderen Version" : andereVersion)
                            + ". Der Launcher sichert es in " + Installation.SicherungsOrdner + " und richtet " + Manifest.SampClient.Name + " ein.");
                    else
                        Schritt("Im GTA-Verzeichnis fehlt SA-MP. Der Launcher richtet " + Manifest.SampClient.Name + " ein; "
                            + "vorhandene Dateien bleiben, wie sie sind.");
                }
                // Was Windows fuer die Pakete braucht (DirectX): ohne laedt SA-MP bzw. SampVoice nicht
                foreach (var voraussetzung in Systemvoraussetzung.Fehlend(Manifest, Gta))
                {
                    noetig.Add(voraussetzung);
                    Schritt(voraussetzung.Name + ": fehlt auf diesem Rechner ("
                        + string.Join(", ", Systemvoraussetzung.Fehlende(voraussetzung, Gta).ToArray()) + ")");
                }
                // Fremde ASI Loader und bekannte Konflikte: nur Hinweis, der Spieler entscheidet im Fenster
                foreach (var fremd in FremdeDateien.Suchen(Gta, Manifest))
                    Melden(new Meldung { Nummer = naechsteNummer++, Art = MeldungArt.Warnung, Text = fremd.Text, Fertig = true });

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
                    installation.SampErsetzen = sampErsetzen;
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
                    // SA-MP gehoert danach dem Spieler: nicht in den Stand, also nie wieder geprueft oder ersetzt
                    foreach (var komponenteStand in installation.Staende)
                    {
                        if (!sampEinrichten || komponenteStand.Id != Manifest.SampClient.Id) neuerStand.Komponenten.Add(komponenteStand);
                    }
                    ergebnis.Nachgeladen = noetig.Count;
                    if (sampEinrichten)
                    {
                        plan = SampPlanen();
                        if (plan.Art != "exe" && plan.Art != "dll")
                            return Fehlschlag(ergebnis, "SA-MP wurde eingerichtet, passt aber nicht zur Liste des Servers.", 4);
                    }
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
                    Spielstart.GtaEintragen(Gta, Testmodus);
                    if (Manifest.Anmeldung != "")
                    {
                        Schritt("Melde " + Name + " beim Server an" + (Kontopasswort != "" ? ", mit deinem Passwort" : ""));
                        bool passwortFalsch;
                        var fehler = Spielstart.Anmelden(Manifest.Anmeldung, Name, Manifest.Stand, Kontopasswort, Testmodus, out passwortFalsch);
                        // Falsches Passwort: nicht starten, sondern Gelegenheit zum Korrigieren geben
                        if (passwortFalsch) return Fehlschlag(ergebnis, fehler, 8);
                        if (fehler != "")
                        {
                            // Das Spiel startet trotzdem; ohne Anmeldung erklaert der Server selbst, was fehlt
                            ergebnis.Warnung = fehler;
                            Melden(new Meldung { Nummer = naechsteNummer++, Art = MeldungArt.Warnung, Text = fehler, Fertig = true });
                        }
                    }
                    // Mit Serverpasswort immer ueber gta_sa.exe: dort kommt es mit -z sicher an (wie beim open.mp-Launcher)
                    if (Passwort != "" && plan.Art == "exe")
                    {
                        plan.Art = "dll";
                        plan.Dll = Path.Combine(Gta, GtaVerzeichnis.SampDatei);
                    }
                    if (plan.Art == "dll")
                    {
                        Schritt("Starte GTA San Andreas mit SA-MP " + plan.Version + (plan.VonOpenMp ? " aus dem open.mp-Launcher" : "")
                            + " und verbinde mit " + Manifest.ServerAdresse + ":" + Manifest.ServerPort
                            + (Passwort != "" ? ", mit Serverpasswort" : ""));
                        try { SampStart.Starten(Gta, Manifest.ServerAdresse, Manifest.ServerPort, Name, plan.Dll, Passwort); }
                        catch (Exception fehler)
                        {
                            return Fehlschlag(ergebnis, "SA-MP konnte nicht gestartet werden: " + fehler.Message, 6);
                        }
                    }
                    else
                    {
                        Schritt("Starte SA-MP und verbinde mit " + Manifest.ServerAdresse + ":" + Manifest.ServerPort);
                        try { Spielstart.Starten(Gta, Manifest.ServerAdresse, Manifest.ServerPort); }
                        catch (Exception fehler)
                        {
                            return Fehlschlag(ergebnis, "samp.exe konnte nicht gestartet werden: " + fehler.Message, 6);
                        }
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

        /// <summary>
        /// Wie SA-MP startet. Liegt im GTA-Verzeichnis eine samp.dll, entscheidet sie: passende Version mit samp.exe
        /// startet ueber samp.exe, ohne samp.exe wird sie geladen, eine andere Version haelt den Start an (sie wird nie
        /// ersetzt). Ohne samp.dll im GTA-Verzeichnis: eigene Kopie oder die des open.mp-Launchers laden, sonst
        /// samp_client einrichten.
        /// </summary>
        public SampPlan SampPlanen()
        {
            return Planen(Gta, Manifest);
        }

        public static SampPlan Planen(string gta, Manifest manifest)
        {
            var plan = new SampPlan();
            var gtaDll = Path.Combine(gta, GtaVerzeichnis.SampDatei);
            if (File.Exists(gtaDll))
            {
                plan.Version = GtaVerzeichnis.SampVersionVonDatei(gtaDll, manifest);
                if (plan.Version == "")
                {
                    var andere = Spielstart.SampDateiVersion(gtaDll);
                    plan.AndereVersion = true;
                    plan.Andere = andere;
                    plan.Grund = "Im GTA-Verzeichnis liegt SA-MP " + (andere == "" ? "in einer anderen Version" : andere)
                        + ". Der Server läuft nur mit SA-MP " + Benoetigt(manifest) + ". "
                        + (manifest.SampClient != null && manifest.SampVersionen.Count > 0
                            ? "Beim Klick auf „Spielen“ fragt der Launcher, ob er die passende Version einrichten soll; dein bisheriges SA-MP sichert er dabei."
                            : "Entferne samp.dll und samp.exe aus dem GTA-Verzeichnis und installiere SA-MP " + Benoetigt(manifest) + ".");
                    return plan;
                }
                if (File.Exists(Path.Combine(gta, Spielstart.SampExe))) plan.Art = "exe";
                else { plan.Art = "dll"; plan.Dll = gtaDll; }
                return plan;
            }
            string version;
            bool vonOpenMp;
            var dll = SampStart.DllFinden(manifest, out version, out vonOpenMp);
            if (dll != "")
            {
                plan.Art = "dll";
                plan.Dll = dll;
                plan.Version = version;
                plan.VonOpenMp = vonOpenMp;
                return plan;
            }
            if (manifest.SampClient != null)
            {
                plan.Art = "einrichten";
                plan.Version = manifest.SampVersionen.Count > 0 ? manifest.SampVersionen[0].Name : "";
                return plan;
            }
            plan.Grund = "Im GTA-Verzeichnis fehlt SA-MP " + Benoetigt(manifest) + ". Installiere es in diesen Ordner.";
            return plan;
        }

        /// <summary>Leer, wenn gestartet werden kann; sonst der Grund (Name, Server, SA-MP).</summary>
        public string StartVoraussetzung(SampPlan plan)
        {
            var name = Spielstart.NamePruefen(Name);
            if (name != "") return name;
            if (Manifest.ServerAdresse == "" || Manifest.ServerPort <= 0)
                return "Die Server-Liste nennt keinen Server. Lade den aktuellen Launcher von der Website.";
            return plan.Art == "" ? plan.Grund : "";
        }

        public static string Benoetigt(Manifest manifest)
        {
            // Ohne Liste (Bildschirmfotos) die Version, fuer die der Server gebaut ist
            if (manifest == null) return "0.3.7-R1";
            var namen = new List<string>();
            foreach (var version in manifest.SampVersionen) namen.Add(version.Name);
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
