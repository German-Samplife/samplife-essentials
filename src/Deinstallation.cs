using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using Microsoft.Win32;

namespace SamplifeEssentials
{
    public class DeinstallationErgebnis
    {
        public bool Erfolg;
        public string Fehler = "";
        public int Entfernt;
        public int Wiederhergestellt;
        public int Zurueckbenannt;
        public int Uebrig;             // Dateien, die nicht entfernt werden konnten
        public bool Geschaetzt;        // die Liste wurde aus Daten aelterer Fassungen rekonstruiert
    }

    /// <summary>
    /// „Deinstallieren“: entfernt alles, was der Launcher angelegt hat, und stellt her, was er ersetzt oder ausgeschaltet hat.
    /// Im GTA-Verzeichnis: Dateien der Pakete (laut samplife-launcher\installiert.json), was die Komponenten im Spiel anlegen
    /// ("entfernen" in der Liste), die Sicherungen, den Stand und das Protokoll. Ausserhalb: den eigenen Ordner unter
    /// %LOCALAPPDATA% (ohne die laufende .exe), die Adresse samplife:// und den Schluessel von SA-MP, wenn der Launcher ihn
    /// angelegt hat. Die DirectX-Laufzeit bleibt: Sie gehoert zu Windows und anderen Spielen.
    /// </summary>
    public class Deinstallation
    {
        public string Gta = "";
        public bool Testmodus;
        public Action<Meldung> Melden = delegate { };

        int nummer = 200000;
        readonly HashSet<string> ordner = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        public DeinstallationErgebnis Ausfuehren()
        {
            var ergebnis = new DeinstallationErgebnis();
            try
            {
                var mitGta = GtaVerzeichnis.IstGtaVerzeichnis(Gta);
                if (mitGta)
                {
                    Gta = Path.GetFullPath(Gta).TrimEnd(Path.DirectorySeparatorChar);
                    if (SpielLaeuft())
                        return Fehlschlag(ergebnis, "GTA San Andreas läuft noch. Schließe das Spiel und versuche es dann noch einmal.");
                    if (!GtaVerzeichnis.IstBeschreibbar(Gta))
                        return Fehlschlag(ergebnis, "In das GTA-Verzeichnis darf nicht geschrieben werden. Starte den Launcher als Administrator.");
                    // 4GB-Patch zuruecknehmen, wenn der Launcher ihn gesetzt hat (vor dem Aufraeumen, die Marke liegt in samplife-launcher)
                    var laa = Laa.Anwenden(Gta, false);
                    if (laa != "") Schritt(laa);
                    GtaAufraeumen(ergebnis);
                }
                else Schritt("Kein GTA-Verzeichnis gewählt: Der Launcher räumt nur seine eigenen Einstellungen auf.");

                AusserhalbAufraeumen(ergebnis);
                ergebnis.Erfolg = true;
                var text = "Fertig. " + Anzahl(ergebnis.Entfernt, "Datei", "Dateien") + " entfernt";
                if (ergebnis.Wiederhergestellt > 0) text += ", " + Anzahl(ergebnis.Wiederhergestellt, "Datei", "Dateien") + " in der Fassung von vorher zurückgeholt";
                if (ergebnis.Zurueckbenannt > 0) text += ", " + Anzahl(ergebnis.Zurueckbenannt, "ausgeschaltete Datei", "ausgeschaltete Dateien") + " wieder eingeschaltet";
                Melden(new Meldung { Nummer = nummer++, Art = MeldungArt.Erfolg, Text = text + ".", Fertig = true });
                if (ergebnis.Uebrig > 0)
                    Warnung(Anzahl(ergebnis.Uebrig, "Datei ließ", "Dateien ließen") + " sich nicht entfernen. Sie stehen oben in der Liste.");
            }
            catch (Exception fehler)
            {
                return Fehlschlag(ergebnis, "Unerwarteter Fehler: " + fehler.Message);
            }
            return ergebnis;
        }

        bool SpielLaeuft()
        {
            foreach (var name in new[] { "gta_sa", "samp" })
            {
                foreach (var prozess in Process.GetProcessesByName(name))
                {
                    using (prozess)
                    {
                        try
                        {
                            var pfad = prozess.MainModule.FileName;
                            if (pfad.StartsWith(Gta + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase)) return true;
                        }
                        catch (Exception)
                        {
                            // Pfad nicht lesbar (z. B. als Administrator gestartet): sicherheitshalber als laufend werten
                            if (!Testmodus) return true;
                        }
                    }
                }
            }
            return false;
        }

        // ---------------------------------------------------------------- GTA-Verzeichnis

        void GtaAufraeumen(DeinstallationErgebnis ergebnis)
        {
            var liste = Installiert.Laden(Gta);
            ergebnis.Geschaetzt = liste.Geschaetzt;
            Schritt("Entferne die Dateien des Launchers aus " + Gta);
            if (liste.Geschaetzt)
                Schritt("Diese Installation stammt von einer älteren Fassung des Launchers. Was er angelegt hat, ergibt sich aus "
                    + "dem gesicherten Stand, dem Protokoll und der ältesten Sicherung.");

            // Was die Komponenten im Spiel anlegen, steht in der gesicherten Liste des Servers
            var entfernen = new List<string>();
            var stand = Stand.Lesen(Gta);
            if (stand != null)
            {
                try
                {
                    var manifest = Manifest.Lesen(stand.ManifestJson);
                    foreach (var komponente in manifest.Alle())
                    {
                        if (stand.Suchen(komponente.Id) != null) entfernen.AddRange(komponente.Entfernen);
                    }
                }
                catch (Exception) { }
            }

            // Das Original von vorbisFile.dll merken: Ohne vorbisFile.dll startet GTA nicht. Im Paket von SampVoice heisst es
            // vorbisHooked.dll (der ASI Loader reicht die Aufrufe dorthin weiter).
            byte[] originalVorbis = null;
            try
            {
                var hooked = Path.Combine(Gta, "vorbisHooked.dll");
                if (File.Exists(hooked) && !FremdeDateien.IstAsiLoader(hooked)) originalVorbis = File.ReadAllBytes(hooked);
            }
            catch (Exception) { }

            foreach (var datei in liste.Dateien)
            {
                var pfad = Voll(datei.Pfad);
                if (pfad == null || string.Equals(datei.Pfad, GtaVerzeichnis.SpielDatei, StringComparison.OrdinalIgnoreCase)) continue;
                if (datei.Vorher == Installiert.Vorhanden) continue;
                var kopie = datei.Vorher == Installiert.Gesichert ? Voll(datei.Sicherung) : null;
                try
                {
                    if (kopie != null && File.Exists(kopie))
                    {
                        Entsperren(pfad);
                        File.Copy(kopie, pfad, true);
                        ergebnis.Wiederhergestellt++;
                        Datei("Hole " + datei.Pfad + " in der Fassung von vor dem Launcher zurück");
                    }
                    else if (File.Exists(pfad))
                    {
                        Entsperren(pfad);
                        File.Delete(pfad);
                        ergebnis.Entfernt++;
                        Datei("Entferne " + datei.Pfad);
                        OrdnerMerken(pfad);
                    }
                }
                catch (Exception fehler)
                {
                    ergebnis.Uebrig++;
                    Warnung(datei.Pfad + " ließ sich nicht entfernen: " + fehler.Message);
                }
            }

            foreach (var rest in entfernen)
            {
                var pfad = Voll(rest);
                if (pfad == null) continue;
                try
                {
                    if (Directory.Exists(pfad))
                    {
                        Directory.Delete(pfad, true);
                        Datei("Entferne den Ordner " + rest + " (im Spiel angelegt)");
                        OrdnerMerken(pfad);
                    }
                    else if (File.Exists(pfad))
                    {
                        File.Delete(pfad);
                        ergebnis.Entfernt++;
                        Datei("Entferne " + rest + " (im Spiel angelegt)");
                        OrdnerMerken(pfad);
                    }
                }
                catch (Exception fehler)
                {
                    ergebnis.Uebrig++;
                    Warnung(rest + " ließ sich nicht entfernen: " + fehler.Message);
                }
            }

            // Ausgeschaltete Dateien des Spielers wieder einschalten
            foreach (var aus in liste.Abgeschaltet)
            {
                var ablage = Voll(aus.Ablage);
                var ziel = Voll(aus.Pfad);
                if (ablage == null || ziel == null || !File.Exists(ablage)) continue;
                try
                {
                    if (File.Exists(ziel))
                    {
                        Warnung(aus.Pfad + " gibt es schon wieder; " + aus.Ablage + " bleibt ausgeschaltet liegen.");
                        continue;
                    }
                    File.Move(ablage, ziel);
                    ergebnis.Zurueckbenannt++;
                    Datei("Schalte " + aus.Pfad + " wieder ein");
                }
                catch (Exception fehler)
                {
                    ergebnis.Uebrig++;
                    Warnung(aus.Pfad + " ließ sich nicht wieder einschalten: " + fehler.Message);
                }
            }

            // GTA braucht vorbisFile.dll. Fehlt sie jetzt, kommt das Original zurueck.
            var vorbis = Path.Combine(Gta, "vorbisFile.dll");
            if (!File.Exists(vorbis) && originalVorbis != null)
            {
                try
                {
                    File.WriteAllBytes(vorbis, originalVorbis);
                    Datei("vorbisFile.dll fehlte danach; das Original aus dem Spiel liegt wieder da, damit GTA startet");
                }
                catch (Exception fehler) { Warnung("vorbisFile.dll konnte nicht zurückgelegt werden: " + fehler.Message); }
            }

            // Leere Ordner, die durch das Entfernen entstanden sind (z. B. cef, SAMP)
            var sortiert = new List<string>(ordner);
            sortiert.Sort(delegate(string a, string b) { return b.Length.CompareTo(a.Length); });
            foreach (var eintrag in sortiert) LeereOrdnerEntfernen(eintrag);

            // Eigene Verwaltung: Stand, Liste, Sicherungen, Protokoll
            foreach (var eigen in new[] { Stand.Ordner, Installation.SicherungsOrdner })
            {
                var pfad = Path.Combine(Gta, eigen);
                try
                {
                    if (Directory.Exists(pfad))
                    {
                        Directory.Delete(pfad, true);
                        Datei("Entferne den Ordner " + eigen);
                    }
                }
                catch (Exception fehler)
                {
                    ergebnis.Uebrig++;
                    Warnung(eigen + " ließ sich nicht entfernen: " + fehler.Message);
                }
            }
            try
            {
                var protokoll = Path.Combine(Gta, Installation.ProtokollDatei);
                if (File.Exists(protokoll)) { File.Delete(protokoll); Datei("Entferne " + Installation.ProtokollDatei); }
            }
            catch (Exception) { ergebnis.Uebrig++; }
        }

        /// <summary>Voller Pfad im GTA-Verzeichnis, oder null, wenn er hinausfuehrt.</summary>
        string Voll(string relativ)
        {
            if (string.IsNullOrEmpty(relativ)) return null;
            try
            {
                var pfad = Path.GetFullPath(Path.Combine(Gta, relativ));
                return pfad.StartsWith(Gta + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase) ? pfad : null;
            }
            catch (Exception) { return null; }
        }

        static void Entsperren(string pfad)
        {
            if (File.Exists(pfad) && (File.GetAttributes(pfad) & FileAttributes.ReadOnly) != 0)
                File.SetAttributes(pfad, File.GetAttributes(pfad) & ~FileAttributes.ReadOnly);
        }

        void OrdnerMerken(string pfad)
        {
            var oben = Path.GetDirectoryName(pfad);
            if (!string.IsNullOrEmpty(oben)) ordner.Add(oben);
        }

        void LeereOrdnerEntfernen(string pfad)
        {
            while (!string.IsNullOrEmpty(pfad) && pfad.StartsWith(Gta + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
            {
                try
                {
                    if (!Directory.Exists(pfad) || Directory.GetFileSystemEntries(pfad).Length > 0) return;
                    Directory.Delete(pfad);
                }
                catch (Exception) { return; }
                pfad = Path.GetDirectoryName(pfad);
            }
        }

        // ---------------------------------------------------------------- Ausserhalb des GTA-Verzeichnisses

        void AusserhalbAufraeumen(DeinstallationErgebnis ergebnis)
        {
            // samplife:// fuer „Verbinden“ auf der Website
            var schema = @"Software\Classes\" + Spielstart.AktuellesSchema(Testmodus);
            try
            {
                using (var schluessel = Registry.CurrentUser.OpenSubKey(schema))
                {
                    if (schluessel != null)
                    {
                        Registry.CurrentUser.DeleteSubKeyTree(schema, false);
                        Datei("Entferne die Adresse " + Spielstart.AktuellesSchema(Testmodus) + ":// aus Windows");
                    }
                }
            }
            catch (Exception fehler) { Warnung("Die Adresse samplife:// ließ sich nicht entfernen: " + fehler.Message); }

            // Schluessel von SA-MP (Name, GTA-Pfad), nur wenn der Launcher ihn angelegt hat
            if (Spielstart.SampSchluesselVomLauncher(Testmodus))
            {
                try
                {
                    Registry.CurrentUser.DeleteSubKeyTree(Spielstart.SampSchluesselPfad(Testmodus), false);
                    Datei("Entferne die Einstellungen von SA-MP (Name und GTA-Pfad), die der Launcher angelegt hat");
                }
                catch (Exception fehler) { Warnung("Die Einstellungen von SA-MP ließen sich nicht entfernen: " + fehler.Message); }
            }
            else Schritt("Die Einstellungen von SA-MP (dein Name in der Registry) bleiben: Sie gab es schon vor dem Launcher.");

            // Eigener Ordner: Einstellungen, Bilder, Schriften, samp.dll aus samp_client
            var eigen = Path.GetFullPath(Pfade.Launcher).TrimEnd(Path.DirectorySeparatorChar);
            if (!Directory.Exists(eigen)) return;
            var exe = "";
            try { exe = Path.GetFullPath(Aktualisierung.EigeneDatei); }
            catch (Exception) { }
            var exeDrin = exe.StartsWith(eigen + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase);
            var rest = 0;
            foreach (var datei in Directory.GetFiles(eigen, "*", SearchOption.AllDirectories))
            {
                if (exeDrin && datei.StartsWith(exe, StringComparison.OrdinalIgnoreCase)) continue;
                try { File.Delete(datei); }
                catch (Exception) { rest++; }
            }
            var unterordner = new List<string>(Directory.GetDirectories(eigen, "*", SearchOption.AllDirectories));
            unterordner.Sort(delegate(string a, string b) { return b.Length.CompareTo(a.Length); });
            foreach (var unter in unterordner)
            {
                try { if (Directory.GetFileSystemEntries(unter).Length == 0) Directory.Delete(unter); }
                catch (Exception) { }
            }
            try { if (Directory.GetFileSystemEntries(eigen).Length == 0) Directory.Delete(eigen); }
            catch (Exception) { }
            var oben = Path.GetDirectoryName(eigen);
            try { if (Directory.Exists(oben) && Directory.GetFileSystemEntries(oben).Length == 0) Directory.Delete(oben); }
            catch (Exception) { }

            if (rest > 0 && !exeDrin)
            {
                // Die Schriften des Fensters sind geoeffnet, solange es laeuft: kurz nach dem Schliessen entfernen
                SpaeterEntfernen(eigen, oben);
                Datei("Entferne " + eigen + " (Einstellungen, Bilder, Schriften), sobald der Launcher geschlossen ist");
            }
            else Datei("Entferne " + eigen + " (Einstellungen, Bilder, Schriften, SA-MP des Launchers)");
        }

        static void SpaeterEntfernen(string eigen, string oben)
        {
            try
            {
                var befehl = "/c ping -n 4 127.0.0.1 >nul & rmdir /s /q \"" + eigen + "\" & rmdir \"" + oben + "\"";
                var start = new ProcessStartInfo(Path.Combine(Environment.SystemDirectory, "cmd.exe"), befehl);
                start.CreateNoWindow = true;
                start.UseShellExecute = false;
                start.WindowStyle = ProcessWindowStyle.Hidden;
                Process.Start(start);
            }
            catch (Exception) { }
        }

        // ---------------------------------------------------------------- Meldungen

        DeinstallationErgebnis Fehlschlag(DeinstallationErgebnis ergebnis, string text)
        {
            ergebnis.Erfolg = false;
            ergebnis.Fehler = text;
            Melden(new Meldung { Nummer = nummer++, Art = MeldungArt.Fehler, Text = text, Fertig = true });
            return ergebnis;
        }

        void Schritt(string text)
        {
            Melden(new Meldung { Nummer = nummer++, Art = MeldungArt.Schritt, Text = text, Fertig = true });
        }

        void Datei(string text)
        {
            Melden(new Meldung { Nummer = nummer++, Art = MeldungArt.Datei, Text = text, Fertig = true });
        }

        void Warnung(string text)
        {
            Melden(new Meldung { Nummer = nummer++, Art = MeldungArt.Warnung, Text = text, Fertig = true });
        }

        static string Anzahl(int anzahl, string einzahl, string mehrzahl)
        {
            return anzahl + " " + (anzahl == 1 ? einzahl : mehrzahl);
        }
    }
}
