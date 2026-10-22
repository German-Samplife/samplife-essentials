using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

namespace SamplifeEssentials
{
    /// <summary>
    /// Start ohne samp.exe, so wie der open.mp-Launcher es macht (openmultiplayer/launcher, src-tauri/src/injector.rs):
    /// gta_sa.exe mit den Parametern von SA-MP starten und die samp.dll in das laufende Spiel laden. Die samp.dll liegt
    /// dafuer nicht im GTA-Verzeichnis, sondern im Ordner des Launchers (aus samp_client) oder im Ordner des
    /// open.mp-Launchers. Geladen wird nur eine samp.dll, deren Version in der Liste des Servers steht, und nur in das
    /// gerade gestartete gta_sa.exe.
    /// </summary>
    public static class SampStart
    {
        /// <summary>
        /// Eine samp.dll fuer den Start ohne samp.exe, in dieser Reihenfolge: die eigene Kopie, dann die des
        /// open.mp-Launchers (samp\&lt;Version&gt;\samp.dll). Leer, wenn es keine passende gibt.
        /// </summary>
        public static string DllFinden(Manifest manifest, out string version, out bool vonOpenMp)
        {
            foreach (var sampVersion in manifest.SampVersionen)
            {
                var eigene = EigeneDll(sampVersion.Name);
                if (GtaVerzeichnis.SampVersionVonDatei(eigene, manifest) == sampVersion.Name)
                {
                    version = sampVersion.Name;
                    vonOpenMp = false;
                    return eigene;
                }
                var openMp = Path.Combine(Pfade.OpenMp, "samp", sampVersion.Name, GtaVerzeichnis.SampDatei);
                if (GtaVerzeichnis.SampVersionVonDatei(openMp, manifest) == sampVersion.Name)
                {
                    version = sampVersion.Name;
                    vonOpenMp = true;
                    return openMp;
                }
            }
            version = "";
            vonOpenMp = false;
            return "";
        }

        /// <summary>Hierhin legt samp_client die samp.dll (Paket mit "ablage": "launcher").</summary>
        public static string EigeneDll(string version)
        {
            return Path.Combine(Pfade.Launcher, "samp", version, GtaVerzeichnis.SampDatei);
        }

        /// <summary>Startet gta_sa.exe mit Name, Adresse und Port und laedt die samp.dll hinein.</summary>
        public static Process Starten(string gta, string adresse, int port, string name, string dll)
        {
            var start = new ProcessStartInfo(Path.Combine(gta, GtaVerzeichnis.SpielDatei),
                "-c -n " + name + " -h " + Ipv4(adresse) + " -p " + port);
            start.WorkingDirectory = gta;
            start.UseShellExecute = false;
            var spiel = Process.Start(start);
            try { Laden(spiel, dll); }
            catch (Exception)
            {
                // Ohne SA-MP liefe nur der Einzelspieler; das Spiel wieder schliessen, damit der Fehler klar ist
                try { if (!spiel.HasExited) spiel.Kill(); }
                catch (Exception) { }
                throw;
            }
            return spiel;
        }

        static string Ipv4(string adresse)
        {
            IPAddress ip;
            if (IPAddress.TryParse(adresse, out ip)) return ip.ToString();
            foreach (var gefunden in Dns.GetHostAddresses(adresse))
            {
                if (gefunden.AddressFamily == AddressFamily.InterNetwork) return gefunden.ToString();
            }
            throw new InvalidOperationException("Für " + adresse + " gibt es keine IPv4-Adresse.");
        }

        /// <summary>
        /// Laedt die DLL ueber LoadLibraryW im Spiel. Direkt nach dem Start ist der Prozess unter Umstaenden noch nicht
        /// bereit; wie der open.mp-Launcher mehrere Versuche im Abstand von einer halben Sekunde.
        /// </summary>
        static void Laden(Process spiel, string dll)
        {
            var pfad = Encoding.Unicode.GetBytes(Path.GetFullPath(dll) + "\0");
            var loadLibrary = GetProcAddress(GetModuleHandle("kernel32.dll"), "LoadLibraryW");
            if (loadLibrary == IntPtr.Zero) throw new InvalidOperationException("LoadLibraryW wurde nicht gefunden.");
            Exception letzter = null;
            for (var versuch = 0; versuch < 10; versuch++)
            {
                if (spiel.HasExited) throw new InvalidOperationException("GTA San Andreas hat sich direkt nach dem Start beendet.");
                try
                {
                    if (EinmalLaden(spiel.Id, pfad, loadLibrary)) return;
                }
                catch (Exception fehler) { letzter = fehler; }
                Thread.Sleep(500);
            }
            throw new InvalidOperationException("SA-MP konnte nicht ins Spiel geladen werden" + (letzter == null ? "." : ": " + letzter.Message));
        }

        static bool EinmalLaden(int prozessId, byte[] pfad, IntPtr loadLibrary)
        {
            // PROCESS_CREATE_THREAD | PROCESS_QUERY_INFORMATION | PROCESS_VM_OPERATION | PROCESS_VM_WRITE | PROCESS_VM_READ
            var prozess = OpenProcess(0x043A, false, prozessId);
            if (prozess == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
            var speicher = IntPtr.Zero;
            try
            {
                // MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE
                speicher = VirtualAllocEx(prozess, IntPtr.Zero, (UIntPtr)pfad.Length, 0x3000, 0x04);
                if (speicher == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
                UIntPtr geschrieben;
                if (!WriteProcessMemory(prozess, speicher, pfad, (UIntPtr)pfad.Length, out geschrieben))
                    throw new Win32Exception(Marshal.GetLastWin32Error());
                uint fadenId;
                var faden = CreateRemoteThread(prozess, IntPtr.Zero, UIntPtr.Zero, loadLibrary, speicher, 0, out fadenId);
                if (faden == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
                try
                {
                    WaitForSingleObject(faden, 15000);
                    uint modul;
                    // Rueckgabe von LoadLibraryW: Adresse der geladenen DLL, 0 bei einem Fehler
                    return GetExitCodeThread(faden, out modul) && modul != 0;
                }
                finally { CloseHandle(faden); }
            }
            finally
            {
                // MEM_RELEASE
                if (speicher != IntPtr.Zero) VirtualFreeEx(prozess, speicher, UIntPtr.Zero, 0x8000);
                CloseHandle(prozess);
            }
        }

        [DllImport("kernel32.dll", SetLastError = true)]
        static extern IntPtr OpenProcess(uint zugriff, bool erben, int prozessId);
        [DllImport("kernel32.dll", SetLastError = true)]
        static extern IntPtr VirtualAllocEx(IntPtr prozess, IntPtr adresse, UIntPtr groesse, uint art, uint schutz);
        [DllImport("kernel32.dll", SetLastError = true)]
        static extern bool VirtualFreeEx(IntPtr prozess, IntPtr adresse, UIntPtr groesse, uint art);
        [DllImport("kernel32.dll", SetLastError = true)]
        static extern bool WriteProcessMemory(IntPtr prozess, IntPtr adresse, byte[] daten, UIntPtr groesse, out UIntPtr geschrieben);
        [DllImport("kernel32.dll", SetLastError = true)]
        static extern IntPtr CreateRemoteThread(IntPtr prozess, IntPtr attribute, UIntPtr stapel, IntPtr start, IntPtr parameter,
            uint flags, out uint fadenId);
        [DllImport("kernel32.dll", SetLastError = true)]
        static extern uint WaitForSingleObject(IntPtr handle, uint millisekunden);
        [DllImport("kernel32.dll", SetLastError = true)]
        static extern bool GetExitCodeThread(IntPtr faden, out uint code);
        [DllImport("kernel32.dll", SetLastError = true)]
        static extern bool CloseHandle(IntPtr handle);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
        static extern IntPtr GetModuleHandle(string modul);
        [DllImport("kernel32.dll", CharSet = CharSet.Ansi, ExactSpelling = true)]
        static extern IntPtr GetProcAddress(IntPtr modul, string name);
    }
}
