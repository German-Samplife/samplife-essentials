//! Watchdog thread, started from DllMain (it runs once the loader lock is released).
//! Writes a sign of life right away and then, every second for 60 s and every 30 s
//! up to 10 minutes, what the entries into the game thread are doing. It also
//! installs entry B as soon as GTA's Direct3D device exists.

use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use winapi::shared::minwindef::HMODULE;
use winapi::um::libloaderapi::{GetModuleFileNameW, GetModuleHandleA};
use winapi::um::winver::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};

use crate::hooks::{self, *};
use crate::logic;

pub static MODULE: AtomicUsize = AtomicUsize::new(0);

pub fn start() {
    std::thread::spawn(|| {
        let _ = std::panic::catch_unwind(run);
    });
}

fn module_path(module: HMODULE) -> String {
    let mut buffer = vec![0u16; 1024];
    let len = unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    OsString::from_wide(&buffer[..len.min(buffer.len())])
        .to_string_lossy()
        .into_owned()
}

fn samp_module() -> HMODULE {
    unsafe { GetModuleHandleA(c"samp.dll".as_ptr()) }
}

/// File version of samp.dll as "a.b.c.d" plus the SA-MP name samp-client-api uses
/// (version.rs: major = MS & 0xFF, minor = LS >> 16 & 0xFF, rc = LS & 0xFF).
fn samp_version() -> Option<String> {
    let module = samp_module();

    if module.is_null() {
        return None;
    }

    let path: Vec<u16> = std::ffi::OsStr::new(&module_path(module))
        .encode_wide()
        .chain(Some(0))
        .collect();

    unsafe {
        let size = GetFileVersionInfoSizeW(path.as_ptr(), std::ptr::null_mut());

        if size == 0 {
            return Some("Version unbekannt".to_owned());
        }

        let mut data = vec![0u8; size as usize];

        if GetFileVersionInfoW(path.as_ptr(), 0, size, data.as_mut_ptr() as *mut _) == 0 {
            return Some("Version unbekannt".to_owned());
        }

        let root: Vec<u16> = "\\".encode_utf16().chain(Some(0)).collect();
        let mut info: winapi::shared::minwindef::LPVOID = std::ptr::null_mut();
        let mut len = 0u32;

        if VerQueryValueW(data.as_ptr() as *const _, root.as_ptr(), &mut info, &mut len) == 0
            || info.is_null()
            || len < 16
        {
            return Some("Version unbekannt".to_owned());
        }

        // VS_FIXEDFILEINFO: dwSignature, dwStrucVersion, dwFileVersionMS, dwFileVersionLS
        let ms = (info as *const u32).add(2).read_unaligned();
        let ls = (info as *const u32).add(3).read_unaligned();
        let name = match (ms & 0xFF, (ls >> 16) & 0xFF, ls & 0xFF) {
            (3, 7, 0) => "0.3.7 R1",
            (3, 7, 2) => "0.3.7 R3",
            (3, 8, 0) => "0.3.DL",
            _ => "nicht unterstützt",
        };

        Some(format!(
            "{}.{}.{}.{} = {name}",
            ms >> 16,
            ms & 0xFFFF,
            ls >> 16,
            ls & 0xFFFF
        ))
    }
}

fn site_text() -> String {
    let (op, target) = site_state();
    let mine = if site_is_ours() { "unser Hook" } else { "NICHT unser Hook" };

    match op {
        0xE8 => format!("call {target:#x} ({mine})"),
        0xE9 => format!("jmp {target:#x} ({mine})"),
        _ => format!("Opcode {op:#04x} ({mine})"),
    }
}

fn run() {
    let module = MODULE.load(Ordering::SeqCst) as HMODULE;

    crate::log!("samplife-kleidung {} geladen", crate::ASI_VERSION);
    crate::log!("ASI-Datei: {}", module_path(module));
    crate::log!("EXE: {}", module_path(std::ptr::null_mut()));
    crate::log!(
        "Logdatei: {}",
        crate::log::path().map(|p| p.display().to_string()).unwrap_or_default()
    );
    crate::log!(
        "Kennung an 0x401000: {:#010x} ({})",
        EXE_PROBE.load(Ordering::SeqCst),
        if exe_ok() { "GTA SA 1.0 US" } else { "NICHT GTA SA 1.0 US, das ASI bleibt aus" }
    );
    crate::log!(
        "Einstieg A (0x53E981): {}, vorheriges Ziel {:#x} (CGame::Process ist {:#x}), jetzt: {}",
        status_text(HOOK_STATUS.load(Ordering::SeqCst)),
        FIRST_TARGET.load(Ordering::SeqCst),
        hooks::CGAME_PROCESS,
        site_text()
    );
    crate::log!(
        "samp.dll beim Laden: {}",
        samp_version().unwrap_or_else(|| "noch nicht geladen".to_owned())
    );

    if !exe_ok() {
        return;
    }

    let mut seconds = 0u32;

    while seconds < 600 {
        let step = if seconds < 60 { 1 } else { 30 };
        std::thread::sleep(Duration::from_secs(step));
        seconds += step as u32;

        if let Some(text) = install_present_hook() {
            crate::log!("{text}");
        }

        crate::log!(
            "t+{seconds}s: samp.dll {} | 0x53E981 {} | Aufrufe A {} B {} | Logik über A {} über B {} | neu eingehakt {} | Einstieg B {} (Gerät {:#x}) | SA-MP erkannt: {} | /kleidungtest: {}",
            samp_version().unwrap_or_else(|| "nicht geladen".to_owned()),
            site_text(),
            PROCESS_CALLS.load(Ordering::Relaxed),
            PRESENT_CALLS.load(Ordering::Relaxed),
            LOGIC_FROM_PROCESS.load(Ordering::Relaxed),
            LOGIC_FROM_PRESENT.load(Ordering::Relaxed),
            REHOOKS.load(Ordering::Relaxed),
            status_text(PRESENT_STATUS.load(Ordering::SeqCst)),
            PRESENT_DEVICE.load(Ordering::SeqCst),
            logic::samp_text(),
            logic::command_text()
        );
    }

    crate::log!("Protokoll im Sekundentakt beendet");
}
