using System;
using System.IO;
using System.Reflection;

namespace SamplifeEssentials
{
    /// <summary>
    /// Inter und Inter Display (SIL Open Font License 1.1, LIZENZEN/Inter-OFL-1.1.txt) stecken in der .exe, damit der
    /// Launcher ueberall aussieht wie die Website. WPF laedt Schriften nur aus Dateien oder kompilierten Ressourcen; deshalb
    /// werden sie beim Start nach %LOCALAPPDATA%\German Samplife\Launcher\schriften gelegt, wenn sie dort fehlen.
    /// Inter Display ersetzt Inter Tight der Website: Inter Tight gibt es nur als variable Schrift, und davon zeigt WPF nur
    /// eine Staerke.
    /// </summary>
    public static class Schriften
    {
        static readonly string[] Dateien =
        {
            "Inter-Regular.ttf", "Inter-Medium.ttf", "Inter-SemiBold.ttf", "Inter-Bold.ttf",
            "InterDisplay-SemiBold.ttf", "InterDisplay-Bold.ttf"
        };

        /// <summary>Ordner mit den Schriften als Adresse fuer FontFamily, oder null, wenn das nicht geht.</summary>
        public static Uri Bereitstellen()
        {
            try
            {
                var ordner = Path.Combine(Pfade.Launcher, "schriften", Programm.Version);
                Directory.CreateDirectory(ordner);
                var programm = Assembly.GetExecutingAssembly();
                foreach (var datei in Dateien)
                {
                    var ziel = Path.Combine(ordner, datei);
                    using (var quelle = programm.GetManifestResourceStream("schrift." + datei))
                    {
                        if (quelle == null) return null;
                        if (File.Exists(ziel) && new FileInfo(ziel).Length == quelle.Length) continue;
                        using (var ausgabe = File.Create(ziel)) quelle.CopyTo(ausgabe);
                    }
                }
                return new Uri(ordner.TrimEnd(Path.DirectorySeparatorChar) + Path.DirectorySeparatorChar);
            }
            catch (Exception) { return null; }
        }
    }
}
