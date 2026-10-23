using System;
using System.Collections.Generic;
using System.IO;

namespace SamplifeEssentials
{
    public enum FremdArt
    {
        Loader,          // ein zweiter ASI Loader unter einem anderen Namen (z. B. dinput8.dll): laesst sich ausschalten
        LoaderErsetzt,   // ein fremder ASI Loader als vorbisFile.dll: SampVoice ersetzt ihn beim Einrichten und sichert ihn vorher
        Konflikt         // eine Datei aus "konflikte" der Liste: laesst sich ausschalten
    }

    /// <summary>Eine Datei des Spielers, die mit den Komponenten nicht zusammenpasst.</summary>
    public class FremdeDatei
    {
        public string Pfad;            // relativ zum GTA-Verzeichnis
        public FremdArt Art;
        public string Grund = "";

        public bool Abschaltbar { get { return Art != FremdArt.LoaderErsetzt; } }

        public string Text
        {
            get
            {
                switch (Art)
                {
                    case FremdArt.Loader:
                        return Pfad + ": ein zweiter ASI Loader. Der Launcher bringt mit SampVoice schon einen mit (vorbisFile.dll); "
                            + "zwei Loader sind eine häufige Ursache für Abstürze beim Start.";
                    case FremdArt.LoaderErsetzt:
                        return Pfad + ": ein anderer ASI Loader. SampVoice ersetzt ihn durch seinen eigenen und sichert ihn vorher; "
                            + "deine .asi-Dateien lädt der neue genauso.";
                    default:
                        return Pfad + (Grund == "" ? ": passt nicht zu den Dateien des Servers." : ": " + Grund);
                }
            }
        }
    }

    /// <summary>
    /// Sucht im GTA-Verzeichnis nach ASI Loadern, die nicht vom Launcher stammen, und nach Dateien aus "konflikte" der Liste.
    /// Ausschalten heisst umbenennen (Endung .samplife-aus); die Deinstallation benennt sie zurueck. Geloescht wird nichts.
    /// </summary>
    public static class FremdeDateien
    {
        public const string Endung = ".samplife-aus";
        const string EigenerLoader = "vorbisFile.dll";

        // Namen, unter denen ASI Loader (Ultimate ASI Loader, Silent's ASI Loader und andere) neben gta_sa.exe liegen.
        // GTA San Andreas bringt keine davon mit, nur vorbisFile.dll.
        static readonly string[] LoaderNamen =
        {
            "dinput8.dll", "winmm.dll", "version.dll", "dsound.dll", "d3d8.dll", "d3d9.dll", "d3d10.dll", "d3d11.dll", "dxgi.dll",
            "ddraw.dll", "wininet.dll", "winhttp.dll", "msacm32.dll", "msvfw32.dll", "xlive.dll", "binkw32.dll", "x3daudio1_7.dll",
            "xinput1_3.dll"
        };

        public static List<FremdeDatei> Suchen(string gta, Manifest manifest)
        {
            var funde = new List<FremdeDatei>();
            if (!GtaVerzeichnis.IstGtaVerzeichnis(gta)) return funde;
            var eigene = Installiert.Laden(gta);
            foreach (var name in LoaderNamen)
            {
                var pfad = Path.Combine(gta, name);
                if (eigene.Suchen(name) == null && IstAsiLoader(pfad)) funde.Add(new FremdeDatei { Pfad = name, Art = FremdArt.Loader });
            }
            // vorbisFile.dll gehoert zum Spiel; ein Loader dort ist in Ordnung, wenn er vom Launcher kommt
            if (eigene.Suchen(EigenerLoader) == null && IstAsiLoader(Path.Combine(gta, EigenerLoader)))
                funde.Add(new FremdeDatei { Pfad = EigenerLoader, Art = FremdArt.LoaderErsetzt });
            if (manifest != null)
            {
                foreach (var konflikt in manifest.Konflikte)
                {
                    try
                    {
                        if (eigene.Suchen(konflikt.Datei) == null && File.Exists(Path.Combine(gta, konflikt.Datei)))
                            funde.Add(new FremdeDatei { Pfad = konflikt.Datei, Art = FremdArt.Konflikt, Grund = konflikt.Grund });
                    }
                    catch (Exception) { }
                }
            }
            return funde;
        }

        /// <summary>
        /// Ein ASI Loader sucht nach „*.asi“; diese Zeichenkette (ASCII oder UTF-16) steht in seiner DLL. Die Original-DLLs von
        /// Windows und GTA enthalten sie nicht. Dateien ueber 16 MB sind keine Loader und werden nicht gelesen.
        /// </summary>
        public static bool IstAsiLoader(string pfad)
        {
            try
            {
                var info = new FileInfo(pfad);
                if (!info.Exists || info.Length > 16 * 1024 * 1024) return false;
                var daten = File.ReadAllBytes(pfad);
                for (var i = 0; i + 3 < daten.Length; i++)
                {
                    if (daten[i] != (byte)'.') continue;
                    if (Klein(daten[i + 1]) == 'a' && Klein(daten[i + 2]) == 's' && Klein(daten[i + 3]) == 'i'
                        && (i + 4 == daten.Length || !Buchstabe(daten[i + 4])))
                        return true;
                    // UTF-16: . 0 a 0 s 0 i 0, danach kein weiterer Buchstabe
                    if (i + 7 < daten.Length && daten[i + 1] == 0 && Klein(daten[i + 2]) == 'a' && daten[i + 3] == 0
                        && Klein(daten[i + 4]) == 's' && daten[i + 5] == 0 && Klein(daten[i + 6]) == 'i' && daten[i + 7] == 0
                        && !(i + 9 < daten.Length && Buchstabe(daten[i + 8]) && daten[i + 9] == 0))
                        return true;
                }
            }
            catch (Exception) { }
            return false;
        }

        static char Klein(byte b)
        {
            return char.ToLowerInvariant((char)b);
        }

        static bool Buchstabe(byte b)
        {
            return (b >= 'a' && b <= 'z') || (b >= 'A' && b <= 'Z') || (b >= '0' && b <= '9') || b == '_';
        }

        /// <summary>
        /// Benennt die Datei in &lt;Name&gt;.samplife-aus um und haelt das fest. Rueckgabe: der neue Name relativ zum GTA-Verzeichnis.
        /// Wirft bei Fehlern (z. B. fehlende Rechte).
        /// </summary>
        public static string Abschalten(string gta, FremdeDatei datei)
        {
            if (!datei.Abschaltbar) throw new InvalidOperationException(datei.Pfad + " lässt sich nicht ausschalten.");
            var basis = Path.GetFullPath(gta).TrimEnd(Path.DirectorySeparatorChar);
            var quelle = Path.GetFullPath(Path.Combine(basis, datei.Pfad));
            if (!quelle.StartsWith(basis + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
                throw new InvalidOperationException("Ungültiger Pfad: " + datei.Pfad);
            var ziel = quelle + Endung;
            for (var n = 2; File.Exists(ziel); n++) ziel = quelle + Endung + n;
            var verzeichnis = Installiert.Laden(gta);
            File.Move(quelle, ziel);
            var ablage = ziel.Substring(basis.Length + 1);
            verzeichnis.Abgeschaltet.Add(new AbgeschalteteDatei { Pfad = datei.Pfad, Ablage = ablage });
            try { verzeichnis.Speichern(gta); }
            catch (Exception)
            {
                // Ohne Eintrag wuesste die Deinstallation nichts davon: lieber rueckgaengig machen
                File.Move(ziel, quelle);
                throw;
            }
            return ablage;
        }
    }
}
