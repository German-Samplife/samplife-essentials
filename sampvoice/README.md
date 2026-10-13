# SampVoice ohne Sprecheranzeige

German Samplife nutzt für den Sprach-Chat [SampVoice 3.1](https://github.com/CyberMor/sampvoice/tree/v3.1) von CyberMor (MIT-Lizenz, siehe [`LIZENZEN/sampvoice-MIT.txt`](../LIZENZEN/sampvoice-MIT.txt)).

Der Original-Client zeigt immer an, wer gerade spricht:
- eine Liste mit Namen am linken Bildschirmrand,
- ein Lautsprecher-Symbol über dem Kopf des Sprechers.

Auf dem Server hört man Sprache nur in der Nähe, im Funk oder am Telefon. Die Anzeige verrät mehr, als die Figur im Spiel wissen kann, und stört deshalb. Der Server kann sie nicht abschalten, denn der Client schaltet sie beim Spielstart fest ein.

## Die Änderung

Eine einzige Stelle in [`ohne-sprecheranzeige.patch`](ohne-sprecheranzeige.patch):

- In `client/Plugin.cpp` (`Plugin::OnInitGame`) entfällt der Aufruf `SpeakerList::Show()`.
- Ohne ihn zeichnet `SpeakerList::Render` nichts. Liste und Kopf-Symbol fallen damit beide weg.

Alles andere bleibt wie im Original:
- Mikrofon-Symbol unten rechts
- Menü mit den Einstellungen
- Sprechtasten
- Lautstärke

## Bauen

Das übernimmt GitHub mit [`.github/workflows/sampvoice.yml`](../.github/workflows/sampvoice.yml):

1. Den Quelltext von SampVoice holen, Commit `60a511af30903c4f8a456f5b0ff1e45bf7f498f5` (Tag `v3.1`), und den Patch anwenden.
2. Mit Visual Studio 2022 für SA-MP 0.3.7 R1 und englisches Menü bauen (`SAMP_R1`, `SAMP_EN`). Dazu kommen:
   - [`Directory.Build.targets`](Directory.Build.targets) stellt das alte Projekt (v141_xp, Windows SDK 7.1A, DirectX SDK) auf aktuelle Werkzeuge um.
   - D3DX 9 kommt aus dem NuGet-Paket `Microsoft.DXSDK.D3DX`.
   - Die C++-Laufzeit ist fest eingebaut (`/MT`). Die `.asi` braucht deshalb keine VC++-Laufzeit beim Spieler.
3. Prüfen, dass die `.asi` 32 Bit ist und `samp.dll` einbindet.
4. Das Original-Paket `sv_client_037_r1_english.zip` laden und seine Prüfsumme kontrollieren.
   - Darin wird nur `sampvoice.asi` ersetzt und `sampvoice-lizenz.txt` ergänzt.
   - ASI Loader (`vorbisFile.dll`, `vorbisHooked.dll`) und BASS (`bass.dll`, `bass_fx.dll`) bleiben unverändert.

Ein Push auf einen Branch, der `sampvoice/` ändert, baut und prüft nur. Das Paket hängt dann als Artefakt am Lauf.

Ein Tag wie `sampvoice-3.1-sl1` legt ein Release mit `sampvoice-037-r1-samplife.zip` und `SHA256SUMS.txt` an. Prüfsumme und Größe für das Manifest stehen im Release-Text.
