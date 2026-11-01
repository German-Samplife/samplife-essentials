using System;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

namespace SamplifeEssentials
{
    /// <summary>
    /// Eingefrorenes Spiel: Haengt gta_sa.exe, beendet Windows es irgendwann mit 0xCFFFFFFF („reagiert nicht“), oder der
    /// Spieler schliesst es selbst. CrashInfo.SA.asi und SA-MP schreiben dabei nichts, weil kein Fehler auftritt. Deshalb
    /// fragt der Launcher, solange er offen ist, jede Sekunde, ob das Fenster von GTA haengt (IsHungAppWindow, Windows sagt
    /// das nach 5 Sekunden ohne Nachrichten). Haengt es Fenster.HaengerSchwelle Sekunden am Stueck, schreibt er einen
    /// Speicherabzug (Minidump mit allen Threads und ihren Stapeln, ohne den uebrigen Arbeitsspeicher), gepackt nach
    /// „%LOCALAPPDATA%\German Samplife\Launcher\haenger“. Daran sieht man, wo das Spiel steht. Nach dem Spiel geht er mit der
    /// Diagnose an das Team (Feld speicherabzug, Base64). Es bleiben nur die letzten AufbewahrenAnzahl Dateien liegen.
    /// </summary>
    public static class Haenger
    {
        // Gepackt groesser darf er nicht sein, sonst geht die Diagnose ohne ihn
        public const long MaxGroesse = 8 * 1024 * 1024;
        const int AufbewahrenAnzahl = 3;
        // MiniDumpNormal (Stapel aller Threads, Module) mit MiniDumpWithUnloadedModules und MiniDumpWithThreadInfo
        const uint DumpArt = 0x00000020 | 0x00001000;

        [DllImport("user32.dll")]
        static extern bool IsHungAppWindow(IntPtr fenster);

        [DllImport("dbghelp.dll", SetLastError = true)]
        static extern bool MiniDumpWriteDump(IntPtr prozess, uint prozessId, SafeFileHandle datei, uint art,
            IntPtr ausnahme, IntPtr benutzer, IntPtr rueckruf);

        public static string Ordner
        {
            get { return Path.Combine(Pfade.Launcher, "haenger"); }
        }

        /// <summary>Haengt das Hauptfenster des Spiels gerade? false, wenn es kein Fenster gibt oder sich nichts lesen laesst.</summary>
        public static bool Haengt(Process prozess)
        {
            try
            {
                prozess.Refresh();
                var fenster = prozess.MainWindowHandle;
                return fenster != IntPtr.Zero && IsHungAppWindow(fenster);
            }
            catch (Exception) { return false; }
        }

        /// <summary>
        /// Schreibt den Speicherabzug und packt ihn (gta_sa-haenger-JJJJMMTT-HHMMSS.dmp.gz). Rueckgabe: Pfad, leer bei einem
        /// Fehler (z. B. wenn GTA als Administrator laeuft und der Launcher nicht).
        /// </summary>
        public static string Schreiben(Process prozess)
        {
            string roh = null;
            try
            {
                Directory.CreateDirectory(Ordner);
                var name = "gta_sa-haenger-" + DateTime.Now.ToString("yyyyMMdd-HHmmss", System.Globalization.CultureInfo.InvariantCulture);
                roh = Path.Combine(Ordner, name + ".dmp");
                var gepackt = roh + ".gz";
                using (var datei = new FileStream(roh, FileMode.Create, FileAccess.ReadWrite, FileShare.None))
                {
                    if (!MiniDumpWriteDump(prozess.Handle, (uint)prozess.Id, datei.SafeFileHandle, DumpArt, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero))
                        return "";
                }
                using (var quelle = File.OpenRead(roh))
                using (var ziel = File.Create(gepackt))
                using (var packer = new GZipStream(ziel, CompressionLevel.Optimal))
                    quelle.CopyTo(packer);
                Aufraeumen();
                return gepackt;
            }
            catch (Exception) { return ""; }
            finally
            {
                try { if (roh != null && File.Exists(roh)) File.Delete(roh); }
                catch (Exception) { }
            }
        }

        /// <summary>Gepackter Speicherabzug als Base64, leer, wenn er fehlt oder groesser als MaxGroesse ist.</summary>
        public static string Lesen(string datei)
        {
            try
            {
                if (string.IsNullOrEmpty(datei) || !File.Exists(datei) || new FileInfo(datei).Length > MaxGroesse) return "";
                return Convert.ToBase64String(File.ReadAllBytes(datei));
            }
            catch (Exception) { return ""; }
        }

        /// <summary>Juengster Speicherabzug seit seitUtc (fuer „Problem melden“), sonst leer.</summary>
        public static string Neuester(DateTime seitUtc)
        {
            try
            {
                if (!Directory.Exists(Ordner)) return "";
                string neuester = "";
                var zeit = seitUtc;
                foreach (var datei in new DirectoryInfo(Ordner).GetFiles("gta_sa-haenger-*.dmp.gz"))
                {
                    if (datei.LastWriteTimeUtc <= zeit) continue;
                    zeit = datei.LastWriteTimeUtc;
                    neuester = datei.FullName;
                }
                return neuester;
            }
            catch (Exception) { return ""; }
        }

        /// <summary>Groesse in KB fuer Vorschau und Hinweis, 0, wenn die Datei fehlt.</summary>
        public static long GroesseKb(string datei)
        {
            try { return string.IsNullOrEmpty(datei) || !File.Exists(datei) ? 0 : (new FileInfo(datei).Length + 1023) / 1024; }
            catch (Exception) { return 0; }
        }

        static void Aufraeumen()
        {
            try
            {
                var dateien = new DirectoryInfo(Ordner).GetFiles("gta_sa-haenger-*.dmp.gz");
                Array.Sort(dateien, (a, b) => b.LastWriteTimeUtc.CompareTo(a.LastWriteTimeUtc));
                for (var i = AufbewahrenAnzahl; i < dateien.Length; i++)
                {
                    try { dateien[i].Delete(); }
                    catch (Exception) { }
                }
            }
            catch (Exception) { }
        }
    }
}
