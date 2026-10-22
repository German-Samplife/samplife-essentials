using System;
using System.IO;

namespace SamplifeEssentials
{
    /// <summary>
    /// Ordner ausserhalb des GTA-Verzeichnisses. Tests geben mit --test --appdaten einen eigenen Ordner vor, damit sie weder
    /// die Einstellungen des Rechners noch den Ordner des open.mp-Launchers anfassen.
    /// </summary>
    public static class Pfade
    {
        static string lokal = "";

        /// <summary>%LOCALAPPDATA% oder der Ordner fuer Tests.</summary>
        public static string Lokal
        {
            get { return lokal != "" ? lokal : Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData); }
            set { lokal = value ?? ""; }
        }

        /// <summary>Eigener Ordner: Einstellungen, Bilder, Schriften, SA-MP fuer den Start ohne samp.exe.</summary>
        public static string Launcher
        {
            get { return Path.Combine(Lokal, "German Samplife", "Launcher"); }
        }

        /// <summary>Hier legt der open.mp-Launcher je Version eine samp.dll ab (samp\0.3.7-R1\samp.dll).</summary>
        public static string OpenMp
        {
            get { return Path.Combine(Lokal, "mp.open.launcher"); }
        }
    }
}
