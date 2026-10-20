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
| `/kleidungtest mix` | eigene Figur Outfit 1, jede fremde CJ-Figur Outfit 2; jede Figur wird einzeln gebaut |
| `/kleidungtest mix2` | umgekehrt: eigene Figur Outfit 2, fremde Outfit 1 |
| `/kleidungtest aus` | Standard-CJ wiederherstellen und den Test beenden |
| `/kleidungtest info` | Version des ASI und von SA-MP, gefundene CJ-Figuren (eigene, fremde mit Spieler-ID), Zahl der Umbauten mit Dauer, wartende Figuren, ob der Kleidungs-Code gepatcht ist und welche Umbau-Variante wirkt |

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
4. **Verschiedene Outfits auf einem Client (`mix`, ab 0.4.0):** Beide geben `/kleidungtest mix` ein. Erwartet: Jeder sieht sich selbst in Outfit 1 (schwarzes Unterhemd, Afro, Tarnhose) und den anderen in Outfit 2 (roter Anzug, Cowboyhut). Gegenprobe mit `/kleidungtest mix2` bei einem der beiden: Bei ihm tauschen sich die Outfits, beim anderen bleibt alles. Danach Respawn, Weg- und Wiederkommen und Fahrzeug wie oben; jede Figur muss ihr eigenes Outfit behalten. `/kleidungtest info` zeigt in der fünften Zeile, ob Figuren mit verschiedenem Outfit Geometrie oder Texturen teilen (muss „nein“ sein).
5. `/kleidungtest info` bei beiden ausführen und die drei Zeilen abschreiben oder ein Bild machen. Wichtig ist die Angabe „Kleidungsdaten: N verschiedene“: Steht dort „teilweise GETEILT“, teilen sich mehrere Figuren dieselben Kleidungsdaten (siehe Risiken).
6. `/kleidungtest aus` bei beiden: Standard-CJ wiederhergestellt?
7. Danach `samplife-kleidung.log` aus dem GTA-Ordner beider Tester einsammeln. Dort steht jeder Umbau mit Dauer, jede Prüfung mit den gefundenen Figuren und bei Fehlern der Grund.

## Was das Ergebnis bedeutet

- **Eigene Figur geht, fremde nicht:** Die Kleidung lässt sich nur am lokalen Spieler zeigen. Für alle sichtbare Kleidung bräuchten wir dann einen anderen Weg (eigene Skins pro Outfit über open.mp-Custom-Skins).
- **Beides geht, auch nach Respawn, Streaming und Fahrzeug:** Der Weg ist tragfähig. Im Ausbau schickt der Server pro Spieler die gewählten Teile (z. B. über CEF oder ein eigenes RPC), das ASI zieht jede Figur mit ihren eigenen Teilen an statt alle gleich, und das ASI kommt über den Essentials-Installer zu allen Spielern.
- **„teilweise GETEILT“ in der Info:** Mehrere Figuren benutzen dieselben Kleidungsdaten. Gleiche Outfits gehen dann, verschiedene Outfits pro Spieler nicht ohne Weiteres; das ASI müsste jeder Figur eigene Daten geben (so macht es MTA).
- **Umbau dauert spürbar lange (Ruckler):** Das Umziehen lädt Kleidung aus `player.img` und baut das Modell neu. Für viele Spieler müsste man das verteilen oder zwischenspeichern.

## Wie es funktioniert

Alle Adressen gelten für GTA San Andreas 1.0 US; Quellen stehen auch im Quelltext.

**Einstieg in den Spiel-Thread.** Es gibt zwei Wege; die Test-Logik läuft pro Bild genau einmal, über A, wenn A läuft, sonst über B.

- **A:** Im `DllMain` wird der Aufruf von `CGame::Process` in `Idle()` bei `0x53E981` auf das ASI umgeleitet; das ASI ruft zuerst das bisherige Ziel auf (`CGame::Process` `0x53BEE0` oder den Hook eines anderen ASI) und arbeitet danach. Das ist dieselbe Stelle wie `gameProcessEvent` im plugin-sdk ([`shared/Events.h`](https://github.com/DK22Pac/plugin-sdk/blob/master/shared/Events.h)). Überschreibt später jemand die Stelle, ohne das ASI weiter aufzurufen, hakt es sich nach 120 Bildern ohne Aufruf erneut ein und ruft dann das neue Ziel auf (höchstens fünfmal).
- **B:** `IDirect3DDevice9::Present` des Geräts, mit dem GTA zeichnet (RenderWare-Variable `RwD3D9Device` bei `0xC97C28`, wie in samp-client-api `gta/d3d9.rs`). Das ASI ersetzt den Present-Eintrag in der Funktionstabelle dieses Geräts und ruft den vorherigen Eintrag weiter auf. So bleiben der Geräte-Proxy von SA-MP, der Present-Hook des CEF-Clients und andere Hooks in der Kette. Present ruft GTA am Ende jedes Bilds im Spiel-Thread auf. Solange das Pausenmenü offen ist, läuft die Logik über B nicht.

Vor allem anderen prüft das ASI an `0x401000`, ob es GTA 1.0 US ist ([plugin-sdk `shared/GameVersion.cpp`](https://github.com/DK22Pac/plugin-sdk/blob/master/shared/GameVersion.cpp)); sonst bleibt es aus.

**SA-MP.** Versionserkennung, Spieler-Pool und `CInput` kommen aus [samp-client-api](https://github.com/zottce/samp-client-api) (Stand `d497ffad`, derselbe wie in samp-cef). Diese Bibliothek kann weder in den Chat schreiben noch Befehle anlegen; dafür ruft das ASI die Funktionen von samp.dll direkt auf. Adressen aus [BlastHackNet/SAMP-API](https://github.com/BlastHackNet/SAMP-API/tree/multiver/src/sampapi):

| | 0.3.7 R1 | 0.3.7 R3 |
|---|---|---|
| `CChat*` (`RefChat`) | `samp.dll+0x21A0E4` | `samp.dll+0x26E8C8` |
| `CChat::AddMessage(color, text)` | `samp.dll+0x645A0` | `samp.dll+0x679F0` |
| `CInput*` (`RefInputBox`, auch `CINPUT` in samp-client-api) | `samp.dll+0x21A0E8` | `samp.dll+0x26E8CC` |
| `CInput::AddCommand(name, proc)` | `samp.dll+0x65AD0` | `samp.dll+0x69000` |

**Figuren finden.** Die eigene Figur ist `CLocalPlayer::m_pPed->m_pGamePed` aus dem Spieler-Pool von SA-MP (Rückfall: `CWorld::Players[0].m_pPed`, `0xB7CD98`). Fremde Figuren sind `CPlayerInfo::m_pPlayer->m_pPed->m_pGamePed` für jeden Eintrag im Spieler-Pool (samp-client-api `players::players()` und `gta_ped()`); ist der Spieler nicht gestreamt, gibt es keine Figur. Genommen wird jede Figur mit Modell 0 (`CEntity::m_nModelIndex`, `+0x22`).

**Anziehen.** Jede Spielerfigur hat über `CPed::m_pPlayerData` (`+0x480`) einen `CPlayerPedData`, darin `m_pPedClothesDesc` (`+0x4`), die Kleidungsbeschreibung (`CPedClothesDesc`, `0x78` Bytes: 10 Modell-Schlüssel, 18 Textur-Schlüssel, Fett, Muskeln). Das ASI schreibt die Schlüssel genau wie `CPedClothesDesc::SetTextureAndModel` (`0x5A8080`; die Funktion selbst wird nicht aufgerufen, weil sie laut gta-reversed einen Parameter im Register erwartet), berechnet mit `CKeyGen::GetUppercaseKey` (`0x53CF30`), und ruft dann `CClothes::RebuildPlayer(ped, true)` (`0x5A82C0`) auf. `true` heißt: Fett und Muskeln bleiben, wie sie sind, statt die Werte des eigenen Spielers zu übernehmen. Quellen: [gta-reversed](https://github.com/gta-reversed/gta-reversed) `Clothes.cpp`, `PedClothesDesc.cpp/.h`, `PlayerPedData.h`, `Core/KeyGen.cpp`, `Enums/eClothes*Part.h`; Offsets aus den `VALIDATE_OFFSET`-Zeilen in plugin-sdk `CPed.h`, `CEntity.h`, `CPlayerInfo.h`. Die Teilenamen stammen aus der Kleidungsliste von MTA ([`CClientPlayerClothes.cpp`](https://github.com/multitheftauto/mtasa-blue/blob/master/Client/mods/deathmatch/logic/CClientPlayerClothes.cpp)).

**Wenn SA-MP den Umbau stilllegt.** Im ersten Live-Test (0.2.0) lief `RebuildPlayer` in 0,0 ms durch, ohne dass sich etwas änderte. Seit 0.3.0 vergleicht das ASI deshalb den Code des Kleidungswegs im Speicher mit der `gta_sa.exe` auf der Festplatte: den Bereich `0x5A4190` bis `0x5A8400` (CClothesBuilder, CPedClothesDesc, CClothes) und die Funktionen, die `RebuildPlayer` aufruft (`CPed::Dress` `0x5E0130`, `CPed::SetModelIndex` `0x5E4880`, `CPed::DeleteRwObject` `0x5DEBF0`, `CPedModelInfo::SetClump` `0x4C7340`, `CClumpModelInfo::SetClump`/`CreateInstance`, `CStreaming::RequestFile`/`LoadAllRequestedModels`, `CTimer::Suspend`/`Resume`, `CWorld::Add`/`Remove`, die Animations-Helfer und `CKeyGen::GetUppercaseKey`; Adressen aus gta-reversed, Liste in `src/diag.rs`). Jede Abweichung steht im Log mit Adresse, Originalbytes, Bytes im Speicher und, bei einem eingesetzten Sprung oder Aufruf, dem Ziel und dem Modul (z. B. `samp.dll`). Das passiert 15 und 60 Sekunden nach dem Start, vor dem ersten Umbau und bei `/kleidungtest info`.

Umgebaut wird dann mit der ersten Variante, die wirkt; die wirksame wird ab da zuerst genommen:

1. **nachgebaut** (seit 0.4.0 zuerst, immer vollständiger Bau): dieselben Schritte wie `RebuildPlayer` in gta-reversed, aber mit den inneren Funktionen des Spiels (`RpAnimBlendClumpExtractAssociations` `0x4D6BE0`, IK-Aufgabe abbrechen, `CPed::DeleteRwObject`, `CWorld::Remove` `0x563280`, `CClothes::ConstructPedModel` `0x5A81E0`, `CPed::Dress`, `RpAnimBlendClumpGiveAssociations` `0x4D6C30`, `PlayerClothes` `0xBC1C78`). Hilft, wenn nur `RebuildPlayer` selbst gepatcht ist.
2. **RebuildPlayer:** `CClothes::RebuildPlayer` wie bisher (vorher wird `PlayerClothes` geleert, damit gebaut wird).
3. **Originalbytes:** `RebuildPlayer`, während alle gefundenen Abweichungen auf die Bytes der EXE zurückgesetzt sind. Direkt nach dem Aufruf kommen genau die Bytes von SA-MP wieder hin (im Spiel-Thread).

„Wirkt“ heißt: Modell 0 hat danach einen neuen Clump und die Figur dessen Geometrie (`CPedModelInfo::m_pRwObject`, Modellinfo über `0xA9B0C8`), der Kleidungs-Baukasten hat also wirklich etwas gebaut. Für jeden Versuch stehen im Log Dauer, Clump der Figur vorher/nachher, Clump von Modell 0 vorher/nachher und der Stand von `player.img` (Bild-ID `0xBC12F8`, Einträge im Verzeichnis `0xBC12C0`). Wirkt dreimal hintereinander keine Variante, hört das ASI mit Umbauen auf und sagt das im Chat. `/kleidungtest info` zeigt in einer vierten Zeile, ob der Kleidungs-Code gepatcht ist (mit der ersten Stelle), welche Variante wirkt und den Stand von `player.img`.

**Jede Figur einzeln (ab 0.4.0).** Der Live-Test mit 0.3.0 hat gezeigt: SA-MP legt `RebuildPlayer` mit einem `ret` an `0x5A82C0` still; der nachgebaute Umbau wirkt (rund 10 ms). Er ist deshalb jetzt die erste Variante. Bei der fremden Figur meldete 0.3.0 „wirkt nicht“: Ihre Kleidungsdaten glichen dem zuletzt gebauten Stand (`PlayerClothes`), und `CreateSkinnedClump` baut dann nichts, sondern die Figur bekommt nur eine Kopie des schon gebauten Modells. Seit 0.4.0 ruft das ASI `ConstructPedModel` ohne alten Stand auf, damit jede Figur neu gebaut wird.

Jeder Bau erzeugt neue Geometrie, Materialien und Texturen (gta-reversed `ClothesBuilder.cpp`: `ConstructGeometryArray`, `CopyTexture` mit `RwRasterCreate`/`RwTextureCreate`, `RpMaterialCreate`). `CPed::Dress` kopiert den Modell-Clump; die Kopie teilt sich die Geometrie nur mit dem Modell (mit Referenzzähler). Ersetzt der nächste Bau das Modell, behält die Figur ihre eigene Geometrie und ihre Texturen. Deshalb baut das ASI Figur für Figur: Modell 0 mit den Kleidungsdaten dieser Figur bauen, genau diese Figur anziehen, dann die nächste. Freigegeben wird, was das Spiel selbst freigibt (`DeleteRwObject` der Figur und des Modells); das ASI legt keine eigenen RenderWare-Objekte an.

Bei jeder Prüfung vergleicht das ASI pro Figur ihr Soll-Outfit mit den Kleidungsdaten, mit ihrem Clump und mit der Geometrie, die es für sie gebaut hat (erste Geometrie über `RpClumpForAllAtomics` `0x749B70`). Teilt eine Figur ihre Geometrie mit einer Figur, die ein anderes Outfit tragen soll, wird sie neu gebaut. Nach jedem Umbau steht im Log für jede CJ-Figur Clump, Geometrie, erste Materialtextur und Outfit.

**Nur umbauen, wenn nötig.** `RebuildPlayer` baut nicht nur die Figur um, sondern auch das gemeinsame Modell 0, aus dem jede neu erscheinende CJ-Figur kopiert wird. Das ASI merkt sich deshalb, welches Outfit das Modell gerade zeigt und welche Figuren schon vor dem letzten Umbau da waren. Eine CJ-Figur, die danach hereinstreamt oder respawnt, trägt das Outfit schon und wird nur übernommen („ohne Umbau“). Umgebaut werden Figuren, deren Kleidungsdaten nicht passen oder deren Modell älter ist.

**Wann nicht.** `RebuildPlayer` setzt die Figur über `CPed::Dress` auf „steht“ und fügt sie neu in die Welt ein. Figuren im Fahrzeug (`bInVehicle`, Flags `+0x46C` Bit 8) und tote Figuren (`m_ePedState` `+0x530`, 54 bis 56) werden deshalb nicht umgebaut, sondern danach.

## Bekannte Risiken

- **Gemeinsame Kleidungsdaten.** In GTA zeigt `m_pPlayerData` jeder Spielerfigur auf `CWorld::Players[n]`, und GTA kennt nur zwei solche Plätze. Wie SA-MP das für viele Spieler löst, ist nicht belegt; `/kleidungtest info` zeigt, ob sich Figuren die Daten teilen. Für den Test mit gleichem Outfit auf allen ist das egal.
- **Modell 0 ist für alle gleich.** Nach dem Test tragen neu erscheinende CJs das zuletzt gebaute Outfit, bis jemand wieder umgebaut wird. `aus` baut deshalb alle CJs auf das Standard-Outfit um. Ob das exakt so aussieht wie der CJ von SA-MP vor dem Test, ist Teil des Tests; ganz sicher ist der Ursprungszustand erst nach Neustart des Spiels. Gibt man `aus`, während keine CJ in der Nähe ist, bleibt das Modell im letzten Outfit.
- **Ruckler.** Jeder Umbau lädt Kleidung synchron nach. Das ASI baut höchstens eine Figur pro Bild um.
- **Speicherzugriffe.** Alle Zeiger werden vor dem Lesen geprüft (`VirtualQuery`). Ein Absturz innerhalb von `RebuildPlayer` selbst lässt sich so nicht abfangen; deshalb bitte Abstürze mit Log melden.
- **Andere Mods.** Mods, die CJ ebenfalls umziehen oder `player.img` ersetzen, können das Ergebnis verfälschen. Am besten mit möglichst sauberem GTA testen.

## Logdatei

`samplife-kleidung.log` wird bei jedem Spielstart neu angelegt, und zwar gleich beim Laden des ASI, noch bevor SA-MP fertig ist. Sie liegt im GTA-Ordner. Lässt sich dort nicht schreiben (z. B. GTA unter `C:\Program Files` ohne Administratorrechte), liegt sie unter `Dokumente\GTA San Andreas User Files\` und als letzte Möglichkeit in `%TEMP%`. Die erste Zeile mit „Logdatei:“ nennt den Ort.

Inhalt:

- Beim Laden: ASI-Version, Pfad der ASI und der EXE, Wert an `0x401000` (GTA 1.0 US oder nicht), Stand von Einstieg A (vorheriges Ziel, was jetzt an `0x53E981` steht), ob samp.dll schon geladen ist.
- Eine Minute lang jede Sekunde, danach bis zehn Minuten alle 30 Sekunden eine Zeile `t+…s:` mit Version der samp.dll, Inhalt von `0x53E981` (unser Hook oder nicht), Zähler der Aufrufe über A und B, wie oft die Logik über A bzw. B lief, wie oft neu eingehakt wurde, Stand von Einstieg B, erkannte SA-MP-Version und Stand des Befehls `/kleidungtest`.
- „Spiel-Thread erreicht“, sobald die Logik das erste Mal läuft.
- Adressen von CChat, CInput und CNetGame, sobald sie sich ändern, und das Ergebnis jedes `CInput::AddCommand`.
- Danach jede Prüfung, deren Ergebnis sich geändert hat, jeden Umbau mit Dauer und bei `/kleidungtest info` die Adressen der gefundenen Figuren und ihrer Kleidungsdaten.

**Fehlersuche, wenn `/kleidungtest` fehlt** („unbekannter Befehl“ vom Server):

- Gar keine Logdatei, auch nicht in `Dokumente\GTA San Andreas User Files` oder `%TEMP%`: Das ASI wurde nicht geladen. Prüfen, ob ein ASI Loader da ist (`vorbisFile.dll` und `vorbisHooked.dll` von SampVoice) und ob `cef.asi` im selben Ordner lädt.
- „NICHT GTA SA 1.0 US“: andere `gta_sa.exe`, das ASI bleibt aus.
- Zähler A und B bleiben bei 0: Keiner der beiden Einstiege läuft; die Zeilen `t+…s` zeigen, was an `0x53E981` steht und ob Einstieg B eingehakt ist.
- „/kleidungtest: warte auf CChat/CInput/CNetGame“: SA-MP ist noch nicht verbunden oder eine Adresse passt nicht zur Version.

## Bauen

Nur über GitHub: [`.github/workflows/kleidung.yml`](../.github/workflows/kleidung.yml) baut bei jedem Push auf einen Branch außer `main`, der `kleidung/` oder den Workflow ändert, und von Hand über „Run workflow“. Ergebnis ist das Artefakt `samplife-kleidung` mit `samplife-kleidung.asi`, `SHA256SUMS.txt` und der `Cargo.lock` des Laufs. Es gibt bewusst kein Release.

Lokal ginge es mit Rust stable und dem Ziel `i686-pc-windows-msvc` (in `kleidung/`: `cargo build --release --target i686-pc-windows-msvc`); die C-Laufzeit wird statisch gelinkt (`.cargo/config.toml`), damit das ASI ohne VC++-Redistributable läuft.

| Datei | Inhalt |
|---|---|
| `src/lib.rs` | `DllMain` |
| `src/hooks.rs` | Einstiege A und B in den Spiel-Thread |
| `src/watchdog.rs` | Lebenszeichen und Protokoll im Sekundentakt, richtet Einstieg B ein |
| `src/game.rs` | GTA-Adressen der Kleidung, Offsets, Speicherprüfung, Zugriff auf Figuren |
| `src/samp.rs` | SA-MP-Version, Chat, Befehl, eigene und fremde Figuren |
| `src/outfit.rs` | die Outfits |
| `src/diag.rs` | Vergleich des Kleidungs-Codes mit gta_sa.exe, Originalbytes für einen Aufruf |
| `src/rebuild.rs` | die drei Umbau-Varianten und die Auswahl der wirksamen |
| `src/logic.rs` | Befehl, Prüfung jede Sekunde, Umbauten, Info |
| `src/log.rs` | Logdatei |

## Lizenz

Der Quelltext steht wie der Rest dieses Repositorys ohne festgelegte Lizenz. Eingebunden wird [samp-client-api](https://github.com/zottce/samp-client-api) von ZOTTCE; das Repository enthält keine Lizenzangabe (siehe auch [`cef/README.md`](../cef/README.md)). Adressen und Strukturen stammen aus den oben verlinkten öffentlichen Projekten; Code daraus ist nicht übernommen.
