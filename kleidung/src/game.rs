//! GTA San Andreas 1.0 US: addresses, offsets and safe-ish accessors.
//!
//! Sources (all public):
//! - gta-reversed (https://github.com/gta-reversed/gta-reversed), `RH_ScopedInstall`
//!   lines and `// 0x...` comments name the original address of each function.
//! - plugin-sdk (https://github.com/DK22Pac/plugin-sdk), `VALIDATE_OFFSET` lines
//!   in the headers name the field offsets.

use std::ffi::{c_char, c_void};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use winapi::um::memoryapi::{VirtualProtect, VirtualQuery};
use winapi::um::processthreadsapi::{FlushInstructionCache, GetCurrentProcess};
use winapi::um::winnt::{
    MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY,
    PAGE_GUARD, PAGE_NOACCESS, PAGE_READWRITE, PAGE_WRITECOPY,
};

// ---------------------------------------------------------------------------
// Executable version
// plugin-sdk shared/GameVersion.cpp, detect_game_id():
//   GetUInt(0x401000) == 0x53EC8B55 -> GAME_10US_COMPACT
//   GetUInt(0x401000) == 0x16197BE9 -> GAME_10US_HOODLUM
// Both are 1.0 US and share all addresses below.
const VERSION_PROBE: usize = 0x401000;
const VERSION_10US_COMPACT: u32 = 0x53EC8B55;
const VERSION_10US_HOODLUM: u32 = 0x16197BE9;

// ---------------------------------------------------------------------------
// Per-frame hook
// plugin-sdk shared/Events.h (GTA SA): `gameProcessEvent` = AddressList<0x53E981, H_CALL>,
// i.e. the `call CGame::Process` inside Idle(). Hooking a call site (instead of
// the function entry) is what plugin-sdk ASIs do and chains with other hooks:
// we call whatever the call pointed to before.
const IDLE_CALL_GAME_PROCESS: usize = 0x53E981;
// gta-reversed source/game_sa/Game.cpp: RH_ScopedInstall(Process, 0x53BEE0)
pub const CGAME_PROCESS: usize = 0x53BEE0;

// ---------------------------------------------------------------------------
// Functions
// gta-reversed source/game_sa/Clothes.cpp: RH_ScopedInstall(RebuildPlayer, 0x5A82C0)
// plugin-sdk CClothes.cpp: "cdecl void CClothes::RebuildPlayer(CPlayerPed*, bool) 0x5A82C0"
const CCLOTHES_REBUILD_PLAYER: usize = 0x5A82C0;
// gta-reversed source/game_sa/Core/KeyGen.cpp: RH_ScopedOverloadedInstall(GetUppercaseKey, "", 0x53CF30, ...)
// plugin-sdk CKeyGen.cpp: "cdecl uint CKeyGen::GetUppercaseKey(char const*) 0x53CF30"
const CKEYGEN_GET_UPPERCASE_KEY: usize = 0x53CF30;
// CPedClothesDesc::SetTextureAndModel(const char*, const char*, eClothesTexturePart) is at
// 0x5A8080 (gta-reversed PedClothesDesc.cpp), but gta-reversed notes that its compiled
// form takes texturePart in EDX instead of on the stack. We therefore do not call it and
// write the same two keys ourselves (see `ClothesDesc::set_texture_and_model`).

// ---------------------------------------------------------------------------
// Globals
// gta-reversed source/game_sa/World.h: Players = StaticRef<CPlayerInfo[MAX_PLAYERS]>(0xB7CD98)
// plugin-sdk CPlayerInfo.h: VALIDATE_OFFSET(CPlayerInfo, m_pPed, 0x0)
const CWORLD_PLAYERS: usize = 0xB7CD98;

// ---------------------------------------------------------------------------
// Offsets (plugin-sdk CEntity.h / CPed.h VALIDATE_OFFSET, gta-reversed PlayerPedData.h)
const ENTITY_RW_OBJECT: usize = 0x18; // CEntity::m_pRwObject
const ENTITY_MODEL_INDEX: usize = 0x22; // CEntity::m_nModelIndex (int16)
const PED_FLAGS: usize = 0x46C; // CPed flags, bit 8 = bInVehicle (field_468 at 0x468 + 4)
const PED_PLAYER_DATA: usize = 0x480; // CPed::m_pPlayerData (CPlayerPedData*)
const PED_STATE: usize = 0x530; // CPed::m_ePedState
const PLAYER_DATA_CLOTHES_DESC: usize = 0x4; // CPlayerPedData::m_pPedClothesDesc (after m_pWanted)
const CPLAYERPED_SIZE: usize = 0x7A4; // VALIDATE_SIZE(CPlayerPed, 0x7A4)

// gta-reversed Enums/ePedState.h (PEDSTATE_NONE = 0, counted)
const PEDSTATE_DIE: u32 = 54;
const PEDSTATE_DEAD: u32 = 55;
const PEDSTATE_DIE_BY_STEALTH: u32 = 56;

pub const MODEL_PLAYER: i16 = 0;

// gta-reversed Enums/eClothesModelPart.h / eClothesTexturePart.h
pub const CLOTHES_MODEL_TOTAL: usize = 10;
pub const CLOTHES_TEXTURE_TOTAL: usize = 18;
pub const TEX_TORSO: usize = 0;
pub const TEX_HEAD: usize = 1;
pub const TEX_LEGS: usize = 2;
pub const TEX_SHOES: usize = 3;
pub const TEX_NECKLACE: usize = 13;
pub const TEX_BRACELET: usize = 14; // watch
pub const TEX_GLASSES: usize = 15;
pub const TEX_HATS: usize = 16;
#[allow(dead_code)]
pub const TEX_SPECIAL: usize = 17;

/// CClothes::GetTextureDependency (gta-reversed Clothes.cpp, 0x5A7EA0):
/// texture part -> model part, None for tattoos.
pub fn texture_dependency(texture_part: usize) -> Option<usize> {
    match texture_part {
        0 => Some(0),  // TORSO -> CLOTHES_MODEL_TORSO
        1 => Some(1),  // HEAD -> CLOTHES_MODEL_HEAD
        2 => Some(3),  // LEGS -> CLOTHES_MODEL_LEGS
        3 => Some(4),  // SHOES -> CLOTHES_MODEL_SHOES
        13 => Some(5), // NECKLACE -> CLOTHES_MODEL_NECKLACE
        14 => Some(6), // BRACELET -> CLOTHES_MODEL_BRACELET
        15 => Some(7), // GLASSES -> CLOTHES_MODEL_GLASSES
        16 => Some(8), // HATS -> CLOTHES_MODEL_HATS
        17 => Some(9), // SPECIAL -> CLOTHES_MODEL_SPECIAL
        _ => None,
    }
}

/// CPedClothesDesc, VALIDATE_SIZE(CPedClothesDesc, 0x78) in gta-reversed PedClothesDesc.h
#[repr(C)]
#[derive(Clone, Copy, PartialEq)]
pub struct ClothesDesc {
    pub model_keys: [u32; CLOTHES_MODEL_TOTAL],
    pub texture_keys: [u32; CLOTHES_TEXTURE_TOTAL],
    pub fat_stat: f32,
    pub muscle_stat: f32,
}

const _: () = assert!(std::mem::size_of::<ClothesDesc>() == 0x78);

impl ClothesDesc {
    /// Same writes as CPedClothesDesc::SetTextureAndModel (gta-reversed PedClothesDesc.cpp, 0x5A8050/0x5A8080).
    pub fn set_texture_and_model(&mut self, texture_key: u32, model_key: u32, texture_part: usize) {
        self.texture_keys[texture_part] = texture_key;

        if let Some(model_part) = texture_dependency(texture_part) {
            self.model_keys[model_part] = model_key;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.texture_keys.iter().all(|&k| k == 0)
    }
}

// ---------------------------------------------------------------------------
// Memory helpers

pub fn readable(addr: usize, len: usize) -> bool {
    region_ok(addr, len, false)
}

pub fn writable(addr: usize, len: usize) -> bool {
    region_ok(addr, len, true)
}

fn region_ok(addr: usize, len: usize, write: bool) -> bool {
    if addr < 0x10000 || addr.checked_add(len).is_none() {
        return false;
    }

    let mut current = addr;
    let end = addr + len;

    while current < end {
        let mut info: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<MEMORY_BASIC_INFORMATION>();

        if unsafe { VirtualQuery(current as *const c_void as _, &mut info, size) } == 0 {
            return false;
        }

        if info.State != MEM_COMMIT || info.Protect == 0 || info.Protect & (PAGE_NOACCESS | PAGE_GUARD) != 0 {
            return false;
        }

        if write
            && info.Protect
                & (PAGE_READWRITE | PAGE_WRITECOPY | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY)
                == 0
        {
            return false;
        }

        let region_end = info.BaseAddress as usize + info.RegionSize;

        if region_end <= current {
            return false;
        }

        current = region_end;
    }

    true
}

pub fn read_u32(addr: usize) -> Option<u32> {
    if readable(addr, 4) {
        Some(unsafe { (addr as *const u32).read_unaligned() })
    } else {
        None
    }
}

pub fn read_ptr(addr: usize) -> Option<usize> {
    read_u32(addr).map(|v| v as usize).filter(|&v| v != 0)
}

// ---------------------------------------------------------------------------
// Hook installation (runs inside DllMain)

pub const HOOK_NOT_TRIED: u32 = 0;
pub const HOOK_OK: u32 = 1;
pub const HOOK_WRONG_EXE: u32 = 2;
pub const HOOK_NOT_A_CALL: u32 = 3;
pub const HOOK_PROTECT_FAILED: u32 = 4;

pub static HOOK_STATUS: AtomicU32 = AtomicU32::new(HOOK_NOT_TRIED);
pub static EXE_PROBE: AtomicU32 = AtomicU32::new(0);
pub static ORIGINAL_TARGET: AtomicUsize = AtomicUsize::new(0);

pub fn install_process_hook() {
    let probe = read_u32(VERSION_PROBE).unwrap_or(0);
    EXE_PROBE.store(probe, Ordering::SeqCst);

    if probe != VERSION_10US_COMPACT && probe != VERSION_10US_HOODLUM {
        HOOK_STATUS.store(HOOK_WRONG_EXE, Ordering::SeqCst);
        return;
    }

    let site = IDLE_CALL_GAME_PROCESS;

    if !readable(site, 5) || unsafe { *(site as *const u8) } != 0xE8 {
        HOOK_STATUS.store(HOOK_NOT_A_CALL, Ordering::SeqCst);
        return;
    }

    unsafe {
        let rel = ((site + 1) as *const i32).read_unaligned();
        let target = (site + 5).wrapping_add(rel as isize as usize);
        ORIGINAL_TARGET.store(target, Ordering::SeqCst);

        let mut old = 0;

        if VirtualProtect((site + 1) as *mut c_void as _, 4, PAGE_EXECUTE_READWRITE, &mut old) == 0 {
            HOOK_STATUS.store(HOOK_PROTECT_FAILED, Ordering::SeqCst);
            return;
        }

        let new_rel = (game_process_hook as extern "C" fn() as usize).wrapping_sub(site + 5) as i32;
        ((site + 1) as *mut i32).write_unaligned(new_rel);

        let mut ignored = 0;
        VirtualProtect((site + 1) as *mut c_void as _, 4, old, &mut ignored);
        FlushInstructionCache(GetCurrentProcess(), site as *const c_void as _, 5);
    }

    HOOK_STATUS.store(HOOK_OK, Ordering::SeqCst);
}

extern "C" fn game_process_hook() {
    let target = ORIGINAL_TARGET.load(Ordering::Relaxed);

    if target != 0 {
        let original: extern "C" fn() = unsafe { std::mem::transmute(target) };
        original();
    }

    // never unwind into GTA
    let _ = std::panic::catch_unwind(crate::logic::frame);
}

// ---------------------------------------------------------------------------
// Game functions

pub fn uppercase_key(name: &str) -> u32 {
    let mut bytes: Vec<u8> = name.bytes().filter(|&b| b != 0).collect();
    bytes.push(0);

    let get_key: extern "C" fn(*const c_char) -> u32 =
        unsafe { std::mem::transmute(CKEYGEN_GET_UPPERCASE_KEY) };

    get_key(bytes.as_ptr() as *const c_char)
}

/// CClothes::RebuildPlayer(CPlayerPed*, bool bIgnoreFatAndMuscle). With `true` the
/// fat/muscle values stay as they are in the ped's CPedClothesDesc, otherwise the
/// local player's stats would be copied into every ped.
pub fn rebuild_player(ped: usize) {
    let rebuild: extern "C" fn(*mut c_void, bool) =
        unsafe { std::mem::transmute(CCLOTHES_REBUILD_PLAYER) };

    rebuild(ped as *mut c_void, true);
}

/// CWorld::Players[0].m_pPed, fallback for the local ped
pub fn world_player0_ped() -> Option<usize> {
    read_ptr(CWORLD_PLAYERS)
}

// ---------------------------------------------------------------------------
// Ped view

#[derive(Clone, Copy)]
pub struct Ped(pub usize);

pub enum Skip {
    InVehicle,
    Dead,
    NoModel,
    NoClothesData,
}

impl Ped {
    pub fn valid(self) -> bool {
        readable(self.0, CPLAYERPED_SIZE)
    }

    pub fn model(self) -> i16 {
        unsafe { ((self.0 + ENTITY_MODEL_INDEX) as *const i16).read_unaligned() }
    }

    pub fn rw_object(self) -> usize {
        unsafe { ((self.0 + ENTITY_RW_OBJECT) as *const u32).read_unaligned() as usize }
    }

    pub fn in_vehicle(self) -> bool {
        let flags = unsafe { ((self.0 + PED_FLAGS) as *const u32).read_unaligned() };
        flags & (1 << 8) != 0
    }

    pub fn state(self) -> u32 {
        unsafe { ((self.0 + PED_STATE) as *const u32).read_unaligned() }
    }

    pub fn dead(self) -> bool {
        matches!(self.state(), PEDSTATE_DIE | PEDSTATE_DEAD | PEDSTATE_DIE_BY_STEALTH)
    }

    /// Address of the CPedClothesDesc of this ped, if CPlayerPedData and the desc exist.
    pub fn clothes_desc_ptr(self) -> Option<usize> {
        let data = read_ptr(self.0 + PED_PLAYER_DATA)?;
        let desc = read_ptr(data + PLAYER_DATA_CLOTHES_DESC)?;

        if writable(desc, std::mem::size_of::<ClothesDesc>()) {
            Some(desc)
        } else {
            None
        }
    }

    pub fn clothes_desc(self) -> Option<ClothesDesc> {
        let desc = self.clothes_desc_ptr()?;
        Some(unsafe { (desc as *const ClothesDesc).read_unaligned() })
    }

    /// Why this ped can't be rebuilt right now (CPed::Dress, called by RebuildPlayer,
    /// sets PEDSTATE_IDLE and re-adds the ped to the world, so no dead peds and no
    /// peds in vehicles).
    pub fn rebuild_blocker(self) -> Option<Skip> {
        if self.rw_object() == 0 {
            return Some(Skip::NoModel);
        }

        if self.in_vehicle() {
            return Some(Skip::InVehicle);
        }

        if self.dead() {
            return Some(Skip::Dead);
        }

        if self.clothes_desc_ptr().is_none() {
            return Some(Skip::NoClothesData);
        }

        None
    }
}
