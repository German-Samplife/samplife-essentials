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
//! While it is on, the right button only leans out and aims, and the left mouse button
//! shoots, whatever the vehicle keys are set to. `CPad::GetWeapon` (0x540180) decides if a
//! drive-by shoots; in a drive-by task (or attached to something) it returns
//! ButtonCircle ("vehicle fire") plus LeftShoulder1 ("secondary fire", often the right
//! button): `test bl, bl; movsx eax, [esi+0x22]; je +0x1A; movsx ecx, [esi+8]; pop edi;
//! pop ebx; add eax, ecx; pop esi; ret 4`. The plugin puts a `jmp` to its own code over
//! `movsx ecx, [esi+8]; pop edi` at 0x54021B. That code drops LeftShoulder1 and, only when
//! the ped (edi, the argument) is our own (`CWorld::Players[0].m_pPed`, 0xB7CD98), adds the
//! left mouse button (`CPad::NewMouseControllerState.lmb`, 0xB73418). Not by pad: SA-MP
//! writes the keys of remote players into pad 0 while it processes them. Put back when the
//! server switches it off or the connection ends.
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

/// `movsx ecx, word ptr [esi+8]; pop edi` in CPad::GetWeapon, only reached in a drive-by
const FEUER_STELLE: usize = 0x54021B;
const FEUER_ORIGINAL: [u8; 5] = [0x0F, 0xBF, 0x4E, 0x08, 0x5F];
/// CWorld::Players[0].m_pPed, the local player (FindPlayerPed 0x56E210)
const EIGENE_FIGUR: u32 = 0xB7CD98;
const MOUSE_LMB: u32 = 0xB73418;

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
    /// the jump in CPad::GetWeapon is in: left button shoots, secondary fire does not
    linksklick: bool,
    /// our code behind the jump, allocated once and kept
    hoehle: usize,
    pub raus: u32,
    pub rein: u32,
}

impl DriveBy {
    pub fn new() -> DriveBy {
        DriveBy {
            waffen: Vec::new(),
            raus_seit: None,
            grund: "noch nicht geprüft",
            linksklick: false,
            hoehle: 0,
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
            if self.linksklick { "Linksklick und Fire-Taste" } else { "wie in GTA eingestellt" }
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

        self.linksklick(!waffen.is_empty());
        self.waffen = waffen;
        self.raus_seit = None;
    }

    /// Game thread: switches the jump in CPad::GetWeapon, only over the original bytes
    fn linksklick(&mut self, an: bool) {
        if an == self.linksklick {
            return;
        }

        if an && self.hoehle == 0 {
            self.hoehle = hoehle_anlegen();

            if self.hoehle == 0 {
                crate::log!("Drive-by: kein Speicher für den Linksklick, Schießen bleibt, wie es ist");
                return;
            }
        }

        let sprung = sprung_zu(self.hoehle);
        let (vorher, nachher) = if an { (FEUER_ORIGINAL, sprung) } else { (sprung, FEUER_ORIGINAL) };
        let jetzt = readable(FEUER_STELLE, 5).then(|| unsafe { (FEUER_STELLE as *const [u8; 5]).read_unaligned() });

        if jetzt != Some(vorher) {
            crate::log!("Drive-by: 0x{FEUER_STELLE:x} ist {jetzt:02X?} statt {vorher:02X?}, Schießen bleibt, wie es ist");
            return;
        }

        if crate::diag::write_code(FEUER_STELLE, &nachher) {
            self.linksklick = an;
            crate::log!(
                "Drive-by: {}",
                if an { "Linksklick schießt aus dem Fenster, Zweitfeuer nicht" } else { "Schießen wieder wie in GTA eingestellt" }
            );
        } else {
            crate::log!("Drive-by: 0x{FEUER_STELLE:x} nicht beschreibbar");
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

/// Code behind the jump: eax holds ButtonCircle, edi/ebx/esi are still pushed.
/// `cmp edi, [EIGENE_FIGUR]; jne +6; or al, [MOUSE_LMB]; pop edi; pop ebx; pop esi; ret 4`
fn hoehle_code() -> [u8; 20] {
    let mut c = [0u8; 20];
    c[0..2].copy_from_slice(&[0x3B, 0x3D]);
    c[2..6].copy_from_slice(&EIGENE_FIGUR.to_le_bytes());
    c[6..8].copy_from_slice(&[0x75, 0x06]);
    c[8..10].copy_from_slice(&[0x0A, 0x05]);
    c[10..14].copy_from_slice(&MOUSE_LMB.to_le_bytes());
    c[14..20].copy_from_slice(&[0x5F, 0x5B, 0x5E, 0xC2, 0x04, 0x00]);
    c
}

/// `jmp hoehle` written at FEUER_STELLE
fn sprung_zu(hoehle: usize) -> [u8; 5] {
    let rel = (hoehle as u32).wrapping_sub(FEUER_STELLE as u32 + 5);
    let mut b = [0xE9, 0, 0, 0, 0];
    b[1..].copy_from_slice(&rel.to_le_bytes());
    b
}

fn hoehle_anlegen() -> usize {
    use winapi::um::memoryapi::VirtualAlloc;
    use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE};

    let code = hoehle_code();
    let ziel = unsafe { VirtualAlloc(std::ptr::null_mut(), code.len(), MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE) };

    if ziel.is_null() {
        return 0;
    }

    unsafe { std::ptr::copy_nonoverlapping(code.as_ptr(), ziel as *mut u8, code.len()) };
    ziel as usize
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
    fn sprung_landet_in_der_hoehle() {
        let hoehle = 0x1234_5000usize;
        let b = sprung_zu(hoehle);
        assert_eq!(b[0], 0xE9);
        let rel = u32::from_le_bytes([b[1], b[2], b[3], b[4]]);
        assert_eq!((FEUER_STELLE as u32 + 5).wrapping_add(rel), hoehle as u32);
        // also backwards
        let b = sprung_zu(0x1000_0000);
        let rel = u32::from_le_bytes([b[1], b[2], b[3], b[4]]);
        assert_eq!((FEUER_STELLE as u32 + 5).wrapping_add(rel), 0x1000_0000);
    }

    #[test]
    fn hoehle_wie_geplant() {
        let c = hoehle_code();
        assert_eq!(&c[..], &[
            0x3B, 0x3D, 0x98, 0xCD, 0xB7, 0x00, // cmp edi, [0xB7CD98]
            0x75, 0x06, // jne +6 (over the or)
            0x0A, 0x05, 0x18, 0x34, 0xB7, 0x00, // or al, [0xB73418]
            0x5F, 0x5B, 0x5E, 0xC2, 0x04, 0x00, // pop edi; pop ebx; pop esi; ret 4
        ]);
    }

    #[test]
    fn pad_feld_liegt_im_pad() {
        assert!(PAD0_DISABLE_PLAYER_CONTROLS - 0xB73458 < 0x134);
    }
}
