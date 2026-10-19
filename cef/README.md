# samp-cef mit Eingabemodus „nur Maus“

German Samplife nutzt für die Browser-Oberflächen im Spiel [samp-cef](https://github.com/zottce/samp-cef) von ZOTTCE (GPL-3.0, siehe [`LIZENZEN/samp-cef-GPL-3.0.txt`](../LIZENZEN/samp-cef-GPL-3.0.txt)), Stand [`f14fe4f`](https://github.com/zottce/samp-cef/commit/f14fe4fa7f0d036edf445ba02b4bddd4cccac377).

Im Original kennt ein Browser nur zwei Zustände:
- **nicht fokussiert:** Die Seite ist sichtbar, bekommt aber weder Maus noch Tastatur.
- **fokussiert:** Die Seite bekommt Maus und Tastatur, der Spieler steht still.

Für Anzeigen, die man anklicken will, während man weiterläuft oder fährt (Navi, Meldungen, kleine Menüs), fehlt ein Zustand dazwischen. Dafür gibt es hier den Eingabemodus **nur Maus**.

## Die Modi

| Modus | Wert | Maus | Tastatur | Entspricht |
|---|---|---|---|---|
| `CEF_INPUT_NONE` | 0 | Spiel | Spiel | `cef_focus_browser(.., false)` |
| `CEF_INPUT_FULL` | 1 | Browser | Browser | `cef_focus_browser(.., true)` |
| `CEF_INPUT_MOUSE_ONLY` | 2 | Browser | Spiel (ESC an den Browser) | neu |

Der Modus gilt je Browser. FULL und MOUSE_ONLY teilen sich die Warteschlange, die es für `focused = true` schon gab: Der erste Browser hält die Eingabe, weitere warten, bis er sie abgibt.

### Verhalten im Modus nur Maus

- **Cursor** ist sichtbar (SA-MP-Cursormodus 3: Kamera gesperrt, Steuerung frei).
- **Maus:** Bewegung, Klicks und Mausrad gehen an den Browser und nicht an GTA. Die Kamera dreht sich nicht.
- **Kein Schießen, Zielen, Waffenwechsel:** GTA liest Maustasten und Mausrad über DirectInput, nicht über Fenster-Nachrichten. Deshalb leert der Client zusätzlich den Mauszustand von GTA, direkt nachdem das Spiel ihn gelesen hat (Haken auf `CPad::UpdateMouse`, GTA SA 1.0 US, Adressen aus [gta-reversed](https://github.com/gta-reversed/gta-reversed): `0x53F3C0`, `CPad::NewMouseControllerState` `0xB73418`, Größe `0x14`). Abschaltbar mit `"mouse_only_block_game_mouse": false` in `cef/config.json`. Außerdem liefert `GetAsyncKeyState` für die Maustasten und ESC dann 0.
- **Tastatur** geht vollständig an GTA: laufen, sprinten, springen, Fahrzeug, Hupe, Chat mit T, Enter/F. Sprechtasten anderer Erweiterungen (SampVoice B und Z) bleiben unberührt.
- **Andere Browser** mit `cef_always_listen_keys` bekommen weiter alle Tasten, wie wenn kein Browser fokussiert ist.
- **ESC** geht an die Seite im Modus nur Maus und öffnet nicht das Pausemenü. Die Seite entscheidet, was ESC tut (z. B. schließen über ein Ereignis an den Server).
- **Textfelder:** Bekommt ein `input`, `textarea` oder ein `contenteditable`-Element den Fokus, schaltet der Browser von selbst auf FULL (Tippen geht in das Feld, der Spieler bleibt stehen). Klickt der Spieler daneben oder verliert das Feld sonst den Fokus, geht es zurück auf nur Maus. Die Seite braucht dafür keinen Code; erkannt wird das im Renderer (`on_focused_node_changed` mit `is_editable`).
- **SA-MP-Chat, Dialoge, Pausemenü:** Ist die Chat-Eingabe oder ein SA-MP-Dialog offen, bekommt SA-MP Maus, ESC und Cursor wie ohne Browser. Danach stellt der Client den Modus wieder her.
- **Alt+Tab:** Beim Wechsel aus dem Spiel bekommen die Seiten Key-up für alle gedrückten Tasten und Mouse-up für gedrückte Maustasten, damit nichts hängen bleibt. Der Cursormodus wird jedes Bild neu gesetzt, nach der Rückkehr gilt der Modus wieder.
- **Moduswechsel** lösen ebenfalls Key-up für gedrückte Tasten aus.

FULL und NONE verhalten sich genau wie `focused = true` und `false` bisher.

## Neue Schnittstellen

### Pawn (`server/cef.inc`)

```pawn
#define CEF_INPUT_NONE 0
#define CEF_INPUT_FULL 1
#define CEF_INPUT_MOUSE_ONLY 2
#define CEF_CLIENT_VERSION_INPUT_MODE 0x000200

native cef_set_input_mode(player_id, browser_id, mode);
native cef_get_client_version(player_id);

forward OnCefInputModeChanged(player_id, browser_id, mode);
```

- `cef_set_input_mode` gibt 0 zurück, wenn `mode` nicht 0, 1 oder 2 ist.
- `cef_get_client_version` liefert die Version, die der Client beim Verbinden meldet (`RequestJoin.plugin_version`), 0 solange unbekannt. Dieser Client meldet `0x000200`, das Original `0x000100`.
- `OnCefInputModeChanged` kommt bei jeder Änderung des Modus eines Browsers beim Client: nach `cef_set_input_mode` und `cef_focus_browser`, nach `cef.set_focus` und `cef.set_input_mode` aus der Seite und wenn ein Textfeld nur Maus auf FULL schaltet und zurück. `mode` ist der Modus dieses Browsers, auch wenn er gerade in der Warteschlange steht.
- `cef_focus_browser` und `cef_create_browser(.., focused)` bleiben unverändert (FULL bzw. NONE).

Die open.mp-Komponente bietet dieselben Natives und das Ereignis. Mit gRPC-Erweiterung sind `cef_set_input_mode`, `cef_get_client_version` und `OnCefInputModeChanged` dort ebenfalls angemeldet. Das Legacy-Plugin `server/` hat die Natives und das Callback auch.

### JavaScript (auf der Seite)

```js
cef.set_input_mode("none" | "full" | "mouse");
cef.on("cef:input_mode", (modus) => { /* "none", "full" oder "mouse" */ });
cef.version; // "0.2.0" (vorher "0.1.0")
```

`cef:input_mode` kommt nach dem Laden der Seite und bei jeder Änderung. `cef.set_focus(true/false)` funktioniert wie bisher.

### Protokoll

Neu in `proto/packets.proto` (und von Hand gleich in `messages/src/proto/packets.rs`):

| Paket | Id | Richtung | Inhalt |
|---|---|---|---|
| `SetInputMode` | 17 | Server → Client | `browser_id`, `mode` |
| `InputModeChanged` | 18 | Client → Server | `browser_id`, `mode` |

## Abwärtskompatibilität

- **Alter Client, neuer Server:** Der Server merkt sich die Version aus `RequestJoin`. Clients unter `0x000200` bekommen statt `SetInputMode` das alte `FocusBrowser`: NONE wird nicht fokussiert, FULL und MOUSE_ONLY werden fokussiert (also volle Eingabe wie bisher). Wer das anders will, fragt vorher `cef_get_client_version` ab. Ein alter Client würde ein unbekanntes Paket ohnehin nur als `OPEN_CONNECTION` lesen und im verbundenen Zustand verwerfen.
- **Neuer Client, alter Server:** Der alte Server prüft die Version nicht und antwortet mit Erfolg. `InputModeChanged` kennt er nicht; unbekannte Pakete vom Client landen dort im leeren Zweig und werden verworfen. Alles andere läuft wie bisher.
- Seiten, die nur `cef.set_focus` benutzen, merken keinen Unterschied.

## Geänderte Dateien

[`nur-maus.patch`](nur-maus.patch) gegen samp-cef `f14fe4f`:

| Datei | Änderung |
|---|---|
| `client/src/browser/manager.rs` | `InputMode`, Modus je Browser, Textfeld-Zustand, Weiterleitung der Tasten je Modus, Key-up/Mouse-up beim Wechsel |
| `client/src/app.rs` | Cursor je Modus (2 für FULL, 3 für nur Maus), WndProc (Maus und ESC schlucken, Tasten durchlassen), `GetAsyncKeyState`, Alt+Tab, `CEF_PLUGIN_VERSION` = `0x000200` |
| `client/src/mouse_only.rs` | Haken auf `CPad::UpdateMouse`, leert GTAs Mauszustand im Modus nur Maus |
| `client/src/lib.rs`, `client/config.json` | Schalter `mouse_only_block_game_mouse` |
| `client/src/browser/client.rs`, `client/src/network.rs` | Nachrichten `set_input_mode` und `editing` aus dem Renderer, Pakete `SetInputMode`/`InputModeChanged` |
| `cef/src/dom.rs`, `cef/src/handlers/render_process.rs`, `cef/src/rust_to_c/render_process_handler.rs` | `on_focused_node_changed` und `DomNode::is_editable` im CEF-Wrapper |
| `renderer/src/main.rs` | `cef.set_input_mode`, `cef.version` 0.2.0, Textfeld-Erkennung |
| `proto/packets.proto`, `messages/src/…` | neue Pakete |
| `server-core/src/…` | Version des Clients, `set_input_mode` mit Rückfall auf `FocusBrowser`, Ereignis `InputModeChanged` |
| `openmp-component/src/lib.rs`, `openmp-component/component/src/component.cpp` | Natives, Callback, gRPC |
| `server/src/lib.rs`, `server/cef.inc` | Legacy-Plugin, Include |
| `docs/main_en.md` | Beschreibung der neuen Schnittstellen |
| `Cargo.toml`, `Cargo.lock` | `[patch]` auf die angepasste `samp-client-api` in `../samp-client-api` |

[`client-api-cursor.patch`](client-api-cursor.patch) gegen [samp-client-api `d497ffa`](https://github.com/zottce/samp-client-api/commit/d497ffad4903d9cc3f02869c744c3c3d96217cbb) ergänzt in `src/samp/inputs.rs` die Funktionen `set_cursor_mode(mode, hide)` und `process_input_enabling()` (Adressen je SA-MP-Version wie bei `show_cursor`) und die Konstanten der SA-MP-Cursormodi.

## Bauen

Das übernimmt GitHub mit [`.github/workflows/cef.yml`](../.github/workflows/cef.yml):

1. samp-cef auf Commit `f14fe4f…` nach `quelle/` und samp-client-api auf `d497ffa…` nach `samp-client-api/` holen und die beiden Patches anwenden. `Cargo.toml` aus dem Patch zeigt mit `[patch."https://github.com/zottce/samp-client-api.git"]` auf `../samp-client-api`.
2. DirectX SDK (Juni 2010, nur `Lib/x86`) und die CEF-Distribution aus `cef-distribution.json` holen (CEF 150, Prüfsumme wird geprüft).
3. Mit Rust stable `cargo build --release --target i686-pc-windows-msvc` bauen, dazu die Tests von `cef-messages` und `cef-server-core`.
4. Prüfen, dass die Dateien 32 Bit sind und die Änderung enthalten.
5. Mit `scripts/package-client.mjs` packen wie im Original, `samp-cef-lizenz.txt` dazulegen.

Ein Push auf einen Branch, der `cef/` ändert, baut und prüft nur. Das Paket `samp-cef-client-nur-maus-<commit>.zip` hängt dann als Artefakt `samp-cef-client` am Lauf.

Ein Tag wie `cef-f14fe4f-sl1` legt ein Release mit `samp-cef-client-<tag>.zip` und `SHA256SUMS.txt` an. Prüfsumme und Größe für das Manifest stehen im Release-Text.

Der Job `server-pruefen` baut zur Kontrolle auch das Legacy-Plugin und die open.mp-Komponente für Linux x86 (open.mp vom aktuellen Stand, wie im Original-Workflow). Die Dateien hängen als Artefakt `samp-cef-server-linux-nur-zum-testen` am Lauf.

## Server-Seite selbst bauen

Client und Server müssen aus demselben Stand kommen. Kurzfassung für Linux (Rust mit Ziel `i686-unknown-linux-gnu`, `cmake`, `g++-multilib`):

```sh
# samp-cef und samp-client-api nebeneinander, genau diese Commits
git init samp-cef && cd samp-cef
git fetch --depth 1 https://github.com/zottce/samp-cef.git f14fe4fa7f0d036edf445ba02b4bddd4cccac377
git checkout FETCH_HEAD
git apply /pfad/zu/samplife-essentials/cef/nur-maus.patch
cd ..
git init samp-client-api && cd samp-client-api
git fetch --depth 1 https://github.com/zottce/samp-client-api.git d497ffad4903d9cc3f02869c744c3c3d96217cbb
git checkout FETCH_HEAD
git apply /pfad/zu/samplife-essentials/cef/client-api-cursor.patch
cd ../samp-cef

# open.mp-Komponente
git clone https://github.com/openmultiplayer/open.mp ../open.mp
git -C ../open.mp submodule update --init --recursive SDK lib/pawn
scripts/build-openmp-component.sh --openmp-root ../open.mp
# Ergebnis: target/openmp-component-build/cef.so, dazu server/cef.inc ins Gamemode-Include
```

Die Server-Seite benutzt `samp-client-api` nicht. Cargo braucht den Ordner `../samp-client-api` trotzdem, weil `Cargo.toml` ihn als `[patch]` für den ganzen Workspace nennt; ohne ihn bricht jeder `cargo build` ab.

## Lizenz

samp-cef stammt von ZOTTCE und steht unter der GPL-3.0 ([`LIZENZEN/samp-cef-GPL-3.0.txt`](../LIZENZEN/samp-cef-GPL-3.0.txt)). Unsere Änderungen daran stehen unter derselben Lizenz. samp-client-api ist ebenfalls von ZOTTCE und wird von samp-cef als Abhängigkeit eingebunden; das Repository enthält selbst keine Lizenzangabe. Der vollständige Quelltext jedes Pakets aus diesem Workflow ist der genannte Original-Commit plus die Patches in diesem Ordner im Stand des jeweiligen Tags. CEF selbst ist die unveränderte offizielle Distribution und steht unter der BSD-Lizenz (`cef/LICENSE.txt` und `cef/CREDITS.html` im Paket).
