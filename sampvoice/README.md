# SampVoice ohne Sprecheranzeige

German Samplife nutzt für den Sprach-Chat [SampVoice 3.1](https://github.com/CyberMor/sampvoice/tree/v3.1) von CyberMor (MIT-Lizenz, siehe [`LIZENZEN/sampvoice-MIT.txt`](../LIZENZEN/sampvoice-MIT.txt)).

Der Original-Client zeigt immer an, wer gerade spricht:
- eine Liste mit Namen am linken Bildschirmrand,
- ein Lautsprecher-Symbol über dem Kopf des Sprechers,
- ein eigenes Mikrofon-Symbol unten rechts, das beim Sprechen aufleuchtet.

Auf dem Server hört man Sprache nur in der Nähe, im Funk oder am Telefon. Die Anzeige verrät mehr, als die Figur im Spiel wissen kann, und stört deshalb. Ob man selbst gerade spricht, flüstert, schreit, funkt oder telefoniert, zeigt das HUD des Servers; das Mikrofon-Symbol von SampVoice wäre doppelt. Der Server kann beides nicht abschalten, denn der Client schaltet es beim Spielstart fest ein.

## Die Änderung

Eine einzige Stelle in [`ohne-sprecheranzeige.patch`](ohne-sprecheranzeige.patch):

- In `client/Plugin.cpp` (`Plugin::OnInitGame`) entfallen die Aufrufe `SpeakerList::Show()` und `MicroIcon::Show()`.
- Ohne sie zeichnen `SpeakerList::Render` und `MicroIcon::Render` nichts. Liste, Kopf-Symbol und Mikrofon-Symbol fallen damit weg. Nichts sonst im Client ruft `Show()` auf.

Dazu kommt [`cef-reihenfolge.patch`](cef-reihenfolge.patch) (ab `sampvoice-3.1-sl3`):

- SampVoice und samp-cef hängen sich beide an GTAs Aufruf von `Direct3DCreate9` (`0x807C2B`). SampVoice sofort beim Laden, `cef.asi` später aus einem eigenen Faden. Wer zuletzt kommt, liegt außen.
- Liegt samp-cef außen (bei Windows 10 die Regel), merkt sich GTA die Hülle von samp-cef unter `0xC97C20`. `IDirect3D9Hook::CreateDevice` in `client/include/util/Render.cpp` übernahm das Gerät nur, wenn `this` genau dieser Zeiger ist, und stieg sonst still aus.
- Ohne Gerät läuft `Plugin::OnRender` nie: Die Uhr (`Timer::Tick`) bleibt auf 0, der Sprachfaden schickt nie ein Keep-Alive an Port 7780 (der Server meldet nie `identified`), B und F11 tun nichts.
- Die Prüfung auf `this` entfällt. GTAs Gerät erkennt weiter der Zeiger `&pGameDevice` (`0xC97C28`), den samp-cef unverändert durchreicht. Kette dann GTA → samp-cef → SampVoice → Gerät, in beiden Reihenfolgen.
- Ab `sampvoice-3.1-sl4`: Fenstermodus-ASIs (z. B. `iii.vc.sa.windowedmode.asi`) rufen `Direct3DCreate9` selbst über `0x807C2B` auf, dann hängt SampVoice zweimal in der Kette. Ohne die Prüfung auf `this` übernahmen in `sl3` beide Hüllen das Gerät; die zweite gab die Grafik der ersten frei, und das Spiel stürzte in `sampvoice.asi` ab (Sprung auf 0). Ein Zähler über verschachtelte `CreateDevice`-Aufrufe lässt nur die äußerste SampVoice-Hülle übernehmen.

Und [`neuer-compiler.patch`](neuer-compiler.patch). Er ändert nichts am Verhalten. Er setzt nur an einer Stelle in `client/Network.cpp` eine ausdrückliche Typumwandlung, die der heutige Compiler verlangt (Fehler C2398).

Ab `sampvoice-3.1-sl5` dazu [`deutsch.patch`](deutsch.patch) und [`logo.png`](logo.png):

- Das Menü (F11) ist deutsch. `client/PluginMenu.h` bekommt eine dritte Sprache `SAMP_DE` neben `SAMP_RU` und `SAMP_EN`; [`Directory.Build.targets`](Directory.Build.targets) baut mit `SAMP_DE`. Die Umlaute stehen als UTF-8-Bytes (`\xC3\xA4`) im Quelltext, so hängt das Ergebnis nicht von der Codepage beim Bauen ab. Die Schrift des Menüs (`resources/font.ttf`, Zeichensatz Kyrillisch samt Latin-1) hat alle Umlaute.
- Mit `SAMP_DE` fehlen im Menü die Einstellungen für das Symbol über den Spielern und das Mikrofon-Symbol (Größe, Position, Verschieben). Beides zeigt dieser Client nicht (siehe oben), die Regler täten nichts.
- Oben rechts im Menü steht unser Logo statt „SAMP VOICE Version 3.0“: [`logo.png`](logo.png) ersetzt beim Bauen `client/resources/logo.png`, 512 x 128 Pixel wie das Original (Sechseck aus `docs/logo` in samplife2026, daneben „German Samplife“ und „Sprach-Chat“ in Inter).

Ab `sampvoice-3.1-sl6` dazu [`einstellungen-export.patch`](einstellungen-export.patch): Die Einstellungen des Menüs lassen sich auch im Control Panel des Servers (`/icp`, Reiter Audio) bedienen.

- Die neue Datei `client/Export.cpp` exportiert C-Funktionen (`__cdecl`). Das Plugin `samplife-kleidung.dll` sucht sie zur Laufzeit mit `GetProcAddress` (`kleidung/src/stimme.rs`); mit einer älteren `sampvoice.asi` fehlen sie, und das Control Panel verweist weiter auf F11.
- Alles geht über dieselben Funktionen wie im Menü (`Playback`, `Record`, `BlackList`) und landet in `PluginConfig`. Nach jeder Änderung speichert `Export.cpp` gleich `svconfig.bin`, nicht erst beim Beenden. Ist das Menü dabei offen, zieht es die Werte nach (`PluginMenu::SyncOptions`, dafür jetzt öffentlich).
- Dazu nur zwei kleine Abfragen: `Playback::IsLoaded()` (BASS läuft) und `Record::IsInit()` (mit dem Sprach-Server verbunden, Geräteliste da).
- Nur aus dem Faden des Spiels aufrufen, dort laufen auch Menü und Sprach-Client.

| Export | Rückgabe |
|---|---|
| `int SamplifeStimmeVersion()` | Stand der Schnittstelle, jetzt `1` |
| `int SamplifeStimmeLesen(int feld)` | Wert des Feldes, `-1` bei unbekanntem Feld |
| `int SamplifeStimmeSetzen(int feld, int wert)` | `1` übernommen, `0` geht gerade nicht |
| `int SamplifeStimmeGeraet(int index, char* puffer, int groesse)` | Name des Eingabegeräts (UTF-8), Länge oder `-1` |
| `int SamplifeStimmeStummEintrag(int index, char* puffer, int groesse)` | Name in `puffer`, Spieler-ID (`65535` nicht online) oder `-1` |
| `int SamplifeStimmeStumm(int spieler)` | Spieler (online) stummschalten, `1` steht in der Liste |
| `int SamplifeStimmeFrei(const char* name)` | Stummschaltung aufheben, `1` stand in der Liste |

| Feld | Bedeutung | Setzen |
|---|---|---|
| 0 | Ton an | 0/1 |
| 1 | Lautstärke | 0 bis 100 |
| 2 | Lautstärke angleichen | 0/1 |
| 3 | Filter für hohe Töne (im Menü „Hochpassfilter“, BASS-Tiefpass bei 3400 Hz) | 0/1 |
| 4 | Mikrofon an | 0/1 |
| 5 | Mikrofon-Lautstärke | 0 bis 100 |
| 6 | Eingabegerät (Index, `-1` keins) | Index |
| 7 | Mikrofontest (man hört sich selbst), wird nicht gespeichert | 0/1 |
| 8 | Wiedergabe bereit (BASS läuft) | nur lesen |
| 9 | Aufnahme bereit (mit dem Sprach-Server verbunden) | nur lesen |
| 10 | Anzahl der Eingabegeräte | nur lesen |
| 11 | Anzahl der stummgeschalteten Spieler | nur lesen |

Alles andere bleibt wie im Original:
- Einstellungen im Menü (Ton, Lautstärke, Mikrofon, Gerät, Mikrofontest, Stummschalten einzelner Spieler)
- Sprechtasten
- Lautstärke

## Bauen

Das übernimmt GitHub mit [`.github/workflows/sampvoice.yml`](../.github/workflows/sampvoice.yml):

1. Den Quelltext von SampVoice holen, Commit `60a511af30903c4f8a456f5b0ff1e45bf7f498f5` (Tag `v3.1`), und den Patch anwenden.
2. Die Patches anwenden, `logo.png` übernehmen und mit Visual Studio 2022 für SA-MP 0.3.7 R1 mit deutschem Menü bauen (`SAMP_R1`, `SAMP_DE`). Dazu kommen:
   - [`Directory.Build.targets`](Directory.Build.targets) stellt das alte Projekt (v141_xp, Windows SDK 7.1A, DirectX SDK) auf aktuelle Werkzeuge um.
   - D3DX 9 kommt aus dem NuGet-Paket `Microsoft.DXSDK.D3DX`.
   - Die C++-Laufzeit ist fest eingebaut (`/MT`). Die `.asi` braucht deshalb keine VC++-Laufzeit beim Spieler.
3. Prüfen, dass die `.asi` 32 Bit ist, BASS einbindet und keine VC++-Laufzeit braucht.
4. Das Original-Paket `sv_client_037_r1_english.zip` laden und seine Prüfsumme kontrollieren.
   - Darin wird nur `sampvoice.asi` ersetzt und `sampvoice-lizenz.txt` ergänzt.
   - ASI Loader (`vorbisFile.dll`, `vorbisHooked.dll`) und BASS (`bass.dll`, `bass_fx.dll`) bleiben unverändert.

Ein Push auf einen Branch, der `sampvoice/` ändert, baut und prüft nur. Das Paket hängt dann als Artefakt am Lauf.

Ein Tag wie `sampvoice-3.1-sl2` legt ein Release mit `sampvoice-037-r1-samplife.zip` und `SHA256SUMS.txt` an. Prüfsumme und Größe für das Manifest stehen im Release-Text.
