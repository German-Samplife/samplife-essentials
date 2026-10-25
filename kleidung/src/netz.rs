//! Connection to the server over samp-cef (client plugin, see cef/plugin-ereignisse.patch).
//!
//! samp-cef loads every DLL in `GTA/cef/plugins/` and, with our patch, hands it
//! every event the server sends with `cef_emit_event` (`cef_server_event`) and a
//! function to send events to the server (`cef_server_emitter`). Events starting with
//! `kleidung:` or `anim:` are ours and don't go on to the browsers.
//!
//! | Event | Direction | Arguments |
//! |---|---|---|
//! | `kleidung:hallo` | client -> server | ASI version; after connecting and on `kleidung:frage` |
//! | `kleidung:frage` | server -> client | none; the client answers with `kleidung:hallo` |
//! | `kleidung:spieler` | server -> client | player id, 18 parts (see outfit::parse_teile) |
//! | `kleidung:weg` | server -> client | player id; outfit forgotten, the player wears the standard CJ |
//! | `anim:start` | server -> client | player id, pose name (anim.rs); partial animation until `anim:stopp` |
//! | `anim:stopp` | server -> client | player id; the pose fades out |
//!
//! The callbacks run on samp-cef's thread; they only queue. The game thread takes the
//! queue in logic.rs.

use std::ffi::{c_char, c_int, CStr, CString};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::outfit::{self, Teile};

pub type Emitter = unsafe extern "C" fn(event: *const c_char, arguments: *const c_char);

/// samp-cef: do not hand the event on (to the browsers)
pub const EXTERNAL_BREAK: c_int = 1;
pub const EXTERNAL_CONTINUE: c_int = 0;

const PREFIX: &str = "kleidung:";
const PREFIX_ANIM: &str = "anim:";

pub enum Nachricht {
    Spieler(u16, Box<Teile>),
    Weg(u16),
    AnimStart(u16, String),
    AnimStopp(u16),
    Getrennt,
}

static EMITTER: AtomicUsize = AtomicUsize::new(0);
static VERBUNDEN: AtomicBool = AtomicBool::new(false);
static INBOX: Mutex<Vec<Nachricht>> = Mutex::new(Vec::new());

// read by the watchdog and /kleidunginfo
pub static EMPFANGEN: AtomicU32 = AtomicU32::new(0);
pub static VERWORFEN: AtomicU32 = AtomicU32::new(0);
pub static HALLO_GESENDET: AtomicU32 = AtomicU32::new(0);

fn push(nachricht: Nachricht) {
    let mut inbox = match INBOX.lock() {
        Ok(inbox) => inbox,
        Err(poisoned) => poisoned.into_inner(),
    };

    inbox.push(nachricht);
}

/// Game thread: everything that arrived since the last frame.
pub fn take() -> Vec<Nachricht> {
    match INBOX.lock() {
        Ok(mut inbox) => inbox.drain(..).collect(),
        Err(poisoned) => poisoned.into_inner().drain(..).collect(),
    }
}

/// True once samp-cef has loaded us as a plugin with our patch.
pub fn plugin_aktiv() -> bool {
    EMITTER.load(Ordering::SeqCst) != 0
}

pub fn verbunden() -> bool {
    VERBUNDEN.load(Ordering::SeqCst)
}

pub fn set_emitter(emitter: Emitter) {
    EMITTER.store(emitter as usize, Ordering::SeqCst);
    crate::log!("samp-cef: Plugin geladen, Server-Ereignisse aktiv");
}

fn emit(event: &str, arguments: &str) -> bool {
    let emitter = EMITTER.load(Ordering::SeqCst);

    if emitter == 0 {
        return false;
    }

    let (Ok(event), Ok(arguments)) = (CString::new(event), CString::new(arguments)) else {
        return false;
    };

    let emitter: Emitter = unsafe { std::mem::transmute(emitter) };
    unsafe { emitter(event.as_ptr(), arguments.as_ptr()) };
    true
}

fn hallo() {
    if emit("kleidung:hallo", crate::ASI_VERSION) {
        HALLO_GESENDET.fetch_add(1, Ordering::Relaxed);
        crate::log!("kleidung:hallo {} gesendet", crate::ASI_VERSION);
    } else {
        crate::log!("kleidung:hallo nicht gesendet: samp-cef ohne Plugin-Ereignisse (Client älter als 0x000300?)");
    }
}

pub fn connect() {
    VERBUNDEN.store(true, Ordering::SeqCst);
    crate::log!("samp-cef: mit dem Server verbunden");
    hallo();
}

pub fn disconnect() {
    VERBUNDEN.store(false, Ordering::SeqCst);
    crate::log!("samp-cef: Verbindung zum Server getrennt, alle Outfits vergessen");
    push(Nachricht::Getrennt);
}

fn parse_id(token: Option<&str>) -> Option<u16> {
    token?.parse::<u16>().ok().filter(|&id| id < 1000)
}

/// `cef_server_event`: every event from the server. Returns `EXTERNAL_BREAK` for ours.
pub fn server_event(event: &CStr, arguments: &CStr) -> c_int {
    let Ok(event) = event.to_str() else {
        return EXTERNAL_CONTINUE;
    };

    // (prefix, name): kleidung:frage -> ("kleidung:", "frage"), anim:start -> ("anim:", "start")
    let Some((prefix, name)) = [PREFIX, PREFIX_ANIM]
        .into_iter()
        .find_map(|prefix| event.strip_prefix(prefix).map(|name| (prefix, name)))
    else {
        return EXTERNAL_CONTINUE;
    };

    EMPFANGEN.fetch_add(1, Ordering::Relaxed);

    let arguments = arguments.to_string_lossy();
    let mut tokens = arguments.split_whitespace();

    let ergebnis = match (prefix, name) {
        (PREFIX, "frage") => {
            hallo();
            Ok(())
        }

        (PREFIX, "spieler") => match parse_id(tokens.next()) {
            None => Err("Spieler-ID fehlt oder ungültig".to_owned()),
            Some(id) => outfit::parse_teile(tokens).map(|teile| push(Nachricht::Spieler(id, Box::new(teile)))),
        },

        (PREFIX, "weg") => match parse_id(tokens.next()) {
            None => Err("Spieler-ID fehlt oder ungültig".to_owned()),
            Some(id) => {
                push(Nachricht::Weg(id));
                Ok(())
            }
        },

        (PREFIX_ANIM, "start") => match (parse_id(tokens.next()), tokens.next(), tokens.next()) {
            (None, _, _) => Err("Spieler-ID fehlt oder ungültig".to_owned()),
            (Some(_), None, _) => Err("Name der Pose fehlt".to_owned()),
            (Some(_), Some(_), Some(_)) => Err("zu viele Argumente".to_owned()),
            (Some(_), Some(pose), None) if !crate::anim::gueltiger_name(pose) => Err("Name der Pose ungültig".to_owned()),
            (Some(id), Some(pose), None) => {
                push(Nachricht::AnimStart(id, pose.to_owned()));
                Ok(())
            }
        },

        (PREFIX_ANIM, "stopp") => match parse_id(tokens.next()) {
            None => Err("Spieler-ID fehlt oder ungültig".to_owned()),
            Some(id) => {
                push(Nachricht::AnimStopp(id));
                Ok(())
            }
        },

        _ => Err("unbekanntes Ereignis".to_owned()),
    };

    if let Err(grund) = ergebnis {
        VERWORFEN.fetch_add(1, Ordering::Relaxed);
        let kurz: String = arguments.chars().take(200).collect();
        crate::log!("{event} verworfen ({grund}): {kurz}");
    }

    EXTERNAL_BREAK
}
