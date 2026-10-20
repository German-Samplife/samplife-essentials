//! samplife-kleidung: feasibility test for changing CJ's (skin 0) clothes in the
//! SA-MP 0.3.7 client, for the local ped and for remote player peds.
//!
//! Loaded by an ASI loader. In `DllMain` only one 5-byte call site in GTA's
//! `Idle()` is redirected (see `game::install_process_hook`); everything else
//! happens on the game thread right after `CGame::Process()`.

#![allow(clippy::missing_safety_doc)]

mod game;
mod log;
mod logic;
mod outfit;
mod samp;

use winapi::shared::minwindef::{BOOL, DWORD, HINSTANCE, LPVOID, TRUE};
use winapi::um::libloaderapi::DisableThreadLibraryCalls;
use winapi::um::winnt::DLL_PROCESS_ATTACH;

pub const ASI_VERSION: &str = env!("CARGO_PKG_VERSION");

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn DllMain(instance: HINSTANCE, reason: DWORD, _reserved: LPVOID) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        DisableThreadLibraryCalls(instance);
        // No file I/O or LoadLibrary under the loader lock; the result is
        // stored and written to the log from the first game frame.
        game::install_process_hook();

        let status = game::HOOK_STATUS.load(std::sync::atomic::Ordering::SeqCst);

        if status != game::HOOK_OK {
            // the hook is the only way in, so report the failure from a thread
            // (it starts after the loader lock is released)
            std::thread::spawn(move || {
                let reason = match status {
                    game::HOOK_WRONG_EXE => "gta_sa.exe ist nicht Version 1.0 US",
                    game::HOOK_NOT_A_CALL => "an 0x53E981 steht kein Aufruf von CGame::Process",
                    game::HOOK_PROTECT_FAILED => "Speicherschutz an 0x53E981 ließ sich nicht ändern",
                    _ => "unbekannter Fehler",
                };
                crate::log!(
                    "samplife-kleidung {} inaktiv: {reason} (Kennung an 0x401000: {:#010x})",
                    ASI_VERSION,
                    game::EXE_PROBE.load(std::sync::atomic::Ordering::SeqCst)
                );
            });
        }
    }

    TRUE
}
