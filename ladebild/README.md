# Ladebild

Ein eigenes Ladebild beim Start von GTA San Andreas mit SA-MP, statt des Bildes von SA-MP. Der open.mp-Launcher zeigt sein Bild über den open.mp-Client (`omp-client.dll`); der Samplife Launcher lädt nur die `samp.dll`, deshalb kam bisher das Standardbild von SA-MP.

## So funktioniert es

- **Dateien:** `samplife-ladebild.asi` und `samplife-ladebild.jpg`, beide im GTA-Verzeichnis neben `gta_sa.exe`. Der ASI Loader von SampVoice lädt das ASI beim Start.
- **Zeichnen:** Das ASI hängt sich wie `samplife-kleidung` in `IDirect3DDevice9::Present` des Geräts ein, mit dem GTA rendert (`RwD3D9Device`, 0xC97C28). Den vorigen Eintrag ruft es weiter auf. Vor jedem Present legt es das Bild bildschirmfüllend über alles. Abweichende Seitenverhältnisse wie 4:3 schneidet es an den Rändern ab, die Mitte bleibt.
- **Ende:** Sobald `gGameState` (0xC8D4C0) auf 9 steht, also das Spiel läuft, blendet es in einer halben Sekunde aus. Danach gibt es die Textur frei und reicht Present nur noch durch.
- **Unverändert bleiben:** `samp.dll`, `samp.saa` und die Ladebilder von GTA (`loadscs.txd`).
- **Ohne Wirkung startet das Spiel ganz normal**, wenn eines davon zutrifft:
  - `gta_sa.exe` ist nicht 1.0 US
  - das Bild fehlt oder ist kein JPEG
  - die Textur lässt sich nicht anlegen
  - beim Zeichnen tritt ein Fehler auf
- **Protokoll:** `samplife-ladebild.log` neben `gta_sa.exe`, sonst in „Dokumente\GTA San Andreas User Files“ oder `%TEMP%`.

Das Bild tauschen: `samplife-ladebild.jpg` ersetzen, JPEG, höchstens 4096 × 4096, am besten 1920 × 1080. Wichtiges gehört in die mittlere 4:3-Fläche.

## Bauen und veröffentlichen

GitHub baut bei jedem Push auf einen anderen Branch als `main`, der `ladebild/` ändert (`.github/workflows/ladebild.yml`). Das Artefakt `samplife-ladebild` enthält `samplife-ladebild.zip` mit beiden Dateien. Ein Tag wie `ladebild-0.1.0` (passend zu `Cargo.toml`) legt das Release an; dessen Text nennt SHA-256 und Größe für das Manifest.
