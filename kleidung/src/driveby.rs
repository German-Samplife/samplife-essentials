//! Drive-by while aiming: a passenger with an allowed weapon leans out of the window as
//! long as he holds the right mouse button and sits back down as soon as he lets go.
//!
//! SA-MP itself only knows H: `CLocalPlayer::Process` calls `CPed::EnablePassengerDrivebyMode`
//! when the horn button (CControllerState::ShockButtonL) is down and sets
//! `CLocalPlayer::m_bPassengerDriveBy`; back in only when the weapon in the hand becomes the
//! fist (then it calls the function that deletes the task). A second H does nothing. This
//! module drives the same two samp.dll functions from the aim button and keeps the flag in
//! step, because the passenger sync sends exactly this flag as the drive-by bit (R1 0x5491:
//! `mov dl, [ebp+0x1A7]; shl dl, 6`). So the other players see him lean out, and samp.dll's
//! own handling (fist -> back in) keeps working.
//!
//! The server switches it on with `driveby:waffen <ids>` (netz.rs) after `kleidung:hallo` and
//! sends the weapon ids that may be used from the window; without ids it is off. Which
//! weapons are allowed lives only in the gamemode. While it is on, a drive-by with a weapon
//! that is not in the list or without the aim button ends at once, also one started with H.
//!
//! While it is on, only "fire weapon" (left mouse button by default) shoots from the window.
//! `CPad::GetWeapon` (0x540180) normally adds LeftShoulder1 ("secondary fire", right mouse
//! button by default) when the ped is in a drive-by task (or attached to something):
//! `test bl, bl; movsx eax, [esi+0x22]; je +0x1A; movsx ecx, [esi+8]; add eax, ecx`. The
//! `je` at 0x540219 (74 1A) becomes `jmp` (EB 1A), so holding the right button only leans
//! out and aims. Put back when the server switches it off or the connection ends.
//!
//! Nothing changes in the pause menu, with the SA-MP chat input open or while the game has
//! the player's controls disabled (cutscene, SA-MP cursor locking the controls). The
//! mouse-only mode of samp-cef clears GTA's mouse state, so a page that takes the mouse
//! (phone, vehicle menu) brings him back in.
//!
//! Addresses, read from samp.dll 0.3.7 R1 (0.3.7.0) and R3-1 with a disassembler; names of
//! the first function and CLocalPlayer::Process from BlastHackNet/SAMP-API (branch multiver):
//!
//! | | R1 | R3 |
//! |---|---|---|
//! | `CPed::EnablePassengerDrivebyMode()` thiscall, BOOL | 0xA7850 | 0xAC700 |
//! | drive-by task active? thiscall, BOOL (task type 0x3FE in primary slot 3) | 0xA78F0 | 0xAC7A0 |
//! | end drive-by (deletes that task) thiscall | 0xA7940 | 0xAC7F0 |
//! | `CLocalPlayer::m_bPassengerDriveBy` (BOOL) | +0x1A7 | +0x1CB |
//!
//! Callers in `CLocalPlayer::Process`: R1 0x782E / 0x7865, R3 0x784B / 0x7882.
//!
//! GTA SA 1.0 US: `CPad::NewMouseControllerState` 0xB73418 (rmb at +1), `Pads[0]`
//! 0xB73458 with `DisablePlayerControls` (u16) at +0x10E, `FrontEndMenuManager.m_bMenuActive`
//! 0xBA67A4; CPed::m_pVehicle +0x58C and CVehicle::m_pDriver +0x460 as samp.dll uses them
//! (`CPed::IsInVehicle`/`IsAPassenger`, R1 0xA6880).

use std::ffi::c_void;
use std::time::{Duration, Instant};

use crate::game::{read_ptr, read_u32, readable, writable, Ped};
use crate::samp::{self, SampVersion};

const MOUSE_RMB: usize = 0xB73418 + 1;
const PAD0_DISABLE_PLAYER_CONTROLS: usize = 0xB73458 + 0x10E;
const MENU_ACTIVE: usize = 0xBA67A4;

/// `je` in CPad::GetWeapon that skips LeftShoulder1 outside a drive-by (see above)
const ZWEITFEUER_SPRUNG: usize = 0x540219;
const ZWEITFEUER_AN: [u8; 2] = [0x74, 0x1A];
const ZWEITFEUER_AUS: [u8; 2] = [0xEB, 0x1A];

const PED_VEHICLE: usize = 0x58C;
const VEHICLE_DRIVER: usize = 0x460;

/// highest weapon id the server may send (46 parachute)
const WAFFE_MAX: u32 = 46;
/// at most this many weapon ids in `driveby:waffen`
pub const WAFFEN_MAX: usize = 24;

/// After leaning out, GTA puts the task in place a frame later. If it is not there after
/// this time (SA-MP refused, task ended by itself), the flag is reset, and holding the
/// button tries again.
const AUF_TASK_WARTEN: Duration = Duration::from_millis(500);

struct Adressen {
    enable: usize,
    aktiv: usize,
    beenden: usize,
}

fn adressen(ver: SampVersion) -> Option<Adressen> {
    match ver {
        SampVersion::R1 => Some(Adressen { enable: 0xA7850, aktiv: 0xA78F0, beenden: 0xA7940 }),
        SampVersion::R3 => Some(Adressen { enable: 0xAC700, aktiv: 0xAC7A0, beenden: 0xAC7F0 }),
        SampVersion::Dl => None,
    }
}

pub struct DriveBy {
    /// allowed weapon ids; empty = off
    waffen: Vec<u32>,
    /// we leaned out and the task has not shown up yet
    raus_seit: Option<Instant>,
    /// what the last frame found, for /kleidunginfo
    grund: &'static str,
    /// secondary fire does not shoot from the window (patch in CPad::GetWeapon is in)
    nur_hauptfeuer: bool,
    pub raus: u32,
    pub rein: u32,
}

impl DriveBy {
    pub fn new() -> DriveBy {
        DriveBy {
            waffen: Vec::new(),
            raus_seit: None,
            grund: "noch nicht geprüft",
            nur_hauptfeuer: false,
            raus: 0,
            rein: 0,
        }
    }

    pub fn zustand(&self) -> String {
        if self.waffen.is_empty() {
            return "aus".to_owned();
        }

        let waffen: Vec<String> = self.waffen.iter().map(u32::to_string).collect();
        format!(
            "an, Waffen {}, raus {}x, rein {}x, zuletzt: {}, schießen {}",
            waffen.join(" "),
            self.raus,
            self.rein,
            self.grund,
            if self.nur_hauptfeuer { "nur Hauptfeuer" } else { "auch Zweitfeuer" }
        )
    }

    // -----------------------------------------------------------------------
    // Server events

    pub fn setzen(&mut self, waffen: Vec<u32>) {
        if waffen.is_empty() {
            if !self.waffen.is_empty() {
                crate::log!("Drive-by beim Zielen aus");
            }
        } else {
            crate::log!("Drive-by beim Zielen an, Waffen {waffen:?}");
        }

        self.nur_hauptfeuer(!waffen.is_empty());
        self.waffen = waffen;
        self.raus_seit = None;
    }

    /// Game thread: switches the patch in CPad::GetWeapon, only over the original bytes
    fn nur_hauptfeuer(&mut self, an: bool) {
        if an == self.nur_hauptfeuer {
            return;
        }

        let (vorher, nachher) = if an { (ZWEITFEUER_AN, ZWEITFEUER_AUS) } else { (ZWEITFEUER_AUS, ZWEITFEUER_AN) };
        let jetzt = lese_u16(ZWEITFEUER_SPRUNG).map(u16::to_le_bytes);

        if jetzt != Some(vorher) {
            crate::log!(
                "Drive-by: 0x{ZWEITFEUER_SPRUNG:x} ist {:02X?} statt {vorher:02X?}, Zweitfeuer bleibt, wie es ist",
                jetzt
            );
            return;
        }

        if crate::diag::write_code(ZWEITFEUER_SPRUNG, &nachher) {
            self.nur_hauptfeuer = an;
            crate::log!("Drive-by: Zweitfeuer schießt {}", if an { "nicht mehr aus dem Fenster" } else { "wieder" });
        } else {
            crate::log!("Drive-by: 0x{ZWEITFEUER_SPRUNG:x} nicht beschreibbar");
        }
    }

    pub fn getrennt(&mut self) {
        self.setzen(Vec::new());
    }

    // -----------------------------------------------------------------------
    // Game thread, every frame

    pub fn frame(&mut self, ver: SampVersion) {
        if self.waffen.is_empty() {
            return;
        }

        let Some(adr) = adressen(ver) else {
            return;
        };

        let Some((samp_ped, flag)) = samp::local_driveby(ver) else {
            self.grund = "kein SA-MP-Spieler";
            return;
        };

        let Some(ped) = read_ptr(samp_ped + samp::SAMP_PED_GAME_PED).filter(|&p| p != 0) else {
            self.grund = "keine Spielfigur";
            return;
        };

        if !beifahrer(ped) {
            self.raus_seit = None;
            self.grund = "kein Beifahrer";
            return;
        }

        // pause menu, chat input, controls off: leave everything as it is
        if eingabe_gesperrt() {
            self.grund = "Eingabe gesperrt";
            return;
        }

        let zielt = lese_u8(MOUSE_RMB).is_some_and(|rmb| rmb != 0);
        let erlaubt = crate::ziehen::waffe_von(ped).is_some_and(|w| self.waffen.contains(&w));
        let raus = lese_u32(flag).is_some_and(|f| f != 0);
        let now = Instant::now();

        self.grund = match (zielt, erlaubt) {
            (_, false) => "Waffe nicht erlaubt",
            (false, true) => "zielt nicht",
            (true, true) => "zielt",
        };

        if !raus {
            self.raus_seit = None;

            if zielt && erlaubt && thiscall_bool(samp::base() + adr.enable, samp_ped) {
                schreibe_u32(flag, 1);
                self.raus_seit = Some(now);
                self.raus += 1;
            }

            return;
        }

        if !zielt || !erlaubt {
            thiscall(samp::base() + adr.beenden, samp_ped);
            schreibe_u32(flag, 0);
            self.raus_seit = None;
            self.rein += 1;
            return;
        }

        // still aiming and leaned out: does the task exist by now?
        if thiscall_bool(samp::base() + adr.aktiv, samp_ped) {
            self.raus_seit = None;
        } else if self.raus_seit.is_none_or(|seit| now.duration_since(seit) >= AUF_TASK_WARTEN) {
            // the drive-by ended without us (or never started): try again next frame
            schreibe_u32(flag, 0);
            self.raus_seit = None;
        }
    }
}

/// Weapon ids from `driveby:waffen`; an error if one is not a weapon id
pub fn parse_waffen<'a>(tokens: impl Iterator<Item = &'a str>) -> Result<Vec<u32>, String> {
    let mut waffen = Vec::new();

    for token in tokens {
        match token.parse::<u32>() {
            Ok(w) if (1..=WAFFE_MAX).contains(&w) => {
                if !waffen.contains(&w) {
                    waffen.push(w);
                }
            }
            _ => return Err(format!("ungültige Waffen-ID {token:?}")),
        }

        if waffen.len() > WAFFEN_MAX {
            return Err(format!("mehr als {WAFFEN_MAX} Waffen"));
        }
    }

    Ok(waffen)
}

/// Sits in a vehicle and does not drive it. An empty driver seat counts, as in SA-MP's
/// `CPed::IsAPassenger` (R1 0xA6880: passenger when `m_pDriver != ped`).
fn beifahrer(ped: usize) -> bool {
    let view = Ped(ped);

    if !view.valid() || !view.seated_in_vehicle() || view.dead() {
        return false;
    }

    let Some(vehicle) = read_ptr(ped + PED_VEHICLE).filter(|&v| v != 0) else {
        return false;
    };

    read_u32(vehicle + VEHICLE_DRIVER).is_some_and(|driver| driver as usize != ped)
}

fn eingabe_gesperrt() -> bool {
    lese_u8(MENU_ACTIVE).is_some_and(|m| m != 0)
        || lese_u16(PAD0_DISABLE_PLAYER_CONTROLS).is_some_and(|d| d != 0)
        || client_api::samp::inputs::Input::is_active()
}

fn thiscall_bool(func: usize, this: usize) -> bool {
    let f: extern "thiscall" fn(*mut c_void) -> i32 = unsafe { std::mem::transmute(func) };
    f(this as *mut c_void) != 0
}

fn thiscall(func: usize, this: usize) {
    let f: extern "thiscall" fn(*mut c_void) = unsafe { std::mem::transmute(func) };
    f(this as *mut c_void);
}

fn lese_u8(addr: usize) -> Option<u8> {
    readable(addr, 1).then(|| unsafe { (addr as *const u8).read() })
}

fn lese_u16(addr: usize) -> Option<u16> {
    readable(addr, 2).then(|| unsafe { (addr as *const u16).read_unaligned() })
}

fn lese_u32(addr: usize) -> Option<u32> {
    readable(addr, 4).then(|| unsafe { (addr as *const u32).read_unaligned() })
}

fn schreibe_u32(addr: usize, wert: u32) {
    if writable(addr, 4) {
        unsafe { (addr as *mut u32).write_unaligned(wert) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waffen_lesen() {
        assert_eq!(parse_waffen("22 23 25".split_whitespace()), Ok(vec![22, 23, 25]));
        assert_eq!(parse_waffen("".split_whitespace()), Ok(vec![]));
        assert_eq!(parse_waffen("31 31".split_whitespace()), Ok(vec![31]));
        assert!(parse_waffen("0".split_whitespace()).is_err());
        assert!(parse_waffen("47".split_whitespace()).is_err());
        assert!(parse_waffen("m4".split_whitespace()).is_err());
    }

    #[test]
    fn sprung_bleibt_im_ziel() {
        // je and jmp with the same distance land on the same return (0x540235)
        assert_eq!(ZWEITFEUER_AN[1], ZWEITFEUER_AUS[1]);
        assert_eq!(ZWEITFEUER_SPRUNG + 2 + ZWEITFEUER_AN[1] as usize, 0x540235);
    }

    #[test]
    fn pad_feld_liegt_im_pad() {
        assert!(PAD0_DISABLE_PLAYER_CONTROLS - 0xB73458 < 0x134);
    }
}
