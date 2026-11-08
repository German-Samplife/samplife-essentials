//! Drawing a weapon: when a ped on foot switches to a firearm (fist -> Desert Eagle,
//! Desert Eagle -> M4, ...), it plays `waffe_raus` from our block once as a partial
//! animation (upper body only, the legs keep walking), and the local player cannot fire
//! until it is over, like the draw animation known from FiveM.
//!
//! The server switches it on with `anim:ziehen <ms>` (netz.rs) after `kleidung:hallo` and
//! says how long drawing takes; `0` switches it off. The animation is sped up or slowed
//! down to exactly that time, so the duration lives only in the gamemode. Every client
//! watches the weapons of all streamed peds itself (the local one and the remote players,
//! whose weapon SA-MP sets from the sync), so the animation starts in the same frame as
//! the switch, without a round trip to the server. The server additionally drops shots
//! within that time (OnPlayerWeaponShot), for clients that do not block.
//!
//! Not counted: switches in a vehicle, while dead and in the first half second after
//! getting on foot (getting out of a vehicle, spawning), when SA-MP gives the weapon back.
//! Switching to something that is not a firearm fades the animation out and frees the
//! trigger again.
//!
//! Firing is blocked with GTA's own switch for scripts (SET_PLAYER_FIRE_BUTTON):
//! `CPad::bDisablePlayerFireWeapon` of pad 0, which `CPad::GetWeapon` (0x540180) checks.
//! Aiming stays possible.
//!
//! Layouts: gta-reversed (https://github.com/gta-reversed/gta-reversed) Pad.h
//! (`VALIDATE_SIZE(CPad, 0x134)`, `Pads` at 0xB73458; CControllerState 0x30 bytes, so
//! the bools after bApplyBrakes 0x118 are EnterCar 0x119, Duck 0x11A, FireWeapon 0x11B),
//! Entity/Ped/Ped.h and Weapon.h (`m_aWeapons[13]` at 0x5A0, CWeapon 0x1C with `m_Type`
//! first, `m_nActiveWeaponSlot` 0x718, same as plugin-sdk).

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::anim::{self, Anims, ANIM_BLEND_AUTO_REMOVE, ANIM_FINISH_AUTO_REMOVE, ANIM_PARTIAL, ANIM_PLAYING, ASSOC_SIZE};
use crate::game::{readable, writable, Ped};
use crate::samp::{self, SampVersion};

/// Name of the animation in `anim/samplife.ifp`
pub const ANIMATION: &str = "waffe_raus";

const PED_WEAPONS: usize = 0x5A0;
const WEAPON_SIZE: usize = 0x1C;
const WEAPON_SLOTS: usize = 13;
const PED_ACTIVE_SLOT: usize = 0x718;

const PAD0: usize = 0xB73458;
const PAD_DISABLE_FIRE_WEAPON: usize = 0x11B;

/// eWeaponType: 22 Colt 45 up to 34 sniper rifle (pistols, shotguns, SMGs, assault
/// rifles, rifles); not melee, heavy weapons, throwables and the rest
const SCHUSSWAFFE_ERSTE: u32 = 22;
const SCHUSSWAFFE_LETZTE: u32 = 34;

/// in over an eighth of a second; out over a quarter when done (CAnimBlendAssociation::UpdateTime, flag 0x8)
const BLEND_IN: f32 = 8.0;
const BLEND_OUT: f32 = -8.0;
/// after getting on foot, a different weapon is no draw yet
const RUHE_ZU_FUSS: Duration = Duration::from_millis(500);

/// what the server may send for `anim:ziehen` besides 0
pub const DAUER_MS: std::ops::RangeInclusive<u32> = 100..=3000;

pub fn ist_schusswaffe(waffe: u32) -> bool {
    (SCHUSSWAFFE_ERSTE..=SCHUSSWAFFE_LETZTE).contains(&waffe)
}

struct Stand {
    /// SA-MP player id the ped belonged to (a ped can be reused for another player)
    id: u16,
    waffe: u32,
    zu_fuss_seit: Option<Instant>,
}

pub struct Ziehen {
    dauer: Option<Duration>,
    peds: HashMap<usize, Stand>,
    sperre_bis: Option<Instant>,
    /// we set bDisablePlayerFireWeapon and have to clear it again
    gesperrt: bool,
    pub gestartet: u32,
    pub eigene: u32,
}

impl Ziehen {
    pub fn new() -> Ziehen {
        Ziehen {
            dauer: None,
            peds: HashMap::new(),
            sperre_bis: None,
            gesperrt: false,
            gestartet: 0,
            eigene: 0,
        }
    }

    pub fn zustand(&self) -> String {
        match self.dauer {
            None => "aus".to_owned(),
            Some(dauer) => format!(
                "an, {} ms, gespielt {}x (eigene {}x), Schießen {}",
                dauer.as_millis(),
                self.gestartet,
                self.eigene,
                if self.gesperrt { "gesperrt" } else { "frei" }
            ),
        }
    }

    // -----------------------------------------------------------------------
    // Server events

    pub fn setzen(&mut self, ms: u32) {
        if ms == 0 {
            if self.dauer.take().is_some() {
                crate::log!("Waffe ziehen aus");
            }
            self.entsperren();
            self.peds.clear();
            return;
        }

        crate::log!("Waffe ziehen an, {ms} ms");
        self.dauer = Some(Duration::from_millis(u64::from(ms)));
    }

    pub fn getrennt(&mut self) {
        self.setzen(0);
    }

    // -----------------------------------------------------------------------
    // Game thread, every frame (after Anims::frame, which loads the block)

    pub fn frame(&mut self, ver: SampVersion, anims: &Anims) {
        let Some(dauer) = self.dauer else {
            return;
        };

        let now = Instant::now();
        let hier = anims.hier(ANIMATION);
        let lokal = samp::local_ped(ver);

        let mut peds = samp::remote_peds();
        if let Some(ped) = lokal {
            peds.push((samp::local_id().unwrap_or(u16::MAX), ped));
        }

        let mut gesehen = Vec::with_capacity(peds.len());

        for (id, ped) in peds {
            let view = Ped(ped);

            if !view.valid() {
                continue;
            }

            let Some(waffe) = waffe_von(ped) else {
                continue;
            };

            gesehen.push(ped);

            let zu_fuss = !view.in_vehicle() && !view.dead();
            let alt = self.peds.get(&ped).filter(|stand| stand.id == id);
            let zu_fuss_seit = if zu_fuss { alt.and_then(|stand| stand.zu_fuss_seit).or(Some(now)) } else { None };
            let gewechselt = alt.is_some_and(|stand| stand.waffe != waffe);
            let ruhig = zu_fuss_seit.is_some_and(|seit| now.duration_since(seit) >= RUHE_ZU_FUSS);
            let eigen = lokal == Some(ped);

            if gewechselt && ruhig {
                if ist_schusswaffe(waffe) {
                    if let Some(hier) = hier {
                        self.abspielen(view, hier, dauer);
                    }

                    if eigen {
                        self.sperre_bis = Some(now + dauer);
                        self.eigene += 1;
                    }
                } else {
                    if let Some(hier) = hier {
                        ausblenden(view, hier);
                    }

                    if eigen {
                        self.entsperren();
                    }
                }
            }

            self.peds.insert(ped, Stand { id, waffe, zu_fuss_seit });
        }

        self.peds.retain(|ped, _| gesehen.contains(ped));

        if let Some(bis) = self.sperre_bis {
            let lokal_zu_fuss = lokal.map(Ped).is_some_and(|view| view.valid() && !view.in_vehicle() && !view.dead());

            if now >= bis || !lokal_zu_fuss {
                self.entsperren();
            } else {
                // every frame: CPad::Clear resets it, e.g. after the pause menu
                feuer_sperre(true);
                self.gesperrt = true;
            }
        }
    }

    fn abspielen(&mut self, view: Ped, hier: usize, dauer: Duration) {
        let clump = view.rw_object();

        if clump == 0 || !readable(clump, 0x30) {
            return;
        }

        // still running from the last switch (within the time plus fading out): let it end,
        // BlendAnimation would only hand back the old one
        if anim::find_assoc(clump, |h| h == hier).is_some() {
            return;
        }

        // poses fade out (anim.rs waits for the end), the talking face stays
        let lippen = crate::lippen::hier();
        let assoc = anim::blend_schonend(
            clump,
            hier,
            ANIM_PLAYING | ANIM_BLEND_AUTO_REMOVE | ANIM_FINISH_AUTO_REMOVE | ANIM_PARTIAL,
            BLEND_IN,
            |h| lippen != 0 && h == lippen,
        );

        if assoc == 0 || !writable(assoc, ASSOC_SIZE) {
            return;
        }

        let laenge = anim::gesamtzeit(hier);

        if laenge > 0.0 {
            anim::geschwindigkeit_setzen(assoc, laenge / dauer.as_secs_f32());
        }

        self.gestartet += 1;
    }

    fn entsperren(&mut self) {
        self.sperre_bis = None;

        if self.gesperrt {
            feuer_sperre(false);
            self.gesperrt = false;
        }
    }
}

/// eWeaponType in the active slot of a ped (also driveby.rs)
pub fn waffe_von(ped: usize) -> Option<u32> {
    if !readable(ped + PED_ACTIVE_SLOT, 1) {
        return None;
    }

    let slot = unsafe { *((ped + PED_ACTIVE_SLOT) as *const u8) } as usize;

    if slot >= WEAPON_SLOTS {
        return None;
    }

    crate::game::read_u32(ped + PED_WEAPONS + slot * WEAPON_SIZE)
}

fn ausblenden(view: Ped, hier: usize) {
    let clump = view.rw_object();

    if clump == 0 || !readable(clump, 0x30) {
        return;
    }

    if let Some(assoc) = anim::find_assoc(clump, |h| h == hier) {
        if anim::blend_delta(assoc) >= 0.0 && writable(assoc, ASSOC_SIZE) {
            anim::ausblenden_assoc(assoc, BLEND_OUT);
        }
    }
}

fn feuer_sperre(an: bool) {
    let addr = PAD0 + PAD_DISABLE_FIRE_WEAPON;

    if writable(addr, 1) {
        unsafe { (addr as *mut u8).write(u8::from(an)) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schusswaffen() {
        assert!(!ist_schusswaffe(0)); // Faust
        assert!(!ist_schusswaffe(4)); // Messer
        assert!(ist_schusswaffe(22)); // Colt 45
        assert!(ist_schusswaffe(24)); // Desert Eagle
        assert!(ist_schusswaffe(31)); // M4
        assert!(ist_schusswaffe(34)); // Scharfschützengewehr
        assert!(!ist_schusswaffe(35)); // RPG
        assert!(!ist_schusswaffe(41)); // Spraydose
    }

    #[test]
    fn pad_bool_liegt_im_pad() {
        assert!(PAD_DISABLE_FIRE_WEAPON < 0x134);
        assert_eq!(PED_WEAPONS + WEAPON_SLOTS * WEAPON_SIZE, 0x70C);
    }
}
