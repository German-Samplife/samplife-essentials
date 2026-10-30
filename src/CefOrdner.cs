using System;
using System.Diagnostics;
using System.IO;
using System.Security.AccessControl;
using System.Security.Principal;

namespace SamplifeEssentials
{
    /// <summary>
    /// Ordner cef im GTA-Verzeichnis: samp-cef legt dort seinen Zwischenspeicher (cef\user_data) und sein Log (cef\cef.log)
    /// an. Darf es dort nicht schreiben, scheitert der Start von Chromium („CEF runtime initialization failed“), und keine
    /// Browser-Seite laedt. Das passiert vor allem, wenn GTA unter „Programme“ liegt. Ebenso scheitert er, wenn ein
    /// cef-renderer.exe von einem abgestuerzten Spiel noch den Zwischenspeicher haelt.
    /// </summary>
    public static class CefOrdner
    {
        public const string Renderer = "cef-renderer";

        static string Ordner(string gta)
        {
            return Path.Combine(gta, "cef");
        }

        /// <summary>Darf dieser Prozess in cef\user_data schreiben? Probe mit einer leeren Datei.</summary>
        public static bool Beschreibbar(string gta)
        {
            if (!Directory.Exists(Ordner(gta))) return true;
            try
            {
                var daten = Path.Combine(Ordner(gta), "user_data");
                Directory.CreateDirectory(daten);
                var probe = Path.Combine(daten, ".samplife-probe-" + Guid.NewGuid().ToString("N") + ".tmp");
                File.WriteAllText(probe, "");
                File.Delete(probe);
                return true;
            }
            catch (Exception) { return false; }
        }

        public static bool IstAdmin()
        {
            try { return new WindowsPrincipal(WindowsIdentity.GetCurrent()).IsInRole(WindowsBuiltInRole.Administrator); }
            catch (Exception) { return false; }
        }

        /// <summary>
        /// Nur als Administrator: Benutzer duerfen in cef aendern (Unterordner und Dateien erben es). So startet CEF auch, wenn
        /// GTA unter „Programme“ liegt. Rueckgabe: Satz fuers Protokoll, leer, wenn nichts zu tun war.
        /// </summary>
        public static string SchreibrechtGeben(string gta)
        {
            var ordner = Ordner(gta);
            if (!Directory.Exists(ordner) || !IstAdmin()) return "";
            try
            {
                var info = new DirectoryInfo(ordner);
                var sicherheit = info.GetAccessControl();
                var benutzer = new SecurityIdentifier(WellKnownSidType.BuiltinUsersSid, null);
                foreach (FileSystemAccessRule regel in sicherheit.GetAccessRules(true, true, typeof(SecurityIdentifier)))
                {
                    if (regel.IdentityReference == benutzer && regel.AccessControlType == AccessControlType.Allow
                        && (regel.FileSystemRights & FileSystemRights.Modify) == FileSystemRights.Modify
                        && (regel.InheritanceFlags & (InheritanceFlags.ContainerInherit | InheritanceFlags.ObjectInherit))
                            == (InheritanceFlags.ContainerInherit | InheritanceFlags.ObjectInherit))
                        return "";
                }
                sicherheit.AddAccessRule(new FileSystemAccessRule(benutzer, FileSystemRights.Modify,
                    InheritanceFlags.ContainerInherit | InheritanceFlags.ObjectInherit, PropagationFlags.None, AccessControlType.Allow));
                info.SetAccessControl(sicherheit);
                return "Browser-Oberfläche: Der Ordner cef ist jetzt für alle Benutzer beschreibbar, damit sie auch hier startet.";
            }
            catch (Exception fehler)
            {
                return "Browser-Oberfläche: Schreibrecht für den Ordner cef ließ sich nicht setzen: " + fehler.Message;
            }
        }

        /// <summary>
        /// Laeuft kein gta_sa.exe, sind cef-renderer.exe aus diesem GTA-Verzeichnis uebrig geblieben (Absturz) und halten
        /// den Zwischenspeicher fest: beenden. Rueckgabe: wie viele.
        /// </summary>
        public static int VerwaisteBeenden(string gta)
        {
            var anzahl = 0;
            try
            {
                var spiele = Process.GetProcessesByName(Path.GetFileNameWithoutExtension(GtaVerzeichnis.SpielDatei));
                var spielLaeuft = spiele.Length > 0;
                foreach (var spiel in spiele) spiel.Dispose();
                if (spielLaeuft) return 0;
                var ordner = Path.GetFullPath(Ordner(gta)).TrimEnd('\\') + "\\";
                foreach (var prozess in Process.GetProcessesByName(Renderer))
                {
                    try
                    {
                        if (!Path.GetFullPath(prozess.MainModule.FileName).StartsWith(ordner, StringComparison.OrdinalIgnoreCase)) continue;
                        prozess.Kill();
                        prozess.WaitForExit(3000);
                        anzahl++;
                    }
                    catch (Exception) { }
                    finally { prozess.Dispose(); }
                }
            }
            catch (Exception) { }
            return anzahl;
        }
    }
}
