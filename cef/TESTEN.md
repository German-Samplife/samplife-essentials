# Eingabemodus „nur Maus“ im Spiel testen

Vieles hängt an SA-MP und GTA selbst und lässt sich nur im Spiel prüfen. Diese Liste einmal mit SA-MP 0.3.7 R1 und einmal mit R3 durchgehen.

## Vorbereitung

1. **Client:** Paket `samp-cef-client-….zip` aus dem Artefakt `samp-cef-client` des Workflow-Laufs ins GTA-Verzeichnis entpacken (ersetzt `cef.asi` und den Ordner `cef/`). Vorher den alten Stand sichern.
2. **Testseiten:** [`beispiel/index.html`](beispiel/index.html) und [`beispiel/tasten.html`](beispiel/tasten.html) nach `<GTA>/cef/assets/nur-maus-test/` kopieren.
3. **Server:** Komponente bzw. Plugin aus demselben Stand bauen (siehe [README](README.md#server-seite-selbst-bauen)), `server/cef.inc` aus dem Patch verwenden und das Filterscript [`beispiel/nurmaus_test.pwn`](beispiel/nurmaus_test.pwn) laden.
4. Im Spiel `/nurmaus` eingeben. Rechts erscheint die Testseite im Modus `mouse`, links die zweite Seite, die mit `cef_always_listen_keys` alle Tasten mitliest. Im Chat steht die Client-Version (`200`) und jede Änderung des Modus.

Befehle: `/nurmaus voll`, `/nurmaus maus`, `/nurmaus aus`, `/nurmaus alt` (bisheriges `cef_focus_browser`), `/nurmaus weg`. Die Knöpfe `none`, `full`, `mouse` auf der Seite schalten über `cef.set_input_mode`.

## Checkliste

Jeweils im Modus `mouse`, wenn nichts anderes steht.

### Bewegung und Fahrzeug
- [ ] Laufen mit WASD, die Figur bewegt sich, die Kamera bleibt fest.
- [ ] Sprinten (Leertaste) und Springen (Shift) funktionieren.
- [ ] Ins Fahrzeug steigen (F/Enter), fahren, bremsen, hupen, aussteigen.
- [ ] Die Maus dreht die Kamera nicht, weder zu Fuß noch im Fahrzeug.

### Maus auf der Seite
- [ ] Cursor ist sichtbar, Knöpfe reagieren auf Hover und Klick, „Klicks“ zählt hoch.
- [ ] Mausrad über der Seite: „Mausrad“ zählt.
- [ ] Doppelklick in das Textfeld markiert ein Wort.

### Waffe in der Hand (wichtigster Punkt)
- [ ] Mit Waffe (z. B. Deagle) Linksklick auf die Seite und daneben: **es fällt kein Schuss**.
- [ ] Rechtsklick: **die Figur zielt nicht**.
- [ ] Mausrad: **die Waffe wechselt nicht**.
- [ ] Nach `/nurmaus aus` schießt, zielt und wechselt die Waffe wieder normal.
- [ ] Gegenprobe mit `"mouse_only_block_game_mouse": false` in `<GTA>/cef/config.json` (Spiel neu starten): zeigt, ob SA-MP-Cursormodus 3 allein schon reicht. Ergebnis notieren.

### Textfelder
- [ ] Klick ins Textfeld: Chat meldet `full`, Seite zeigt `full`. Tippen landet im Feld, die Figur läuft nicht, WASD erscheinen in der Tastenliste der Testseite.
- [ ] Umlaute und Sonderzeichen im Feld (ä, ö, ü, ß, @).
- [ ] Klick neben das Feld: zurück auf `mouse`, WASD bewegen wieder die Figur.
- [ ] Beim Wechsel hängt keine Taste: W gedrückt halten, ins Feld klicken, loslassen, wieder herausklicken, die Figur läuft nicht von selbst weiter.
- [ ] Mehrzeiliges Feld (textarea) verhält sich genauso.

### ESC
- [ ] ESC im Modus `mouse` öffnet **nicht** das Pausemenü und erscheint in der Tastenliste der Testseite.
- [ ] ESC nach `/nurmaus aus` öffnet das Pausemenü wie gewohnt.

### Andere Seiten und Tasten
- [ ] Die zweite Seite (links) zeigt alle Tasten (WASD, I, E, M, Komma, F3), solange die Testseite auf `mouse` steht.
- [ ] Während ein Textfeld Fokus hat (`full`), bekommt die zweite Seite keine Tasten.
- [ ] SampVoice: Sprechtaste B und Funk Z funktionieren im Modus `mouse`.
- [ ] F8 (Screenshot) und F7 (Chat ein/aus) gehen weiter.

### SA-MP
- [ ] Chat mit T öffnen, schreiben, mit Enter senden; ESC schließt die Chat-Eingabe. Danach ist wieder `mouse` aktiv (Cursor da, Kamera fest).
- [ ] Ein SA-MP-Dialog (z. B. über einen Befehl des Servers) lässt sich mit der Maus bedienen, das Eingabefeld darin nimmt Text, ESC schließt ihn. Danach wieder `mouse`.
- [ ] Pausemenü über `/nurmaus aus`, ESC, wieder zurück, `/nurmaus maus`: alles wie vorher.

### Fenster
- [ ] Alt+Tab aus dem Spiel und zurück: Modus bleibt `mouse`, Cursor da, Kamera fest, keine Taste hängt (vorher W gedrückt halten und während Alt+Tab loslassen).
- [ ] Fenstermodus und Vollbild.

### Bisheriges Verhalten
- [ ] `/nurmaus voll` und `/nurmaus alt`: wie bisher, Spieler steht, alle Tasten an die Seite, Pausemenü nicht per ESC.
- [ ] `/nurmaus weg` im Modus `mouse`: Cursor verschwindet, Kamera und Maus gehören wieder dem Spiel.
- [ ] Bestehende Seiten des Servers (Login, Navi, Bankautomat) funktionieren unverändert.
- [ ] Alter Client (Release `cef-f14fe4f`) mit neuem Server: `/nurmaus` zeigt Version `100` und den Hinweis, die Testseite bekommt volle Eingabe, nichts stürzt ab.
- [ ] Neuer Client mit altem Server (ohne Patch): Verbindung klappt, bestehende Seiten laufen.

Gefundene Probleme bitte mit SA-MP-Version, Schritt und `cef_client.log` aus dem GTA-Verzeichnis melden (`"log_level": "debug"` in `cef/config.json` schreibt mehr).
