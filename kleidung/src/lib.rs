//! samplife-kleidung: CJ's (skin 0) clothes per player in the SA-MP 0.3.7 client,
//! for the local ped and for remote player peds, with the outfits the server sends.
//!
//! A samp-cef client plugin (`GTA/cef/plugins/samplife-kleidung.dll`): samp-cef
//! loads it and hands it the server's events (netz.rs, needs cef/plugin-ereignisse.patch).
//! `DllMain` redirects one call in GTA's `Idle()` (entry A, see hooks.rs) and starts a
//! watchdog thread that logs a sign of life, installs entry B (Direct3D Present) and
//! reports what happens. The clothes logic runs on the game thread (logic.rs), and so do
//! the partial animations (poses) the server starts per player (anim.rs).

#![allow(clippy::missing_safety_doc)]

mod anim;
mod diag;
mod game;
mod hooks;
mod log;
mod logic;
mod netz;
mod outfit;
mod rebuild;
mod samp;
mod watchdog;

use std::ffi::{c_char, c_int, CStr};
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

// ---------------------------------------------------------------------------
// samp-cef client plugin (client/src/external.rs with cef/plugin-ereignisse.patch).
// Never unwind into samp-cef.

/// Called once right after samp-cef loaded the DLL: sends events to the server.
#[no_mangle]
pub extern "C" fn cef_server_emitter(emitter: netz::Emitter) {
    let _ = std::panic::catch_unwind(|| netz::set_emitter(emitter));
}

/// Every event the server sends with `cef_emit_event`, arguments joined by spaces.
#[no_mangle]
pub unsafe extern "C" fn cef_server_event(event: *const c_char, arguments: *const c_char) -> c_int {
    if event.is_null() || arguments.is_null() {
        return netz::EXTERNAL_CONTINUE;
    }

    let (event, arguments) = (CStr::from_ptr(event), CStr::from_ptr(arguments));

    std::panic::catch_unwind(|| netz::server_event(event, arguments)).unwrap_or(netz::EXTERNAL_CONTINUE)
}

/// Connected to the CEF server of the SA-MP server.
#[no_mangle]
pub extern "C" fn cef_connect() {
    let _ = std::panic::catch_unwind(netz::connect);
}

/// Connection to the CEF server lost (also when leaving the SA-MP server).
#[no_mangle]
pub extern "C" fn cef_disconnect() {
    let _ = std::panic::catch_unwind(netz::disconnect);
}
