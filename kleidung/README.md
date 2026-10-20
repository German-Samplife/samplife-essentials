# Machbarkeitstest: CJ-Kleidung im Spiel

`samplife-kleidung.asi` prüft, ob ein SA-MP-Client die Kleidung von CJ (Skin 0) so ändern kann wie im Singleplayer, und zwar **bei der eigenen Figur und bei den Figuren anderer Spieler**. Das ist nur ein Test für das Team und **noch nichts für Spieler**: Es gibt keine Verbindung zum Server, jeder Client zieht alle CJs in seiner Nähe gleich an. Die Synchronisation über den Server kommt erst, wenn der Test zeigt, dass es geht.

## Installation

1. `samplife-kleidung.asi` aus dem Artefakt des Workflows [`Kleidung`](../.github/workflows/kleidung.yml) (Lauf öffnen, unten „Artifacts“, `samplife-kleidung`) in den GTA-Ordner legen, also neben `gta_sa.exe`.
2. Spiel wie immer über SA-MP starten und verbinden.

Einen ASI Loader bringt SampVoice schon mit (`vorbisFile.dll`). Vorausgesetzt werden GTA San Andreas 1.0 US (wie für SA-MP ohnehin nötig) und SA-MP **0.3.7 R1 oder R3**. Bei anderen Versionen bleibt das ASI still; bei 0.3.DL sagt es das einmal im Chat, bei allen anderen steht es nur in der Logdatei.

Zum Entfernen die Datei wieder löschen. Das ASI ändert keine Dateien, nur den laufenden Speicher des Spiels; nach einem Neustart des Spiels ist alles wie vorher.

## Befehl

Der Befehl `/kleidungtest` existiert nur im eigenen Client und geht nicht an den Server.

| Eingabe | Wirkung |
|---|---|
| `/kleidungtest` oder `/kleidungtest 1` | Outfit 1 auf **alle** Figuren mit Skin 0 im Streamingbereich, eigene und fremde |
| `/kleidungtest 2` | Outfit 2, deutlich anders als Outfit 1 |
| `/kleidungtest aus` | Standard-CJ wiederherstellen und den Test beenden |
| `/kleidungtest info` | Version des ASI und von SA-MP, gefundene CJ-Figuren (eigene, fremde mit Spieler-ID), Zahl der Umbauten mit Dauer, wartende Figuren |

Solange ein Outfit aktiv ist, prüft das ASI jede Sekunde, ob neue CJ-Figuren gestreamt wurden oder eine Figur ihr Outfit verloren hat (Respawn, Streaming, Skinwechsel), und zieht sie dann wieder an. Umgebaut wird höchstens eine Figur pro Bild und nur, wenn es nötig ist (siehe „Wie es funktioniert“).

| Outfit | Teile (Textur/Modell aus `models/player.img`) |
|---|---|
| 1 | schwarzes Unterhemd `vestblack/vest`, Afro `afro/afro`, grüne Tarnhose `worktrcamogrn/worktr`, Cowboystiefel `cowboyboot/biker`, Goldkette `neckgold/neck2`, dunkle Brille `glasses04dark/glasses04` |
| 2 | roter Anzug `suit1red/suit1`, Glatze `bald/head`, rote Anzughose `suit1trred/suit1tr`, schwarze Schuhe `shoedressblk/shoe`, Uhr `watchgno/watch`, Cowboyhut `cowboy/cowboy` |
| Standard (`aus`) | weißes Unterhemd `vest/vest`, kurze Haare `player_face/head`, Jeans `jeansdenim/jeans`, Turnschuhe `sneakerbincblk/sneaker` (Start im Singleplayer) |

Hatte eine Figur vor dem Test schon eigene Kleidungsdaten, stellt `aus` genau diese wieder her, sonst das Standard-Outfit.

## Testablauf für zwei Tester

Beide Tester haben das ASI im GTA-Ordner und sind auf dem Server.

1. Ein Admin gibt beiden Skin 0: `/setskin [ID] 0` für jeden Tester.
2. Beide stellen sich nebeneinander. **Beide** geben `/kleidungtest` ein. Im Chat erscheint „Outfit 1 aktiv …“ und kurz danach eine Zeile wie „2 CJ-Figuren (eigene 1, fremde 1): 2 umgebaut, 0 ohne Umbau“.
3. Prüfen und notieren:
   1. **Eigene Figur umgezogen?** Schwarzes Unterhemd, Afro, Tarnhose, Cowboystiefel, Kette, Brille.
   2. **Sieht jeder den anderen umgezogen?** Das ist die eigentliche Frage. Im Test zieht jeder Client alle CJs gleich an; deshalb muss das ASI bei beiden laufen und `/kleidungtest` bei beiden aktiv sein. Gegenprobe: Tester A gibt `/kleidungtest 2` ein, Tester B bleibt bei 1. A muss jetzt sich und B im roten Anzug sehen, B beide in Outfit 1.
   3. **Bleibt das Outfit?** Nacheinander: Respawn (`/kill` oder Tod), weit weglaufen oder teleportieren, bis der andere verschwindet, und zurückkommen (Streaming), Fahrzeug ein- und aussteigen, Animationen (z. B. `/sit`, Tanzen, Hinlegen), Schießen und Waffenwechsel, Skin kurz wechseln und zurück auf 0. Während einer im Fahrzeug sitzt oder tot ist, wartet das ASI und zieht ihn danach wieder an.
   4. **Leistung:** Mit drei oder mehr CJs in der Nähe die Bildrate beobachten, vor allem beim ersten Anziehen und beim Hereinstreamen. `/kleidungtest info` zeigt die Dauer des letzten und des längsten Umbaus in Millisekunden.
   5. **Abstürze und Grafikfehler:** fehlende Körperteile, falsche Texturen, unsichtbare Figuren, Absturz beim Umziehen, beim Verlassen des Servers oder beim Beenden des Spiels.
4. `/kleidungtest info` bei beiden ausführen und die drei Zeilen abschreiben oder ein Bild machen. Wichtig ist die Angabe „Kleidungsdaten: N verschiedene“: Steht dort „teilweise GETEILT“, teilen sich mehrere Figuren dieselben Kleidungsdaten (siehe Risiken).
5. `/kleidungtest aus` bei beiden: Standard-CJ wiederhergestellt?
6. Danach `samplife-kleidung.log` aus dem GTA-Ordner beider Tester einsammeln. Dort steht jeder Umbau mit Dauer, jede Prüfung mit den gefundenen Figuren und bei Fehlern der Grund.

## Was das Ergebnis bedeutet

- **Eigene Figur geht, fremde nicht:** Die Kleidung lässt sich nur am lokalen Spieler zeigen. Für alle sichtbare Kleidung bräuchten wir dann einen anderen Weg (eigene Skins pro Outfit über open.mp-Custom-Skins).
- **Beides geht, auch nach Respawn, Streaming und Fahrzeug:** Der Weg ist tragfähig. Im Ausbau schickt der Server pro Spieler die gewählten Teile (z. B. über CEF oder ein eigenes RPC), das ASI zieht jede Figur mit ihren eigenen Teilen an statt alle gleich, und das ASI kommt über den Essentials-Installer zu allen Spielern.
- **„teilweise GETEILT“ in der Info:** Mehrere Figuren benutzen dieselben Kleidungsdaten. Gleiche Outfits gehen dann, verschiedene Outfits pro Spieler nicht ohne Weiteres; das ASI müsste jeder Figur eigene Daten geben (so macht es MTA).
- **Umbau dauert spürbar lange (Ruckler):** Das Umziehen lädt Kleidung aus `player.img` und baut das Modell neu. Für viele Spieler müsste man das verteilen oder zwischenspeichern.

## Wie es funktioniert

Alle Adressen gelten für GTA San Andreas 1.0 US; Quellen stehen auch im Quelltext.

**Einstieg.** Im `DllMain` wird nur der Aufruf von `CGame::Process` in `Idle()` bei `0x53E981` auf das ASI umgeleitet; das ASI ruft zuerst das bisherige Ziel auf (`CGame::Process` `0x53BEE0` oder den Hook eines anderen ASI) und arbeitet danach. Das ist dieselbe Stelle wie `gameProcessEvent` im plugin-sdk ([`shared/Events.h`](https://github.com/DK22Pac/plugin-sdk/blob/master/shared/Events.h)). Vorher prüft es an `0x401000`, ob es GTA 1.0 US ist ([plugin-sdk `shared/GameVersion.cpp`](https://github.com/DK22Pac/plugin-sdk/blob/master/shared/GameVersion.cpp)). Alles Weitere läuft im Spiel-Thread.

**SA-MP.** Versionserkennung, Spieler-Pool und `CInput` kommen aus [samp-client-api](https://github.com/zottce/samp-client-api) (Stand `d497ffad`, derselbe wie in samp-cef). Diese Bibliothek kann weder in den Chat schreiben noch Befehle anlegen; dafür ruft das ASI die Funktionen von samp.dll direkt auf. Adressen aus [BlastHackNet/SAMP-API](https://github.com/BlastHackNet/SAMP-API/tree/multiver/src/sampapi):

| | 0.3.7 R1 | 0.3.7 R3 |
|---|---|---|
| `CChat*` (`RefChat`) | `samp.dll+0x21A0E4` | `samp.dll+0x26E8C8` |
| `CChat::AddMessage(color, text)` | `samp.dll+0x645A0` | `samp.dll+0x679F0` |
| `CInput*` (`RefInputBox`, auch `CINPUT` in samp-client-api) | `samp.dll+0x21A0E8` | `samp.dll+0x26E8CC` |
| `CInput::AddCommand(name, proc)` | `samp.dll+0x65AD0` | `samp.dll+0x69000` |

**Figuren finden.** Die eigene Figur ist `CLocalPlayer::m_pPed->m_pGamePed` aus dem Spieler-Pool von SA-MP (Rückfall: `CWorld::Players[0].m_pPed`, `0xB7CD98`). Fremde Figuren sind `CPlayerInfo::m_pPlayer->m_pPed->m_pGamePed` für jeden Eintrag im Spieler-Pool (samp-client-api `players::players()` und `gta_ped()`); ist der Spieler nicht gestreamt, gibt es keine Figur. Genommen wird jede Figur mit Modell 0 (`CEntity::m_nModelIndex`, `+0x22`).

**Anziehen.** Jede Spielerfigur hat über `CPed::m_pPlayerData` (`+0x480`) einen `CPlayerPedData`, darin `m_pPedClothesDesc` (`+0x4`), die Kleidungsbeschreibung (`CPedClothesDesc`, `0x78` Bytes: 10 Modell-Schlüssel, 18 Textur-Schlüssel, Fett, Muskeln). Das ASI schreibt die Schlüssel genau wie `CPedClothesDesc::SetTextureAndModel` (`0x5A8080`; die Funktion selbst wird nicht aufgerufen, weil sie laut gta-reversed einen Parameter im Register erwartet), berechnet mit `CKeyGen::GetUppercaseKey` (`0x53CF30`), und ruft dann `CClothes::RebuildPlayer(ped, true)` (`0x5A82C0`) auf. `true` heißt: Fett und Muskeln bleiben, wie sie sind, statt die Werte des eigenen Spielers zu übernehmen. Quellen: [gta-reversed](https://github.com/gta-reversed/gta-reversed) `Clothes.cpp`, `PedClothesDesc.cpp/.h`, `PlayerPedData.h`, `Core/KeyGen.cpp`, `Enums/eClothes*Part.h`; Offsets aus den `VALIDATE_OFFSET`-Zeilen in plugin-sdk `CPed.h`, `CEntity.h`, `CPlayerInfo.h`. Die Teilenamen stammen aus der Kleidungsliste von MTA ([`CClientPlayerClothes.cpp`](https://github.com/multitheftauto/mtasa-blue/blob/master/Client/mods/deathmatch/logic/CClientPlayerClothes.cpp)).

**Nur umbauen, wenn nötig.** `RebuildPlayer` baut nicht nur die Figur um, sondern auch das gemeinsame Modell 0, aus dem jede neu erscheinende CJ-Figur kopiert wird. Das ASI merkt sich deshalb, welches Outfit das Modell gerade zeigt und welche Figuren schon vor dem letzten Umbau da waren. Eine CJ-Figur, die danach hereinstreamt oder respawnt, trägt das Outfit schon und wird nur übernommen („ohne Umbau“). Umgebaut werden Figuren, deren Kleidungsdaten nicht passen oder deren Modell älter ist.

**Wann nicht.** `RebuildPlayer` setzt die Figur über `CPed::Dress` auf „steht“ und fügt sie neu in die Welt ein. Figuren im Fahrzeug (`bInVehicle`, Flags `+0x46C` Bit 8) und tote Figuren (`m_ePedState` `+0x530`, 54 bis 56) werden deshalb nicht umgebaut, sondern danach.

## Bekannte Risiken

- **Gemeinsame Kleidungsdaten.** In GTA zeigt `m_pPlayerData` jeder Spielerfigur auf `CWorld::Players[n]`, und GTA kennt nur zwei solche Plätze. Wie SA-MP das für viele Spieler löst, ist nicht belegt; `/kleidungtest info` zeigt, ob sich Figuren die Daten teilen. Für den Test mit gleichem Outfit auf allen ist das egal.
- **Modell 0 ist für alle gleich.** Nach dem Test tragen neu erscheinende CJs das zuletzt gebaute Outfit, bis jemand wieder umgebaut wird. `aus` baut deshalb alle CJs auf das Standard-Outfit um. Ob das exakt so aussieht wie der CJ von SA-MP vor dem Test, ist Teil des Tests; ganz sicher ist der Ursprungszustand erst nach Neustart des Spiels. Gibt man `aus`, während keine CJ in der Nähe ist, bleibt das Modell im letzten Outfit.
- **Ruckler.** Jeder Umbau lädt Kleidung synchron nach. Das ASI baut höchstens eine Figur pro Bild um.
- **Speicherzugriffe.** Alle Zeiger werden vor dem Lesen geprüft (`VirtualQuery`). Ein Absturz innerhalb von `RebuildPlayer` selbst lässt sich so nicht abfangen; deshalb bitte Abstürze mit Log melden.
- **Andere Mods.** Mods, die CJ ebenfalls umziehen oder `player.img` ersetzen, können das Ergebnis verfälschen. Am besten mit möglichst sauberem GTA testen.

## Logdatei

`samplife-kleidung.log` im GTA-Ordner wird bei jedem Spielstart neu angelegt. Sie enthält Version und Hook-Stand, die erkannte SA-MP-Version, jede Prüfung, deren Ergebnis sich geändert hat, jeden Umbau mit Dauer und bei `/kleidungtest info` die Adressen der gefundenen Figuren und ihrer Kleidungsdaten.

## Bauen

Nur über GitHub: [`.github/workflows/kleidung.yml`](../.github/workflows/kleidung.yml) baut bei jedem Push auf einen Branch außer `main`, der `kleidung/` oder den Workflow ändert, und von Hand über „Run workflow“. Ergebnis ist das Artefakt `samplife-kleidung` mit `samplife-kleidung.asi`, `SHA256SUMS.txt` und der `Cargo.lock` des Laufs. Es gibt bewusst kein Release.

Lokal ginge es mit Rust stable und dem Ziel `i686-pc-windows-msvc` (in `kleidung/`: `cargo build --release --target i686-pc-windows-msvc`); die C-Laufzeit wird statisch gelinkt (`.cargo/config.toml`), damit das ASI ohne VC++-Redistributable läuft.

| Datei | Inhalt |
|---|---|
| `src/lib.rs` | `DllMain`, Hook einrichten |
| `src/game.rs` | GTA-Adressen, Offsets, Speicherprüfung, Zugriff auf Figuren |
| `src/samp.rs` | SA-MP-Version, Chat, Befehl, eigene und fremde Figuren |
| `src/outfit.rs` | die Outfits |
| `src/logic.rs` | Befehl, Prüfung jede Sekunde, Umbauten, Info |
| `src/log.rs` | Logdatei |

## Lizenz

Der Quelltext steht wie der Rest dieses Repositorys ohne festgelegte Lizenz. Eingebunden wird [samp-client-api](https://github.com/zottce/samp-client-api) von ZOTTCE; das Repository enthält keine Lizenzangabe (siehe auch [`cef/README.md`](../cef/README.md)). Adressen und Strukturen stammen aus den oben verlinkten öffentlichen Projekten; Code daraus ist nicht übernommen.
