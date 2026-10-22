@echo off
rem Startet den Samplife Launcher mit der Liste fuer den Testserver (german-samplife.de:7799) statt der echten.
cd /d "%~dp0"
start "" "Samplife.Launcher.exe" --manifest https://raw.githubusercontent.com/German-Samplife/samplife-essentials/feature/launcher/test/testserver-manifest.json
