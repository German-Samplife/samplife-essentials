using System;
using System.Globalization;
using System.IO;

namespace SamplifeEssentials
{
    /// <summary>
    /// 4GB-Patch (Large Address Aware): Das Bit IMAGE_FILE_LARGE_ADDRESS_AWARE im Dateikopf der gta_sa.exe erlaubt dem
    /// 32-Bit-Spiel auf 64-Bit-Windows 4 statt 2 GB Speicher. Ohne reicht der Platz bei hoher Aufloesung (1440p, 4K) fuer
    /// samp-cef oft nicht: Chromium bricht dann mit 0xE0000008 (kein Speicher) ab. Freiwillig ueber den Haken „4GB-Patch“.
    /// Der Launcher setzt das Bit nur, wenn es fehlt, sichert die gta_sa.exe vorher nach samplife-essentials-sicherung und
    /// merkt sich in samplife-launcher\laa-gesetzt, dass er es war. Nur dann nimmt er es beim Abwaehlen oder Deinstallieren
    /// wieder zurueck; ein Bit, das der Spieler selbst gesetzt hat, bleibt.
    /// </summary>
    public static class Laa
    {
        const ushort Bit = 0x0020;
        public const string Marke = "laa-gesetzt";

        static string MarkenPfad(string gta)
        {
            return Path.Combine(gta, Stand.Ordner, Marke);
        }

        /// <summary>Stelle der Characteristics im Dateikopf (PE), -1, wenn die Datei keine PE-Datei ist.</summary>
        static long KopfStelle(FileStream strom)
        {
            var puffer = new byte[4];
            strom.Seek(0x3C, SeekOrigin.Begin);
            if (strom.Read(puffer, 0, 4) != 4) return -1;
            var pe = BitConverter.ToInt32(puffer, 0);
            if (pe <= 0 || pe > strom.Length - 24) return -1;
            strom.Seek(pe, SeekOrigin.Begin);
            if (strom.Read(puffer, 0, 4) != 4 || puffer[0] != 'P' || puffer[1] != 'E' || puffer[2] != 0 || puffer[3] != 0) return -1;
            // Signatur (4), dann IMAGE_FILE_HEADER: Machine, NumberOfSections, TimeDateStamp, PointerToSymbolTable,
            // NumberOfSymbols, SizeOfOptionalHeader (zusammen 18 Bytes), dann Characteristics
            return pe + 4 + 18;
        }

        static ushort Lesen(FileStream strom, long stelle)
        {
            var puffer = new byte[2];
            strom.Seek(stelle, SeekOrigin.Begin);
            if (strom.Read(puffer, 0, 2) != 2) throw new IOException("Der Dateikopf der gta_sa.exe ist unvollständig.");
            return BitConverter.ToUInt16(puffer, 0);
        }

        static void Schreiben(FileStream strom, long stelle, ushort wert)
        {
            strom.Seek(stelle, SeekOrigin.Begin);
            strom.Write(BitConverter.GetBytes(wert), 0, 2);
            strom.Flush(true);
        }

        /// <summary>
        /// Haken an (gewuenscht): Bit setzen, wenn es fehlt. Haken aus: nur zuruecknehmen, was der Launcher selbst gesetzt
        /// hat. Rueckgabe: Satz fuers Protokoll, leer, wenn nichts zu tun war. Fehler kommen als Text zurueck und halten den
        /// Start nicht auf.
        /// </summary>
        public static string Anwenden(string gta, bool gewuenscht)
        {
            var exe = Path.Combine(gta, GtaVerzeichnis.SpielDatei);
            var marke = MarkenPfad(gta);
            try
            {
                if (!File.Exists(exe)) return "";
                if (gewuenscht && !Environment.Is64BitOperatingSystem)
                    return "4GB-Patch: Auf 32-Bit-Windows bringt er nichts, der Launcher lässt die gta_sa.exe, wie sie ist.";
                if (!gewuenscht && !File.Exists(marke)) return "";
                using (var strom = new FileStream(exe, FileMode.Open, FileAccess.ReadWrite, FileShare.Read))
                {
                    var stelle = KopfStelle(strom);
                    if (stelle < 0) return "4GB-Patch: Die gta_sa.exe hat keinen gültigen Dateikopf, der Launcher lässt sie, wie sie ist.";
                    var merkmale = Lesen(strom, stelle);
                    var gesetzt = (merkmale & Bit) != 0;
                    if (gewuenscht)
                    {
                        if (gesetzt) return "4GB-Patch: In deiner gta_sa.exe ist er schon gesetzt.";
                        var ordner = Path.Combine(gta, Installation.SicherungsOrdner,
                            "4gb-patch_" + DateTime.Now.ToString("yyyy-MM-dd_HH-mm-ss", CultureInfo.InvariantCulture));
                        Directory.CreateDirectory(ordner);
                        strom.Seek(0, SeekOrigin.Begin);
                        using (var sicherung = new FileStream(Path.Combine(ordner, GtaVerzeichnis.SpielDatei), FileMode.CreateNew)) strom.CopyTo(sicherung);
                        Schreiben(strom, stelle, (ushort)(merkmale | Bit));
                        Directory.CreateDirectory(Path.GetDirectoryName(marke));
                        File.WriteAllText(marke, "");
                        return "4GB-Patch gesetzt: GTA darf jetzt 4 GB Speicher nutzen. Die alte gta_sa.exe liegt in "
                            + Path.Combine(Installation.SicherungsOrdner, Path.GetFileName(ordner)) + ".";
                    }
                    if (gesetzt) Schreiben(strom, stelle, (ushort)(merkmale & ~Bit));
                }
                File.Delete(marke);
                return "4GB-Patch zurückgenommen: GTA nutzt wieder höchstens 2 GB Speicher.";
            }
            catch (Exception fehler)
            {
                return "4GB-Patch ging nicht: " + fehler.Message;
            }
        }
    }
}
