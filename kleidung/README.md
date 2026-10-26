# CJ-Kleidung je Spieler

`samplife-kleidung.dll` zieht im SA-MP-Client jede Figur mit Skin 0 (CJ) so an, wie der Server es für ihren Spieler vorgibt, **die eigene und die der anderen Spieler**, wie im Singleplayer mit Kleidung aus `models/player.img`. Der Server schickt die Outfits über samp-cef; das Plugin selbst entscheidet nichts.

Ab 0.6.0 spielt es außerdem **Posen**: Teilanimationen nur für den Oberkörper, mit denen man weiterläuft (siehe [Posen](#posen-teilanimationen)). Die Posen stecken in der DLL.

Ab 0.8.0 bewegen sich die **Lippen**, wenn jemand im Sprach-Chat spricht (siehe [Lippen](#lippen-beim-sprechen)): Taste gedrückt und Stimme am Mikrofon.

Stand: Der Essentials-Installer bringt das Plugin und den passenden samp-cef-Client mit. Im Gamemode ist die Kleidung noch nicht; getestet wird mit dem Filterscript [`beispiel/kleidung_test.pwn`](beispiel/kleidung_test.pwn).

## Voraussetzungen

- GTA San Andreas 1.0 US und SA-MP **0.3.7 R1 oder R3**. Bei anderen Versionen bleibt das Plugin aus; bei 0.3.DL sagt es das einmal im Chat, bei allen anderen steht es nur in der Logdatei.
- **samp-cef-Client ab Version `0x000300`**, also mit [`cef/plugin-ereignisse.patch`](../cef/README.md#server-ereignisse-für-client-plugins). Ältere Clients laden das Plugin zwar, geben ihm aber keine Server-Ereignisse; dann bleibt die Kleidung aus.
- Die Server-Seite von samp-cef bleibt, wie sie ist: Der Patch betrifft nur den Client.

## Installation

1. Am einfachsten über den Essentials-Installer: Er legt `samplife-kleidung.dll` aus dem Release `kleidung-<version>` nach **`GTA/cef/plugins/`** und bringt den passenden samp-cef-Client mit. Von Hand: die DLL aus dem Release oder dem Artefakt des Workflows [`Kleidung`](../.github/workflows/kleidung.yml) dorthin legen. Den Ordner `plugins` gibt es vorher nicht.
2. Nicht neben `gta_sa.exe` und nicht als `.asi`: Dann lädt der ASI Loader die Datei, samp-cef kennt sie nicht, und es kommen keine Daten vom Server an.
3. Spiel wie immer über SA-MP starten und verbinden.

Zum Entfernen die Datei löschen. Das Plugin ändert keine Dateien, nur den laufenden Speicher des Spiels.

## Protokoll

Alle Ereignisse laufen über samp-cef (`cef_emit_event` auf dem Server, `cef_subscribe` für die Antwort). Ereignisse, die mit `kleidung:` beginnen, nimmt das Plugin an; sie gehen nicht an die Browser-Seiten.

| Ereignis | Richtung | Argumente |
|---|---|---|
| `kleidung:hallo` | Client → Server | Version des Plugins, z. B. `0.5.0`. Kommt nach dem Verbinden mit dem CEF-Server und als Antwort auf `kleidung:frage`. Erst danach nimmt der Client Outfits an. |
| `kleidung:frage` | Server → Client | keine. Der Client antwortet mit `kleidung:hallo`, wenn das Plugin läuft. |
| `kleidung:spieler` | Server → Client | Spieler-ID und genau 18 Teile, durch Leerzeichen getrennt |
| `kleidung:koerper` | Server → Client | Spieler-ID, Fett und Muskeln, je eine ganze Zahl von 0 bis 1000 wie die Statistiken `STAT_FAT` und `STAT_MUSCLE` von GTA (ab 0.7.0). Darf vor oder nach `kleidung:spieler` kommen. Fett wirkt ab 200 und ist bei 1000 voll; Muskeln wirken ab 0. Ohne dieses Ereignis bleibt der Körper, wie er ist. Ältere Plugins verwerfen es mit einer Zeile im Log. |
| `kleidung:weg` | Server → Client | Spieler-ID. Das Outfit wird vergessen, der Spieler trägt wieder den Standard-CJ. Beim Verlassen des Servers schicken, damit die ID nichts vererbt. |

Die 18 Teile stehen in der Reihenfolge von `eClothesTexturePart`: Oberkörper, Kopf (Haare), Beine, Schuhe, neun Tattoo-Stellen (4 bis 12), Kette, Uhr, Brille, Hut, Spezial. Jedes Teil ist `textur/modell` (z. B. `suit1red/suit1`), bei Tattoos nur `textur` (z. B. `4weed` an Stelle 4) oder `-` für nichts. Namen wie in der Kleidungsliste von MTA ([`CClientPlayerClothes.cpp`](https://github.com/multitheftauto/mtasa-blue/blob/master/Client/mods/deathmatch/logic/CClientPlayerClothes.cpp)), nur Buchstaben, Ziffern und `_`. Ein Outfit mit falscher Anzahl oder ungültigem Namen wird ganz verworfen und steht im Log.

```pawn
cef_emit_event(empfaenger, "kleidung:spieler", CEFINT(spieler),
    CEFSTR("suit1red/suit1 bald/head suit1trred/suit1tr shoedressblk/shoe - - - - - - - - - - watchgno/watch - cowboy/cowboy -"));
```

Verhalten:

- Solange der Server in einer Verbindung kein `kleidung:spieler` geschickt hat, macht das Plugin nichts. Auf Servern ohne Kleidung bleibt CJ so, wie SA-MP ihn zeigt.
- Danach trägt jede CJ-Figur das Outfit ihres Spielers, die eigene eingeschlossen. Spieler ohne Outfit tragen den Standard-CJ (weißes Unterhemd `vest/vest`, kurze Haare `player_face/head`, Jeans `jeansdenim/jeans`, Turnschuhe `sneakerbincblk/sneaker`).
- Das Plugin merkt sich alle Outfits, auch von Spielern, die gerade nicht in der Nähe sind. Kommt einer herein oder respawnt er, zieht es ihn innerhalb einer Sekunde an. Der Server schickt ein Outfit also nur bei einer Änderung und nach jedem `kleidung:hallo` einmal alle bekannten.
- Figuren mit einem anderen Skin (Fraktions-Uniformen) rührt das Plugin nicht an. Bekommen sie wieder Skin 0, zieht es sie wieder an.
- Beim Trennen vom CEF-Server (auch beim Verlassen des SA-MP-Servers) vergisst das Plugin alle Outfits.

## Posen (Teilanimationen)

SA-MP spielt jede Animation von `ApplyAnimation` über den ganzen Körper und hält den Spieler fest, auch wenn die Animation nur den Oberkörper bewegt. Die Posen spielt deshalb das Plugin, so wie GTA beim Rauchen oder Trinken im Gehen.

| Ereignis | Richtung | Argumente |
|---|---|---|
| `anim:start` | Server → Client | Spieler-ID und Name der Pose, z. B. `anim:start 4 handy_funk`. Die Pose läuft in Schleife, bis `anim:stopp` kommt. |
| `anim:stopp` | Server → Client | Spieler-ID. Die Pose blendet in einer Viertelsekunde aus. |

Ereignisse, die mit `anim:` beginnen, nimmt das Plugin an; sie gehen nicht an die Browser-Seiten. Der Server erkennt Clients mit Posen an `kleidung:hallo` ab Version `0.6.0`.

Verhalten:

- Die Posen funktionieren unabhängig von der Kleidung, auch ohne ein einziges `kleidung:spieler` und bei jedem Skin.
- Das Plugin merkt sich die Pose jedes Spielers, auch wenn er gerade nicht in der Nähe ist. Kommt er herein, respawnt er oder steigt er aus einem Fahrzeug, spielt es die Pose innerhalb einer Viertelsekunde wieder. Der Server schickt eine Pose also nur beim Start und beim Ende und nach jedem `kleidung:hallo` einmal alle laufenden.
- In Fahrzeugen und bei toten Figuren pausiert die Pose. Blendet GTA sie zugunsten einer eigenen Teilanimation aus (z. B. Tragen), versucht das Plugin es höchstens einmal pro Sekunde wieder; der Server beendet Posen deshalb, bevor er so etwas startet.
- Beim Trennen vom CEF-Server blendet das Plugin alle Posen aus und vergisst sie.

**Die Posen-Datei.** `anim/samplife.ifp` (Block `samplife`) wird mit `include_bytes!` in die DLL gebaut und beim ersten Bild im Spiel aus dem Speicher geladen: `RwStreamOpen` mit `rwSTREAMMEMORY` (`0x7ECEF0`), `CAnimManager::LoadAnimFile` (`0x4D47F0`), danach `AddAnimBlockRef` (`0x4D3FB0`), damit der Block nie ausgelagert wird. Abgespielt wird mit `CAnimManager::BlendAnimation(clump, hierarchy, flags, 4.0)` (`0x4D4410`) und den Flags laufend, Schleife und Teilanimation (`0x1 | 0x2 | 0x10`); beendet, indem das Plugin `m_BlendDelta` auf `-4` und das Flag „nach dem Ausblenden löschen“ (`0x4`) setzt. Adressen aus gta-reversed (`Animation/AnimManager.cpp`, `Plugins/RpAnimBlendPlugin/RpAnimBlend.cpp`, `RenderWare/rw/rwplcore.cpp`), Aufbau von `CAnimBlock` und `CAnimBlendAssociation` aus `AnimBlock.h` und `AnimBlendAssociation.h`. Vorher prüft das Plugin, dass GTA noch Platz hat (höchstens 180 Blöcke und 2500 Animationen).

Eine Pose bewegt nur die Knochen, die in der Datei stehen. `anim/posen_bauen.py` behält deshalb nur die Knochen des Oberkörpers, dieselben wie GTAs `CARRY/crry_prtial` (Wirbelsäule, Kopf, Arme, Hände, Bauch, Brust), und gibt starren Posen einen zweiten Keyframe, damit sie nicht 0 lang sind. Neue Posen:

```bash
python anim/posen_bauen.py anim/samplife.ifp quelle.ifp:handy_halten=handy_halten quelle.ifp:handy_funk=handy_funk
```

Jede Angabe ist `datei.ifp:animation=name` (ANP3, wie GTA SA); Namen höchstens 23 Zeichen aus `a-z`, `0-9` und `_`. Alle Posen müssen in einem Aufruf stehen, die Datei wird jedes Mal neu gebaut. Danach eine neue Plugin-Version und im Gamemode die Namen eintragen (`funktionen/posen.inc`). Bisher: `handy_halten` (Hände vorn, Blick nach unten) und `handy_funk` (Hand an der Schulter).

Testen: [`beispiel/pose_test.pwn`](beispiel/pose_test.pwn), laden mit `/rcon loadfs pose_test`, dann `/pose halten`, `/pose funk`, `/pose aus`. `/kleidunginfo` zeigt eine Zeile „Posen“ mit den geladenen Posen und wie viele gerade laufen; im Log stehen das Laden und jede Pose mit Spieler-ID.

## Lippen beim Sprechen

Spricht ein Spieler im Sprach-Chat (Nähe, Telefon, Funk, Sender), bewegt sich das Gesicht seiner Figur. Der Server entscheidet: Sprech- oder Funktaste gedrückt **und** das Plugin des Sprechers hört Stimme am Mikrofon. Taste allein reicht nicht, Stimme ohne Taste auch nicht.

| Ereignis | Richtung | Argumente |
|---|---|---|
| `lippen:mikro` | Server → Client | `1`: eine Sprechtaste dieses Spielers ist gedrückt (nur wenn SampVoice ein Mikrofon gefunden hat), `0`: losgelassen. Nur an den Sprecher selbst. |
| `lippen:pegel` | Client → Server | `1`: Stimme am Mikrofon, solange die Taste gedrückt ist, `0`: wieder still. Nur bei einem Wechsel. |
| `lippen:an` | Server → Client | Spieler-ID. Das Gesicht spricht, bis `lippen:aus` kommt. An alle, auch an den Sprecher. |
| `lippen:aus` | Server → Client | Spieler-ID. Das Gesicht hört auf. |

Ereignisse, die mit `lippen:` beginnen, nimmt das Plugin an; sie gehen nicht an die Browser-Seiten. Der Server erkennt Clients mit Lippen an `kleidung:hallo` ab Version `0.8.0`.

**Stimme erkennen** (`src/mikro.rs`). Nach `lippen:mikro 1` öffnet das Plugin das Standard-Aufnahmegerät, von dem auch SampVoice aufnimmt, über waveIn (`winmm.dll`, Windows teilt das Gerät), 16 kHz mono, vier Puffer zu 40 ms. Die Puffer holt es in jedem Bild im Spiel-Thread ab, ohne eigenen Thread. Stimme heißt: zwei Puffer hintereinander mindestens 10 dB über dem Grundrauschen und lauter als -55 dBFS (ein Tastenklick ist kürzer); still ist es 300 ms nach dem letzten lauten Puffer. Das Grundrauschen lernt das Plugin laufend (sinkt sofort, steigt um 2,5 dB pro Sekunde). An den Server geht nur ja oder nein, kein Ton. Das Gerät bleibt 30 Sekunden nach dem Loslassen offen, damit das nächste Drücken nicht ruckelt (das Öffnen hält den Spiel-Thread kurz an), und schließt beim Trennen. Lässt es sich nicht öffnen, zählt jedes Drücken als Stimme; das steht im Log.

**Gesicht bewegen** (`src/lippen.rs`). Gespielt wird GTAs eigene Sprech-Animation `factalk` aus `ped.ifp` (Block `ped`, immer geladen), dieselbe wie bei sprechenden Passanten, als Teilanimation in Schleife mit `CAnimManager::BlendAnimation` (`0x4D4410`). Sie bewegt nur das Gesicht, Laufen, Fahren und Posen gehen weiter, auch im Fahrzeug. `BlendAnimation` blendet beim Start einer Teilanimation alle anderen Teilanimationen der Figur aus; das Plugin merkt sich vorher deren Blend-Delta und Flags und schreibt sie direkt danach zurück, noch im selben Bild. Posen und Lippen laufen so nebeneinander, und auch GTAs eigene Teilanimationen (Tragen, Rauchen, Telefon) bleiben. Blendet GTA die Lippen aus, startet das Plugin sie höchstens einmal pro Sekunde neu. Wie bei den Posen merkt es sich, wer spricht, auch außer Sichtweite.

`/kleidunginfo` zeigt zwei Zeilen „Lippen“: ob `factalk` gefunden ist, wie viele gerade sprechen und wie oft gestartet wurde, und den Stand des eigenen Mikrofons (offen, Taste, Stimme, letzter Pegel, Grundrauschen, wie oft gemeldet). Im Log stehen Öffnen und Schließen des Mikrofons und Fehler.

## Test mit dem Server

[`beispiel/kleidung_test.pwn`](beispiel/kleidung_test.pwn) ist ein Filterscript, das das Protokoll mit drei festen Outfits spricht. Es braucht die samp-cef-Server-Komponente und deren `cef.inc`.

1. Zwei Tester installieren mit dem Essentials-Installer (samp-cef-Client ab `0x000300` und `samplife-kleidung.dll` in `GTA/cef/plugins/`) und verbinden.
2. Filterscript laden: `/rcon loadfs kleidung_test`. Es fragt alle verbundenen Clients; bei jedem mit Plugin erscheint „[Kleidung] Plugin <Version> gemeldet“. Fehlt die Zeile: `/kleidung frage`, dann `/kleidung info`. Steht dort „Plugin fehlt“, siehe Fehlersuche unten.
3. Beide bekommen Skin 0: dauerhaft über einen Befehl des Gamemodes (bei German Samplife `/setskin [ID] 0`), nur bis zum nächsten Spawn mit `/kleidung skin`. Dann Tester A `/kleidung 1` und Tester B `/kleidung 2`.
4. Prüfen:
   1. Jeder sieht sich und den anderen im eigenen Outfit (A schwarzes Unterhemd, Afro, Tarnhose; B roter Anzug, Cowboyhut).
   2. A wechselt auf `/kleidung 3` (Hoodie, Jogginghose, Cap, Tattoo am linken Oberarm): Beide sehen den Wechsel sofort, B bleibt unverändert.
   3. Respawn, weit weg und zurück (Streaming), Fahrzeug, Skin kurz wechseln und zurück auf 0: Jeder behält sein Outfit.
   4. Im Fahrzeug, als Fahrer und als Beifahrer: Outfit wechseln (`/kleidung 2`). Beide sehen den Wechsel sofort, die Figur bleibt sitzen, und das Fahrzeug lässt sich weiter fahren.
   5. Ein dritter Spieler verbindet später: Er sieht A und B sofort richtig.
   6. A gibt `/kleidung aus` ein: Alle sehen A als Standard-CJ. Verlässt A den Server und bekommt ein anderer seine ID, trägt der den Standard-CJ und nicht das Outfit von A.
   7. Auf einem Server ohne Kleidung (oder ohne Filterscript) bleibt CJ unverändert.
5. `/kleidunginfo` bei beiden ausführen und `samplife-kleidung.log` aus dem GTA-Ordner einsammeln.
6. Zum Schluss `/rcon unloadfs kleidung_test`.

## `/kleidunginfo`

Ein Befehl nur im eigenen Client, für den Support. Er zeigt:

1. Version, SA-MP-Version und Zustand: nicht als Plugin geladen, nicht mit dem CEF-Server verbunden, keine Kleidung vom Server oder aktiv.
2. Die gefundenen CJ-Figuren mit Spieler-ID, die Zahl der Outfits vom Server und die eigene ID.
3. Umbauten mit Dauer und wartende Figuren.
4. Ob der Kleidungs-Code gepatcht ist, welche Umbau-Variante wirkt und wie viele Ereignisse ankamen oder verworfen wurden.
5. Nur bei einem Fehler: dass sich Figuren mit verschiedenem Outfit Geometrie oder Texturen teilen.

## Wie es funktioniert

Alle Adressen gelten für GTA San Andreas 1.0 US; Quellen stehen auch im Quelltext.

**Laden.** samp-cef lädt beim Start jede DLL in `GTA/cef/plugins/` (`client/src/external.rs`). Mit [`cef/plugin-ereignisse.patch`](../cef/README.md#server-ereignisse-für-client-plugins) ruft es danach einmal `cef_server_emitter` auf und gibt jedes Server-Ereignis zuerst an `cef_server_event`. Die Hooks richtet `DllMain` beim Laden ein, wie zuvor beim ASI Loader.

**Einstieg in den Spiel-Thread.** Es gibt zwei Wege; die Logik läuft pro Bild genau einmal, über A, wenn A läuft, sonst über B.

- **A:** Im `DllMain` wird der Aufruf von `CGame::Process` in `Idle()` bei `0x53E981` auf das Plugin umgeleitet; das Plugin ruft zuerst das bisherige Ziel auf (`CGame::Process` `0x53BEE0` oder den Hook eines anderen ASI) und arbeitet danach. Das ist dieselbe Stelle wie `gameProcessEvent` im plugin-sdk ([`shared/Events.h`](https://github.com/DK22Pac/plugin-sdk/blob/master/shared/Events.h)). Überschreibt später jemand die Stelle, ohne das Plugin weiter aufzurufen, hakt es sich nach 120 Bildern ohne Aufruf erneut ein und ruft dann das neue Ziel auf (höchstens fünfmal).
- **B:** `IDirect3DDevice9::Present` des Geräts, mit dem GTA zeichnet (RenderWare-Variable `RwD3D9Device` bei `0xC97C28`, wie in samp-client-api `gta/d3d9.rs`). Das Plugin ersetzt den Present-Eintrag in der Funktionstabelle dieses Geräts und ruft den vorherigen Eintrag weiter auf. So bleiben der Geräte-Proxy von SA-MP, der Present-Hook des CEF-Clients und andere Hooks in der Kette. Present ruft GTA am Ende jedes Bilds im Spiel-Thread auf. Solange das Pausenmenü offen ist, läuft die Logik über B nicht.

Vor allem anderen prüft das Plugin an `0x401000`, ob es GTA 1.0 US ist ([plugin-sdk `shared/GameVersion.cpp`](https://github.com/DK22Pac/plugin-sdk/blob/master/shared/GameVersion.cpp)); sonst bleibt es aus.

**SA-MP.** Versionserkennung, Spieler-Pool und `CInput` kommen aus [samp-client-api](https://github.com/zottce/samp-client-api) (Stand `d497ffad`, derselbe wie in samp-cef). Diese Bibliothek kann weder in den Chat schreiben noch Befehle anlegen; dafür ruft das Plugin die Funktionen von samp.dll direkt auf. Adressen aus [BlastHackNet/SAMP-API](https://github.com/BlastHackNet/SAMP-API/tree/multiver/src/sampapi):

| | 0.3.7 R1 | 0.3.7 R3 |
|---|---|---|
| `CChat*` (`RefChat`) | `samp.dll+0x21A0E4` | `samp.dll+0x26E8C8` |
| `CChat::AddMessage(color, text)` | `samp.dll+0x645A0` | `samp.dll+0x679F0` |
| `CInput*` (`RefInputBox`, auch `CINPUT` in samp-client-api) | `samp.dll+0x21A0E8` | `samp.dll+0x26E8CC` |
| `CInput::AddCommand(name, proc)` | `samp.dll+0x65AD0` | `samp.dll+0x69000` |

**Figuren finden.** Die eigene Figur (Spieler-ID aus `CPlayerPool::m_localInfo.m_nId`) ist `CLocalPlayer::m_pPed->m_pGamePed` aus dem Spieler-Pool von SA-MP (Rückfall: `CWorld::Players[0].m_pPed`, `0xB7CD98`). Fremde Figuren sind `CPlayerInfo::m_pPlayer->m_pPed->m_pGamePed` für jeden Eintrag im Spieler-Pool (samp-client-api `players::players()` und `gta_ped()`); ist der Spieler nicht gestreamt, gibt es keine Figur. Genommen wird jede Figur mit Modell 0 (`CEntity::m_nModelIndex`, `+0x22`).

**Anziehen.** Jede Spielerfigur hat über `CPed::m_pPlayerData` (`+0x480`) einen `CPlayerPedData`, darin `m_pPedClothesDesc` (`+0x4`), die Kleidungsbeschreibung (`CPedClothesDesc`, `0x78` Bytes: 10 Modell-Schlüssel, 18 Textur-Schlüssel, Fett, Muskeln). Das Plugin schreibt die Schlüssel genau wie `CPedClothesDesc::SetTextureAndModel` (`0x5A8080`; die Funktion selbst wird nicht aufgerufen, weil sie laut gta-reversed einen Parameter im Register erwartet), berechnet mit `CKeyGen::GetUppercaseKey` (`0x53CF30`), und ruft dann `CClothes::RebuildPlayer(ped, true)` (`0x5A82C0`) auf. `true` heißt: Fett und Muskeln bleiben, wie sie sind, statt die Werte des eigenen Spielers zu übernehmen. Quellen: [gta-reversed](https://github.com/gta-reversed/gta-reversed) `Clothes.cpp`, `PedClothesDesc.cpp/.h`, `PlayerPedData.h`, `Core/KeyGen.cpp`, `Enums/eClothes*Part.h`; Offsets aus den `VALIDATE_OFFSET`-Zeilen in plugin-sdk `CPed.h`, `CEntity.h`, `CPlayerInfo.h`. Die Teilenamen stammen aus der Kleidungsliste von MTA ([`CClientPlayerClothes.cpp`](https://github.com/multitheftauto/mtasa-blue/blob/master/Client/mods/deathmatch/logic/CClientPlayerClothes.cpp)).

**Wenn SA-MP den Umbau stilllegt.** Im ersten Live-Test (0.2.0) lief `RebuildPlayer` in 0,0 ms durch, ohne dass sich etwas änderte. Seit 0.3.0 vergleicht das Plugin deshalb den Code des Kleidungswegs im Speicher mit der `gta_sa.exe` auf der Festplatte: den Bereich `0x5A4190` bis `0x5A8400` (CClothesBuilder, CPedClothesDesc, CClothes) und die Funktionen, die `RebuildPlayer` aufruft (`CPed::Dress` `0x5E0130`, `CPed::SetModelIndex` `0x5E4880`, `CPed::DeleteRwObject` `0x5DEBF0`, `CPedModelInfo::SetClump` `0x4C7340`, `CClumpModelInfo::SetClump`/`CreateInstance`, `CStreaming::RequestFile`/`LoadAllRequestedModels`, `CTimer::Suspend`/`Resume`, `CWorld::Add`/`Remove`, die Animations-Helfer und `CKeyGen::GetUppercaseKey`; Adressen aus gta-reversed, Liste in `src/diag.rs`). Jede Abweichung steht im Log mit Adresse, Originalbytes, Bytes im Speicher und, bei einem eingesetzten Sprung oder Aufruf, dem Ziel und dem Modul (z. B. `samp.dll`). Das passiert 15 und 60 Sekunden nach dem Start, vor dem ersten Umbau und bei `/kleidunginfo`.

Umgebaut wird dann mit der ersten Variante, die wirkt; die wirksame wird ab da zuerst genommen:

1. **nachgebaut** (seit 0.4.0 zuerst, immer vollständiger Bau): dieselben Schritte wie `RebuildPlayer` in gta-reversed, aber mit den inneren Funktionen des Spiels (`RpAnimBlendClumpExtractAssociations` `0x4D6BE0`, IK-Aufgabe abbrechen, `CPed::DeleteRwObject`, `CWorld::Remove` `0x563280`, `CClothes::ConstructPedModel` `0x5A81E0`, `CPed::Dress`, `RpAnimBlendClumpGiveAssociations` `0x4D6C30`, `PlayerClothes` `0xBC1C78`). Hilft, wenn nur `RebuildPlayer` selbst gepatcht ist.
2. **RebuildPlayer:** `CClothes::RebuildPlayer` wie bisher (vorher wird `PlayerClothes` geleert, damit gebaut wird).
3. **Originalbytes:** `RebuildPlayer`, während alle gefundenen Abweichungen auf die Bytes der EXE zurückgesetzt sind. Direkt nach dem Aufruf kommen genau die Bytes von SA-MP wieder hin (im Spiel-Thread).

„Wirkt“ heißt: Modell 0 hat danach einen neuen Clump und die Figur dessen Geometrie (`CPedModelInfo::m_pRwObject`, Modellinfo über `0xA9B0C8`), der Kleidungs-Baukasten hat also wirklich etwas gebaut. Für jeden Versuch stehen im Log Dauer, Clump der Figur vorher/nachher, Clump von Modell 0 vorher/nachher und der Stand von `player.img` (Bild-ID `0xBC12F8`, Einträge im Verzeichnis `0xBC12C0`). Wirkt dreimal hintereinander keine Variante, hört das Plugin mit Umbauen auf und sagt das im Chat. `/kleidunginfo` zeigt in der vierten Zeile, ob der Kleidungs-Code gepatcht ist und welche Variante wirkt.

**Jede Figur einzeln (ab 0.4.0).** Der Live-Test mit 0.3.0 hat gezeigt: SA-MP legt `RebuildPlayer` mit einem `ret` an `0x5A82C0` still; der nachgebaute Umbau wirkt (rund 10 ms). Er ist deshalb jetzt die erste Variante. Bei der fremden Figur meldete 0.3.0 „wirkt nicht“: Ihre Kleidungsdaten glichen dem zuletzt gebauten Stand (`PlayerClothes`), und `CreateSkinnedClump` baut dann nichts, sondern die Figur bekommt nur eine Kopie des schon gebauten Modells. Seit 0.4.0 ruft das Plugin `ConstructPedModel` ohne alten Stand auf, damit jede Figur neu gebaut wird.

Jeder Bau erzeugt neue Geometrie, Materialien und Texturen (gta-reversed `ClothesBuilder.cpp`: `ConstructGeometryArray`, `CopyTexture` mit `RwRasterCreate`/`RwTextureCreate`, `RpMaterialCreate`). `CPed::Dress` kopiert den Modell-Clump; die Kopie teilt sich die Geometrie nur mit dem Modell (mit Referenzzähler). Ersetzt der nächste Bau das Modell, behält die Figur ihre eigene Geometrie und ihre Texturen. Deshalb baut das Plugin Figur für Figur: Modell 0 mit den Kleidungsdaten dieser Figur bauen, genau diese Figur anziehen, dann die nächste. Freigegeben wird, was das Spiel selbst freigibt (`DeleteRwObject` der Figur und des Modells); das Plugin legt keine eigenen RenderWare-Objekte an.

Bei jeder Prüfung vergleicht das Plugin pro Figur ihr Soll-Outfit mit den Kleidungsdaten, mit ihrem Clump und mit der Geometrie, die es für sie gebaut hat (erste Geometrie über `RpClumpForAllAtomics` `0x749B70`). Teilt eine Figur ihre Geometrie mit einer Figur, die ein anderes Outfit tragen soll, wird sie neu gebaut. Nach jedem Umbau steht im Log für jede CJ-Figur Clump, Geometrie, erste Materialtextur und Outfit.

**Nur umbauen, wenn nötig.** `RebuildPlayer` baut nicht nur die Figur um, sondern auch das gemeinsame Modell 0, aus dem jede neu erscheinende CJ-Figur kopiert wird. Das Plugin merkt sich deshalb, welches Outfit das Modell gerade zeigt und welche Figuren schon vor dem letzten Umbau da waren. Eine CJ-Figur, die danach hereinstreamt oder respawnt, trägt das Outfit schon und wird nur übernommen („ohne Umbau“). Umgebaut werden Figuren, deren Kleidungsdaten nicht passen oder deren Modell älter ist.

**Im Fahrzeug (ab 0.5.1).** `CPed::Dress` setzt jede Figur außer dem Fahrer (`PEDSTATE_DRIVING`) auf „steht“, also auch Beifahrer, und fügt sie mit `CWorld::Add` neu in die Welt ein (gta-reversed `Ped.cpp`). Sitzt eine Figur fest im Fahrzeug (`bInVehicle`, Flags `+0x46C` Bit 8, und `m_ePedState` `+0x530` 50 bis 52: Fahrer, Beifahrer, Taxi-Fahrgast), baut das Plugin sie deshalb ohne `CWorld::Remove` und ohne `Dress` um: Nach `ConstructPedModel` ruft es nur `CPed::SetModelIndex` (`0x5E4880`), den Teil von `Dress`, der den neuen Clump anlegt, und gibt die Sitz-Animation mit `RpAnimBlendClumpGiveAssociations` zurück. Zustand und Platz in der Welt bleiben, wie sie sind. Im Fahrzeug läuft nur diese Variante; wirkt sie nicht, zählt das nicht als Fehlschlag, und die Figur wird nach dem Aussteigen normal umgebaut.

**Fett und Muskeln (ab 0.7.0).** Der Kleidungs-Baukasten mischt jedes Teil zwischen den Formen „normal“, „fat“ und „ripped“ (`CClothesBuilder::CreateSkinnedClump`, Blend-Verhältnisse bei `0x5A42B0` in gta-reversed `ClothesBuilder.cpp`): Fett `(m_fFatStat - 200) / 800` aus der Kleidungsbeschreibung, Muskeln `STAT_MUSCLE / 1000` aus der Statistik des eigenen Spielers. Das Plugin schreibt beide Werte aus `kleidung:koerper` in die Beschreibung (für den Abgleich) und setzt die Statistik `STAT_MUSCLE` (`CStats::StatTypesFloat` `0xB79380`, Index 23) nur für die Dauer des Umbaus einer Figur auf deren Muskeln und stellt sie danach zurück. Kraft, Ausdauer und alles andere im Spiel bleiben unverändert.

**Wann nicht.** Beim Ein- und Aussteigen (`bInVehicle` gesetzt, aber noch nicht 50 bis 52) und bei toten Figuren (`m_ePedState` 54 bis 56) wartet das Plugin und baut sie danach um.

## Bekannte Risiken

- **Gemeinsame Kleidungsdaten.** In GTA zeigt `m_pPlayerData` jeder Spielerfigur auf `CWorld::Players[n]`, und GTA kennt nur zwei solche Plätze. Der Live-Test mit `/kleidungtest mix` (0.4.0) hat verschiedene Outfits auf einem Client gezeigt; `/kleidunginfo` meldet, falls sich Figuren mit verschiedenem Outfit doch etwas teilen.
- **Modell 0 ist für alle gleich.** Neu erscheinende CJs tragen zuerst das zuletzt gebaute Outfit, bis das Plugin sie bei der nächsten Prüfung (spätestens nach einer Sekunde) umbaut. Nach dem Verlassen eines Servers mit Kleidung bleibt das Modell im letzten Outfit, bis das Spiel neu startet; auf einem Server ohne Kleidung kann ein CJ deshalb so aussehen.
- **Ruckler.** Jeder Umbau lädt Kleidung synchron nach. Das Plugin baut höchstens eine Figur pro Bild um.
- **Speicherzugriffe.** Alle Zeiger werden vor dem Lesen geprüft (`VirtualQuery`). Ein Absturz innerhalb von `RebuildPlayer` selbst lässt sich so nicht abfangen; deshalb bitte Abstürze mit Log melden.
- **Mikrofon.** Exklusiver Modus oder gesperrter Mikrofonzugriff für Desktop-Apps in Windows: Dann hört auch SampVoice nichts. Lässt sich das Gerät nicht öffnen, bewegen sich die Lippen nach der Taste.
- **Andere Mods.** Mods, die CJ ebenfalls umziehen oder `player.img` ersetzen, können das Ergebnis verfälschen. Am besten mit möglichst sauberem GTA testen.

## Logdatei

`samplife-kleidung.log` wird bei jedem Spielstart neu angelegt, und zwar gleich beim Laden des Plugins, noch bevor SA-MP fertig ist. Sie liegt im GTA-Ordner. Lässt sich dort nicht schreiben (z. B. GTA unter `C:\Program Files` ohne Administratorrechte), liegt sie unter `Dokumente\GTA San Andreas User Files\` und als letzte Möglichkeit in `%TEMP%`. Die erste Zeile mit „Logdatei:“ nennt den Ort.

Inhalt:

- Beim Laden: Plugin-Version, Pfad der DLL und der EXE, Wert an `0x401000` (GTA 1.0 US oder nicht), Stand von Einstieg A (vorheriges Ziel, was jetzt an `0x53E981` steht), ob samp.dll schon geladen ist.
- Eine Minute lang jede Sekunde, danach bis zehn Minuten alle 30 Sekunden eine Zeile `t+…s:` mit Version der samp.dll, Inhalt von `0x53E981` (unser Hook oder nicht), Zähler der Aufrufe über A und B, wie oft die Logik über A bzw. B lief, wie oft neu eingehakt wurde, Stand von Einstieg B, erkannte SA-MP-Version, Stand des Befehls `/kleidunginfo`, ob samp-cef das Plugin geladen hat und verbunden ist, Zahl der Outfits und der Ereignisse.
- „Spiel-Thread erreicht“, sobald die Logik das erste Mal läuft.
- Adressen von CChat, CInput und CNetGame, sobald sie sich ändern, und das Ergebnis jedes `CInput::AddCommand`.
- Jedes Verbinden und Trennen mit dem CEF-Server, jedes gesendete `kleidung:hallo`, jedes neue Outfit (Spieler-ID, Oberteil, Zahl der Teile) und jedes verworfene Ereignis mit Grund.
- Danach jede Prüfung, deren Ergebnis sich geändert hat, jeden Umbau mit Dauer und bei `/kleidunginfo` die Adressen der gefundenen Figuren und ihrer Kleidungsdaten.

**Fehlersuche, wenn `/kleidunginfo` fehlt** („unbekannter Befehl“ vom Server) oder der Server kein `kleidung:hallo` bekommt:

- Gar keine Logdatei, auch nicht in `Dokumente\GTA San Andreas User Files` oder `%TEMP%`: Das Plugin wurde nicht geladen. Liegt `samplife-kleidung.dll` in `GTA/cef/plugins/`? Läuft samp-cef überhaupt (Browser-Seiten im Spiel)?
- „NICHT GTA SA 1.0 US“: andere `gta_sa.exe`, das Plugin bleibt aus.
- Zähler A und B bleiben bei 0: Keiner der beiden Einstiege läuft; die Zeilen `t+…s` zeigen, was an `0x53E981` steht und ob Einstieg B eingehakt ist.
- „/kleidunginfo: warte auf CChat/CInput/CNetGame“: SA-MP ist noch nicht verbunden oder eine Adresse passt nicht zur Version.
- „samp-cef: nicht als Plugin geladen“ in den Zeilen `t+…s`: Die Datei liegt nicht in `cef/plugins/`, oder der samp-cef-Client ist älter als `0x000300` (ohne `cef/plugin-ereignisse.patch`).
- „kleidung:hallo nicht gesendet“: wie eben, der Client kennt `cef_server_emitter` nicht.
- „kleidung:spieler verworfen“: Das Outfit vom Server hatte nicht genau 18 Teile oder einen ungültigen Namen; der Grund steht dahinter.

## Bauen

Nur über GitHub: [`.github/workflows/kleidung.yml`](../.github/workflows/kleidung.yml) baut bei jedem Push auf einen Branch außer `main`, der `kleidung/` oder den Workflow ändert, und von Hand über „Run workflow“. Vorher laufen die Tests für das Format von `kleidung:spieler` (`cargo test`), danach prüft der Workflow, dass die DLL 32 Bit ist, ohne VC++-Laufzeit auskommt und die vier Exporte für samp-cef hat. Ergebnis ist das Artefakt `samplife-kleidung` mit `samplife-kleidung.dll`, `SHA256SUMS.txt` und der `Cargo.lock` des Laufs. Ein Release gibt es erst, wenn das Plugin in den Installer kommt.

Lokal geht es mit Rust stable und dem Ziel `i686-pc-windows-msvc` (in `kleidung/`: `cargo build --release --target i686-pc-windows-msvc`, Tests mit `cargo test --release --target i686-pc-windows-msvc`); die C-Laufzeit wird statisch gelinkt (`.cargo/config.toml`), damit das Plugin ohne VC++-Redistributable läuft.

| Datei | Inhalt |
|---|---|
| `src/lib.rs` | `DllMain` und die Exporte für samp-cef |
| `src/netz.rs` | Ereignisse vom und zum Server (`kleidung:*`) |
| `src/hooks.rs` | Einstiege A und B in den Spiel-Thread |
| `src/watchdog.rs` | Lebenszeichen und Protokoll im Sekundentakt, richtet Einstieg B ein |
| `src/game.rs` | GTA-Adressen der Kleidung, Offsets, Speicherprüfung, Zugriff auf Figuren |
| `src/samp.rs` | SA-MP-Version, Chat, Befehl, eigene ID, eigene und fremde Figuren |
| `src/outfit.rs` | Format der 18 Teile, Schlüssel, Standard-CJ |
| `src/diag.rs` | Vergleich des Kleidungs-Codes mit gta_sa.exe, Originalbytes für einen Aufruf |
| `src/rebuild.rs` | die drei Umbau-Varianten und die Auswahl der wirksamen |
| `src/logic.rs` | Outfits je Spieler, Prüfung jede Sekunde, Umbauten, `/kleidunginfo` |
| `src/anim.rs` | Posen: eingebettete IFP laden, Teilanimationen je Spieler starten, wiederholen, ausblenden; Blenden, ohne andere Teilanimationen zu beenden |
| `src/lippen.rs` | Lippen: `factalk` je sprechendem Spieler starten, wiederholen, ausblenden |
| `src/mikro.rs` | Pegel des eigenen Mikrofons über waveIn, Stimme erkennen, `lippen:pegel` |
| `anim/samplife.ifp` | die Posen (Block `samplife`), wird in die DLL gebaut |
| `anim/posen_bauen.py` | baut `anim/samplife.ifp` aus IFP-Dateien, nur Oberkörper |
| `src/log.rs` | Logdatei |
| `beispiel/kleidung_test.pwn` | Filterscript für den Test mit dem Server |
| `beispiel/pose_test.pwn` | Filterscript für den Test der Posen |

## Lizenz

Der Quelltext steht wie der Rest dieses Repositorys ohne festgelegte Lizenz. Eingebunden wird [samp-client-api](https://github.com/zottce/samp-client-api) von ZOTTCE; das Repository enthält keine Lizenzangabe (siehe auch [`cef/README.md`](../cef/README.md)). Adressen und Strukturen stammen aus den oben verlinkten öffentlichen Projekten; Code daraus ist nicht übernommen.
