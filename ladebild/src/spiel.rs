//! Was das Ladebild aus GTA San Andreas 1.0 US liest. Die Adressen sind dieselben, die
//! samplife-kleidung benutzt (kleidung/src/hooks.rs), plus der Zustand des Spiels.

use winapi::um::memoryapi::VirtualQuery;
use winapi::um::winnt::{MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_GUARD, PAGE_NOACCESS};

// plugin-sdk shared/GameVersion.cpp, detect_game_id():
//   GetUInt(0x401000) == 0x53EC8B55 -> GAME_10US_COMPACT, == 0x16197BE9 -> GAME_10US_HOODLUM
const VERSION_PROBE: usize = 0x401000;
const VERSION_10US_COMPACT: u32 = 0x53EC8B55;
const VERSION_10US_HOODLUM: u32 = 0x16197BE9;

/// RenderWare-Global RwD3D9Device (samp-client-api gta/d3d9.rs, `D3D9_DEVICE_PTR`)
pub const RW_D3D9_DEVICE: usize = 0xC97C28;

/// gGameState (plugin-sdk game_sa/common.h, gta-reversed Game.h: `gGameState` 0xC8D4C0).
/// 9 = GAME_STATE_IDLE: Das Spiel läuft, das Laden ist vorbei.
pub const GAME_STATE: usize = 0xC8D4C0;
pub const GAME_STATE_LAEUFT: u32 = 9;

pub fn exe_ok() -> bool {
    let probe = lesen_u32(VERSION_PROBE).unwrap_or(0);
    probe == VERSION_10US_COMPACT || probe == VERSION_10US_HOODLUM
}

pub fn lesbar(adresse: usize, laenge: usize) -> bool {
    unsafe {
        let mut info: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
        if VirtualQuery(adresse as _, &mut info, std::mem::size_of::<MEMORY_BASIC_INFORMATION>()) == 0 {
            return false;
        }
        let ende = info.BaseAddress as usize + info.RegionSize;
        info.State == MEM_COMMIT
            && info.Protect & (PAGE_NOACCESS | PAGE_GUARD) == 0
            && adresse + laenge <= ende
    }
}

pub fn lesen_u32(adresse: usize) -> Option<u32> {
    if !lesbar(adresse, 4) {
        return None;
    }
    Some(unsafe { (adresse as *const u32).read_unaligned() })
}

pub fn lesen_zeiger(adresse: usize) -> Option<usize> {
    lesen_u32(adresse).map(|wert| wert as usize).filter(|&wert| wert != 0)
}

pub fn spiel_laeuft() -> bool {
    lesen_u32(GAME_STATE) == Some(GAME_STATE_LAEUFT)
}
