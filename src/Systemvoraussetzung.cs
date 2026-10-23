using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;

namespace SamplifeEssentials
{
    /// <summary>
    /// Was Windows fuer die Pakete braucht, aber zu keinem Paket gehoert: bisher die DirectX-Laufzeit (D3DX9). samp.dll braucht
    /// d3dx9_25.dll, sampvoice.asi braucht d3dx9_43.dll; beide fehlen auf einem frischen Windows 10 und 11. Eingerichtet wird mit
    /// dem Installer von Microsoft (Adresse und Pruefsumme in der Liste unter "systemvoraussetzungen").
    /// </summary>
    public static class Systemvoraussetzung
    {
        static string ordner = "";

        /// <summary>
        /// 32-Bit-Systemordner (SysWOW64, auf 32-Bit-Windows System32): Dort sucht das Spiel als 32-Bit-Programm die DLLs.
        /// Tests geben mit --test --systemordner einen eigenen Ordner vor.
        /// </summary>
        public static string Ordner
        {
            get { return ordner != "" ? ordner : Environment.GetFolderPath(Environment.SpecialFolder.SystemX86); }
            set { ordner = value ?? ""; }
        }

        /// <summary>Dateien der Voraussetzung, die weder im Systemordner noch im GTA-Verzeichnis liegen.</summary>
        public static List<string> Fehlende(Komponente voraussetzung, string gta)
        {
            var fehlt = new List<string>();
            foreach (var datei in voraussetzung.SystemDateien)
            {
                var da = false;
                try
                {
                    da = File.Exists(Path.Combine(Ordner, datei)) || (!string.IsNullOrEmpty(gta) && File.Exists(Path.Combine(gta, datei)));
                }
                catch (Exception) { }
                if (!da) fehlt.Add(datei);
            }
            return fehlt;
        }

        /// <summary>Alle Voraussetzungen der Liste, bei denen etwas fehlt.</summary>
        public static List<Komponente> Fehlend(Manifest manifest, string gta)
        {
            var liste = new List<Komponente>();
            if (manifest == null) return liste;
            foreach (var voraussetzung in manifest.Systemvoraussetzungen)
            {
                if (Fehlende(voraussetzung, gta).Count > 0) liste.Add(voraussetzung);
            }
            return liste;
        }

        /// <summary>
        /// Richtet die Voraussetzung mit der geladenen und geprueften Datei ein. Wirft InstallationsFehler, wenn es nicht klappt.
        /// </summary>
        public static void Einrichten(Komponente voraussetzung, string download, string gta, bool testmodus, Action<string> schritt)
        {
            if (voraussetzung.SystemTyp != "directx")
                throw new InstallationsFehler("Diese Fassung des Launchers kann " + voraussetzung.Name + " nicht einrichten.");
            var arbeit = Path.Combine(Path.GetTempPath(), "samplife-directx-" + Guid.NewGuid().ToString("N"));
            try
            {
                Directory.CreateDirectory(arbeit);
                // Der Installer von Microsoft entpackt sich ohne Administratorrechte; erst DXSETUP.exe braucht sie
                var redist = Path.Combine(arbeit, "directx_redist.exe");
                File.Copy(download, redist);
                var ziel = Path.Combine(arbeit, "dateien");
                Directory.CreateDirectory(ziel);
                schritt("Entpacke " + voraussetzung.Name);
                var entpacken = new ProcessStartInfo(redist, "/Q /T:\"" + ziel + "\"");
                entpacken.UseShellExecute = false;
                entpacken.CreateNoWindow = true;
                using (var prozess = Process.Start(entpacken))
                {
                    if (!prozess.WaitForExit(5 * 60 * 1000))
                        throw new InstallationsFehler(voraussetzung.Name + " ließ sich nicht entpacken (keine Antwort nach 5 Minuten).");
                    if (prozess.ExitCode != 0)
                        throw new InstallationsFehler(voraussetzung.Name + " ließ sich nicht entpacken (Fehler " + prozess.ExitCode + ").");
                }
                var setup = Path.Combine(ziel, "DXSETUP.exe");
                if (!File.Exists(setup)) throw new InstallationsFehler("Im Paket von " + voraussetzung.Name + " fehlt DXSETUP.exe.");

                schritt("Richte " + voraussetzung.Name + " ein. Windows fragt dafür einmal nach Administratorrechten.");
                var einrichten = new ProcessStartInfo(setup, "/silent");
                einrichten.WorkingDirectory = ziel;
                if (testmodus)
                {
                    // Tests laufen ohne Abfrage der Benutzerkontensteuerung
                    einrichten.UseShellExecute = false;
                    einrichten.CreateNoWindow = true;
                }
                else
                {
                    einrichten.UseShellExecute = true;
                    einrichten.Verb = "runas";
                }
                Process prozessSetup;
                try { prozessSetup = Process.Start(einrichten); }
                catch (Win32Exception fehler)
                {
                    if (fehler.NativeErrorCode == 1223)
                        throw new InstallationsFehler("Die Abfrage von Windows wurde abgelehnt. Ohne " + voraussetzung.Name
                            + " startet SA-MP nicht. Versuche es noch einmal und bestätige die Abfrage.");
                    throw new InstallationsFehler(voraussetzung.Name + " konnte nicht gestartet werden: " + fehler.Message);
                }
                using (prozessSetup)
                {
                    if (!prozessSetup.WaitForExit(15 * 60 * 1000))
                        throw new InstallationsFehler(voraussetzung.Name + " antwortet nicht (15 Minuten).");
                    if (prozessSetup.ExitCode != 0)
                        throw new InstallationsFehler(voraussetzung.Name + " meldet einen Fehler (" + prozessSetup.ExitCode + ").");
                }
                var fehlt = Fehlende(voraussetzung, gta);
                if (fehlt.Count > 0)
                    throw new InstallationsFehler(voraussetzung.Name + " ist eingerichtet, aber " + string.Join(", ", fehlt.ToArray()) + " fehlt weiterhin.");
            }
            finally
            {
                try { if (Directory.Exists(arbeit)) Directory.Delete(arbeit, true); }
                catch (Exception) { }
            }
        }
    }
}
