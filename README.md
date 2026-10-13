# Samplife Essentials

Downloads für den **Samplife Essentials Installer** des SA-MP-Servers German Samplife. Der Installer lädt die Pakete von den Releases dieses Repositorys und richtet sie im GTA-San-Andreas-Verzeichnis ein.

Dieses Repository enthält keinen eigenen Quelltext, nur fertig gebaute Pakete fremder Open-Source-Projekte.

## Pakete

### samp-cef Client (Browser-Oberfläche im Spiel)

| | |
|---|---|
| Datei | `samp-cef-client-f14fe4f.zip` |
| Quelle | [zottce/samp-cef](https://github.com/zottce/samp-cef), Commit [`f14fe4fa7f0d036edf445ba02b4bddd4cccac377`](https://github.com/zottce/samp-cef/commit/f14fe4fa7f0d036edf445ba02b4bddd4cccac377) |
| Lizenz | GPL-3.0, siehe [`LICENSE`](LICENSE) |
| Gebaut | unverändert aus dem genannten Commit, Windows x86, mit `cargo build --release --target i686-pc-windows-msvc -p cef-client -p cef-renderer -p cef-loader` und `node scripts/package-client.mjs` |
| CEF | 150.0.11 (Chromium 150.0.7871.115), offizielle `windows32`-Distribution |
| SA-MP | 0.3.7 R1 und 0.3.7 R3 |
| SHA-256 | `46c5da7f29632d77d2e43c129329d94cbf83fd9e49d3e34216a987587fdb7e4c` |
| Größe | 154.686.832 Bytes, entpackt rund 339 MB |

Inhalt: `cef.asi` und der Ordner `cef/` mit dem Client, dem Renderer und der vollständigen CEF-Laufzeit. Die Lizenz von CEF liegt als `cef/LICENSE.txt` bei, die von samp-cef als `cef/samp-cef-LICENSE.txt`.

Der Quelltext zu diesem Paket ist der oben verlinkte Commit; es wurden keine Änderungen vorgenommen.

## Hinweise

- Zum Laden von `cef.asi` braucht GTA San Andreas einen ASI Loader.
- Der Client funktioniert nur zusammen mit einem Server, auf dem die samp-cef-Komponente aus demselben Stand läuft.
