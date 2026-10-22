@echo off
rem Startet den Samplife Launcher mit der Vorab-Liste aus dem Repository (german-samplife.de:7777), solange die Liste auf der Website den Launcher noch nicht kennt.
cd /d "%~dp0"
start "" "Samplife.Launcher.exe" --manifest https://raw.githubusercontent.com/German-Samplife/samplife-essentials/feature/launcher/test/vorab-manifest.json
