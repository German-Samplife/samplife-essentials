using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Security.Cryptography;
using System.Text.RegularExpressions;
using System.Threading;
using Microsoft.Win32;

namespace SamplifeEssentials
{
    /// <summary>Ein bei der automatischen Suche gefundenes GTA-Verzeichnis.</summary>
    public class Fund
    {
        public string Pfad { get; set; }
        public string Herkunft { get; set; }
        public bool HatSamp { get; set; }
        public string Detail
        {
            get { return (HatSamp ? "Spiel und SA-MP gefunden" : "Spiel gefunden") + " · " + Herkunft; }
        }
    }

    public static class GtaVerzeichnis
    {
        public const string SpielDatei = "gta_sa.exe";
        public const string SampDatei = "samp.dll";

        // Ordner, in denen kein Spiel liegt und deren Durchsuchen nur Zeit kostet
        static readonly string[] Uebersprungen =
        {
            "windows", "programdata", "system volume information", "recovery", "perflogs", "msocache",
            "node_modules", "appdata", "windowsapps", "microsoft", "common files"
        };

        /// <summary>
        /// Sucht GTA-Verzeichnisse: Eintraege von SA-MP, Rockstar und Steam, zuletzt gestartete Programme
        /// und zum Schluss die obersten Ordnerebenen der Festplatten. Verzeichnisse mit SA-MP stehen vorn.
        /// </summary>
        public static List<Fund> Suchen(CancellationToken abbruch)
        {
            var funde = new List<Fund>();
            var gesehen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            Action<string, string> pruefen = delegate(string verzeichnis, string herkunft)
            {
                if (string.IsNullOrWhiteSpace(verzeichnis)) return;
                string voll;
                try { voll = Path.GetFullPath(verzeichnis.Trim().Trim('"')).TrimEnd(Path.DirectorySeparatorChar); }
                catch (Exception) { return; }
                if (!gesehen.Add(voll) || !SiehtNachSpielAus(voll)) return;
                funde.Add(new Fund { Pfad = voll, Herkunft = herkunft, HatSamp = HatSamp(voll) });
            };

            foreach (var kandidat in AusRegistrierung()) pruefen(kandidat.Key, kandidat.Value);
            foreach (var bibliothek in SteamBibliotheken())
                pruefen(Path.Combine(bibliothek, @"steamapps\common\Grand Theft Auto San Andreas"), "Steam");
            foreach (var basis in new[] { Environment.GetEnvironmentVariable("ProgramFiles(x86)"), Environment.GetEnvironmentVariable("ProgramFiles") })
            {
                if (string.IsNullOrEmpty(basis)) continue;
                pruefen(Path.Combine(basis, @"Rockstar Games\GTA San Andreas"), "üblicher Installationsort");
            }

            // Zum Schluss die Festplatten selbst, begrenzt in Tiefe und Zeit
            var uhr = Stopwatch.StartNew();
            var start = new List<KeyValuePair<string, int>>();
            foreach (var ordner in new[] { Environment.SpecialFolder.DesktopDirectory, Environment.SpecialFolder.MyDocuments, Environment.SpecialFolder.UserProfile })
            {
                var pfad = Environment.GetFolderPath(ordner);
                if (pfad != "") start.Add(new KeyValuePair<string, int>(pfad, 2));
            }
            try
            {
                foreach (var laufwerk in DriveInfo.GetDrives())
                {
                    if (laufwerk.DriveType == DriveType.Fixed && laufwerk.IsReady)
                        start.Add(new KeyValuePair<string, int>(laufwerk.RootDirectory.FullName, 3));
                }
            }
            catch (Exception) { }
            foreach (var eintrag in start)
            {
                Durchsuchen(eintrag.Key, eintrag.Value, pruefen, uhr, abbruch);
            }

            funde.Sort(delegate(Fund a, Fund b) { return b.HatSamp.CompareTo(a.HatSamp); });
            return funde;
        }

        static void Durchsuchen(string ordner, int tiefe, Action<string, string> pruefen, Stopwatch uhr, CancellationToken abbruch)
        {
            if (abbruch.IsCancellationRequested || uhr.ElapsedMilliseconds > 8000) return;
            pruefen(ordner, "auf der Festplatte gefunden");
            if (tiefe <= 0) return;
            string[] unterordner;
            try { unterordner = Directory.GetDirectories(ordner); }
            catch (Exception) { return; }
            foreach (var unter in unterordner)
            {
                var name = Path.GetFileName(unter).ToLowerInvariant();
                if (name.StartsWith("$") || name.StartsWith(".") || Array.IndexOf(Uebersprungen, name) >= 0) continue;
                // Verknuepfte Ordner (z. B. "Dokumente und Einstellungen") fuehren auf dieselben Dateien und werden ausgelassen
                try { if ((File.GetAttributes(unter) & FileAttributes.ReparsePoint) != 0) continue; }
                catch (Exception) { continue; }
                Durchsuchen(unter, tiefe - 1, pruefen, uhr, abbruch);
            }
        }

        static IEnumerable<KeyValuePair<string, string>> AusRegistrierung()
        {
            var ergebnis = new List<KeyValuePair<string, string>>();
            // SA-MP merkt sich hier den Pfad zur gta_sa.exe
            try
            {
                using (var schluessel = Registry.CurrentUser.OpenSubKey(@"Software\SAMP"))
                {
                    var exe = schluessel == null ? null : schluessel.GetValue("gta_sa_exe") as string;
                    if (!string.IsNullOrEmpty(exe)) ergebnis.Add(new KeyValuePair<string, string>(Path.GetDirectoryName(exe.Trim('"')), "aus den Einstellungen von SA-MP"));
                }
            }
            catch (Exception) { }
            // Eintrag der Original-Installation von Rockstar
            foreach (var sicht in new[] { RegistryView.Registry32, RegistryView.Registry64 })
            {
                try
                {
                    using (var basis = RegistryKey.OpenBaseKey(RegistryHive.LocalMachine, sicht))
                    using (var schluessel = basis.OpenSubKey(@"SOFTWARE\Rockstar Games\GTA San Andreas\Installation"))
                    {
                        var exe = schluessel == null ? null : schluessel.GetValue("ExePath") as string;
                        if (!string.IsNullOrEmpty(exe)) ergebnis.Add(new KeyValuePair<string, string>(Path.GetDirectoryName(exe.Trim('"')), "aus der Installation von Rockstar"));
                    }
                }
                catch (Exception) { }
            }
            // Windows merkt sich gestartete Programme mit vollem Pfad
            foreach (var ort in new[]
            {
                @"Software\Classes\Local Settings\Software\Microsoft\Windows\Shell\MuiCache",
                @"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Compatibility Assistant\Store"
            })
            {
                try
                {
                    using (var schluessel = Registry.CurrentUser.OpenSubKey(ort))
                    {
                        if (schluessel == null) continue;
                        foreach (var name in schluessel.GetValueNames())
                        {
                            var klein = name.ToLowerInvariant();
                            var stelle = klein.IndexOf(@"\gta_sa.exe", StringComparison.Ordinal);
                            if (stelle < 0) stelle = klein.IndexOf(@"\samp.exe", StringComparison.Ordinal);
                            if (stelle > 0) ergebnis.Add(new KeyValuePair<string, string>(name.Substring(0, stelle), "zuletzt von hier gestartet"));
                        }
                    }
                }
                catch (Exception) { }
            }
            return ergebnis;
        }

        static IEnumerable<string> SteamBibliotheken()
        {
            var ergebnis = new List<string>();
            try
            {
                string steam;
                using (var schluessel = Registry.CurrentUser.OpenSubKey(@"Software\Valve\Steam"))
                {
                    steam = schluessel == null ? null : schluessel.GetValue("SteamPath") as string;
                }
                if (string.IsNullOrEmpty(steam)) return ergebnis;
                steam = steam.Replace('/', Path.DirectorySeparatorChar);
                ergebnis.Add(steam);
                var liste = Path.Combine(steam, @"steamapps\libraryfolders.vdf");
                if (File.Exists(liste))
                {
                    foreach (Match treffer in Regex.Matches(File.ReadAllText(liste), "\"path\"\\s+\"([^\"]+)\""))
                        ergebnis.Add(treffer.Groups[1].Value.Replace(@"\\", @"\"));
                }
            }
            catch (Exception) { }
            return ergebnis;
        }

        /// <summary>Bestes Ergebnis der Suche, sonst leer (fuer den Aufruf ohne Fenster).</summary>
        public static string Finden()
        {
            var funde = Suchen(CancellationToken.None);
            return funde.Count > 0 ? funde[0].Pfad : "";
        }

        /// <summary>
        /// Strenger als IstGtaVerzeichnis: Die Suche schlaegt nur Ordner vor, in denen neben der gta_sa.exe
        /// auch die Spieldaten liegen. Eine einzelne gta_sa.exe im Download-Ordner zaehlt nicht.
        /// </summary>
        public static bool SiehtNachSpielAus(string verzeichnis)
        {
            try
            {
                return IstGtaVerzeichnis(verzeichnis)
                    && Directory.Exists(Path.Combine(verzeichnis, "models"))
                    && Directory.Exists(Path.Combine(verzeichnis, "data"));
            }
            catch (Exception) { return false; }
        }

        public static bool IstGtaVerzeichnis(string verzeichnis)
        {
            try
            {
                return !string.IsNullOrWhiteSpace(verzeichnis) && File.Exists(Path.Combine(verzeichnis, SpielDatei));
            }
            catch (Exception) { return false; }
        }

        public static bool HatSamp(string verzeichnis)
        {
            try { return File.Exists(Path.Combine(verzeichnis, SampDatei)); }
            catch (Exception) { return false; }
        }

        /// <summary>
        /// Name der installierten SA-MP-Version laut Liste, sonst leer. Zuerst ueber die Pruefsummen der Liste, sonst ueber
        /// die Dateiversion der samp.dll (nur Versionen, die in der Liste stehen).
        /// </summary>
        public static string SampVersion(string verzeichnis, Manifest manifest)
        {
            if (!HatSamp(verzeichnis)) return "";
            string pruefsumme;
            try { pruefsumme = Sha256(Path.Combine(verzeichnis, SampDatei)); }
            catch (Exception) { return ""; }
            foreach (var version in manifest.SampVersionen)
            {
                if (version.Sha256 != "" && version.Sha256 == pruefsumme) return version.Name;
            }
            var dateiVersion = Spielstart.SampDateiVersion(verzeichnis);
            foreach (var version in manifest.SampVersionen)
            {
                if (dateiVersion != "" && string.Equals(version.Name, dateiVersion, StringComparison.OrdinalIgnoreCase)) return version.Name;
            }
            return "";
        }

        /// <summary>Prueft, ob im Verzeichnis geschrieben werden darf (unter "Programme" meist nur als Administrator).</summary>
        public static bool IstBeschreibbar(string verzeichnis)
        {
            var probe = Path.Combine(verzeichnis, ".samplife-essentials-" + Guid.NewGuid().ToString("N") + ".tmp");
            try
            {
                File.WriteAllText(probe, "");
                File.Delete(probe);
                return true;
            }
            catch (Exception) { return false; }
        }

        public static string Sha256(string datei)
        {
            using (var strom = File.OpenRead(datei))
            using (var sha = SHA256.Create())
            {
                var bytes = sha.ComputeHash(strom);
                return BitConverter.ToString(bytes).Replace("-", "").ToLowerInvariant();
            }
        }
    }
}
