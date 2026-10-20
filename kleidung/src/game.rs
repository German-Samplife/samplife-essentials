//! GTA San Andreas 1.0 US: addresses, offsets and safe-ish accessors.
//!
//! Sources (all public):
//! - gta-reversed (https://github.com/gta-reversed/gta-reversed), `RH_ScopedInstall`
//!   lines and `// 0x...` comments name the original address of each function.
//! - plugin-sdk (https://github.com/DK22Pac/plugin-sdk), `VALIDATE_OFFSET` lines
//!   in the headers name the field offsets.

use std::ffi::{c_char, c_void};
use winapi::um::memoryapi::VirtualQuery;
use winapi::um::winnt::{
    MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_EXECUTE_READWRITE, PAGE_EXECUTE_WRITECOPY,
    PAGE_GUARD, PAGE_NOACCESS, PAGE_READWRITE, PAGE_WRITECOPY,
};

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

// ---------------------------------------------------------------------------
// Rebuild by hand (variant b), the steps of CClothes::RebuildPlayer in gta-reversed
// Clothes.cpp, with the game's own inner functions:
// gta-reversed RpAnimBlend.cpp: RpAnimBlendClumpExtractAssociations 0x4D6BE0, ...GiveAssociations 0x4D6C30
// gta-reversed Ped.cpp: DeleteRwObject 0x5DEBF0 (CPlayerPed doesn't override it), Dress 0x5E0130
// gta-reversed World.cpp: Remove 0x563280
// gta-reversed Clothes.cpp: ConstructPedModel 0x5A81E0, PlayerClothes = StaticRef<CPedClothesDesc>(0xBC1C78)
const RP_ANIM_EXTRACT: usize = 0x4D6BE0;
const RP_ANIM_GIVE: usize = 0x4D6C30;
const CPED_DELETE_RW_OBJECT: usize = 0x5DEBF0;
const CPED_DRESS: usize = 0x5E0130;
const CWORLD_REMOVE: usize = 0x563280;
const CCLOTHES_CONSTRUCT_PED_MODEL: usize = 0x5A81E0;
const PLAYER_CLOTHES: usize = 0xBC1C78;
// plugin-sdk CPed.h: VALIDATE_OFFSET(CPed, m_pIntelligence, 0x47C)
const PED_INTELLIGENCE: usize = 0x47C;
// gta-reversed PedIntelligence.h (m_pPed, then m_TaskMgr) and TaskManager.h
// (5 primary, then 6 secondary tasks, TASK_SECONDARY_IK = 5): 4 + 5*4 + 5*4
const INTELLIGENCE_TASK_SECONDARY_IK: usize = 0x2C;
// gta-reversed Tasks/Task.h: vtable 0 dtor, 1 Clone, 2 GetSubTask, 3 IsSimple,
// 4 GetTaskType, 5 StopTimer, 6 MakeAbortable; ABORT_PRIORITY_IMMEDIATE = 2
const TASK_VTABLE_MAKE_ABORTABLE: usize = 6;

// gta-reversed / samp-client-api gta/rw.rs: RpClumpForAllAtomics 0x749B70 (cdecl,
// callback returns the atomic to continue or null to stop)
const RP_CLUMP_FOR_ALL_ATOMICS: usize = 0x749B70;

extern "C" fn first_atomic_cb(atomic: usize, data: *mut usize) -> usize {
    unsafe { *data = atomic };
    0
}

/// First atomic's geometry and the texture of its first material, to see whether two
/// figures share their clothes (structs from samp-client-api gta/rw/rpworld.rs).
pub fn clump_parts(clump: usize) -> (usize, usize) {
    use client_api::gta::rw::rpworld::{RpAtomic, RpGeometry};

    if clump == 0 || !readable(clump, 0x30) {
        return (0, 0);
    }

    let mut atomic = 0usize;
    let for_all: extern "C" fn(usize, extern "C" fn(usize, *mut usize) -> usize, *mut usize) -> usize =
        unsafe { std::mem::transmute(RP_CLUMP_FOR_ALL_ATOMICS) };
    for_all(clump, first_atomic_cb, &mut atomic);

    let geometry = read_ptr(atomic + std::mem::offset_of!(RpAtomic, geometry)).unwrap_or(0);
    if geometry == 0 {
        return (0, 0);
    }

    // RpGeometry::matList.materials -> first RpMaterial -> texture (first field)
    let texture = read_ptr(geometry + std::mem::offset_of!(RpGeometry, matList))
        .and_then(read_ptr)
        .and_then(read_ptr)
        .unwrap_or(0);

    (geometry, texture)
}

/// `force`: pass no "old clothes" to ConstructPedModel, so CreateSkinnedClump always
/// builds (with PlayerClothes it returns early when the clothes equal the last build).
pub fn rebuild_player_manual(ped: usize, desc_ptr: usize, force: bool) {
    unsafe {
        let extract: extern "C" fn(usize) -> usize = std::mem::transmute(RP_ANIM_EXTRACT);
        let give: extern "C" fn(usize, usize) = std::mem::transmute(RP_ANIM_GIVE);
        let delete_rw: extern "thiscall" fn(usize) = std::mem::transmute(CPED_DELETE_RW_OBJECT);
        let dress: extern "thiscall" fn(usize) = std::mem::transmute(CPED_DRESS);
        let world_remove: extern "C" fn(usize) = std::mem::transmute(CWORLD_REMOVE);
        let construct: extern "C" fn(u32, usize, usize, bool) =
            std::mem::transmute(CCLOTHES_CONSTRUCT_PED_MODEL);

        let ped_view = Ped(ped);
        let assoc = extract(ped_view.rw_object());

        if let Some(intelligence) = read_ptr(ped + PED_INTELLIGENCE) {
            if let Some(task) = read_ptr(intelligence + INTELLIGENCE_TASK_SECONDARY_IK) {
                if let Some(func) = read_ptr(task).and_then(|vt| read_ptr(vt + TASK_VTABLE_MAKE_ABORTABLE * 4)) {
                    let make_abortable: extern "thiscall" fn(usize, usize, i32, usize) -> bool =
                        std::mem::transmute(func);
                    make_abortable(task, ped, 2, 0);
                }
            }
        }

        delete_rw(ped);
        world_remove(ped);
        let old_clothes = if force { 0 } else { PLAYER_CLOTHES };
        construct(ped_view.model() as u16 as u32, desc_ptr, old_clothes, false);
        dress(ped);
        give(ped_view.rw_object(), assoc);

        let desc = (desc_ptr as *const ClothesDesc).read_unaligned();
        (PLAYER_CLOTHES as *mut ClothesDesc).write_unaligned(desc);
    }
}

// gta-reversed Models/ModelInfo.h: ms_modelInfoPtrs = StaticRef<...>(0xA9B0C8);
// plugin-sdk CModelInfo.cpp reads the array pointer from 0x40CD67 (limit adjusters).
// plugin-sdk CBaseModelInfo.h: VALIDATE_OFFSET(CBaseModelInfo, m_pRwObject, 0x1C)
const MODEL_INFO_PTRS_REF: usize = 0x40CD67;
const MODEL_INFO_PTRS: usize = 0xA9B0C8;

/// RwObject (clump) of the model info of model 0, the clump every CJ is cloned from.
/// Makes the next RebuildPlayer/ConstructPedModel with &PlayerClothes build in any case.
pub fn forget_player_clothes() {
    if crate::game::writable(PLAYER_CLOTHES, std::mem::size_of::<ClothesDesc>()) {
        unsafe {
            let mut desc = (PLAYER_CLOTHES as *const ClothesDesc).read_unaligned();
            desc.texture_keys = [0; CLOTHES_TEXTURE_TOTAL];
            desc.model_keys = [0; CLOTHES_MODEL_TOTAL];
            (PLAYER_CLOTHES as *mut ClothesDesc).write_unaligned(desc);
        }
    }
}

pub fn model0_clump() -> usize {
    let array = read_ptr(MODEL_INFO_PTRS_REF).unwrap_or(MODEL_INFO_PTRS);
    read_ptr(array).and_then(|info| read_ptr(info + 0x1C)).unwrap_or(0)
}

/// CClothes::ms_clothesImageId (gta-reversed Clothes.h, 0xBC12F8) and the entry count of
/// the player.img directory (ClothesBuilder.cpp: playerImg = StaticRef<CDirectory>(0xBC12C0);
/// Directory.h: m_pEntries, m_nCapacity, m_nNumEntries).
pub fn player_img_state() -> (i32, u32) {
    let image = read_u32(0xBC12F8).unwrap_or(u32::MAX) as i32;
    let entries = read_u32(0xBC12C0 + 8).unwrap_or(0);
    (image, entries)
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
