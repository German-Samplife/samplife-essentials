# Samplife Essentials Installer

Der Installer für Spieler des SA-MP-Servers **German Samplife**. Er richtet die Erweiterungen, die man zum Spielen braucht, im GTA-San-Andreas-Verzeichnis ein: den Sprach-Chat SampVoice und, sobald der Server sie nutzt, die Browser-Oberfläche (CEF).

**Download:** [Samplife Essentials Installer.exe](https://github.com/German-Samplife/samplife-essentials/releases/latest/download/Samplife.Essentials.Installer.exe) (aktuelle Fassung, alle Fassungen unter [Releases](https://github.com/German-Samplife/samplife-essentials/releases))

Eine einzelne Datei, keine Installation nötig. Sie läuft unter Windows 10 und 11 ohne Zusatzsoftware, weil sie das .NET Framework 4.8 benutzt, das dort zum System gehört.

Der gesamte Quelltext des Installers liegt in diesem Repository. Die `.exe` in den Releases baut GitHub aus genau diesem Quelltext (siehe [`.github/workflows/release.yml`](.github/workflows/release.yml)).

> **Windows SmartScreen:** Die `.exe` ist nicht signiert. Windows zeigt beim ersten Start „Der Computer wurde durch Windows geschützt". Über „Weitere Informationen" und „Trotzdem ausführen" geht es weiter.

## Was der Spieler sieht

1. Der Installer sucht beim Start selbst nach dem GTA-Verzeichnis und fragt „Ist das dein GTA-Verzeichnis?". Der Spieler bestätigt mit „Ja, verwenden" oder wählt einen anderen Ordner. Bei mehreren Funden wählt er aus. Ohne diese Bestätigung lässt sich nichts installieren.
   - Gesucht wird in den Einträgen von SA-MP, Rockstar und Steam, in der Liste zuletzt gestarteter Programme und in den obersten drei Ordnerebenen der Festplatten, höchstens acht Sekunden lang.
   - Vorgeschlagen werden nur Ordner, in denen neben `gta_sa.exe` auch die Ordner `models` und `data` liegen.
2. Die SA-MP-Version erkennt der Installer an der `samp.dll` im Verzeichnis. Fehlt sie dort (zum Beispiel beim open.mp-Launcher) oder ist sie unbekannt, wählt der Spieler seine Version selbst aus. Der Installer sagt ihm dabei, welche Versionen die Erweiterungen brauchen und dass er im open.mp-Launcher dieselbe einstellen muss.
3. Die Liste zeigt, was installiert wird: Name, Beschreibung, Größe, Projektseite und Lizenz. Abwählen lässt sich nichts: Alles in der Liste gehört zum Spielen auf dem Server dazu.
4. Nach dem Klick auf „Installieren" erscheint das Feld „Was gerade passiert". Jeder Schritt steht dort im Klartext, Downloads mit vollständiger Adresse und Fortschritt, zum Beispiel:
   `Lade SampVoice von https://…/sampvoice-037-r3.zip herunter (0,4 MB von 0,7 MB)`
5. Dasselbe steht danach in `samplife-essentials-protokoll.txt` im GTA-Verzeichnis.

## Was das Programm zusichert

- **Downloads nur über https**, auch nach Weiterleitungen.
- **Jeder Download wird mit SHA-256 geprüft.** Stimmt die Prüfsumme nicht mit der Liste überein, wird nichts davon installiert.
- **Es schreibt nur ins gewählte GTA-Verzeichnis.** Ein Archiv, das mit `..\` hinausschreiben will, stoppt die Installation.
- **`gta_sa.exe`, `samp.dll`, `samp.exe` und `samp.saa` werden nie ersetzt.**
- **Vorhandene Dateien werden vor dem Ersetzen gesichert**, nach `samplife-essentials-sicherung\<Datum_Uhrzeit>\`. Dateien, die schon aktuell sind, bleiben unangetastet.
- **Keine Administratorrechte ohne Grund.** Nur wenn das GTA-Verzeichnis sie braucht (z. B. unter `C:\Program Files`), bietet das Programm einen Neustart als Administrator an.

## Die Liste der Komponenten (Manifest)

Was installiert wird, steht nicht im Programm, sondern in einer JSON-Datei auf der Website:

```
https://samplife.isjustfucking.pro/essentials/manifest.json
```

So lassen sich Versionen austauschen, ohne den Installer neu zu bauen und neu zu verteilen.

- Die echte Liste liegt auf der Website des Servers unter der Adresse oben und wird dort gepflegt. Du kannst sie im Browser öffnen und nachlesen, was der Installer laden würde.
- [`manifest.beispiel.json`](manifest.beispiel.json) zeigt alle Möglichkeiten des Formats; die Prüfsummen und Adressen darin sind Platzhalter.

Stand der echten Liste: SampVoice 3.1 für SA-MP 0.3.7 R1 und R3, direkt von den GitHub-Releases des Projekts. Das Paket enthält `sampvoice.asi`, den ASI Loader (`vorbisFile.dll`, `vorbisHooked.dll`) und `bass.dll`, `bass_fx.dll`. Ein eigener ASI Loader steht deshalb nicht in der Liste: zwei Loader würden jede Erweiterung doppelt laden. CEF fehlt noch, weil es dafür keinen --zustand start|gefunden|laeuft|fertig|netz|ablauf`en, dauerhaft erreichbaren Download gibt.

| Feld | Bedeutung |
|---|---|
| `version` | Format der Liste, derzeit `1`. Bei einem anderen Wert bittet der Installer darum, die aktuelle Fassung zu laden. |
| `hinweis` | Optionaler Text für die Fußzeile des Fensters. |
| `samp_versionen[]` | `name` jeder unterstützten SA-MP-Version, optional `sha256` der zugehörigen `samp.dll`. Mit Prüfsumme erkennt der Installer die Version selbst, ohne bietet er sie nur zur Auswahl an. |
| `komponenten[].id`, `name`, `beschreibung` | Anzeige in der Liste. |
| `komponenten[].quelle`, `lizenz` | Projektseite (wird verlinkt) und Lizenz. |
| `komponenten[].pflicht` | Ab Installer 1.2.0 ohne Wirkung, weil nichts mehr abwählbar ist. Ältere Installer (1.0 und 1.1) sperren damit das Abwählen; deshalb steht es in der echten Liste auf `true`. |
| `pakete[].url` | Adresse des Downloads, muss mit `https://` beginnen. |
| `pakete[].sha256` | Prüfsumme der Datei, 64 Hex-Zeichen. |
| `pakete[].groesse` | Größe in Bytes, für Anzeige und Fortschritt. |
| `pakete[].typ` | `zip` (Standard) wird entpackt, `datei` wird als einzelne Datei abgelegt. |
| `pakete[].samp` | Optional: Paket nur für diese SA-MP-Version. Ohne Angabe gilt es für alle. |
| `pakete[].ziel` | Optional: Unterordner im GTA-Verzeichnis. |
| `pakete[].zip_unterordner` | Optional: nur diesen Ordner des Archivs entpacken, ohne den Ordner selbst. |
| `pakete[].dateiname` | Bei `typ: datei`: Name der Zieldatei. |

Hat eine Komponente nur Pakete für bestimmte SA-MP-Versionen und keine passt, ist sie in der Liste ausgegraut. Solange keine Version erkannt oder gewählt ist, gilt das für alle versionsgebundenen Komponenten.

Prüfsumme und Größe einer Datei unter Windows:

```
Get-FileHash .\sampvoice-037-r3.zip -Algorithm SHA256
(Get-Item .\sampvoice-037-r3.zip).Length
```

### Noch offen

- **CEF in der Liste.** Das Client-Paket ist gebaut und liegt in diesem Repository als Release (siehe unten). In die Liste kommt es, sobald die passende Komponente auf dem Server läuft.
- **Prüfsummen der `samp.dll`.** Ohne sie wählt jeder Spieler seine Version von Hand. Mit den Prüfsummen von 0.3.7 R1 und R3 in `samp_versionen` erkennt der Installer sie bei Spielern, deren `samp.dll` im GTA-Verzeichnis liegt.
- **SA-MP 0.3.DL, R4 und R5.** Für diese Versionen gibt es keinen SampVoice-Client 3.1.

## Bauen

```
bauen.cmd
```

Das Script ruft den C#-Compiler auf, der zu Windows gehört (`%WINDIR%\Microsoft.NET\Framework\v4.0.30319\csc.exe`). Es wird nichts heruntergeladen. Ergebnis: `ausgabe\Samplife Essentials Installer.exe`. Der Ordner `ausgabe` wird von Git ignoriert.

Ist die alte `.exe` gerade geöffnet, lässt sie sich nicht überschreiben. Dann in einen anderen Ordner bauen: `bauen.cmd ausgabe-neu`.

Der Compiler versteht C# 5: kein `$"…"`, kein `?.`, keine Ausdruckskörper.

| Datei | Inhalt |
|---|---|
| `src/Programm.cs` | Start, Aufrufparameter, Laden der Liste |
| `src/Manifest.cs` | Lesen und Prüfen der Liste |
| `src/GtaVerzeichnis.cs` | GTA-Verzeichnis finden und prüfen, SA-MP-Version erkennen |
| `src/Installation.cs` | Download, Prüfsumme, Entpacken, Sicherung, Protokoll |
| `src/Fenster.xaml`, `src/Fenster.cs` | Oberfläche |
| `symbol.ico` | Programmsymbol |

## Testen

```
python test/pruefen.py
```

Ein anderer Pfad zur `.exe` kann als Parameter folgen.

Das Script baut ein nachgebautes GTA-Verzeichnis und Testpakete in einem temporären Ordner, liefert sie über einen lokalen Webserver aus und startet die `.exe` ohne Fenster. Geprüft werden unter anderem: Auswahl des Pakets nach SA-MP-Version, Sicherung ersetzter Dateien, falsche Prüfsumme, Archiv mit `..\`, Archiv mit `gta_sa.exe`, Download über unverschlüsseltes http.

Aufrufparameter für Tests:

| Parameter | Wirkung |
|---|---|
| `--manifest <Adresse oder Datei>` | andere Liste verwenden |
| `--gta <Ordner>` | GTA-Verzeichnis vorgeben, die Suche entfällt |
| `--samp <Version>` | SA-MP-Version vorgeben, falls sie nicht erkannt wird (für `--still`) |
| `--suche <Datei.txt>` | nur suchen und die gefundenen Verzeichnisse in die Datei schreiben |
| `--test` | erlaubt eine lokale Liste und http vom eigenen Rechner (`127.0.0.1`) |
| `--still` | ohne Fenster alle verfügbaren Komponenten installieren; Rückgabe 0 bei Erfolg |
| `--bildschirmfoto <Datei.png> --zustand <Name>` | Fenster als Bild speichern, ohne etwas zu installieren. Zustände: `start`, `gefunden`, `laeuft`, `fertig` (Beispieldaten), `netz` (Liste wirklich laden), `ablauf` (Start wie beim Spieler, Fund bestätigen, Version wählen) |

## Veröffentlichen

Eine neue Fassung entsteht durch einen Tag:

1. Version in `src/Programm.cs` und `src/AssemblyInfo.cs` anheben und committen.
2. Tag setzen und pushen, zum Beispiel `git tag v1.2.1 && git push origin v1.2.1`.
3. GitHub baut die `.exe`, führt `test/pruefen.py` aus und hängt die Datei samt SHA-256 an das Release.

## Pakete in diesem Repository

Die meisten Pakete lädt der Installer direkt von den Projekten selbst. Was es dort nicht als fertigen Download gibt, liegt hier als Release.

### samp-cef Client (Browser-Oberfläche im Spiel)

| | |
|---|---|
| Datei | [`samp-cef-client-f14fe4f.zip`](https://github.com/German-Samplife/samplife-essentials/releases/tag/cef-f14fe4f) |
| Quelle | [zottce/samp-cef](https://github.com/zottce/samp-cef), Commit [`f14fe4fa7f0d036edf445ba02b4bddd4cccac377`](https://github.com/zottce/samp-cef/commit/f14fe4fa7f0d036edf445ba02b4bddd4cccac377) |
| Lizenz | GPL-3.0, siehe [`LIZENZEN/samp-cef-GPL-3.0.txt`](LIZENZEN/samp-cef-GPL-3.0.txt) |
| Gebaut | unverändert aus dem genannten Commit, Windows x86, mit `cargo build --release --target i686-pc-windows-msvc -p cef-client -p cef-renderer -p cef-loader` und `node scripts/package-client.mjs` |
| CEF | 150.0.11 (Chromium 150.0.7871.115), offizielle `windows32`-Distribution |
| SA-MP | 0.3.7 R1 und 0.3.7 R3 |
| SHA-256 | `46c5da7f29632d77d2e43c129329d94cbf83fd9e49d3e34216a987587fdb7e4c` |

Der Quelltext zu diesem Paket ist der verlinkte Commit; es wurden keine Änderungen vorgenommen. Der Client funktioniert nur zusammen mit einem Server, auf dem die samp-cef-Komponente aus demselben Stand läuft.

## Lizenz

Der Quelltext des Installers ist öffentlich einsehbar. Eine Lizenz dafür ist noch nicht festgelegt; bis dahin bleiben alle Rechte bei German Samplife. Die Pakete fremder Projekte stehen unter deren Lizenzen, siehe oben und in der Liste des Installers.
