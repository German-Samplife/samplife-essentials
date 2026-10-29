using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;
using Microsoft.Win32;

namespace SamplifeEssentials
{
    /// <summary>
    /// Diagnose, wenn Sprach-Chat oder Browser-Oberflaechen nicht laden. Overlays, Aufnahme- und FPS-Tools und Virenscanner
    /// haengen sich in gta_sa.exe ein und koennen die Grafik-Hooks von SampVoice und samp-cef stoeren. Deshalb liest der
    /// Launcher, waehrend das Spiel laeuft, welche DLLs in gta_sa.exe geladen sind (Fenster.SpielBeobachten), und schreibt
    /// sie nach letzte-module.txt. Nach dem Spiel prueft er svlog.txt: Fehlt die Zeile „device interface … created“, hat
    /// SampVoice das Grafikgeraet von GTA nie gesehen, und der Launcher bietet an, Liste und Logs ueber dieselbe Adresse wie
    /// den Absturzbericht zu schicken (Art „diagnose“). Pfade im Benutzerordner stehen als %USERPROFILE%, aus dem GTA-Verzeichnis
    /// nur relativ, damit kein Windows-Name mitgeht.
    /// </summary>
    public static class Diagnose
    {
        public const int MaxModule = 30000;
        public const int MaxLog = 20000;
        public const string SvLog = "svlog.txt";

        public static string ModulDatei
        {
            get { return Path.Combine(Pfade.Launcher, "letzte-module.txt"); }
        }

        /// <summary>Liste der DLLs in gta_sa.exe aus diesem GTA-Verzeichnis als Text, leer, wenn das Spiel nicht laeuft.</summary>
        public static string ModuleLesen(string gta)
        {
            string ziel;
            Process[] prozesse;
            try
            {
                ziel = Path.GetFullPath(Path.Combine(gta, GtaVerzeichnis.SpielDatei));
                prozesse = Process.GetProcessesByName(Path.GetFileNameWithoutExtension(GtaVerzeichnis.SpielDatei));
            }
            catch (Exception) { return ""; }
            var text = "";
            foreach (var prozess in prozesse)
            {
                try
                {
                    if (text != "" || prozess.HasExited) continue;
                    string pfad;
                    try { pfad = Path.GetFullPath(prozess.MainModule.FileName); }
                    catch (Exception fehler)
                    {
                        // Laeuft GTA mit Administratorrechten (Kompatibilitaetsmodus), darf der Launcher es nicht lesen
                        text = "Module in gta_sa.exe ließen sich nicht lesen: " + fehler.Message + "\n" + Kompatibilitaet(ziel);
                        continue;
                    }
                    if (!string.Equals(pfad, ziel, StringComparison.OrdinalIgnoreCase)) continue;
                    text = Auflisten(prozess, gta, ziel);
                }
                catch (Exception fehler) { text = "Module in gta_sa.exe ließen sich nicht lesen: " + fehler.Message; }
                finally { prozess.Dispose(); }
            }
            if (text.Length > MaxModule) text = text.Substring(0, MaxModule);
            return text;
        }

        static string Auflisten(Process prozess, string gta, string ziel)
        {
            var windows = Environment.GetFolderPath(Environment.SpecialFolder.Windows).TrimEnd('\\') + "\\";
            var gtaOrdner = Path.GetFullPath(gta).TrimEnd('\\') + "\\";
            var fremde = new List<string>();
            var imGta = new List<string>();
            var system = new List<string>();
            foreach (ProcessModule modul in prozess.Modules)
            {
                string datei;
                try { datei = modul.FileName; }
                catch (Exception) { continue; }
                if (datei.StartsWith(gtaOrdner, StringComparison.OrdinalIgnoreCase))
                    imGta.Add(datei.Substring(gtaOrdner.Length) + Herkunft(datei, false));
                else if (datei.StartsWith(windows, StringComparison.OrdinalIgnoreCase))
                    system.Add(Path.GetFileName(datei));
                else
                    fremde.Add(Anonym(datei) + Herkunft(datei, true));
            }
            var text = new StringBuilder();
            text.Append("Module in gta_sa.exe, Launcher ").Append(Programm.Version).Append(", ").Append(Absturzbericht.Windows()).Append('\n');
            text.Append(Kompatibilitaet(ziel)).Append('\n');
            text.Append("\nFremde Programme (nicht Windows, nicht GTA): ").Append(fremde.Count).Append('\n');
            foreach (var zeile in fremde) text.Append("  ").Append(zeile).Append('\n');
            text.Append("\nIm GTA-Verzeichnis: ").Append(imGta.Count).Append('\n');
            foreach (var zeile in imGta) text.Append("  ").Append(zeile).Append('\n');
            system.Sort(StringComparer.OrdinalIgnoreCase);
            text.Append("\nWindows: ").Append(system.Count).Append('\n').Append("  ").Append(string.Join(", ", system)).Append('\n');
            return text.ToString();
        }

        // Hersteller und Produkt aus der Datei, daran erkennt man Overlays (Discord, Steam, RivaTuner, OBS, Virenscanner)
        static string Herkunft(string datei, bool produkt)
        {
            try
            {
                var info = FileVersionInfo.GetVersionInfo(datei);
                var teile = new List<string>();
                if (!string.IsNullOrEmpty(info.CompanyName)) teile.Add(info.CompanyName.Trim());
                if (produkt && !string.IsNullOrEmpty(info.ProductName)) teile.Add(info.ProductName.Trim());
                if (!string.IsNullOrEmpty(info.FileVersion)) teile.Add(info.FileVersion.Trim());
                return teile.Count == 0 ? "" : " (" + string.Join(", ", teile) + ")";
            }
            catch (Exception) { return ""; }
        }

        /// <summary>Kompatibilitaetsmodus von Windows fuer gta_sa.exe (z. B. „RUNASADMIN WINXPSP3“), stoert die Hooks oft.</summary>
        static string Kompatibilitaet(string exe)
        {
            var werte = new List<string>();
            foreach (var wurzel in new[] { Registry.CurrentUser, Registry.LocalMachine })
            {
                try
                {
                    using (var schluessel = wurzel.OpenSubKey(@"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers"))
                    {
                        var wert = schluessel == null ? null : schluessel.GetValue(exe) as string;
                        if (!string.IsNullOrEmpty(wert)) werte.Add(wert.Trim());
                    }
                }
                catch (Exception) { }
            }
            return "Kompatibilitätsmodus: " + (werte.Count == 0 ? "aus" : string.Join(" / ", werte));
        }

        /// <summary>Ersetzt den Benutzerordner durch %USERPROFILE%.</summary>
        public static string Anonym(string text)
        {
            var profil = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
            if (string.IsNullOrEmpty(text) || string.IsNullOrEmpty(profil)) return text ?? "";
            var stelle = text.IndexOf(profil, StringComparison.OrdinalIgnoreCase);
            while (stelle >= 0)
            {
                text = text.Substring(0, stelle) + "%USERPROFILE%" + text.Substring(stelle + profil.Length);
                stelle = text.IndexOf(profil, stelle + 13, StringComparison.OrdinalIgnoreCase);
            }
            return text;
        }

        /// <summary>Haelt die Liste fuer den naechsten Bericht fest (auch nach einem Neustart des Launchers).</summary>
        public static void ModuleMerken(string text)
        {
            try
            {
                Directory.CreateDirectory(Pfade.Launcher);
                File.WriteAllText(ModulDatei, text, new UTF8Encoding(false));
            }
            catch (Exception) { }
        }

        /// <summary>Gemerkte Liste, wenn sie zwischen seitUtc und bisUtc geschrieben wurde, sonst leer.</summary>
        public static string GemerkteModule(DateTime seitUtc, DateTime bisUtc)
        {
            try
            {
                if (!File.Exists(ModulDatei)) return "";
                var zeit = File.GetLastWriteTimeUtc(ModulDatei);
                if (zeit < seitUtc || zeit > bisUtc) return "";
                return File.ReadAllText(ModulDatei, Encoding.UTF8);
            }
            catch (Exception) { return ""; }
        }

        /// <summary>svlog.txt aus dem GTA-Verzeichnis, wenn SampVoice sie seit seitUtc geschrieben hat, sonst leer.</summary>
        public static string SvLogLesen(string gta, DateTime seitUtc)
        {
            return LogLesen(Path.Combine(gta, SvLog), seitUtc);
        }

        /// <summary>
        /// SampVoice hat geladen, aber das Grafikgeraet von GTA nie gesehen: Dann fehlt die Zeile „[dbg:render:direct] : device
        /// interface … created“, Sprach-Chat und Anzeige gehen nicht.
        /// </summary>
        public static bool SampVoiceOhneGrafik(string svlog)
        {
            return svlog.IndexOf("[dbg:render:init]", StringComparison.Ordinal) >= 0
                && svlog.IndexOf("device interface", StringComparison.Ordinal) < 0;
        }

        /// <summary>Logs von samp-cef (Ordner cef im GTA-Verzeichnis), die seit seitUtc geschrieben wurden.</summary>
        public static string CefLogsLesen(string gta, DateTime seitUtc)
        {
            var text = new StringBuilder();
            try
            {
                var ordner = Path.Combine(gta, "cef");
                if (!Directory.Exists(ordner)) return "";
                var dateien = new List<string>(Directory.GetFiles(ordner, "*.log", SearchOption.TopDirectoryOnly));
                dateien.AddRange(Directory.GetFiles(ordner, "*.txt", SearchOption.TopDirectoryOnly));
                foreach (var datei in dateien)
                {
                    var inhalt = LogLesen(datei, seitUtc);
                    if (inhalt == "") continue;
                    text.Append("== ").Append(Path.GetFileName(datei)).Append(" ==\n").Append(inhalt).Append("\n\n");
                }
            }
            catch (Exception) { }
            var ergebnis = text.ToString();
            return ergebnis.Length > MaxLog ? ergebnis.Substring(ergebnis.Length - MaxLog) : ergebnis;
        }

        // Ende der Datei (die letzten MaxLog Zeichen), UTF-8 oder Windows-1252
        static string LogLesen(string datei, DateTime seitUtc)
        {
            try
            {
                if (!File.Exists(datei) || File.GetLastWriteTimeUtc(datei) < seitUtc.AddSeconds(-5)) return "";
                byte[] daten;
                using (var strom = new FileStream(datei, FileMode.Open, FileAccess.Read, FileShare.ReadWrite | FileShare.Delete))
                {
                    var laenge = (int)Math.Min(strom.Length, MaxLog * 2);
                    strom.Seek(-laenge, SeekOrigin.End);
                    daten = new byte[laenge];
                    var gelesen = 0;
                    while (gelesen < laenge)
                    {
                        var n = strom.Read(daten, gelesen, laenge - gelesen);
                        if (n <= 0) break;
                        gelesen += n;
                    }
                }
                string text;
                try { text = new UTF8Encoding(false, true).GetString(daten); }
                catch (DecoderFallbackException) { text = Encoding.GetEncoding(1252).GetString(daten); }
                text = Anonym(text.Replace("\0", ""));
                return text.Length > MaxLog ? text.Substring(text.Length - MaxLog) : text;
            }
            catch (Exception) { return ""; }
        }
    }
}
