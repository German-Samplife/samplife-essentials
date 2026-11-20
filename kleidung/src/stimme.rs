//! Settings of the voice chat (SampVoice's own menu, F11) in the server's control panel
//! (`/icp`, tab Audio).
//!
//! `sampvoice.asi` from `sampvoice-3.1-sl6` exports C functions for them
//! (sampvoice/einstellungen-export.patch). We look them up at runtime with
//! GetModuleHandle/GetProcAddress, so an older `sampvoice.asi` only means "not available"
//! (`{"da":0}`) and the page keeps pointing to F11.
//!
//! The server only passes events on and stores nothing; SampVoice saves everything itself in
//! `svconfig.bin`. Every request is answered with the whole state:
//!
//! | Event | Direction | Arguments |
//! |---|---|---|
//! | `stimme:frage` | server -> client | none |
//! | `stimme:setzen` | server -> client | field (see `feld`) and value |
//! | `stimme:stumm` | server -> client | player id; muted for this player only, like in F11 |
//! | `stimme:frei` | server -> client | player name; mute lifted (also for players not online) |
//! | `stimme:stand` | client -> server | JSON, ASCII only (see `stand_json`) |
//!
//! Called on the game thread (logic.rs), like SampVoice's menu and voice client
//! (`Plugin::OnRender`).

use std::ffi::{c_char, c_int, CString};

use winapi::um::libloaderapi::{GetModuleHandleA, GetProcAddress};

/// Names of the input devices: at most this many, each cut to this many characters
const MAX_GERAETE: usize = 12;
const MAX_NAME: usize = 64;
/// Muted players in the answer
const MAX_STUMM: usize = 40;

// Fields of SamplifeStimmeLesen/-Setzen (Export.cpp)
const TON: c_int = 0;
const LAUTSTAERKE: c_int = 1;
const ANGLEICHEN: c_int = 2;
const FILTER: c_int = 3;
const MIKRO: c_int = 4;
const MIKRO_LAUTSTAERKE: c_int = 5;
const GERAET: c_int = 6;
const TEST: c_int = 7;
const WIEDERGABE: c_int = 8;
const AUFNAHME: c_int = 9;
const GERAETE: c_int = 10;
const STUMM: c_int = 11;

type Version = unsafe extern "C" fn() -> c_int;
type Lesen = unsafe extern "C" fn(c_int) -> c_int;
type Setzen = unsafe extern "C" fn(c_int, c_int) -> c_int;
type Text = unsafe extern "C" fn(c_int, *mut c_char, c_int) -> c_int;
type Stumm = unsafe extern "C" fn(c_int) -> c_int;
type Frei = unsafe extern "C" fn(*const c_char) -> c_int;

#[derive(Clone, Copy)]
struct Exporte {
    lesen: Lesen,
    setzen: Setzen,
    geraet: Text,
    stumm_eintrag: Text,
    stumm: Stumm,
    frei: Frei,
}

/// Field name in `stimme:setzen` -> field number and the highest value
pub fn feld(name: &str) -> Option<(c_int, c_int)> {
    Some(match name {
        "ton" => (TON, 1),
        "lautstaerke" => (LAUTSTAERKE, 100),
        "angleichen" => (ANGLEICHEN, 1),
        "filter" => (FILTER, 1),
        "mikro" => (MIKRO, 1),
        "mikrolautstaerke" => (MIKRO_LAUTSTAERKE, 100),
        "geraet" => (GERAET, MAX_GERAETE as c_int - 1),
        "test" => (TEST, 1),
        _ => return None,
    })
}

/// SA-MP player names: up to 24 characters of these
pub fn gueltiger_name(name: &str) -> bool {
    (1..=24).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_[]$=()@.".contains(&b))
}

pub struct Stimme {
    exporte: Option<Exporte>,
    /// why not available (for /kleidunginfo)
    grund: &'static str,
    pub anfragen: u32,
    pub abgelehnt: u32,
}

impl Stimme {
    pub fn new() -> Stimme {
        Stimme {
            exporte: None,
            grund: "noch nicht gefragt",
            anfragen: 0,
            abgelehnt: 0,
        }
    }

    pub fn zustand(&self) -> String {
        match self.exporte {
            Some(_) => format!("sampvoice.asi mit Exporten, {} Anfragen, {} abgelehnt", self.anfragen, self.abgelehnt),
            None => format!("nicht verfügbar ({})", self.grund),
        }
    }

    /// The exports of sampvoice.asi; looked up again until found (the ASI loads with GTA anyway)
    fn exporte(&mut self) -> Option<Exporte> {
        if self.exporte.is_none() {
            match finden() {
                Ok(exporte) => {
                    crate::log!("sampvoice.asi: Exporte für /icp gefunden");
                    self.exporte = Some(exporte);
                }
                Err(grund) => {
                    if grund != self.grund {
                        crate::log!("sampvoice.asi: Einstellungen für /icp nicht verfügbar ({grund})");
                    }
                    self.grund = grund;
                }
            }
        }

        self.exporte
    }

    // -----------------------------------------------------------------------
    // Server events, game thread. Each one answers with the state.

    pub fn frage(&mut self) {
        self.anfragen += 1;
        self.antworten();
    }

    pub fn setzen(&mut self, feld: c_int, wert: c_int) {
        self.anfragen += 1;

        if let Some(e) = self.exporte() {
            if unsafe { (e.setzen)(feld, wert) } != 1 {
                self.abgelehnt += 1;
                crate::log!("stimme:setzen {feld} {wert}: von SampVoice abgelehnt (nicht geladen oder ungültig)");
            }
        }

        self.antworten();
    }

    pub fn stumm(&mut self, spieler: u16) {
        self.anfragen += 1;

        if let Some(e) = self.exporte() {
            if unsafe { (e.stumm)(c_int::from(spieler)) } != 1 {
                self.abgelehnt += 1;
            }
        }

        self.antworten();
    }

    pub fn frei(&mut self, name: &str) {
        self.anfragen += 1;

        if let (Some(e), Ok(name)) = (self.exporte(), CString::new(name)) {
            if unsafe { (e.frei)(name.as_ptr()) } != 1 {
                self.abgelehnt += 1;
            }
        }

        self.antworten();
    }

    /// Connection to the server ended: no microphone test left running
    pub fn getrennt(&mut self) {
        if let Some(e) = self.exporte {
            if unsafe { (e.lesen)(TEST) } == 1 {
                unsafe { (e.setzen)(TEST, 0) };
            }
        }
    }

    fn antworten(&mut self) {
        let json = match self.exporte() {
            Some(e) => stand_json(&lesen(e)),
            None => "{\"da\":0}".to_owned(),
        };

        crate::netz::stimme_stand(&json);
    }
}

fn finden() -> Result<Exporte, &'static str> {
    let modul = unsafe { GetModuleHandleA(c"sampvoice.asi".as_ptr()) };

    if modul.is_null() {
        return Err("sampvoice.asi nicht geladen");
    }

    let suchen = |name: &std::ffi::CStr| {
        let adresse = unsafe { GetProcAddress(modul, name.as_ptr()) };
        (!adresse.is_null()).then_some(adresse as usize)
    };

    let Some(version) = suchen(c"SamplifeStimmeVersion") else {
        return Err("sampvoice.asi ohne Exporte, älter als sampvoice-3.1-sl6");
    };

    let version: Version = unsafe { std::mem::transmute(version) };

    if unsafe { version() } < 1 {
        return Err("unbekannter Stand der Exporte");
    }

    match (
        suchen(c"SamplifeStimmeLesen"),
        suchen(c"SamplifeStimmeSetzen"),
        suchen(c"SamplifeStimmeGeraet"),
        suchen(c"SamplifeStimmeStummEintrag"),
        suchen(c"SamplifeStimmeStumm"),
        suchen(c"SamplifeStimmeFrei"),
    ) {
        (Some(lesen), Some(setzen), Some(geraet), Some(stumm_eintrag), Some(stumm), Some(frei)) => unsafe {
            Ok(Exporte {
                lesen: std::mem::transmute::<usize, Lesen>(lesen),
                setzen: std::mem::transmute::<usize, Setzen>(setzen),
                geraet: std::mem::transmute::<usize, Text>(geraet),
                stumm_eintrag: std::mem::transmute::<usize, Text>(stumm_eintrag),
                stumm: std::mem::transmute::<usize, Stumm>(stumm),
                frei: std::mem::transmute::<usize, Frei>(frei),
            })
        },
        _ => Err("Exporte unvollständig"),
    }
}

/// Everything the page shows
#[derive(Default, Debug, PartialEq)]
pub struct Stand {
    pub wiedergabe: bool,
    pub aufnahme: bool,
    pub ton: bool,
    pub lautstaerke: i32,
    pub angleichen: bool,
    pub filter: bool,
    pub mikro: bool,
    pub mikro_lautstaerke: i32,
    pub geraet: i32,
    pub test: bool,
    pub geraete: Vec<String>,
    /// name and player id (None: not online)
    pub stumm: Vec<(String, Option<u16>)>,
}

fn text(funktion: Text, index: c_int) -> Option<(String, c_int)> {
    let mut puffer = [0u8; 256];
    let rueckgabe = unsafe { funktion(index, puffer.as_mut_ptr().cast(), puffer.len() as c_int) };
    let ende = puffer.iter().position(|&b| b == 0).unwrap_or(puffer.len());
    (rueckgabe >= 0).then(|| (String::from_utf8_lossy(&puffer[..ende]).into_owned(), rueckgabe))
}

fn lesen(e: Exporte) -> Stand {
    let wert = |feld| unsafe { (e.lesen)(feld) };

    let geraete = (0..wert(GERAETE).clamp(0, MAX_GERAETE as c_int))
        .map_while(|i| text(e.geraet, i).map(|(name, _)| name))
        .collect();

    let stumm = (0..wert(STUMM).clamp(0, MAX_STUMM as c_int))
        .map_while(|i| text(e.stumm_eintrag, i))
        .map(|(name, id)| (name, u16::try_from(id).ok().filter(|&id| id < 1000)))
        .collect();

    Stand {
        wiedergabe: wert(WIEDERGABE) == 1,
        aufnahme: wert(AUFNAHME) == 1,
        ton: wert(TON) == 1,
        lautstaerke: wert(LAUTSTAERKE),
        angleichen: wert(ANGLEICHEN) == 1,
        filter: wert(FILTER) == 1,
        mikro: wert(MIKRO) == 1,
        mikro_lautstaerke: wert(MIKRO_LAUTSTAERKE),
        geraet: wert(GERAET),
        test: wert(TEST) == 1,
        geraete,
        stumm,
    }
}

/// JSON string with everything outside of printable ASCII as \uXXXX, so it passes the server
/// (Windows-1252) and samp-cef unchanged.
fn json_text(text: &str, max: usize) -> String {
    let mut aus = String::from("\"");

    for c in text.chars().take(max) {
        match c {
            '"' => aus.push_str("\\\""),
            '\\' => aus.push_str("\\\\"),
            ' '..='~' => aus.push(c),
            _ => {
                let mut einheiten = [0u16; 2];
                for einheit in c.encode_utf16(&mut einheiten) {
                    aus.push_str(&format!("\\u{einheit:04x}"));
                }
            }
        }
    }

    aus.push('"');
    aus
}

pub fn stand_json(s: &Stand) -> String {
    let b = |wert: bool| u8::from(wert);
    let geraete: Vec<String> = s.geraete.iter().map(|name| json_text(name, MAX_NAME)).collect();
    let stumm: Vec<String> = s
        .stumm
        .iter()
        .map(|(name, id)| format!("{{\"name\":{},\"id\":{}}}", json_text(name, 24), id.map_or(-1, i32::from)))
        .collect();

    format!(
        "{{\"da\":1,\"wiedergabe\":{},\"aufnahme\":{},\"ton\":{},\"lautstaerke\":{},\"angleichen\":{},\"filter\":{},\"mikro\":{},\
         \"mikrolautstaerke\":{},\"geraet\":{},\"test\":{},\"geraete\":[{}],\"stumm\":[{}]}}",
        b(s.wiedergabe),
        b(s.aufnahme),
        b(s.ton),
        s.lautstaerke.clamp(0, 100),
        b(s.angleichen),
        b(s.filter),
        b(s.mikro),
        s.mikro_lautstaerke.clamp(0, 100),
        s.geraet.max(-1),
        b(s.test),
        geraete.join(","),
        stumm.join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_nur_ascii() {
        assert_eq!(json_text("Mikrofon (Realtek(R) Audio)", 64), "\"Mikrofon (Realtek(R) Audio)\"");
        assert_eq!(json_text("Kopfhörer \"Pro\"", 64), "\"Kopfh\\u00f6rer \\\"Pro\\\"\"");
        assert_eq!(json_text("Микрофон", 3), "\"\\u041c\\u0438\\u043a\"");
        assert_eq!(json_text("a😀", 64), "\"a\\ud83d\\ude00\"");
        assert!(json_text("x\ny\\", 64).is_ascii());
    }

    #[test]
    fn stand_als_json() {
        let stand = Stand {
            wiedergabe: true,
            aufnahme: true,
            ton: true,
            lautstaerke: 80,
            mikro: true,
            mikro_lautstaerke: 75,
            geraet: 1,
            geraete: vec!["A".into(), "B".into()],
            stumm: vec![("Max_Muster".into(), Some(4)), ("Weg".into(), None)],
            ..Stand::default()
        };

        assert_eq!(
            stand_json(&stand),
            "{\"da\":1,\"wiedergabe\":1,\"aufnahme\":1,\"ton\":1,\"lautstaerke\":80,\"angleichen\":0,\"filter\":0,\"mikro\":1,\
             \"mikrolautstaerke\":75,\"geraet\":1,\"test\":0,\"geraete\":[\"A\",\"B\"],\
             \"stumm\":[{\"name\":\"Max_Muster\",\"id\":4},{\"name\":\"Weg\",\"id\":-1}]}"
        );
    }

    #[test]
    fn felder_und_namen() {
        assert_eq!(feld("lautstaerke"), Some((LAUTSTAERKE, 100)));
        assert_eq!(feld("test"), Some((TEST, 1)));
        assert_eq!(feld("wiedergabe"), None);
        assert!(gueltiger_name("Max_Muster"));
        assert!(gueltiger_name("[GSL]Tom.99"));
        assert!(!gueltiger_name("Max Muster"));
        assert!(!gueltiger_name(""));
        assert!(!gueltiger_name("a_very_long_name_over_24c"));
    }
}
