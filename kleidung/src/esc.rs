//! ESC menu of the server: ESC opens a page of the server instead of GTA's pause menu,
//! and the game keeps running.
//!
//! The server switches it on with `esc:an` (netz.rs). Then the plugin replaces GTA's ESC
//! check in `CMenuManager::CheckForMenuClosing` (0x576B70) with a call to `esc_pruefen`.
//! On a press while GTA's menu is closed it tells GTA "no ESC" and sends `esc:gedrueckt`
//! to the server, which opens its page and answers `esc:offen` (or `esc:zu` /
//! `esc:original`). No answer within `RUECKFALL`: the plugin opens GTA's own menu, so a
//! hanging server never leaves the player without a menu. `esc:original` opens GTA's menu
//! on purpose (settings). `esc:beenden` quits the game like SA-MP's `/q`, after the page asked.
//!
//! ESC with GTA's menu open, with the SA-MP chat input or a SA-MP dialog open, before
//! `esc:an` and after the connection ended goes to GTA as before. A samp-cef page with
//! input (full or mouse only) keeps ESC away from GTA anyway (cef/nur-maus.patch,
//! `win_event`), so the check only sees ESC when no page has the input.
//!
//! GTA SA 1.0 US, read from gta_sa.exe with a disassembler and matching gta-reversed
//! (Frontend/MenuManager_Input.cpp, Pad.h):
//!
//! | | |
//! |---|---|
//! | `CPad::NewKeyState.esc` (i16) | 0xB733A8 (0xB73190 + 0x218) |
//! | `CPad::OldKeyState.esc` (i16) | 0xB73138 (0xB72F20 + 0x218) |
//! | `FrontEndMenuManager` | 0xBA6748 |
//! | `m_bActivateMenuNextFrame` | +0x33 |
//! | `m_bMenuActive` | +0x5C |
//!
//! At 0x576B9F CheckForMenuClosing tests `CPad::IsEscJustPressed()` inlined:
//! `cmp [NewKeyState.esc], bx; je 0x576BC7; cmp [OldKeyState.esc], bx; jne 0x576BC7`
//! (ebx is 0). 0x576BC7 checks the two "open anyway" flags, so a cleared ESC still opens
//! the menu when `m_bActivateMenuNextFrame` is set. The 18 bytes become
//! `call esc_pruefen; test al, al; je 0x576BC7` and 5 nops. Only over the original bytes;
//! `esc:aus` and the end of the connection put them back.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use client_api::samp::inputs::{Dialog, Input, CMDPROC};

const ESC_NEU: usize = 0xB733A8;
const ESC_ALT: usize = 0xB73138;
const MENUE: usize = 0xBA6748;
const MENUE_NAECHSTES_BILD: usize = MENUE + 0x33;
const MENUE_AKTIV: usize = MENUE + 0x5C;

const STELLE: usize = 0x576B9F;
const ORIGINAL: [u8; 18] = [
    0x66, 0x39, 0x1D, 0xA8, 0x33, 0xB7, 0x00, // cmp [0xB733A8], bx
    0x74, 0x1F, // je 0x576BC7
    0x66, 0x39, 0x1D, 0x38, 0x31, 0xB7, 0x00, // cmp [0xB73138], bx
    0x75, 0x16, // jne 0x576BC7
];
/// no ESC: on to the "open anyway" flags
const KEIN_ESC: usize = 0x576BC7;

/// No answer from the server after this long: GTA's own menu opens.
const RUECKFALL: Duration = Duration::from_millis(1500);
/// GTA may refuse to open in the frame we ask (radio retune); we keep asking this long.
const ORIGINAL_VERSUCHEN: Duration = Duration::from_millis(500);

/// our call is in and the server wants it (read in esc_pruefen)
static AN: AtomicBool = AtomicBool::new(false);
/// a press esc_pruefen kept from GTA, taken by `Esc::frame`
static GEDRUECKT: AtomicBool = AtomicBool::new(false);

/// Called by GTA instead of its ESC check. True: ESC was just pressed (GTA toggles its menu).
extern "C" fn esc_pruefen() -> bool {
    let kante = lese_i16(ESC_NEU) != 0 && lese_i16(ESC_ALT) == 0;

    if !kante || !AN.load(Ordering::SeqCst) || menue_aktiv() {
        return kante;
    }

    // SA-MP chat input or dialog: ESC belongs to SA-MP as before
    let samp_ui = std::panic::catch_unwind(samp_ui_offen).unwrap_or(true);

    if samp_ui {
        return true;
    }

    GEDRUECKT.store(true, Ordering::SeqCst);
    false
}

fn samp_ui_offen() -> bool {
    Input::is_active() || Dialog::get().is_some_and(|dialog| dialog.m_bIsActive == 1)
}

pub struct Esc {
    /// the server sent `esc:an`
    gewollt: bool,
    /// our call is at STELLE
    eingehakt: bool,
    /// `esc:gedrueckt` sent, no answer yet
    warten_seit: Option<Instant>,
    /// GTA's menu is to open, asked since
    original_seit: Option<Instant>,
    /// the server's page is open (last answer `esc:offen`)
    offen: bool,
    grund: &'static str,
    pub gedrueckt: u32,
    pub rueckfall: u32,
    pub original: u32,
}

impl Esc {
    pub fn new() -> Esc {
        Esc {
            gewollt: false,
            eingehakt: false,
            warten_seit: None,
            original_seit: None,
            offen: false,
            grund: "noch nicht eingeschaltet",
            gedrueckt: 0,
            rueckfall: 0,
            original: 0,
        }
    }

    pub fn zustand(&self) -> String {
        if !self.gewollt {
            return format!("aus ({})", self.grund);
        }

        format!(
            "{}, Seite {}, gedrückt {}x, Rückfall {}x, GTA-Menü {}x",
            if self.eingehakt { "an" } else { self.grund },
            if self.offen { "offen" } else { "zu" },
            self.gedrueckt,
            self.rueckfall,
            self.original
        )
    }

    // -----------------------------------------------------------------------
    // Server events, game thread

    pub fn setzen(&mut self, an: bool) {
        self.gewollt = an;
        self.warten_seit = None;
        self.offen = false;
        let ok = self.einhaken(an);

        if an {
            crate::netz::esc_bereit(ok);
        }
    }

    /// `esc:offen`, `esc:zu`: the server answered, the page is open or closed
    pub fn antwort(&mut self, offen: bool) {
        self.warten_seit = None;
        self.offen = offen;
    }

    /// `esc:original`: open GTA's own menu (settings, quit)
    pub fn original_oeffnen(&mut self) {
        self.warten_seit = None;
        self.offen = false;
        self.original_seit = Some(Instant::now());
        self.original += 1;
    }

    /// `esc:beenden`: quit like SA-MP's `/q`; without the command GTA's menu opens (quit from there)
    pub fn beenden(&mut self) {
        self.warten_seit = None;
        self.offen = false;
        crate::log!("ESC-Menü: Spiel beenden");

        if !std::panic::catch_unwind(|| samp_befehl(&["q", "quit"])).unwrap_or(false) {
            crate::log!("ESC-Menü: /q nicht gefunden, GTA-Menü zum Beenden");
            self.original_oeffnen();
        }
    }

    pub fn getrennt(&mut self) {
        if self.gewollt {
            crate::log!("ESC-Menü aus (Verbindung getrennt)");
        }

        self.gewollt = false;
        self.warten_seit = None;
        self.original_seit = None;
        self.offen = false;
        self.einhaken(false);
        GEDRUECKT.store(false, Ordering::SeqCst);
    }

    /// Puts our call in or the original bytes back, only over the expected bytes.
    fn einhaken(&mut self, an: bool) -> bool {
        AN.store(false, Ordering::SeqCst);

        if an != self.eingehakt {
            let ersatz = ersatz(esc_pruefen as extern "C" fn() -> bool as usize);
            let (vorher, nachher) = if an { (ORIGINAL, ersatz) } else { (ersatz, ORIGINAL) };
            let jetzt =
                crate::game::readable(STELLE, 18).then(|| unsafe { (STELLE as *const [u8; 18]).read_unaligned() });

            if jetzt != Some(vorher) {
                crate::log!("ESC-Menü: 0x{STELLE:x} ist {jetzt:02X?} statt {vorher:02X?}, ESC bleibt beim GTA-Menü");
                self.grund = "fremder Code an der ESC-Prüfung";
                return false;
            }

            // GTA calls into this DLL from now on: samp-cef must not unload it (it does on quit)
            if an && !crate::watchdog::festhalten() {
                crate::log!("ESC-Menü: DLL ließ sich nicht festhalten, ESC bleibt beim GTA-Menü");
                self.grund = "DLL nicht festgehalten";
                return false;
            }

            if !crate::diag::write_code(STELLE, &nachher) {
                crate::log!("ESC-Menü: 0x{STELLE:x} nicht beschreibbar");
                self.grund = "ESC-Prüfung nicht beschreibbar";
                return false;
            }

            self.eingehakt = an;
            crate::log!("ESC-Menü: {}", if an { "ESC geht an den Server" } else { "ESC öffnet wieder das GTA-Menü" });
        }

        self.grund = if an { "an" } else { "vom Server ausgeschaltet" };
        AN.store(an && self.eingehakt, Ordering::SeqCst);
        self.eingehakt == an
    }

    // -----------------------------------------------------------------------
    // Game thread, every frame

    pub fn frame(&mut self) {
        let now = Instant::now();

        if GEDRUECKT.swap(false, Ordering::SeqCst) && self.warten_seit.is_none() && self.original_seit.is_none() {
            self.gedrueckt += 1;

            if crate::netz::esc_gedrueckt() {
                self.warten_seit = Some(now);
            } else {
                crate::log!("ESC-Menü: esc:gedrueckt nicht gesendet, GTA-Menü");
                self.rueckfall += 1;
                self.original_seit = Some(now);
            }
        }

        if self.warten_seit.is_some_and(|seit| now.duration_since(seit) >= RUECKFALL) {
            crate::log!("ESC-Menü: keine Antwort nach {} ms, GTA-Menü", RUECKFALL.as_millis());
            self.warten_seit = None;
            self.rueckfall += 1;
            self.original_seit = Some(now);
        }

        if let Some(seit) = self.original_seit {
            if menue_aktiv() {
                self.original_seit = None;
            } else if now.duration_since(seit) >= ORIGINAL_VERSUCHEN {
                crate::log!("ESC-Menü: GTA hat sein Menü nicht geöffnet");
                self.original_seit = None;
            } else {
                // CheckForMenuClosing opens it in the next frame and clears the flag
                schreibe_u8(MENUE_NAECHSTES_BILD, 1);
            }
        }
    }
}

/// `call ziel; test al, al; je KEIN_ESC; nop x5` written at STELLE
fn ersatz(ziel: usize) -> [u8; 18] {
    let mut b = [0x90u8; 18];
    b[0] = 0xE8;
    b[1..5].copy_from_slice(&(ziel as u32).wrapping_sub(STELLE as u32 + 5).to_le_bytes());
    b[5..7].copy_from_slice(&[0x84, 0xC0]);
    b[7..9].copy_from_slice(&[0x0F, 0x84]);
    b[9..13].copy_from_slice(&(KEIN_ESC as u32).wrapping_sub(STELLE as u32 + 13).to_le_bytes());
    b
}

/// Length of CInput's command list (MAX_CLIENT_CMDS in client_api, not public)
const BEFEHLE_MAX: usize = 144;
const BEFEHL_NAME: usize = 33;

/// Runs a SA-MP client command from CInput's list without arguments, as typing `/name` does.
/// False if SA-MP's input or the command is not there.
fn samp_befehl(namen: &[&str]) -> bool {
    let Some(input) = Input::get() else {
        return false;
    };
    let input: *const Input = input;

    // CInput is packed: read every field unaligned through raw pointers
    unsafe {
        let anzahl = std::ptr::addr_of!((*input).m_nCommandCount).read_unaligned();
        let anzahl = (anzahl.max(0) as usize).min(BEFEHLE_MAX);
        let namen_ptr = std::ptr::addr_of!((*input).m_szCommandName) as *const u8;
        let procs = std::ptr::addr_of!((*input).m_pCommandProc) as *const CMDPROC;

        for i in 0..anzahl {
            let roh = std::slice::from_raw_parts(namen_ptr.add(i * BEFEHL_NAME), BEFEHL_NAME);
            let ende = roh.iter().position(|&b| b == 0).unwrap_or(BEFEHL_NAME);
            let Ok(name) = std::str::from_utf8(&roh[..ende]) else {
                continue;
            };

            if !namen.iter().any(|n| n.eq_ignore_ascii_case(name)) {
                continue;
            }

            if let Some(proc) = procs.add(i).read_unaligned() {
                proc(c"".as_ptr());
                return true;
            }
        }
    }

    false
}

fn menue_aktiv() -> bool {
    unsafe { (MENUE_AKTIV as *const u8).read_volatile() != 0 }
}

/// fixed addresses in GTA's data segment, always mapped
fn lese_i16(addr: usize) -> i16 {
    unsafe { (addr as *const i16).read_volatile() }
}

fn schreibe_u8(addr: usize, wert: u8) {
    if crate::game::writable(addr, 1) {
        unsafe { (addr as *mut u8).write_volatile(wert) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ersatz_wie_geplant() {
        let ziel = 0x1234_5678usize;
        let b = ersatz(ziel);
        assert_eq!(b.len(), ORIGINAL.len());
        assert_eq!(b[0], 0xE8);
        let rel = u32::from_le_bytes([b[1], b[2], b[3], b[4]]);
        assert_eq!((STELLE as u32 + 5).wrapping_add(rel), ziel as u32);
        assert_eq!(&b[5..9], &[0x84, 0xC0, 0x0F, 0x84]);
        let rel = u32::from_le_bytes([b[9], b[10], b[11], b[12]]);
        assert_eq!((STELLE as u32 + 13).wrapping_add(rel), KEIN_ESC as u32);
        assert_eq!(&b[13..], &[0x90; 5]);
    }

    #[test]
    fn original_springt_wie_das_ziel() {
        // je +0x1F after the first cmp and jne +0x16 after the second land at KEIN_ESC
        assert_eq!(STELLE + 9 + 0x1F, KEIN_ESC);
        assert_eq!(STELLE + 18 + 0x16, KEIN_ESC);
        assert_eq!(u32::from_le_bytes([ORIGINAL[3], ORIGINAL[4], ORIGINAL[5], ORIGINAL[6]]) as usize, ESC_NEU);
        assert_eq!(u32::from_le_bytes([ORIGINAL[12], ORIGINAL[13], ORIGINAL[14], ORIGINAL[15]]) as usize, ESC_ALT);
    }
}
