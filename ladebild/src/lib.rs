//! samplife-ladebild: Eigenes Ladebild beim Start von GTA San Andreas mit SA-MP.
//!
//! Ein ASI (`GTA/samplife-ladebild.asi`), das der ASI Loader von SampVoice beim Start lädt.
//! Es legt `samplife-ladebild.jpg` aus dem GTA-Verzeichnis bildschirmfüllend über alles, was
//! GTA und SA-MP beim Laden zeigen, und blendet es aus, sobald das Spiel läuft. samp.dll und
//! samp.saa werden nicht angefasst.
//!
//! Ablauf: `DllMain` startet nur einen Faden (unter der Loader-Sperre passiert nichts). Der
//! liest das Bild, wartet auf das Direct3D-Gerät von GTA und hängt sich in dessen Present ein
//! (zeichnen.rs). Gezeichnet wird im Spielfaden, direkt vor jedem Present.

#![allow(clippy::missing_safety_doc)]

mod bild;
mod log;
mod spiel;
mod zeichnen;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use winapi::shared::minwindef::{BOOL, DWORD, HINSTANCE, LPVOID, TRUE};
use winapi::um::libloaderapi::DisableThreadLibraryCalls;
use winapi::um::winnt::DLL_PROCESS_ATTACH;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

static MODUL: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn DllMain(instance: HINSTANCE, reason: DWORD, _reserved: LPVOID) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        DisableThreadLibraryCalls(instance);
        MODUL.store(instance as usize, Ordering::SeqCst);
        std::thread::spawn(|| {
            let _ = std::panic::catch_unwind(vorbereiten);
        });
    }
    TRUE
}

/// Im eigenen Faden: Bild lesen, auf das Gerät warten, Present einhaken.
fn vorbereiten() {
    log::zeile(&format!("samplife-ladebild {VERSION} geladen"));

    if !spiel::exe_ok() {
        log::zeile("gta_sa.exe ist nicht 1.0 US, das Ladebild bleibt aus");
        return;
    }

    match bild::laden() {
        Ok((breite, hoehe)) => log::zeile(&format!("Bild geladen: {breite} x {hoehe}")),
        Err(fehler) => {
            log::zeile(&format!("Kein Ladebild: {fehler}"));
            return;
        }
    }

    // Das Gerät gibt es erst, wenn RenderWare die Grafik eingerichtet hat; höchstens 2 Minuten warten
    let mut zuletzt = String::new();
    for _ in 0..1200 {
        if zeichnen::fertig() {
            return;
        }
        if let Some(text) = zeichnen::einhaken() {
            // dieselbe Meldung nur einmal
            if text != zuletzt {
                log::zeile(&text);
                zuletzt = text;
            }
            if zeichnen::eingehakt() || zeichnen::fertig() {
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    log::zeile("Kein Direct3D-Gerät gefunden, das Ladebild bleibt aus");
}
