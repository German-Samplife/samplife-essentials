//! Lips: the face of a player's ped talks while he speaks in the voice chat
//! (`lippen:an <id>` until `lippen:aus <id>`, netz.rs). The server decides: talk key
//! held and speech at the microphone (mikro.rs on the speaker's client).
//!
//! Played is GTA's own talking face `factalk` from ped.ifp (block `ped`, always loaded),
//! the one peds use when they speak in the game, as a looped partial animation: it moves
//! only the face bones, so walking, driving and poses go on. Unlike the poses it also
//! plays in vehicles. Starting it would fade out every other partial animation of the
//! ped (BlendAnimation); `anim::blend_schonend` gives them their state back, and the
//! poses do the same for the lips. Like the poses the plugin remembers who speaks, also
//! out of range, and plays it again when the ped comes back.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::anim::{self, ANIM_BLEND_AUTO_REMOVE, ANIM_LOOPED, ANIM_PARTIAL, ANIM_PLAYING, ASSOC_BLEND_DELTA, ASSOC_FLAGS, ASSOC_SIZE};
use crate::game::{readable, writable, Ped};
use crate::samp::SampVersion;

const BLOCK: &str = "ped";
const ANIMATION: &str = "factalk";

/// mouth opens and closes in an eighth of a second
const BLEND_IN: f32 = 8.0;
const BLEND_OUT: f32 = -8.0;
const CHECK_INTERVAL: Duration = Duration::from_millis(250);
/// faded out by another partial animation of the game: back at most once a second
const RETRY_INTERVAL: Duration = Duration::from_millis(1000);

/// hierarchy of `factalk` once found, read by anim.rs so poses don't fade it out
static HIER: AtomicUsize = AtomicUsize::new(0);

pub fn hier() -> usize {
    HIER.load(Ordering::Relaxed)
}

enum Laden {
    Offen,
    Fertig(usize),
    Fehler,
}

pub struct Lippen {
    laden: Laden,
    /// SA-MP player ids whose lips move
    sprechen: HashSet<u16>,
    /// ped -> when its lips were last (re)started
    zuletzt: HashMap<usize, Instant>,
    last_check: Option<Instant>,
    pub gestartet: u32,
}

impl Lippen {
    pub fn new() -> Lippen {
        Lippen {
            laden: Laden::Offen,
            sprechen: HashSet::new(),
            zuletzt: HashMap::new(),
            last_check: None,
            gestartet: 0,
        }
    }

    pub fn zustand(&self) -> String {
        let laden = match self.laden {
            Laden::Offen => "noch nicht geladen",
            Laden::Fertig(_) => "bereit",
            Laden::Fehler => "nicht gefunden (Log)",
        };

        format!("{BLOCK}/{ANIMATION} {laden}, sprechen gerade {}, gestartet {}x", self.sprechen.len(), self.gestartet)
    }

    // -----------------------------------------------------------------------
    // Server events

    pub fn an(&mut self, ver: SampVersion, id: u16) {
        if !self.sprechen.insert(id) {
            return;
        }

        if let Some(ped) = anim::ped_of(ver, id) {
            self.zuletzt.remove(&ped);
        }

        // start right away, not at the next check
        self.last_check = None;
    }

    pub fn aus(&mut self, ver: SampVersion, id: u16) {
        self.sprechen.remove(&id);

        if let Some(ped) = anim::ped_of(ver, id) {
            self.ausblenden(ped);
            self.zuletzt.remove(&ped);
        }
    }

    pub fn getrennt(&mut self, ver: SampVersion) {
        let ids: Vec<u16> = self.sprechen.iter().copied().collect();

        for id in ids {
            self.aus(ver, id);
        }

        self.zuletzt.clear();
    }

    // -----------------------------------------------------------------------
    // Game thread, every frame

    pub fn frame(&mut self, ver: SampVersion) {
        if self.sprechen.is_empty() {
            return;
        }

        if matches!(self.laden, Laden::Offen) {
            self.laden = match anim::animation_im_block(BLOCK, ANIMATION) {
                Ok(hier) => {
                    crate::log!("Lippen: {BLOCK}/{ANIMATION} gefunden");
                    HIER.store(hier, Ordering::Relaxed);
                    Laden::Fertig(hier)
                }
                Err(grund) => {
                    crate::log!("Lippen: {grund}. Lippen bleiben aus.");
                    Laden::Fehler
                }
            };
        }

        let Laden::Fertig(hier) = self.laden else {
            return;
        };

        let now = Instant::now();

        if self.last_check.is_some_and(|last| now.duration_since(last) < CHECK_INTERVAL) {
            return;
        }

        self.last_check = Some(now);
        let mut gesehen = Vec::new();

        for &id in &self.sprechen {
            let Some(ped) = anim::ped_of(ver, id) else {
                continue;
            };

            gesehen.push(ped);
            let view = Ped(ped);

            if !view.valid() || view.dead() {
                continue;
            }

            let clump = view.rw_object();

            if clump == 0 || !readable(clump, 0x30) {
                continue;
            }

            match anim::find_assoc(clump, |h| h == hier) {
                Some(assoc) if anim::blend_delta(assoc) >= 0.0 => {}

                Some(_) if self.zuletzt.get(&ped).is_some_and(|t| now.duration_since(*t) < RETRY_INTERVAL) => {}

                _ => {
                    // every other partial animation (poses, GTA's own) keeps running
                    anim::blend_schonend(clump, hier, ANIM_PLAYING | ANIM_LOOPED | ANIM_PARTIAL, BLEND_IN, |_| true);
                    self.zuletzt.insert(ped, now);
                    self.gestartet += 1;
                }
            }
        }

        self.zuletzt.retain(|ped, _| gesehen.contains(ped));
    }

    fn ausblenden(&self, ped: usize) {
        let Laden::Fertig(hier) = self.laden else {
            return;
        };

        let view = Ped(ped);

        if !view.valid() {
            return;
        }

        let clump = view.rw_object();

        if clump == 0 || !readable(clump, 0x30) {
            return;
        }

        if let Some(a) = anim::find_assoc(clump, |h| h == hier) {
            if writable(a, ASSOC_SIZE) {
                unsafe {
                    ((a + ASSOC_BLEND_DELTA) as *mut f32).write_unaligned(BLEND_OUT);
                    let flags = ((a + ASSOC_FLAGS) as *const u16).read_unaligned();
                    ((a + ASSOC_FLAGS) as *mut u16).write_unaligned(flags | ANIM_BLEND_AUTO_REMOVE);
                }
            }
        }
    }
}
