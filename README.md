# Samplife Launcher

Der Launcher für Spieler des SA-MP-Servers **German Samplife**. Er hält das GTA-San-Andreas-Verzeichnis bei jedem Start auf dem Stand des Servers (Sprach-Chat SampVoice, Browser-Oberfläche CEF, CJ-Kleidung und was der Server sonst braucht), startet SA-MP und verbindet direkt. Bis Version 1.4 hieß er Samplife Essentials Installer.

**Download:** [Samplife Launcher.exe](https://github.com/German-Samplife/samplife-essentials/releases/latest/download/Samplife.Launcher.exe) (aktuelle Fassung, alle Fassungen unter [Releases](https://github.com/German-Samplife/samplife-essentials/releases)). Dieselbe Datei hängt auch als `Samplife.Essentials.Installer.exe` am Release, damit alte Links funktionieren.

Eine einzelne Datei, keine Installation nötig. Sie läuft unter Windows 10 und 11 ohne Zusatzsoftware, weil sie das .NET Framework 4.8 benutzt, das dort zum System gehört.

Der gesamte Quelltext liegt in diesem Repository. Die `.exe` in den Releases baut GitHub aus genau diesem Quelltext (siehe [`.github/workflows/release.yml`](.github/workflows/release.yml)). Entscheidungen und offene Punkte stehen in [`docs/launcher-konzept.md`](docs/launcher-konzept.md).

> **Windows SmartScreen:** Die `.exe` ist nicht signiert. Windows zeigt beim ersten Start „Der Computer wurde durch Windows geschützt". Über „Weitere Informationen" und „Trotzdem ausführen" geht es weiter.

## Was der Spieler sieht

**Bei jedem Start** fragt der Launcher parallel zur Liste der Dateien bei GitHub nach dem neuesten Release. Liegt dort oder in der Liste eine höhere Version als die laufende, lädt er sie, prüft die SHA-256, ersetzt sich selbst und startet neu, ohne dass der Spieler etwas tun muss. Klappt das nicht und verlangt nur GitHub die neue Fassung, geht es mit der laufenden weiter, und der Launcher versucht es beim nächsten Start erneut. Verlangt die Liste sie, bietet er den Download an.

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
7. **Passwort und Serverpasswort:** Neben dem Namen steht freiwillig das Passwort des Accounts. Der Launcher schickt es mit der Anmeldung an die Website; stimmt es, entfällt der Login im Spiel, stimmt es nicht, startet er nicht. Das Kästchen „Serverpasswort“ klappt ein Feld dafür auf; verlangt der Server eins (Abfrage wie die Serverliste von SA-MP), ist es schon angehakt. Der Launcher gibt das Serverpasswort mit `-z` an `gta_sa.exe` weiter und startet dann immer über das Laden der `samp.dll`. Beide Passwörter werden nicht gespeichert und nicht protokolliert.
8. **„Verbinden" auf der Website:** Der Launcher trägt bei jedem Start die Adresse `samplife://` für den Windows-Benutzer ein (ohne Administratorrechte). Ein Link auf `samplife://verbinden` öffnet ihn; ist Verzeichnis und Name schon bekannt, prüft er und verbindet ohne weiteren Klick. Andere Adressen öffnen nur das Fenster.
9. Downloads und geschriebene Dateien stehen auch in `samplife-essentials-protokoll.txt` im GTA-Verzeichnis.
10. **DirectX-Laufzeit:** `samp.dll` braucht `d3dx9_25.dll`, `sampvoice.asi` braucht `d3dx9_43.dll`. Beide gehören zur DirectX-Laufzeit von Microsoft und fehlen auf einem frischen Windows 10 und 11. Fehlt eine davon im 32-Bit-Systemordner (`SysWOW64`) und im GTA-Verzeichnis, lädt der Launcher beim „Spielen“ den Installer von Microsoft (`systemvoraussetzungen` im Manifest), entpackt ihn und startet `DXSETUP.exe /silent`. Windows fragt dafür einmal nach Administratorrechten. In der Liste steht die Laufzeit als „für Windows“ mit „vorhanden“ oder „fehlt auf diesem Rechner“.
11. **Fremde ASI Loader und Konflikte:** Unter dem GTA-Verzeichnis zeigt ein Hinweis, wenn dort ein zweiter ASI Loader liegt (z. B. `dinput8.dll`, `winmm.dll`, `d3d9.dll`; erkannt an der Zeichenkette `*.asi` in der DLL) oder eine Datei aus `konflikte` im Manifest (z. B. eine alte `samplife-kleidung.asi`). „Ausschalten“ benennt sie nach Rückfrage in `<Name>.samplife-aus` um; gelöscht wird nichts. Ein fremder Loader als `vorbisFile.dll` wird nur gemeldet: SampVoice ersetzt ihn beim Einrichten durch seinen eigenen und sichert ihn vorher. Beim „Spielen“ stehen dieselben Hinweise als Warnung im Protokoll; der Start wird nicht verhindert.
12. **„Deinstallieren“** (Fußzeile, nach Rückfrage) entfernt alles, was der Launcher eingerichtet hat, siehe „Deinstallieren“ unten.

## Was das Programm zusichert

- **Downloads nur über https**, auch nach Weiterleitungen.
- **Jeder Download wird mit SHA-256 geprüft.** Stimmt die Prüfsumme nicht mit der Liste überein, wird nichts davon installiert.
- **Es schreibt nur ins gewählte GTA-Verzeichnis.** Ein Archiv, das mit `..\` hinausschreiben will, stoppt die Installation.
- **`gta_sa.exe`, `samp.dll`, `samp.exe` und `samp.saa` werden nie ersetzt.** Eine vorhandene SA-MP-Installation, auch eine andere Version, bleibt unangetastet; der Launcher sagt dann, was zu tun ist.

## Wie SA-MP startet

| Im GTA-Verzeichnis | Start |
|---|---|
| `samp.exe` und eine `samp.dll` aus `samp_versionen` | wie bisher über `samp.exe <adresse>:<port>`, Name aus der Registry |
| eine passende `samp.dll` ohne `samp.exe` | `gta_sa.exe -c -n <Name> -h <IP> -p <Port>` starten und diese `samp.dll` ins Spiel laden |
| eine `samp.dll` in einer anderen Version | kein Start; der Launcher ersetzt sie nicht und erklärt, was zu tun ist |
| keine `samp.dll` | wie oben, mit der eigenen Kopie (`%LOCALAPPDATA%\German Samplife\Launcher\samp\<Version>\samp.dll`) oder der des open.mp-Launchers (`%LOCALAPPDATA%\mp.open.launcher\samp\<Version>\samp.dll`); gibt es keine, richtet der Launcher `samp_client` ein |

Das Laden der `samp.dll` (`src/SampStart.cs`) macht es wie der [open.mp-Launcher](https://github.com/openmultiplayer/launcher/blob/master/src-tauri/src/injector.rs): `LoadLibraryW` im gerade gestarteten `gta_sa.exe`, bis zu zehn Versuche im Abstand einer halben Sekunde. Geladen wird nur eine `samp.dll`, deren Prüfsumme oder Dateiversion zu `samp_versionen` passt.

### SA-MP-Client 0.3.7-R1 (Release `samp-client-0.3.7-R1`)

| | |
|---|---|
| Dateien | [`samp.dll`](https://github.com/German-Samplife/samplife-essentials/releases/download/samp-client-0.3.7-R1/samp.dll) und [`samp-client-0.3.7-R1-dateien.zip`](https://github.com/German-Samplife/samplife-essentials/releases/download/samp-client-0.3.7-R1/samp-client-0.3.7-R1-dateien.zip) (Ordner `SAMP`, `samp.saa`, Schriften, `sampgui.png`, `mouse.png`, `bass.dll`) |
| Quelle | unverändert aus `https://assets.open.mp/samp_clients.7z`, das auch der open.mp-Launcher lädt; MD5 von Archiv und `samp.dll` wie im Quelltext des open.mp-Launchers (`src/constants/app.ts`) |
| Lizenz | Freeware des SA-MP-Teams, keine freie Lizenz |
- **Vorhandene Dateien werden vor dem Ersetzen gesichert**, nach `samplife-essentials-sicherung\<Datum_Uhrzeit>\`. Dateien, die schon aktuell sind, bleiben unangetastet.
- **Es löscht nichts, außer beim „Deinstallieren“.** Fällt eine Komponente aus der Liste oder wählt der Spieler eine optionale ab, bleiben ihre Dateien liegen.
- **Es merkt sich, was es angelegt hat** (`samplife-launcher\installiert.json`): je Datei, ob sie neu war, ob eine Fassung davor gesichert wurde (mit Pfad der Sicherung) oder ob sie schon genau so dalag. Es zählt nur die erste Berührung; spätere Aktualisierungen derselben Datei ändern den Eintrag nicht.
- **Mods des Spielers fasst es nicht an.** Geprüft werden nur Dateien, die der Launcher selbst aus einem Paket abgelegt hat.
- **Der geprüfte Stand wird gesichert** (`samplife-launcher\manifest.json` und `stand.json` im GTA-Verzeichnis, siehe unten). Ist der Server nicht erreichbar, startet der Launcher mit diesem Stand, wenn alle Dateien dazu passen.
- **Keine Administratorrechte ohne Grund.** Nur wenn das GTA-Verzeichnis sie braucht (z. B. unter `C:\Program Files`), bietet das Programm einen Neustart als Administrator an.

## Deinstallieren

„Deinstallieren“ in der Fußzeile geht nur mit bestätigtem GTA-Verzeichnis, auch ohne Server-Liste, und nicht, solange GTA aus diesem Verzeichnis läuft. Nach einer Rückfrage:

1. **Dateien der Pakete** laut `samplife-launcher\installiert.json`: Was neu war, wird gelöscht. Was der Launcher ersetzt hat, kommt aus der Sicherung in der Fassung von vor dem Launcher zurück, nicht in einer älteren eigenen Fassung. Was schon genau so dalag, bleibt. Dazu gehören auch die Dateien, die `samp_client` ins GTA-Verzeichnis gelegt hat.
2. **Im Spiel angelegte Dateien** aus `entfernen` der Komponenten, laut der gesicherten Liste (`samplife-launcher\manifest.json`).
3. **Ausgeschaltete Dateien** (`.samplife-aus`) bekommen ihren Namen zurück.
4. Fehlt danach `vorbisFile.dll`, legt der Launcher das Original aus dem Spiel zurück (im Paket von SampVoice heißt es `vorbisHooked.dll`). Ohne `vorbisFile.dll` startet GTA nicht.
5. Ordner, die dadurch leer sind (z. B. `cef`, `SAMP`), dann `samplife-launcher\`, `samplife-essentials-sicherung\` und das Protokoll.
6. Außerhalb des GTA-Verzeichnisses: der eigene Ordner unter `%LOCALAPPDATA%\German Samplife\Launcher` (ohne die laufende `.exe`, falls sie dort liegt; Schriften, die das Fenster gerade benutzt, kurz nach dem Schließen), die Adresse `samplife://` und `HKEY_CURRENT_USER\Software\SAMP`, aber nur, wenn der Launcher diesen Schlüssel angelegt hat (Merkdatei `samp-schluessel-angelegt`).

Was bleibt: die `.exe` des Launchers, die DirectX-Laufzeit (gehört zu Windows und anderen Spielen), Mods des Spielers und eine SA-MP-Installation, die es vor dem Launcher gab.

**Installationen von Launcher 2.0 und älter** haben keine `installiert.json`. Der Launcher rekonstruiert sie dann einmal: Reicht das Protokoll bis zur ältesten Sicherung zurück, zählen dessen Zeilen „Sichere vorhandene X nach Y“ und „Schreibe X“. Sonst gelten alle Dateien aus `stand.json` als neu, außer sie liegen in der ältesten Sicherung; von dort kommen sie zurück. Die Rekonstruktion wird gespeichert, sobald der Launcher das nächste Mal etwas einrichtet oder ausschaltet, und dann wie eine echte Liste weitergeführt.

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
| `samp_client` | Ab Launcher 2.0: SA-MP selbst, Aufbau wie eine Komponente. Nur für GTA-Verzeichnisse ohne `samp.dll` und nur, wenn auch der open.mp-Launcher keine passende `samp.dll` hat. Im GTA-Verzeichnis legt das Paket nur ab, was fehlt; danach prüft und ersetzt der Launcher es nie. Ein Paket mit `"ablage": "launcher"` (nur `typ: datei`, nur hier) landet im Ordner des Launchers, `ziel` relativ dazu, z. B. die `samp.dll` unter `samp/0.3.7-R1`. Eigener Block, weil Installer 1.x jede Komponente installieren würden. |
| `bilder[]` | Adressen von Bildern aus dem Spiel für den Kopf des Fensters (JPG oder PNG, auf der Website). Der Launcher zeigt bei jedem Start eins davon und merkt sich geladene Bilder für den Start ohne Netz. Die Bilder liegen nicht in diesem Repository. |
| `updates` | Adresse der neuesten Updates aus der Community (`{"updates": [{"slug", "datum", "titel", "kurz", "url"}]}`). |
| `launcher.version`, `url`, `sha256`, `groesse` | Fassung, die der Server mindestens verlangt. Ist sie höher als die laufende, aktualisiert sich der Launcher selbst; klappt das nicht, startet er nicht. Neue Releases findet der Launcher ab 2.2.0 auch ohne diesen Block auf GitHub (siehe „Veröffentlichen“). |
| `pakete[].url` | Adresse des Downloads, muss mit `https://` beginnen. |
| `pakete[].sha256` | Prüfsumme der Datei, 64 Hex-Zeichen. |
| `pakete[].groesse` | Größe in Bytes, für Anzeige und Fortschritt. |
| `pakete[].typ` | `zip` (Standard) wird entpackt, `datei` wird als einzelne Datei abgelegt. |
| `pakete[].samp` | Optional: Paket nur für diese SA-MP-Version. Ohne Angabe gilt es für alle. |
| `pakete[].ziel` | Optional: Unterordner im GTA-Verzeichnis. |
| `pakete[].zip_unterordner` | Optional: nur diesen Ordner des Archivs entpacken, ohne den Ordner selbst. |
| `pakete[].dateiname` | Bei `typ: datei`: Name der Zieldatei. |
| `komponenten[].entfernen[]` | Ab Launcher 2.1: Dateien und Ordner relativ zum GTA-Verzeichnis, die die Komponente erst im Spiel anlegt (Logs, Cache). „Deinstallieren“ entfernt sie mit. |
| `systemvoraussetzungen[]` | Ab Launcher 2.1: was Windows für die Pakete braucht. `id`, `name`, `beschreibung`, `quelle`, `lizenz`, `typ` (bisher nur `directx`; unbekannte Typen werden übergangen), `dateien[]` (Dateinamen, die im 32-Bit-Systemordner oder im GTA-Verzeichnis liegen müssen), `url`, `sha256`, `groesse` des Installers. Bei `directx` der Redist von Microsoft, der sich mit `/Q /T:<Ordner>` entpackt. „Deinstallieren“ lässt sie da. |
| `konflikte[]` | Ab Launcher 2.1: `datei` (relativ zum GTA-Verzeichnis) und `grund`. Liegt die Datei da und stammt nicht vom Launcher, zeigt er einen Hinweis und bietet „Ausschalten“ an. |

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
| `src/Systemvoraussetzung.cs` | DirectX-Laufzeit prüfen und mit dem Installer von Microsoft einrichten |
| `src/Installiert.cs` | Liste der angelegten Dateien (`installiert.json`), Rekonstruktion für Installationen bis 2.0 |
| `src/FremdeDateien.cs` | fremde ASI Loader und Konflikte finden und ausschalten |
| `src/Deinstallation.cs` | „Deinstallieren“ |
| `src/Spielstart.cs` | Name für SA-MP, Version der `samp.dll`, Anmeldung, Start von `samp.exe` |
| `src/Aktualisierung.cs` | Selbstaktualisierung |
| `src/Einstellungen.cs` | gemerktes Verzeichnis und optionale Komponenten |
| `src/Neuigkeiten.cs` | Updates aus der Community für die rechte Spalte |
| `src/Titelbild.cs` | Bild für den Kopf laden und zwischenspeichern |
| `src/Schriften.cs` | mitgebrachte Schriften bereitstellen |
| `src/Pfade.cs` | Ordner außerhalb des GTA-Verzeichnisses |
| `src/Fenster.xaml`, `src/Fenster.cs` | Oberfläche |
| `symbol.ico` | Programmsymbol |

## Testen

```
python test/pruefen.py
```

Ein anderer Pfad zur `.exe` kann als Parameter folgen.

Das Script baut ein nachgebautes GTA-Verzeichnis und Testpakete in einem temporären Ordner, liefert sie über einen lokalen Webserver aus und startet die `.exe` ohne Fenster. Geprüft werden unter anderem: Auswahl des Pakets nach SA-MP-Version, Sicherung ersetzter Dateien, falsche Prüfsumme, Archiv mit `..\`, Archiv mit `gta_sa.exe`, Download über unverschlüsseltes http, Deinstallieren (auch nach Aktualisierungen und für Installationen ohne `installiert.json`), fremde ASI Loader und Konflikte samt Ausschalten, Einrichten der DirectX-Laufzeit mit einem nachgebauten Installer von Microsoft, Selbstaktualisierung aus der Liste und aus einem nachgebauten Release von GitHub (falsche oder fehlende Prüfsumme, Tags anderer Pakete, Vorab-Releases, GitHub nicht erreichbar).

Aufrufparameter für Tests:

| Parameter | Wirkung |
|---|---|
| `--manifest <Adresse oder Datei>` | andere Liste verwenden (dann ohne Ersatzadresse) |
| `--ausweich <Adresse oder Datei>` | Ersatz, falls die Liste mit einem HTTP-Fehler antwortet |
| `--release <Adresse oder Datei>` | andere Quelle für das neueste Release (Aufbau wie die Antwort von `api.github.com/repos/…/releases/latest`). Mit `--test` oder eigener Liste fragt der Launcher GitHub nur, wenn dieser Parameter gesetzt ist |
| `--gta <Ordner>` | GTA-Verzeichnis vorgeben, die Suche entfällt |
| `--samp <Version>` | SA-MP-Version vorgeben, falls sie nicht erkannt wird (für `--still`) |
| `--suche <Datei.txt>` | nur suchen und die gefundenen Verzeichnisse in die Datei schreiben |
| `--deinstallieren` | mit `--still` und `--gta`: alles entfernen, was der Launcher angelegt hat (Rückgabe 0 Erfolg, 1 Fehler) |
| `--fremde <Datei.txt>` | mit `--still`: fremde ASI Loader und Konflikte als `pfad\|art` in die Datei schreiben |
| `--abschalten` | mit `--fremde`: die gefundenen, die sich ausschalten lassen, umbenennen |
| `--systemordner <Ordner>` | mit `--test`: eigener Ordner statt `SysWOW64` für die Prüfung auf DirectX |
| `--selbsttest` | echtes Fenster starten, 6 Sekunden laufen lassen und beenden (Rückgabe 0); ein Absturz endet mit 98 oder 99 statt der Fehlermeldung |
| `--test` | erlaubt eine lokale Liste und http vom eigenen Rechner (`127.0.0.1`) |
| `--still` | ohne Fenster abgleichen, Fehlendes und Veraltetes laden, Stand sichern. Rückgabe: 0 Erfolg, 1 Laden fehlgeschlagen, 2 kein GTA-Verzeichnis, 3 weder Liste noch brauchbare Sicherung, 4 SA-MP fehlt oder falsche Version, 5 Name ungültig, 6 `samp.exe` startet nicht, 7 Selbstaktualisierung fehlgeschlagen, 10 selbst aktualisiert |
| `--spielen` | mit `--still`: danach anmelden und `samp.exe` starten |
| `--name <Name>` | Spielername für `--spielen` |
| `--passwort <Passwort>` | Serverpasswort für `--spielen`; startet dann immer über `gta_sa.exe … -z` |
| `--kontopasswort <Passwort>` | Passwort des Accounts für `--spielen`; geht mit der Anmeldung an die Website (Rückgabe 8, wenn es falsch ist) |
| `--optional <id>` | optionale Komponente wählen (mehrfach möglich) |
| `--gruendlich` | jede Datei per SHA-256 prüfen statt über Größe und Änderungszeit |
| `--link <Adresse>` | so ruft Windows den Launcher über `samplife://` auf; nur `samplife://verbinden` startet direkt |
| `--adresse-eintragen` | nur `samplife://` eintragen und beenden (mit `--test` als `samplife-test://`) |
| `--titelbild <Datei>` | nur für Bildschirmfotos: Bild für den Kopf |
| `--appdaten <Ordner>` | mit `--test`: eigener Ordner statt `%LOCALAPPDATA%` (Einstellungen, Bilder, Schriften) |
| `--kein-neustart` | nach einer Selbstaktualisierung nicht neu starten (Rückgabe 10) |
| `--bildschirmfoto <Datei.png> --zustand <Name>` | Fenster als Bild speichern, ohne etwas zu laden. Zustände: `start`, `gefunden`, `bereit`, `passwort`, `dateien`, `laeuft`, `mehr`, `fertig`, `offline`, `fremd`, `directx`, `deinstalliert` (Beispieldaten), `netz` (Liste wirklich laden), `ablauf` (Start wie beim Spieler, Fund bestätigen, Version wählen) |

## Veröffentlichen

Eine neue Fassung entsteht durch einen Tag:

1. Version in `src/Programm.cs` und `src/AssemblyInfo.cs` anheben und committen.
2. Tag setzen und pushen, zum Beispiel `git tag v2.0.1 && git push origin v2.0.1`.
3. GitHub baut die `.exe`, führt `test/pruefen.py` aus und hängt die Datei samt SHA-256 und Größe an das Release.
4. Fertig: Laufende Launcher ab 2.2.0 finden das Release beim nächsten Start selbst über `api.github.com/repos/German-Samplife/samplife-essentials/releases/latest` und aktualisieren sich. Dafür muss das Release als „Latest“ markiert sein (das macht `release.yml`; Pakete wie Kleidung oder samp-cef legen ihre Releases mit `--latest=false` an), einen Tag `v<Version>` haben und die Datei `Samplife.Launcher.exe` enthalten. Die Prüfsumme liest der Launcher aus der Antwort von GitHub, sonst aus `SHA256SUMS.txt`.
5. Nur wenn der Server eine Fassung verlangt (und für Launcher vor 2.2.0): im Manifest (Repo samplife) den Block `launcher` mit Version, Adresse, SHA-256 und Größe aus dem Release eintragen.

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

## Schriften

Der Launcher bringt Inter und Inter Display mit (`schriften/`, aus [rsms/inter v4.1](https://github.com/rsms/inter/releases/tag/v4.1), SIL Open Font License 1.1, siehe [`LIZENZEN/Inter-OFL-1.1.txt`](LIZENZEN/Inter-OFL-1.1.txt)). Inter Display steht für die Überschriften an Stelle von Inter Tight der Website, weil es Inter Tight nur als variable Schrift gibt und WPF davon nur eine Stärke zeigt.

## Lizenz

Der Quelltext des Installers ist öffentlich einsehbar. Eine Lizenz dafür ist noch nicht festgelegt; bis dahin bleiben alle Rechte bei German Samplife. Die Pakete fremder Projekte stehen unter deren Lizenzen, siehe oben und in der Liste des Installers.
