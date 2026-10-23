# Samplife Launcher – Konzept

Stand: 21.10.2026. Aus dem Essentials Installer wird der **Samplife Launcher**. Er hält bei jedem Start die Dateien im GTA-Verzeichnis auf dem Stand des Servers, startet SA-MP und verbindet direkt mit German Samplife. Der Server lässt nur Spieler herein, die über den Launcher in der verlangten Fassung kommen.

Dieses Dokument hält die Entscheidungen samt Begründung fest. Was noch zu klären ist, steht am Ende.

## Anforderungen

1. Eigene Objekte und Modelle müssen beim Spieler liegen, denn SA-MP 0.3.7-R1 lädt nichts nach.
2. Bei jedem Start gleicht der Launcher die Dateien mit dem Manifest ab und lädt Fehlendes und Veraltetes nach, bevor er verbindet.
3. Er startet `samp.exe` selbst und verbindet direkt.
4. Ist das Manifest nicht erreichbar, startet er mit einer lokalen Sicherung des letzten geprüften Stands.
5. Der Server weist Clients ab, die nicht über den Launcher oder nicht in der verlangten Fassung verbinden.
6. Pflicht-Pakete lassen sich nicht abwählen. Optionale Mods entscheidet der Spieler. Der Fortschritt ist vor dem Verbinden zu sehen.
7. Was der Installer heute kann, bleibt: GTA-Verzeichnis finden und bestätigen lassen, vorhandene Dateien sichern, nichts löschen, Mods des Spielers nicht anfassen.

## Entscheidungen

### 1. Technik: der Installer wird zum Launcher

Der Launcher ist der weitergebaute Installer: C# mit .NET Framework 4.8 und WPF, eine einzelne `.exe`, gebaut mit dem C#-Compiler, der zu Windows gehört (`bauen.cmd`). Er bekommt die Programmversion **2.0.0** und heißt `Samplife Launcher.exe`.

Gründe:

- **Keine Zusatzsoftware.** .NET Framework 4.8 gehört zu Windows 10 und 11.
- **Erprobter Code.** Suche nach dem GTA-Verzeichnis, Download nur über https, SHA-256-Prüfung, Schutz vor `..\` in Archiven, Sicherung ersetzter Dateien und das Protokoll laufen seit Version 1.0 bei den Spielern. Ein Neubau in einer anderen Sprache müsste das alles noch einmal erfinden und testen.
- **Gleiche Pipeline.** Ein Tag `v*` baut, prüft und veröffentlicht die `.exe` wie bisher (`.github/workflows/release.yml`).

### 2. Ablauf beim Spieler

1. **Erster Start:** Der Launcher sucht das GTA-Verzeichnis und lässt es bestätigen, genau wie der Installer. Dazu kommt das Feld für den Spielernamen. Beides merkt er sich.
2. **Jeder Start:**
   1. Manifest laden. Ist der Launcher selbst veraltet, aktualisiert er sich zuerst (siehe 7.).
   2. Dateien prüfen und anzeigen, was aktuell ist und was fehlt.
3. **„Spielen“:**
   1. Fehlendes und Veraltetes laden, mit Fortschrittsbalken und dem Feld „Was gerade passiert“.
   2. Den geprüften Stand sichern.
   3. Beim Server anmelden (siehe 5.).
   4. Den Namen für SA-MP setzen, `samp.exe` starten und den Launcher schließen.

Schlägt ein Schritt fehl, startet das Spiel nicht. Der Launcher sagt, was fehlt. Ausnahme: Fällt nur das Manifest aus, gilt der Weg aus 8.

### 3. Abgleich: Paketprüfsumme im Manifest, Dateiliste beim Spieler

Das Manifest nennt wie bisher **Pakete** mit SHA-256 (Zip oder einzelne Datei), keine Liste aller Dateien.

Beim Installieren schreibt der Launcher für jedes Paket mit, welche Dateien daraus im GTA-Verzeichnis liegen: Pfad, Größe, Änderungszeit und SHA-256. Diese Liste prüft er bei jedem Start.

- **Paket veraltet:** Die Prüfsumme im Manifest weicht von der installierten ab. Das Paket wird neu geladen.
- **Datei fehlt oder ist verändert:** Dann wird das Paket neu geladen. Geprüft wird schnell über Größe und Änderungszeit. Weicht eines ab, rechnet der Launcher die SHA-256 der Datei nach. Die Schaltfläche „Alles gründlich prüfen“ rechnet jede Datei nach.
- **Komponente neu im Manifest:** wird geladen.
- **Komponente fällt aus dem Manifest:** Ihre Dateien bleiben liegen, denn der Launcher löscht nichts. Sie werden nur nicht mehr geprüft.

Gründe:

- Das Manifest bleibt klein und für Menschen lesbar. Allein der CEF-Client hat Hunderte Dateien.
- Ältere Installer lesen das Manifest weiter.
- Prüfsummen für jede einzelne Datei müsste ein Script bei jedem Paket neu erzeugen. Ein vergessener Lauf hieße, dass alle Spieler bei jedem Start nachladen.
- Der schnelle Weg über Größe und Änderungszeit ist nötig, denn die SHA-256 der rund 340 MB CEF bei jedem Start kostet spürbar Zeit.

Dateien, die nicht aus einem Paket stammen (eigene Mods, Skins, Einstellungen des Spielers), kennt der Launcher nicht. Er prüft sie nicht und fasst sie nicht an.

### 4. Manifest: Format 1 bleibt, neue Felder kommen dazu

`version` bleibt `1`. Alle bisherigen Felder behalten ihre Bedeutung. Neu und optional sind:

| Feld | Bedeutung |
|---|---|
| `stand` | Ganze Zahl, wird bei jeder Änderung an Paketen um 1 erhöht. Der Launcher meldet sie bei der Anmeldung (siehe 5.). |
| `server.adresse`, `server.port` | Wohin `samp.exe` verbindet, z. B. `german-samplife.de` und `7777`. |
| `server.anmeldung` | Adresse der Anmeldung, z. B. `https://www.german-samplife.de/api/launcher/start`. |
| `launcher.version`, `url`, `sha256`, `groesse` | Aktuelle Fassung des Launchers für die Selbstaktualisierung. |
| `optionale_komponenten[]` | Aufbau wie `komponenten[]`. Der Spieler wählt sie selbst. |
| `bilder[]` | Bilder aus dem Spiel für den Kopf des Fensters, von der Website geladen und zwischengespeichert (die Screenshots dürfen nicht ins öffentliche Repository). |
| `updates` | Adresse der neuesten Updates aus der Community (`/api/updates`). Der Launcher zeigt sie in der Spalte „Neu auf dem Server“. |

Gründe:

- Installer 1.x prüfen nur `version == 1` und überlesen unbekannte Felder. Wer noch den alten Installer hat, bekommt weiter die Pflicht-Pakete.
- Optionale Komponenten stehen in einer eigenen Liste. Installer ab 1.2 installieren alles aus `komponenten`, ohne zu fragen; sie sollen nichts Optionales einspielen.

### 5. Verlangte Fassung: Manifest für den Launcher, Mindeststand im Gamemode

Die verlangte Fassung wird auf zwei Ebenen mitgeteilt:

- **Was aktuell ist, sagt das Manifest:** `stand` für die Pakete und `launcher.version` für das Programm. Der Launcher richtet sich danach, bevor er verbindet.
- **Was der Server mindestens braucht, steht im Gamemode:** `LAUNCHER_MINDESTSTAND`. Erhöht wird er im selben PR, in dem der Gamemode etwas Neues aus einem Paket braucht, wie heute schon `CEF_CLIENT_VERSION_INPUT_MODE`. Nicht jede Änderung am Manifest muss alle Spieler ausschließen, die gerade spielen. Eine Prüfung im PR stellt sicher, dass der Mindeststand nie über dem `stand` des Manifests liegt.

**Wie der Server davon erfährt:** SA-MP 0.3.7 überträgt beim Verbinden nichts, was ein Launcher frei setzen könnte. Deshalb meldet sich der Launcher direkt vor dem Spielstart über die Website an:

```
POST https://www.german-samplife.de/api/launcher/start
{ "name": "Max_Mustermann", "stand": 4, "launcher": "2.0.0" }
```

- **Website:** Sie schreibt IPv4-Adresse, Name, Stand, Launcher-Version und Zeit in die Tabelle `launcher_anmeldungen` der gemeinsamen Datenbank.
- **Gamemode:** Er sucht beim Verbinden (`OnPlayerConnect`, ohne Bots) nach einer Anmeldung mit derselben IP und demselben Namen aus den letzten **12 Stunden** und einem Stand ab `LAUNCHER_MINDESTSTAND`. Fehlt sie, gibt es Hinweis und Kick nach dem Muster von `CefVersionKick`: Chatzeilen und Fenster mit dem Grund, Eintrag im `KICK_LOG`, Kick nach 1,5 Sekunden.
- **Warum 12 Stunden:** Nach jedem Neustart des Servers (Auto-Deploy, nächtlicher Neustart) verbindet SA-MP von selbst neu, ohne Launcher. Diese Spieler sollen drinbleiben. Wer länger spielt, startet nach einem Neustart einmal den Launcher.
- **Schalter `LAUNCHER_PFLICHT`:** Er steht zunächst auf `false`. Alles kann damit vorab auf `develop` und `main`, ohne dass jemand gekickt wird. Eingeschaltet wird erst, wenn der Launcher veröffentlicht ist und die Website auf ihn verweist.

Gründe für die Anmeldung über die Website:

- Am Client muss nichts geändert werden: kein neuer CEF-Client, kein Plugin.
- Website und Gamemode teilen sich schon die MariaDB.
- Website, Spielserver und Manifest liegen auf demselben Rechner, nur unter IPv4 (keine AAAA-Einträge). Anmeldung und Spiel laufen also über dieselbe Adresse. Die Website nimmt nur IPv4-Anmeldungen an.
- Fällt die Website aus, fehlt auch der Login im Spiel, der eine CEF-Seite von dort ist. Die Anmeldung bringt also keinen neuen Ausfallgrund.

**Grenzen:** Das ist kein Kopierschutz und kein Anti-Cheat. Wer die Anfrage nachbaut, kommt auch ohne Launcher herein. Gedacht ist der Weg für alle, die ehrlich spielen und aktuelle Dateien brauchen.

Bei Anschlüssen, die Web und Spiel über verschiedene öffentliche IPs schicken (manche Carrier-NAT, VPN mit getrenntem Tunnel), passt die IP nicht. Der Hinweis beim Kick nennt deshalb auch die IP, die der Server sieht.

### 6. Pflicht und optional

- **Pflicht** (`komponenten[]`): Sie werden ohne Abwahl geladen. In der Liste steht ein gesperrter Haken.
- **Optional** (`optionale_komponenten[]`): Der Spieler setzt den Haken selbst, der Launcher merkt sich die Wahl.
  - Mit Haken prüft und aktualisiert er die Komponente wie eine Pflicht-Komponente.
  - Ohne Haken lässt er sie in Ruhe. Wird ein Haken entfernt, bleiben die Dateien liegen (nichts löschen).
  - Der Server verlangt keine optionalen Komponenten.

### 7. Selbstaktualisierung

Nennt das Manifest eine höhere `launcher.version`, läuft es so:

1. Der Launcher lädt die neue `.exe` über https.
2. Er prüft die SHA-256.
3. Er benennt sich selbst in `Samplife Launcher.exe.alt` um (das erlaubt Windows auch bei einem laufenden Programm) und legt die neue Datei an seinen Platz.
4. Er startet die neue Datei mit denselben Parametern. Die neue Fassung löscht beim Start eine übrig gebliebene `.alt`.

Schlägt etwas fehl, bleibt die alte Datei. Der Launcher zeigt dann einen Hinweis mit dem Download-Link und startet das Spiel nicht.

Warum selbst aktualisieren statt nur Hinweis:

- Der Spieler muss nichts tun.
- Eine Datei, die ein Programm selbst schreibt, hat keine Download-Markierung des Browsers, SmartScreen fragt deshalb nicht noch einmal.
- Fehlt der Block `launcher` im Manifest, gibt es keine Aktualisierung. So lässt sich das Manifest vorbereiten, bevor das Release existiert.

### 8. Sicherung des letzten geprüften Stands

Im GTA-Verzeichnis liegt der Ordner `samplife-launcher\`:

| Datei | Inhalt |
|---|---|
| `manifest.json` | das zuletzt erfolgreich angewendete Manifest, Byte für Byte wie geladen |
| `stand.json` | Format, Zeitpunkt der Prüfung, `stand`, SHA-256 von `manifest.json`, je Paket die Prüfsumme und die Liste seiner Dateien (Pfad, Größe, Änderungszeit, SHA-256) |

- **Geschrieben** wird nur nach einem vollständig erfolgreichen Abgleich. Beide Dateien gehen zuerst in temporäre Dateien und werden dann ersetzt. Die vorige Fassung bleibt als `.alt` liegen. So hinterlässt ein Absturz keine halbe Datei.
- **Ohne Manifest-Server** (beide Adressen ohne Antwort) prüft der Launcher die Sicherung:
  1. Passt die SHA-256 von `manifest.json` zu `stand.json`? Sonst gilt die `.alt`-Fassung, sonst gibt es keine Sicherung.
  2. Liegen alle Dateien aus der Liste unverändert da (Größe und Zeit, bei Abweichung SHA-256)?

  Stimmt beides, startet er mit dem Hinweis „Server-Liste nicht erreichbar, du spielst mit dem geprüften Stand vom …“. Fehlt etwas, startet er nicht, weil er ohne Liste nichts nachladen kann, und sagt das.
- **Warum im GTA-Verzeichnis:** Die Sicherung gehört zu genau diesen Dateien. Wer zwei GTA-Ordner hat, hat zwei unabhängige Stände.
- **Einstellungen** (bestätigtes GTA-Verzeichnis, Name, Wahl der optionalen Komponenten) liegen dagegen unter `%LOCALAPPDATA%\German Samplife\Launcher\einstellungen.json`, weil sie zum Windows-Benutzer gehören.

Die Sicherung ist nicht signiert. Wer Schreibrechte im GTA-Verzeichnis hat, könnte ohnehin die Dateien selbst ändern.

### 9. Spielstart

- **SA-MP-Version:**
  - Der Launcher liest die Dateiversion der `samp.dll` im GTA-Verzeichnis: `0.3.7.0` ist 0.3.7-R1, so erkennt es auch samp-cef.
  - Andere Fassungen oder eine fehlende `samp.dll` halten den Start mit Hinweis an.
  - Prüfsummen in `samp_versionen` bleiben möglich und haben Vorrang.
- **Name:** Er wird unter `HKEY_CURRENT_USER\Software\SAMP\PlayerName` gespeichert, dort liest SA-MP ihn. Der Launcher prüft vorher die Regeln von SA-MP: 3 bis 20 Zeichen, nur `A–Z a–z 0–9 [ ] ( ) $ @ . _ =`.
- **Start:** `samp.exe german-samplife.de:7777` aus dem GTA-Verzeichnis, Arbeitsordner ebenfalls das GTA-Verzeichnis. Adresse und Port kommen aus dem Manifest.
- **Fehlt `samp.exe`** (zum Beispiel beim open.mp-Launcher, der SA-MP woanders ablegt), startet der Launcher nicht. Er sagt, dass SA-MP 0.3.7-R1 im GTA-Verzeichnis installiert sein muss.
- **Nach dem Start** schließt sich der Launcher.

### 10. Veröffentlichung und Website

- **Datei:** Der Tag `v2.0.0` erzeugt `Samplife.Launcher.exe`. Dieselbe Datei hängt zusätzlich als `Samplife.Essentials.Installer.exe` am Release. So zeigen alte Links auf `releases/latest/download/Samplife.Essentials.Installer.exe` weiter auf eine funktionierende Fassung.
- **Website:**
  - Download-Link auf den Launcher umstellen.
  - Update-Eintrag schreiben.
  - Den Hinweis beim Login und beim CEF-Kick auf den Launcher umstellen.

  Das geht erst live, wenn das Release existiert.
- **Gamemode und Manifest** kommen vorher, aber ohne Wirkung: `LAUNCHER_PFLICHT` steht auf `false`, das Manifest bekommt `stand` und `server`, aber noch keinen Block `launcher`.

### 11. Tests

`test/pruefen.py` wird erweitert und läuft wie bisher ohne Fenster (`--still`) auf `windows-latest`. Es prüft:

- Erstinstallation
- zweiter Start ohne Downloads
- veraltetes Paket
- gelöschte und veränderte Datei
- neue optionale Komponente (mit und ohne Haken)
- Start ohne Manifest-Server: mit gültiger, beschädigter und fehlender Sicherung
- Selbstaktualisierung: falsche Prüfsumme bricht ab, richtige ersetzt die Datei
- Anmeldung gegen einen lokalen Testserver
- Spielstart: Ein kleines Testprogramm, das das Script als `samp.exe` baut, schreibt seine Parameter in eine Datei.

Den echten Spielstart mit GTA prüfen Spieler auf Windows mit dem Testserver, nicht mit dem Live-Server.

### 12. Fassung 2.1: DirectX, fremde ASI Loader, Deinstallieren

Stand: 23.10.2026.

- **DirectX-Laufzeit.** `samp.dll` braucht `d3dx9_25.dll`, `sampvoice.asi` braucht `d3dx9_43.dll`. Beide fehlen auf einem frischen Windows 10 und 11, kein Paket bringt sie mit, und sie dürfen nur über den Installer von Microsoft verteilt werden. Der Launcher prüft sie im 32-Bit-Systemordner und im GTA-Verzeichnis und richtet sonst den Redist vom Juni 2010 ein (`systemvoraussetzungen` im Manifest, Adresse bei download.microsoft.com, SHA-256, signiert von Microsoft). Er entpackt sich ohne Administratorrechte; nur `DXSETUP.exe /silent` braucht die Abfrage von Windows. Das passiert vor allen anderen Downloads, damit eine abgelehnte Abfrage nicht erst nach 150 MB auffällt. Ohne die Laufzeit startet der Launcher das Spiel nicht, weil SA-MP dann ohnehin nicht lädt.
- **Fremde ASI Loader.** Erkannt an der Zeichenkette `*.asi` in DLLs mit den üblichen Namen (`dinput8.dll`, `winmm.dll`, `d3d9.dll` …). Geprüft gegen die DLLs von Windows (kein Treffer), den Ultimate ASI Loader und Silent's ASI Loader (Treffer). Der Launcher meldet sie nur und schaltet sie auf Wunsch durch Umbenennen aus; der Start wird nicht verhindert, weil zwei Loader nicht in jedem Fall stören und der Spieler entscheiden soll. Bekannte Problemfälle (z. B. die alte `samplife-kleidung.asi`) stehen unter `konflikte` im Manifest und werden genauso behandelt.
- **Deinstallieren.** Damit ersetzte Dateien in der Fassung des Spielers zurückkommen und nicht in einer alten eigenen, schreibt der Launcher ab 2.1 `installiert.json` mit der ersten Berührung jeder Datei. Für ältere Installationen wird sie aus Protokoll, Stand und ältester Sicherung rekonstruiert (README, „Deinstallieren“). Die DirectX-Laufzeit bleibt, weil sie zu Windows gehört.
- **Nicht gelöst: falsche GTA-Version.** SA-MP läuft nur mit `gta_sa.exe` 1.0, nicht mit der Steam- oder Rockstar-Launcher-Fassung. Der Launcher kann das Spiel nicht mitliefern und prüft die Version noch nicht; der open.mp-Launcher tut es auch nicht.

## Offen

1. **SA-MP-Client mitliefern:** entschieden (22.10.). Ohne `samp.exe` startet der Launcher wie der open.mp-Launcher: `gta_sa.exe` starten und die `samp.dll` laden, aus seinem eigenen Ordner oder dem des open.mp-Launchers. Fehlt beides, richtet er `samp_client` ein (Release `samp-client-0.3.7-R1`, Dateien unverändert aus dem Archiv des open.mp-Launchers). Eine vorhandene SA-MP-Installation ersetzt er nie. Siehe README, „Wie SA-MP startet“.
2. **Eigene Modelle unter R1.** Neue Objekte gehen unter 0.3.7-R1 nur, indem vorhandene Modelle ersetzt werden, zum Beispiel über einen Ordner für modloader. Welche Technik, welche IDs, und wie das mit Mods der Spieler zusammengeht, braucht ein eigenes Konzept. Das Paketformat (`ziel`, `zip_unterordner`) reicht dafür schon.
3. **Wann `LAUNCHER_PFLICHT` eingeschaltet wird** (vor oder nach dem Start am 23.10.).
4. **Zwischenspeicher für Pakete.** Ohne ihn lädt eine einzige beschädigte CEF-Datei das ganze Paket (rund 150 MB) neu. Ein Zwischenspeicher unter `%LOCALAPPDATA%` würde das vermeiden, kostet aber denselben Platz noch einmal.
5. **Signatur der `.exe`.** Ohne Code-Signatur bleibt die Warnung von SmartScreen beim ersten Download.
6. **Geltungsdauer der Anmeldung** (12 Stunden) und ob Admins einen Schalter im Spiel brauchen, um die Pflicht bei Problemen sofort abzuschalten.
