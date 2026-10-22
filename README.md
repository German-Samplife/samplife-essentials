# Samplife Launcher

Der Launcher für Spieler des SA-MP-Servers **German Samplife**. Er hält das GTA-San-Andreas-Verzeichnis bei jedem Start auf dem Stand des Servers (Sprach-Chat SampVoice, Browser-Oberfläche CEF, CJ-Kleidung und was der Server sonst braucht), startet SA-MP und verbindet direkt. Bis Version 1.4 hieß er Samplife Essentials Installer.

**Download:** [Samplife Launcher.exe](https://github.com/German-Samplife/samplife-essentials/releases/latest/download/Samplife.Launcher.exe) (aktuelle Fassung, alle Fassungen unter [Releases](https://github.com/German-Samplife/samplife-essentials/releases)). Dieselbe Datei hängt auch als `Samplife.Essentials.Installer.exe` am Release, damit alte Links funktionieren.

Eine einzelne Datei, keine Installation nötig. Sie läuft unter Windows 10 und 11 ohne Zusatzsoftware, weil sie das .NET Framework 4.8 benutzt, das dort zum System gehört.

Der gesamte Quelltext liegt in diesem Repository. Die `.exe` in den Releases baut GitHub aus genau diesem Quelltext (siehe [`.github/workflows/release.yml`](.github/workflows/release.yml)). Entscheidungen und offene Punkte stehen in [`docs/launcher-konzept.md`](docs/launcher-konzept.md).

> **Windows SmartScreen:** Die `.exe` ist nicht signiert. Windows zeigt beim ersten Start „Der Computer wurde durch Windows geschützt". Über „Weitere Informationen" und „Trotzdem ausführen" geht es weiter.

## Was der Spieler sieht

1. **Erster Start:** Der Launcher sucht das GTA-Verzeichnis und fragt „Ist das dein GTA-Verzeichnis?". Der Spieler bestätigt mit „Ja, verwenden" oder wählt einen anderen Ordner. Ohne diese Bestätigung lädt der Launcher nichts.
   - Gesucht wird in den Einträgen von SA-MP, Rockstar und Steam, in der Liste zuletzt gestarteter Programme und in den obersten drei Ordnerebenen der Festplatten, höchstens acht Sekunden lang.
   - Vorgeschlagen werden nur Ordner, in denen neben `gta_sa.exe` auch die Ordner `models` und `data` liegen.
   - Das bestätigte Verzeichnis merkt sich der Launcher unter `%LOCALAPPDATA%\German Samplife\Launcher\einstellungen.json`.
2. **Name:** Der Spieler gibt seinen Namen im Spiel ein. Der Launcher prüft die Regeln von SA-MP und speichert ihn dort, wo SA-MP ihn liest (`HKEY_CURRENT_USER\Software\SAMP\PlayerName`).
3. **Dateien:** Die Liste zeigt jede Komponente mit Name, Beschreibung, Projektseite, Lizenz und Zustand („aktuell", „neue Fassung", „noch nicht eingerichtet", „Datei fehlt oder ist verändert"). Pflicht-Komponenten haben einen gesperrten Haken, optionale wählt der Spieler selbst.
4. **Rechts** stehen die neuesten Updates aus der Community. Ein Klick öffnet das Update auf der Website.
5. **„Spielen":**
   1. Der Launcher lädt, was fehlt oder veraltet ist, mit Fortschritt im Feld „Was gerade passiert".
   2. Er sichert den geprüften Stand.
   3. Er meldet den Spieler beim Server an und startet `samp.exe`, das direkt mit dem Server verbindet.
   4. Danach schließt er sich.
6. **„Gründlich prüfen"** rechnet die Prüfsumme jeder Datei nach und lädt neu, was nicht stimmt.
7. **„Verbinden" auf der Website:** Der Launcher trägt bei jedem Start die Adresse `samplife://` für den Windows-Benutzer ein (ohne Administratorrechte). Ein Link auf `samplife://verbinden` öffnet ihn; ist Verzeichnis und Name schon bekannt, prüft er und verbindet ohne weiteren Klick. Andere Adressen öffnen nur das Fenster.
8. Downloads und geschriebene Dateien stehen auch in `samplife-essentials-protokoll.txt` im GTA-Verzeichnis.

## Was das Programm zusichert

- **Downloads nur über https**, auch nach Weiterleitungen.
- **Jeder Download wird mit SHA-256 geprüft.** Stimmt die Prüfsumme nicht mit der Liste überein, wird nichts davon installiert.
- **Es schreibt nur ins gewählte GTA-Verzeichnis.** Ein Archiv, das mit `..\` hinausschreiben will, stoppt die Installation.
- **`gta_sa.exe`, `samp.dll`, `samp.exe` und `samp.saa` werden nie ersetzt.**
- **Vorhandene Dateien werden vor dem Ersetzen gesichert**, nach `samplife-essentials-sicherung\<Datum_Uhrzeit>\`. Dateien, die schon aktuell sind, bleiben unangetastet.
- **Es löscht nichts.** Fällt eine Komponente aus der Liste oder wählt der Spieler eine optionale ab, bleiben ihre Dateien liegen.
- **Mods des Spielers fasst es nicht an.** Geprüft werden nur Dateien, die der Launcher selbst aus einem Paket abgelegt hat.
- **Der geprüfte Stand wird gesichert** (`samplife-launcher\manifest.json` und `stand.json` im GTA-Verzeichnis, siehe unten). Ist der Server nicht erreichbar, startet der Launcher mit diesem Stand, wenn alle Dateien dazu passen.
- **Keine Administratorrechte ohne Grund.** Nur wenn das GTA-Verzeichnis sie braucht (z. B. unter `C:\Program Files`), bietet das Programm einen Neustart als Administrator an.

## Die Liste der Komponenten (Manifest)

Was installiert wird, steht nicht im Programm, sondern in einer JSON-Datei auf der Website:

```
https://www.german-samplife.de/essentials/manifest.json
```

Antwortet diese Adresse mit einem HTTP-Fehler oder gar nicht, fragt der Launcher die frühere Adresse `https://samplife.isjustfucking.pro/essentials/manifest.json`. Installer bis 1.3.0 kennen nur die frühere Adresse; sie muss deshalb erreichbar bleiben, bis diese Fassungen nicht mehr im Umlauf sind.

So lassen sich Versionen austauschen, ohne den Installer neu zu bauen und neu zu verteilen.

- Die echte Liste liegt auf der Website des Servers unter der Adresse oben und wird dort gepflegt. Du kannst sie im Browser öffnen und nachlesen, was der Installer laden würde.
- [`manifest.beispiel.json`](manifest.beispiel.json) zeigt alle Möglichkeiten des Formats; die Prüfsummen und Adressen darin sind Platzhalter.

Stand der echten Liste: SampVoice 3.1 für SA-MP 0.3.7 R1 und R3, direkt von den GitHub-Releases des Projekts. Das Paket enthält `sampvoice.asi`, den ASI Loader (`vorbisFile.dll`, `vorbisHooked.dll`) und `bass.dll`, `bass_fx.dll`. Ein eigener ASI Loader steht deshalb nicht in der Liste: zwei Loader würden jede Erweiterung doppelt laden.

| Feld | Bedeutung |
|---|---|
| `version` | Format der Liste, derzeit `1`. Bei einem anderen Wert bittet der Installer darum, die aktuelle Fassung zu laden. |
| `hinweis` | Optionaler Text für die Fußzeile des Fensters. |
| `samp_versionen[]` | `name` jeder unterstützten SA-MP-Version, optional `sha256` der zugehörigen `samp.dll`. Mit Prüfsumme erkennt der Installer die Version selbst, ohne bietet er sie nur zur Auswahl an. |
| `komponenten[].id`, `name`, `beschreibung` | Anzeige in der Liste. |
| `komponenten[].quelle`, `lizenz` | Projektseite (wird verlinkt) und Lizenz. |
| `komponenten[].pflicht` | Ab Installer 1.2.0 ohne Wirkung: Alles in `komponenten` ist Pflicht. Ältere Installer (1.0 und 1.1) sperren damit das Abwählen; deshalb steht es in der echten Liste auf `true`. |
| `optionale_komponenten[]` | Ab Launcher 2.0: Aufbau wie `komponenten[]`, der Spieler wählt selbst. Ältere Installer kennen die Liste nicht und laden sie nicht. |
| `stand` | Ab Launcher 2.0: ganze Zahl, bei jeder Änderung an Paketen um 1 erhöhen. Der Launcher meldet sie bei der Anmeldung; der Gamemode verlangt mindestens `LAUNCHER_MINDESTSTAND`. |
| `server.adresse`, `server.port` | Wohin `samp.exe` verbindet. |
| `server.anmeldung` | Adresse, bei der sich der Launcher vor dem Start meldet (`POST`, JSON mit `name`, `stand`, `launcher`; Antwort `{"ok": true}` oder `{"fehler": "…"}`). Leer = keine Anmeldung. |
| `bilder[]` | Adressen von Bildern aus dem Spiel für den Kopf des Fensters (JPG oder PNG, auf der Website). Der Launcher zeigt bei jedem Start eins davon und merkt sich geladene Bilder für den Start ohne Netz. Die Bilder liegen nicht in diesem Repository. |
| `updates` | Adresse der neuesten Updates aus der Community (`{"updates": [{"slug", "datum", "titel", "kurz", "url"}]}`). |
| `launcher.version`, `url`, `sha256`, `groesse` | Aktuelle Fassung des Launchers. Ist sie höher als die laufende, aktualisiert sich der Launcher selbst. Ohne den Block gibt es keine Aktualisierung. |
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

Das Script ruft den C#-Compiler auf, der zu Windows gehört (`%WINDIR%\Microsoft.NET\Framework\v4.0.30319\csc.exe`). Es wird nichts heruntergeladen. Ergebnis: `ausgabe\Samplife Launcher.exe`. Der Ordner `ausgabe` wird von Git ignoriert.

Ist die alte `.exe` gerade geöffnet, lässt sie sich nicht überschreiben. Dann in einen anderen Ordner bauen: `bauen.cmd ausgabe-neu`.

Der Compiler versteht C# 5: kein `$"…"`, kein `?.`, keine Ausdruckskörper.

| Datei | Inhalt |
|---|---|
| `src/Programm.cs` | Start, Aufrufparameter, Laden der Liste, Ablauf ohne Fenster |
| `src/Manifest.cs` | Lesen und Prüfen der Liste |
| `src/GtaVerzeichnis.cs` | GTA-Verzeichnis finden und prüfen, SA-MP-Version erkennen |
| `src/Installation.cs` | Download, Prüfsumme, Entpacken, Sicherung ersetzter Dateien, Protokoll |
| `src/Stand.cs` | Sicherung des geprüften Stands, Abgleich der Dateien |
| `src/Ablauf.cs` | „Spielen": abgleichen, laden, sichern, anmelden, starten |
| `src/Spielstart.cs` | Name für SA-MP, Version der `samp.dll`, Anmeldung, Start von `samp.exe` |
| `src/Aktualisierung.cs` | Selbstaktualisierung |
| `src/Einstellungen.cs` | gemerktes Verzeichnis und optionale Komponenten |
| `src/Neuigkeiten.cs` | Updates aus der Community für die rechte Spalte |
| `src/Titelbild.cs` | Bild für den Kopf laden und zwischenspeichern |
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
| `--manifest <Adresse oder Datei>` | andere Liste verwenden (dann ohne Ersatzadresse) |
| `--ausweich <Adresse oder Datei>` | Ersatz, falls die Liste mit einem HTTP-Fehler antwortet |
| `--gta <Ordner>` | GTA-Verzeichnis vorgeben, die Suche entfällt |
| `--samp <Version>` | SA-MP-Version vorgeben, falls sie nicht erkannt wird (für `--still`) |
| `--suche <Datei.txt>` | nur suchen und die gefundenen Verzeichnisse in die Datei schreiben |
| `--test` | erlaubt eine lokale Liste und http vom eigenen Rechner (`127.0.0.1`) |
| `--still` | ohne Fenster abgleichen, Fehlendes und Veraltetes laden, Stand sichern. Rückgabe: 0 Erfolg, 1 Laden fehlgeschlagen, 2 kein GTA-Verzeichnis, 3 weder Liste noch brauchbare Sicherung, 4 SA-MP fehlt oder falsche Version, 5 Name ungültig, 6 `samp.exe` startet nicht, 7 Selbstaktualisierung fehlgeschlagen, 10 selbst aktualisiert |
| `--spielen` | mit `--still`: danach anmelden und `samp.exe` starten |
| `--name <Name>` | Spielername für `--spielen` |
| `--optional <id>` | optionale Komponente wählen (mehrfach möglich) |
| `--gruendlich` | jede Datei per SHA-256 prüfen statt über Größe und Änderungszeit |
| `--link <Adresse>` | so ruft Windows den Launcher über `samplife://` auf; nur `samplife://verbinden` startet direkt |
| `--adresse-eintragen` | nur `samplife://` eintragen und beenden (mit `--test` als `samplife-test://`) |
| `--titelbild <Datei>` | nur für Bildschirmfotos: Bild für den Kopf |
| `--kein-neustart` | nach einer Selbstaktualisierung nicht neu starten (Rückgabe 10) |
| `--bildschirmfoto <Datei.png> --zustand <Name>` | Fenster als Bild speichern, ohne etwas zu laden. Zustände: `start`, `gefunden`, `bereit`, `laeuft`, `fertig`, `offline` (Beispieldaten), `netz` (Liste wirklich laden), `ablauf` (Start wie beim Spieler, Fund bestätigen, Version wählen) |

## Veröffentlichen

Eine neue Fassung entsteht durch einen Tag:

1. Version in `src/Programm.cs` und `src/AssemblyInfo.cs` anheben und committen.
2. Tag setzen und pushen, zum Beispiel `git tag v2.0.1 && git push origin v2.0.1`.
3. GitHub baut die `.exe`, führt `test/pruefen.py` aus und hängt die Datei samt SHA-256 und Größe an das Release.
4. Im Manifest (Repo samplife) den Block `launcher` mit Version, Adresse, SHA-256 und Größe aus dem Release eintragen. Erst dann aktualisieren sich laufende Launcher.

Bei jedem Push auf einen anderen Branch als `main`, der den Launcher ändert, baut und prüft GitHub ebenfalls und legt Bildschirmfotos aller Zustände des Fensters als Artefakt `bildschirmfotos` ab.

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

### samp-cef Client mit Eingabemodus „nur Maus“

Derselbe Stand `f14fe4f` mit einer Änderung: Browser können zusätzlich im Modus „nur Maus“ laufen. Maus und ESC gehen an die Seite, die Tastatur bleibt beim Spiel, der Spieler kann also weiterlaufen und fahren. Textfelder schalten beim Anklicken von selbst auf volle Eingabe. Patches, Schnittstellen, Abwärtskompatibilität und die Server-Seite stehen in [`cef/README.md`](cef/README.md), die Testliste in [`cef/TESTEN.md`](cef/TESTEN.md).

GitHub baut das Paket mit [`.github/workflows/cef.yml`](.github/workflows/cef.yml) aus dem Original-Commit und den Patches in `cef/`. Ein Tag wie `cef-f14fe4f-sl1` legt das Release `samp-cef-client-<tag>.zip` mit SHA-256 an.

## CJ-Kleidung

Im Ordner [`kleidung/`](kleidung/README.md) liegt das samp-cef-Plugin `samplife-kleidung.dll` für `GTA/cef/plugins/`. Es zieht jede Figur mit Skin 0 (CJ) so an, wie der Server es für ihren Spieler über samp-cef schickt (`kleidung:spieler`), die eigene und die der anderen Spieler, so wie im Singleplayer. Dafür braucht es den samp-cef-Client mit [`cef/plugin-ereignisse.patch`](cef/README.md#server-ereignisse-für-client-plugins) (Client-Version `0x000300`). GitHub baut die Datei mit [`.github/workflows/kleidung.yml`](.github/workflows/kleidung.yml) bisher nur als Artefakt, ohne Release; im Manifest steht sie erst, wenn der Gamemode die Kleidung nutzt. Protokoll, Installation, Testablauf mit dem Filterscript [`kleidung/beispiel/kleidung_test.pwn`](kleidung/beispiel/kleidung_test.pwn) und Quellen stehen in [`kleidung/README.md`](kleidung/README.md).

## Lizenz

Der Quelltext des Installers ist öffentlich einsehbar. Eine Lizenz dafür ist noch nicht festgelegt; bis dahin bleiben alle Rechte bei German Samplife. Die Pakete fremder Projekte stehen unter deren Lizenzen, siehe oben und in der Liste des Installers.
