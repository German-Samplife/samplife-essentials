//! samplife-kleidung: feasibility test for changing CJ's (skin 0) clothes in the
//! SA-MP 0.3.7 client, for the local ped and for remote player peds.
//!
//! Loaded by an ASI loader. `DllMain` redirects one call in GTA's `Idle()` (entry A,
//! see hooks.rs) and starts a watchdog thread that logs a sign of life, installs
//! entry B (Direct3D Present) and reports what happens. The test itself runs on
//! the game thread (logic.rs).

#![allow(clippy::missing_safety_doc)]

mod game;
mod hooks;
mod log;
mod logic;
mod outfit;
mod samp;
mod watchdog;

use std::sync::atomic::Ordering;

use winapi::shared::minwindef::{BOOL, DWORD, HINSTANCE, LPVOID, TRUE};
use winapi::um::libloaderapi::DisableThreadLibraryCalls;
use winapi::um::winnt::DLL_PROCESS_ATTACH;

pub const ASI_VERSION: &str = env!("CARGO_PKG_VERSION");

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "system" fn DllMain(instance: HINSTANCE, reason: DWORD, _reserved: LPVOID) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        DisableThreadLibraryCalls(instance);
        watchdog::MODULE.store(instance as usize, Ordering::SeqCst);

        // only memory writes under the loader lock; logging happens in the watchdog thread
        hooks::install_process_hook();
        watchdog::start();
    }

    TRUE
}
