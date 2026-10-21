//! Entry points into the game thread. The clothes logic (`logic::frame`) runs once per
//! frame, from entry A if A runs, otherwise from entry B.
//!
//! A: the `call CGame::Process` inside Idle() at 0x53E981.
//!    plugin-sdk shared/Events.h (GTA SA): `gameProcessEvent` = AddressList<0x53E981, H_CALL>.
//!    gta-reversed source/game_sa/Game.cpp: RH_ScopedInstall(Process, 0x53BEE0).
//!    We call whatever the call pointed to before (chain). If someone overwrites the
//!    call later and does not chain to us, we hook again on top of them (from the game
//!    thread, see `maintenance`).
//!
//! B: IDirect3DDevice9::Present of the device GTA renders with. GTA keeps it in the
//!    RenderWare global RwD3D9Device at 0xC97C28 (samp-client-api gta/d3d9.rs,
//!    `D3D9_DEVICE_PTR`). We replace the Present entry (index from winapi's
//!    IDirect3DDevice9Vtbl) in that object's vtable and call the previous entry, so
//!    SA-MP's device proxy, the samp-cef Present detour and other hooks keep working.
//!    Present is called by GTA's game thread at the end of each frame (Idle ->
//!    RsCameraShowRaster). While the pause menu is open, B does not run the logic.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

use winapi::shared::d3d9::{IDirect3DDevice9, IDirect3DDevice9Vtbl};
use winapi::shared::windef::{HWND, RECT};
use winapi::um::memoryapi::VirtualProtect;
use winapi::um::processthreadsapi::{FlushInstructionCache, GetCurrentProcess};
use winapi::um::wingdi::RGNDATA;
use winapi::um::winnt::{HRESULT, PAGE_EXECUTE_READWRITE, PAGE_READWRITE};

use crate::game::{read_ptr, read_u32, readable};

// plugin-sdk shared/GameVersion.cpp, detect_game_id():
//   GetUInt(0x401000) == 0x53EC8B55 -> GAME_10US_COMPACT, == 0x16197BE9 -> GAME_10US_HOODLUM
const VERSION_PROBE: usize = 0x401000;
const VERSION_10US_COMPACT: u32 = 0x53EC8B55;
const VERSION_10US_HOODLUM: u32 = 0x16197BE9;

pub const IDLE_CALL_GAME_PROCESS: usize = 0x53E981;
pub const CGAME_PROCESS: usize = 0x53BEE0;
pub const RW_D3D9_DEVICE: usize = 0xC97C28;

pub const HOOK_NOT_TRIED: u32 = 0;
pub const HOOK_OK: u32 = 1;
pub const HOOK_WRONG_EXE: u32 = 2;
pub const HOOK_NOT_A_CALL: u32 = 3;
pub const HOOK_PROTECT_FAILED: u32 = 4;

pub static EXE_PROBE: AtomicU32 = AtomicU32::new(0);
/// entry A
pub static HOOK_STATUS: AtomicU32 = AtomicU32::new(HOOK_NOT_TRIED);
pub static ORIGINAL_TARGET: AtomicUsize = AtomicUsize::new(0);
pub static FIRST_TARGET: AtomicUsize = AtomicUsize::new(0);
pub static PROCESS_CALLS: AtomicU32 = AtomicU32::new(0);
pub static REHOOKS: AtomicU32 = AtomicU32::new(0);
/// entry B
pub static PRESENT_STATUS: AtomicU32 = AtomicU32::new(HOOK_NOT_TRIED);
pub static PRESENT_CALLS: AtomicU32 = AtomicU32::new(0);
pub static PRESENT_DEVICE: AtomicUsize = AtomicUsize::new(0);
/// how often the logic ran from A / from B
pub static LOGIC_FROM_PROCESS: AtomicU32 = AtomicU32::new(0);
pub static LOGIC_FROM_PRESENT: AtomicU32 = AtomicU32::new(0);

static RAN_THIS_FRAME: AtomicBool = AtomicBool::new(false);
static IN_PRESENT: AtomicBool = AtomicBool::new(false);
static IN_LOGIC: AtomicBool = AtomicBool::new(false);
static LAST_PROCESS_CALLS: AtomicU32 = AtomicU32::new(0);
static STALLED_FRAMES: AtomicU32 = AtomicU32::new(0);

const MAX_REHOOKS: u32 = 5;
const STALL_FRAMES_BEFORE_REHOOK: u32 = 120;

pub fn exe_ok() -> bool {
    let probe = EXE_PROBE.load(Ordering::SeqCst);
    probe == VERSION_10US_COMPACT || probe == VERSION_10US_HOODLUM
}

pub fn probe_exe() {
    EXE_PROBE.store(read_u32(VERSION_PROBE).unwrap_or(0), Ordering::SeqCst);
}

fn process_hook_addr() -> usize {
    game_process_hook as extern "C" fn() as usize
}

/// Opcode at 0x53E981 and, for call/jmp rel32, the target.
pub fn site_state() -> (u8, usize) {
    let site = IDLE_CALL_GAME_PROCESS;

    if !readable(site, 5) {
        return (0, 0);
    }

    unsafe {
        let op = *(site as *const u8);
        let rel = ((site + 1) as *const i32).read_unaligned();
        let target = if op == 0xE8 || op == 0xE9 {
            (site + 5).wrapping_add(rel as isize as usize)
        } else {
            0
        };
        (op, target)
    }
}

pub fn site_is_ours() -> bool {
    let (op, target) = site_state();
    op == 0xE8 && target == process_hook_addr()
}

/// Points the call at 0x53E981 to our hook; the current target becomes the one we call.
fn patch_site(current_target: usize) -> bool {
    let site = IDLE_CALL_GAME_PROCESS;
    ORIGINAL_TARGET.store(current_target, Ordering::SeqCst);

    unsafe {
        let mut old = 0;

        if VirtualProtect((site + 1) as *mut c_void as _, 4, PAGE_EXECUTE_READWRITE, &mut old) == 0 {
            return false;
        }

        let new_rel = process_hook_addr().wrapping_sub(site + 5) as i32;
        ((site + 1) as *mut i32).write_unaligned(new_rel);

        let mut ignored = 0;
        VirtualProtect((site + 1) as *mut c_void as _, 4, old, &mut ignored);
        FlushInstructionCache(GetCurrentProcess(), site as *const c_void as _, 5);
    }

    true
}

/// Entry A, runs inside DllMain.
pub fn install_process_hook() {
    probe_exe();

    if !exe_ok() {
        HOOK_STATUS.store(HOOK_WRONG_EXE, Ordering::SeqCst);
        return;
    }

    let (op, target) = site_state();
    FIRST_TARGET.store(target, Ordering::SeqCst);

    if op != 0xE8 {
        HOOK_STATUS.store(HOOK_NOT_A_CALL, Ordering::SeqCst);
        return;
    }

    let status = if patch_site(target) { HOOK_OK } else { HOOK_PROTECT_FAILED };
    HOOK_STATUS.store(status, Ordering::SeqCst);
}

extern "C" fn game_process_hook() {
    PROCESS_CALLS.fetch_add(1, Ordering::Relaxed);

    let target = ORIGINAL_TARGET.load(Ordering::Relaxed);

    if target != 0 && target != process_hook_addr() {
        let original: extern "C" fn() = unsafe { std::mem::transmute(target) };
        original();
    }

    run_logic(false);
}

fn run_logic(from_present: bool) {
    if from_present {
        // A already ran this frame
        if RAN_THIS_FRAME.swap(false, Ordering::SeqCst) {
            return;
        }

        if client_api::gta::menu_manager::CMenuManager::get().is_active() {
            return;
        }

        LOGIC_FROM_PRESENT.fetch_add(1, Ordering::Relaxed);
    } else {
        RAN_THIS_FRAME.store(true, Ordering::SeqCst);
        LOGIC_FROM_PROCESS.fetch_add(1, Ordering::Relaxed);
    }

    if IN_LOGIC.swap(true, Ordering::SeqCst) {
        return;
    }

    // never unwind into GTA
    let _ = std::panic::catch_unwind(crate::logic::frame);
    IN_LOGIC.store(false, Ordering::SeqCst);
}

/// Game thread, once per frame from entry B: hooks entry A again if somebody
/// replaced our call at 0x53E981 without calling us.
fn maintenance() {
    if !exe_ok() {
        return;
    }

    let calls = PROCESS_CALLS.load(Ordering::Relaxed);

    if calls != LAST_PROCESS_CALLS.swap(calls, Ordering::Relaxed) {
        // A runs (directly or chained through another hook)
        STALLED_FRAMES.store(0, Ordering::Relaxed);
        return;
    }

    let (op, target) = site_state();

    if (op == 0xE8 && target == process_hook_addr()) || op != 0xE8 || target == 0 {
        return;
    }

    if STALLED_FRAMES.fetch_add(1, Ordering::Relaxed) + 1 < STALL_FRAMES_BEFORE_REHOOK {
        return;
    }

    STALLED_FRAMES.store(0, Ordering::Relaxed);

    if REHOOKS.load(Ordering::Relaxed) >= MAX_REHOOKS {
        return;
    }

    let ok = patch_site(target);
    REHOOKS.fetch_add(1, Ordering::Relaxed);

    if ok {
        HOOK_STATUS.store(HOOK_OK, Ordering::SeqCst);
    }

    crate::log!(
        "Einstieg A: 0x53E981 zeigte auf {target:#x} statt auf unseren Hook, neu eingehakt ({})",
        if ok { "ok" } else { "Speicherschutz fehlgeschlagen" }
    );
}

// ---------------------------------------------------------------------------
// Entry B

type PresentFn = unsafe extern "system" fn(
    *mut IDirect3DDevice9,
    *const RECT,
    *const RECT,
    HWND,
    *const RGNDATA,
) -> HRESULT;

const SLOTS: usize = 4;
static SLOT_VTABLE: [AtomicUsize; SLOTS] = [const { AtomicUsize::new(0) }; SLOTS];
static SLOT_PREVIOUS: [AtomicUsize; SLOTS] = [const { AtomicUsize::new(0) }; SLOTS];

fn present_hook_addr() -> usize {
    present_hook as PresentFn as usize
}

fn previous_present(vtable: usize) -> usize {
    for i in 0..SLOTS {
        if SLOT_VTABLE[i].load(Ordering::SeqCst) == vtable {
            return SLOT_PREVIOUS[i].load(Ordering::SeqCst);
        }
    }

    SLOT_PREVIOUS[0].load(Ordering::SeqCst)
}

unsafe extern "system" fn present_hook(
    device: *mut IDirect3DDevice9,
    source: *const RECT,
    dest: *const RECT,
    window: HWND,
    dirty: *const RGNDATA,
) -> HRESULT {
    let vtable = if device.is_null() { 0 } else { *(device as *const usize) };
    let previous = previous_present(vtable);

    if !IN_PRESENT.swap(true, Ordering::SeqCst) {
        PRESENT_CALLS.fetch_add(1, Ordering::Relaxed);
        let _ = std::panic::catch_unwind(maintenance);
        run_logic(true);
        IN_PRESENT.store(false, Ordering::SeqCst);
    }

    if previous == 0 || previous == present_hook_addr() {
        return 0;
    }

    let previous: PresentFn = std::mem::transmute(previous);
    previous(device, source, dest, window, dirty)
}

/// Entry B. Called from the watchdog thread every second; installs on the device in
/// RwD3D9Device if its Present entry isn't ours yet. Returns a short status text.
pub fn install_present_hook() -> Option<String> {
    if !exe_ok() {
        return None;
    }

    let device = read_ptr(RW_D3D9_DEVICE)?;
    let vtable = read_ptr(device)?;
    let entry = vtable + std::mem::offset_of!(IDirect3DDevice9Vtbl, Present);
    let current = read_u32(entry)? as usize;

    if current == present_hook_addr() {
        return None;
    }

    if current == 0 || !readable(current, 1) {
        return Some(format!("Present-Eintrag {current:#x} ungültig"));
    }

    let Some(slot) = (0..SLOTS).find(|&i| SLOT_VTABLE[i].load(Ordering::SeqCst) == 0) else {
        PRESENT_STATUS.store(HOOK_PROTECT_FAILED, Ordering::SeqCst);
        return Some("keine freien Plätze mehr".to_owned());
    };

    SLOT_PREVIOUS[slot].store(current, Ordering::SeqCst);
    SLOT_VTABLE[slot].store(vtable, Ordering::SeqCst);

    let ok = unsafe {
        let mut old = 0;

        if VirtualProtect(entry as *mut c_void as _, 4, PAGE_READWRITE, &mut old) == 0 {
            false
        } else {
            // aligned 4-byte store, atomic for the game thread
            (*(entry as *const AtomicUsize)).store(present_hook_addr(), Ordering::SeqCst);
            let mut ignored = 0;
            VirtualProtect(entry as *mut c_void as _, 4, old, &mut ignored);
            true
        }
    };

    if ok {
        PRESENT_STATUS.store(HOOK_OK, Ordering::SeqCst);
        PRESENT_DEVICE.store(device, Ordering::SeqCst);
        Some(format!(
            "Einstieg B eingehakt: Gerät {device:#x}, vtable {vtable:#x}, Present vorher {current:#x}"
        ))
    } else {
        SLOT_VTABLE[slot].store(0, Ordering::SeqCst);
        PRESENT_STATUS.store(HOOK_PROTECT_FAILED, Ordering::SeqCst);
        Some(format!("Einstieg B: Speicherschutz an {entry:#x} ließ sich nicht ändern"))
    }
}

pub fn status_text(status: u32) -> &'static str {
    match status {
        HOOK_NOT_TRIED => "noch nicht",
        HOOK_OK => "ok",
        HOOK_WRONG_EXE => "gta_sa.exe nicht 1.0 US",
        HOOK_NOT_A_CALL => "kein call an 0x53E981",
        HOOK_PROTECT_FAILED => "Speicherschutz fehlgeschlagen",
        _ => "?",
    }
}
