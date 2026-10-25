//! Partial animations (upper body only) per player, as the server says (netz.rs, `anim:*`).
//!
//! The plugin carries its own IFP (`anim/samplife.ifp`, block `samplife`, built with
//! `anim/posen_bauen.py`) inside the DLL and loads it once from memory into GTA's
//! animation manager. `anim:start <id> <name>` plays `name` from that block on the
//! player's ped as a looped partial animation: only the bones in the file move (spine
//! and up), so the legs keep following the walk animation, like GTA's own smoking and
//! drinking anims. The plugin remembers every pose, also of players who are not
//! streamed, and plays it again when the ped comes back (streaming, respawn, getting
//! out of a vehicle) until `anim:stopp <id>`.
//!
//! Addresses: GTA San Andreas 1.0 US, from gta-reversed
//! (https://github.com/gta-reversed/gta-reversed), `RH_ScopedInstall` lines in
//! Animation/AnimManager.cpp, Plugins/RpAnimBlendPlugin/RpAnimBlend.cpp and
//! RenderWare/rw/rwplcore.cpp; layouts from Animation/AnimBlock.h and
//! Animation/AnimBlendAssociation.h (VALIDATE_SIZE).

use std::collections::HashMap;
use std::ffi::{c_char, c_void, CString};
use std::time::{Duration, Instant};

use crate::game::{read_ptr, read_u32, readable, writable, Ped};
use crate::samp::{self, SampVersion};

/// Block name inside `anim/samplife.ifp` (CAnimBlock::Name has 16 bytes)
pub const BLOCK: &str = "samplife";
static IFP: &[u8] = include_bytes!("../anim/samplife.ifp");

// RenderWare streams (rwplcore.cpp): RwStreamOpen 0x7ECEF0, RwStreamClose 0x7ECE20;
// RwStreamType rwSTREAMMEMORY = 3, RwStreamAccessType rwSTREAMREAD = 1
const RW_STREAM_OPEN: usize = 0x7ECEF0;
const RW_STREAM_CLOSE: usize = 0x7ECE20;
const RW_STREAM_MEMORY: u32 = 3;
const RW_STREAM_READ: u32 = 1;

// AnimManager.cpp: LoadAnimFile(RwStream*, bool, const char(*)[32]) 0x4D47F0,
// GetAnimationBlock(const char*) 0x4D3940, GetAnimationBlockIndex(const char*) 0x4D3990,
// AddAnimBlockRef(int32) 0x4D3FB0, GetAnimation(const char*, const CAnimBlock*) 0x4D42F0,
// BlendAnimation(RpClump*, CAnimBlendHierarchy*, int32 flags, float delta) 0x4D4410
const LOAD_ANIM_FILE: usize = 0x4D47F0;
const GET_ANIMATION_BLOCK: usize = 0x4D3940;
const GET_ANIMATION_BLOCK_INDEX: usize = 0x4D3990;
const ADD_ANIM_BLOCK_REF: usize = 0x4D3FB0;
const GET_ANIMATION: usize = 0x4D42F0;
const BLEND_ANIMATION: usize = 0x4D4410;

// AnimManager.h: ms_numAnimations 0xB4EA2C (array of 2500), ms_numAnimBlocks 0xB4EA30
// (NUM_ANIM_BLOCKS = 180). Only checked before loading; limit adjusters may allow more,
// then the plugin still stays below the original limits.
const NUM_ANIMATIONS: usize = 0xB4EA2C;
const NUM_ANIM_BLOCKS: usize = 0xB4EA30;
const MAX_ANIMATIONS: u32 = 2500;
const MAX_ANIM_BLOCKS: u32 = 180;

// RpAnimBlend.cpp: RpAnimBlendClumpGetFirstAssociation(RpClump*) 0x4D15E0,
// RpAnimBlendGetNextAssociation(CAnimBlendAssociation*) 0x4D6AB0
const CLUMP_FIRST_ASSOC: usize = 0x4D15E0;
const NEXT_ASSOC: usize = 0x4D6AB0;

// CAnimBlock (0x20): Name[16], IsLoaded at 0x10
const BLOCK_IS_LOADED: usize = 0x10;
// CAnimBlendAssociation (0x3C, vtable first): m_BlendHier 0x14, m_BlendAmount 0x18,
// m_BlendDelta 0x1C, m_Flags (uint16) 0x2E
const ASSOC_SIZE: usize = 0x3C;
const ASSOC_HIER: usize = 0x14;
const ASSOC_BLEND_DELTA: usize = 0x1C;
const ASSOC_FLAGS: usize = 0x2E;

// eAnimationFlags (AnimBlendAssociation.h)
const ANIM_PLAYING: u16 = 0x1;
const ANIM_LOOPED: u16 = 0x2;
const ANIM_BLEND_AUTO_REMOVE: u16 = 0x4;
const ANIM_PARTIAL: u16 = 0x10;

/// blend in/out over a quarter second
const BLEND_IN: f32 = 4.0;
const BLEND_OUT: f32 = -4.0;
const CHECK_INTERVAL: Duration = Duration::from_millis(250);
/// a pose another partial anim faded out comes back at most once a second per ped
const RETRY_INTERVAL: Duration = Duration::from_millis(1000);

/// Animation names in the embedded IFP (ANP3: "ANP3", size, block[24], count, then per
/// animation name[24], sequences, frame bytes, flags and its sequences).
pub fn namen_in_ifp(data: &[u8]) -> Result<(String, Vec<String>), String> {
    let u32_at = |pos: usize| -> Result<u32, String> {
        data.get(pos..pos + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| format!("IFP zu kurz bei {pos}"))
    };
    let name_at = |pos: usize| -> Result<String, String> {
        let raw = data.get(pos..pos + 24).ok_or_else(|| format!("IFP zu kurz bei {pos}"))?;
        let len = raw.iter().position(|&b| b == 0).unwrap_or(24);
        Ok(String::from_utf8_lossy(&raw[..len]).to_ascii_lowercase())
    };

    if data.get(..4) != Some(b"ANP3".as_slice()) {
        return Err("keine ANP3-Datei".to_owned());
    }

    let block = name_at(8)?;
    let count = u32_at(32)? as usize;
    let mut pos = 36;
    let mut names = Vec::with_capacity(count);

    for _ in 0..count {
        names.push(name_at(pos)?);
        let sequences = u32_at(pos + 24)? as usize;
        pos += 36;

        for _ in 0..sequences {
            let frame_type = u32_at(pos + 24)?;
            let frames = u32_at(pos + 28)? as usize;
            let size = match frame_type {
                1 => 20,
                2 => 32,
                3 => 10,
                4 => 16,
                other => return Err(format!("unbekannter Keyframe-Typ {other}")),
            };
            pos += 36 + frames * size;
        }
    }

    if pos > data.len() {
        return Err("IFP abgeschnitten".to_owned());
    }

    Ok((block, names))
}

/// Same check as netz.rs for `anim:start`: what posen_bauen.py allows.
pub fn gueltiger_name(name: &str) -> bool {
    (1..=23).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

#[repr(C)]
struct RwMemory {
    start: *const u8,
    length: u32,
}

enum Laden {
    Offen,
    Fertig(HashMap<String, usize>),
    Fehler,
}

pub struct Anims {
    laden: Laden,
    /// SA-MP player id -> animation name
    posen: HashMap<u16, String>,
    /// ped -> when its pose was last (re)started
    zuletzt: HashMap<usize, Instant>,
    last_check: Option<Instant>,
    pub gestartet: u32,
}

impl Anims {
    pub fn new() -> Anims {
        Anims {
            laden: Laden::Offen,
            posen: HashMap::new(),
            zuletzt: HashMap::new(),
            last_check: None,
            gestartet: 0,
        }
    }

    pub fn aktive_posen(&self) -> usize {
        self.posen.len()
    }

    pub fn zustand(&self) -> String {
        match &self.laden {
            Laden::Offen => "noch nicht geladen".to_owned(),
            Laden::Fehler => "Laden fehlgeschlagen (Log)".to_owned(),
            Laden::Fertig(hiers) => {
                let mut namen: Vec<&str> = hiers.keys().map(String::as_str).collect();
                namen.sort_unstable();
                format!("{} Posen ({})", hiers.len(), namen.join(", "))
            }
        }
    }

    // -----------------------------------------------------------------------
    // Server events

    pub fn start(&mut self, ver: SampVersion, id: u16, name: String) {
        if self.posen.get(&id) == Some(&name) {
            return;
        }

        crate::log!("Pose Spieler-ID {id}: {name}");
        self.posen.insert(id, name);

        // the new pose fades out the old one (both partial), see BlendAnimation
        if let Some(ped) = ped_of(ver, id) {
            self.zuletzt.remove(&ped);
        }
        self.last_check = None;
    }

    pub fn stopp(&mut self, ver: SampVersion, id: u16) {
        if self.posen.remove(&id).is_some() {
            crate::log!("Pose Spieler-ID {id} beendet");
        }

        if let Some(ped) = ped_of(ver, id) {
            self.ausblenden(ped);
            self.zuletzt.remove(&ped);
        }
    }

    pub fn getrennt(&mut self, ver: SampVersion) {
        let ids: Vec<u16> = self.posen.keys().copied().collect();

        for id in ids {
            self.stopp(ver, id);
        }

        self.zuletzt.clear();
    }

    // -----------------------------------------------------------------------
    // Game thread, every frame

    pub fn frame(&mut self, ver: SampVersion) {
        if matches!(self.laden, Laden::Offen) {
            self.laden = match laden() {
                Ok(hiers) => Laden::Fertig(hiers),
                Err(grund) => {
                    crate::log!("Posen: {grund}. Teilanimationen bleiben aus.");
                    Laden::Fehler
                }
            };
        }

        if self.posen.is_empty() {
            return;
        }

        let now = Instant::now();

        if self.last_check.is_some_and(|last| now.duration_since(last) < CHECK_INTERVAL) {
            return;
        }

        self.last_check = Some(now);

        let Laden::Fertig(hiers) = &self.laden else {
            return;
        };

        let mut gesehen = Vec::new();

        for (&id, name) in &self.posen {
            let Some(&hier) = hiers.get(name) else {
                continue;
            };
            let Some(ped) = ped_of(ver, id) else {
                continue;
            };

            gesehen.push(ped);
            let view = Ped(ped);

            if !view.valid() || view.in_vehicle() || view.dead() {
                continue;
            }

            let clump = view.rw_object();

            if clump == 0 || !readable(clump, 0x30) {
                continue;
            }

            match find_assoc(clump, |h| h == hier) {
                // running and not fading out: nothing to do
                Some(assoc) if blend_delta(assoc) >= 0.0 => {}

                // faded out by another partial anim of the game: back at most once a second
                Some(_) if self.zuletzt.get(&ped).is_some_and(|t| now.duration_since(*t) < RETRY_INTERVAL) => {}

                _ => {
                    blend(clump, hier);
                    self.zuletzt.insert(ped, now);
                    self.gestartet += 1;
                }
            }
        }

        self.zuletzt.retain(|ped, _| gesehen.contains(ped));
    }

    /// Fades out every animation of our block on this ped.
    fn ausblenden(&self, ped: usize) {
        let Laden::Fertig(hiers) = &self.laden else {
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

        // one at a time: the list changes only when the game deletes faded-out anims
        let mut assoc = first_assoc(clump);

        while let Some(a) = assoc {
            let hier = read_ptr(a + ASSOC_HIER).unwrap_or(0);

            if hiers.values().any(|&h| h == hier) && writable(a, ASSOC_SIZE) {
                unsafe {
                    ((a + ASSOC_BLEND_DELTA) as *mut f32).write_unaligned(BLEND_OUT);
                    let flags = ((a + ASSOC_FLAGS) as *const u16).read_unaligned();
                    ((a + ASSOC_FLAGS) as *mut u16).write_unaligned(flags | ANIM_BLEND_AUTO_REMOVE);
                }
            }

            assoc = next_assoc(a);
        }
    }
}

/// GTA ped of a SA-MP player id, local or remote; None if not streamed.
fn ped_of(ver: SampVersion, id: u16) -> Option<usize> {
    if samp::local_id() == Some(id) {
        return samp::local_ped(ver);
    }

    samp::remote_peds().into_iter().find(|&(rid, _)| rid == id).map(|(_, ped)| ped)
}

/// Loads the embedded IFP once and looks up every animation in it.
fn laden() -> Result<HashMap<String, usize>, String> {
    let (block, namen) = namen_in_ifp(IFP)?;

    if block != BLOCK {
        return Err(format!("eingebettete IFP hat Block {block}, erwartet {BLOCK}"));
    }

    let block_name = CString::new(BLOCK).map_err(|e| e.to_string())?;

    unsafe {
        let get_block: extern "C" fn(*const c_char) -> usize = std::mem::transmute(GET_ANIMATION_BLOCK);
        let get_block_index: extern "C" fn(*const c_char) -> i32 = std::mem::transmute(GET_ANIMATION_BLOCK_INDEX);
        let add_ref: extern "C" fn(i32) = std::mem::transmute(ADD_ANIM_BLOCK_REF);
        let get_animation: extern "C" fn(*const c_char, usize) -> usize = std::mem::transmute(GET_ANIMATION);

        let mut block_ptr = get_block(block_name.as_ptr());
        let schon_da = block_ptr != 0 && readable(block_ptr + BLOCK_IS_LOADED, 1) && *((block_ptr + BLOCK_IS_LOADED) as *const u8) != 0;

        if !schon_da {
            let anims = read_u32(NUM_ANIMATIONS).unwrap_or(u32::MAX);
            let blocks = read_u32(NUM_ANIM_BLOCKS).unwrap_or(u32::MAX);

            if blocks >= MAX_ANIM_BLOCKS || anims.saturating_add(namen.len() as u32) > MAX_ANIMATIONS {
                return Err(format!("kein Platz: {blocks} Animationsblöcke, {anims} Animationen"));
            }

            let open: extern "C" fn(u32, u32, *const c_void) -> usize = std::mem::transmute(RW_STREAM_OPEN);
            let close: extern "C" fn(usize, *mut c_void) -> i32 = std::mem::transmute(RW_STREAM_CLOSE);
            let load: extern "C" fn(usize, bool, *const c_void) = std::mem::transmute(LOAD_ANIM_FILE);

            let memory = RwMemory {
                start: IFP.as_ptr(),
                length: IFP.len() as u32,
            };
            let stream = open(RW_STREAM_MEMORY, RW_STREAM_READ, &memory as *const RwMemory as *const c_void);

            if stream == 0 {
                return Err("RwStreamOpen für die eingebettete IFP fehlgeschlagen".to_owned());
            }

            load(stream, true, std::ptr::null());
            close(stream, std::ptr::null_mut());

            block_ptr = get_block(block_name.as_ptr());

            if block_ptr == 0 {
                return Err("Block nach dem Laden nicht gefunden".to_owned());
            }

            // keep the block: nothing streams it out while a reference is held
            add_ref(get_block_index(block_name.as_ptr()));

            crate::log!(
                "Posen geladen: Block {BLOCK} ({} Bytes), vorher {blocks} Blöcke und {anims} Animationen",
                IFP.len()
            );
        }

        let mut hiers = HashMap::new();

        for name in namen {
            let c_name = CString::new(name.as_str()).map_err(|e| e.to_string())?;
            let hier = get_animation(c_name.as_ptr(), block_ptr);

            if hier == 0 {
                return Err(format!("Animation {name} nicht im Block {BLOCK}"));
            }

            hiers.insert(name, hier);
        }

        crate::log!("Posen bereit: {}", hiers.keys().cloned().collect::<Vec<_>>().join(", "));
        Ok(hiers)
    }
}

fn blend(clump: usize, hier: usize) {
    let blend: extern "C" fn(usize, usize, i32, f32) -> usize = unsafe { std::mem::transmute(BLEND_ANIMATION) };
    blend(clump, hier, (ANIM_PLAYING | ANIM_LOOPED | ANIM_PARTIAL) as i32, BLEND_IN);
}

fn first_assoc(clump: usize) -> Option<usize> {
    let first: extern "C" fn(usize) -> usize = unsafe { std::mem::transmute(CLUMP_FIRST_ASSOC) };
    Some(first(clump)).filter(|&a| a != 0 && readable(a, ASSOC_SIZE))
}

fn next_assoc(assoc: usize) -> Option<usize> {
    let next: extern "C" fn(usize) -> usize = unsafe { std::mem::transmute(NEXT_ASSOC) };
    Some(next(assoc)).filter(|&a| a != 0 && readable(a, ASSOC_SIZE))
}

fn find_assoc(clump: usize, pick: impl Fn(usize) -> bool) -> Option<usize> {
    let mut assoc = first_assoc(clump);

    while let Some(a) = assoc {
        if pick(read_ptr(a + ASSOC_HIER).unwrap_or(0)) {
            return Some(a);
        }
        assoc = next_assoc(a);
    }

    None
}

fn blend_delta(assoc: usize) -> f32 {
    unsafe { ((assoc + ASSOC_BLEND_DELTA) as *const f32).read_unaligned() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eingebettete_ifp_passt() {
        let (block, namen) = namen_in_ifp(IFP).expect("IFP lesbar");
        assert_eq!(block, BLOCK);
        assert!(!namen.is_empty());
        assert!(namen.iter().all(|n| gueltiger_name(n)), "{namen:?}");
        assert!(BLOCK.len() < 16);
    }

    #[test]
    fn namen_pruefen() {
        assert!(gueltiger_name("handy_halten"));
        assert!(!gueltiger_name("Handy"));
        assert!(!gueltiger_name(""));
        assert!(!gueltiger_name("a b"));
        assert!(!gueltiger_name("abcdefghijklmnopqrstuvwx"));
    }
}
