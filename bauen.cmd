@echo off
rem Baut "Samplife Launcher.exe" mit dem C#-Compiler, der zu Windows gehoert (.NET Framework 4).
rem Es wird nichts heruntergeladen und nichts installiert.
setlocal
set FW=%WINDIR%\Microsoft.NET\Framework\v4.0.30319
if not exist "%FW%\csc.exe" (
  echo Der C#-Compiler wurde nicht gefunden: %FW%\csc.exe
  exit /b 1
)
cd /d "%~dp0"
rem Optional: anderer Ausgabeordner als erster Parameter, z. B. wenn die alte .exe gerade geoeffnet ist
set AUSGABE=%~1
if "%AUSGABE%"=="" set AUSGABE=ausgabe
if not exist "%AUSGABE%" mkdir "%AUSGABE%"
"%FW%\csc.exe" /nologo /target:winexe /platform:x86 /optimize+ /codepage:65001 /warnaserror+ ^
  /out:"%AUSGABE%\Samplife Launcher.exe" ^
  /win32icon:symbol.ico /win32manifest:app.manifest ^
  /resource:src\Fenster.xaml,Fenster.xaml /resource:symbol.ico,symbol.ico ^
  /resource:schriften\Inter-Regular.ttf,schrift.Inter-Regular.ttf /resource:schriften\Inter-Medium.ttf,schrift.Inter-Medium.ttf ^
  /resource:schriften\Inter-SemiBold.ttf,schrift.Inter-SemiBold.ttf /resource:schriften\Inter-Bold.ttf,schrift.Inter-Bold.ttf ^
  /resource:schriften\InterDisplay-SemiBold.ttf,schrift.InterDisplay-SemiBold.ttf /resource:schriften\InterDisplay-Bold.ttf,schrift.InterDisplay-Bold.ttf ^
  /r:"%FW%\WPF\PresentationCore.dll" /r:"%FW%\WPF\PresentationFramework.dll" /r:"%FW%\WPF\WindowsBase.dll" ^
  /r:"%FW%\System.Xaml.dll" /r:"%FW%\System.Web.Extensions.dll" /r:"%FW%\System.Windows.Forms.dll" /r:"%FW%\System.Security.dll" ^
  /r:"%FW%\System.IO.Compression.dll" /r:"%FW%\System.IO.Compression.FileSystem.dll" ^
  src\AssemblyInfo.cs src\Pfade.cs src\Schriften.cs src\Programm.cs src\Manifest.cs src\GtaVerzeichnis.cs src\Installation.cs src\Stand.cs ^
  src\Ablauf.cs src\Systemvoraussetzung.cs src\Installiert.cs src\FremdeDateien.cs src\Deinstallation.cs src\Spielstart.cs src\SampStart.cs src\Aktualisierung.cs src\Einstellungen.cs src\Neuigkeiten.cs src\Titelbild.cs src\Absturzbericht.cs src\Diagnose.cs src\Laa.cs src\Fenster.cs
if errorlevel 1 exit /b 1
echo Fertig: %AUSGABE%\Samplife Launcher.exe
