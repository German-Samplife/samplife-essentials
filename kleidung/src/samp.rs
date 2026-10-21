//! SA-MP 0.3.7 R1/R3: chat output, client command, local and remote peds.
//!
//! Version detection, player pool and CInput come from samp-client-api (zottce,
//! rev d497ffad). samp-client-api has no chat and no AddCmdProc, so those two
//! functions are called directly. Addresses (offsets from samp.dll) from
//! BlastHackNet/SAMP-API (https://github.com/BlastHackNet/SAMP-API, branch multiver):
//!
//! | | 0.3.7 R1 (src/sampapi/0.3.7-R1) | 0.3.7 R3 (src/sampapi/0.3.7-R3-1) |
//! |---|---|---|
//! | `RefChat()` (CChat*) | 0x21A0E4 | 0x26E8C8 |
//! | `CChat::AddMessage(D3DCOLOR, const char*)` thiscall | 0x645A0 | 0x679F0 |
//! | `RefInputBox()` (CInput*) | 0x21A0E8 | 0x26E8CC |
//! | `CInput::AddCommand(const char*, CMDPROC)` thiscall | 0x65AD0 | 0x69000 |
//!
//! The CInput pointers are the same as `CINPUT` in samp-client-api (v037.rs, v037r3.rs).
//! 0.3.DL (src/sampapi/0.3.DL-1): `RefChat()` 0x2ACA10, `AddMessage` 0x67BE0 - only used
//! to say once that DL is not supported.

use std::ffi::{c_char, c_void, CStr};

use client_api::samp::version::{version, Version};

use crate::game::{readable, read_ptr};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SampVersion {
    R1,
    R3,
    Dl,
}

impl SampVersion {
    pub fn name(self) -> &'static str {
        match self {
            SampVersion::R1 => "0.3.7 R1",
            SampVersion::R3 => "0.3.7 R3",
            SampVersion::Dl => "0.3.DL",
        }
    }

    fn chat_offsets(self) -> (usize, usize) {
        match self {
            SampVersion::R1 => (0x21A0E4, 0x645A0),
            SampVersion::R3 => (0x26E8C8, 0x679F0),
            SampVersion::Dl => (0x2ACA10, 0x67BE0),
        }
    }

    fn input_offsets(self) -> Option<(usize, usize)> {
        match self {
            SampVersion::R1 => Some((client_api::samp::v037::CINPUT, 0x65AD0)),
            SampVersion::R3 => Some((client_api::samp::v037r3::CINPUT, 0x69000)),
            SampVersion::Dl => None,
        }
    }
}

pub fn loaded() -> bool {
    client_api::samp::is_loaded()
}

fn base() -> usize {
    client_api::samp::handle() as usize
}

/// None: samp.dll has a version we don't know at all (R2, R4, R5, ...)
pub fn detect() -> Option<SampVersion> {
    match version() {
        Version::V037 => Some(SampVersion::R1),
        Version::V037R3 => Some(SampVersion::R3),
        Version::V03DL => Some(SampVersion::Dl),
        _ => None,
    }
}

pub fn chat(ver: SampVersion) -> Option<usize> {
    let (chat, _) = ver.chat_offsets();
    let ptr = read_ptr(base() + chat)?;
    readable(ptr, 4).then_some(ptr)
}

pub fn input(ver: SampVersion) -> Option<usize> {
    let (input, _) = ver.input_offsets()?;
    let ptr = read_ptr(base() + input)?;
    readable(ptr, 4).then_some(ptr)
}

pub fn netgame_ready(ver: SampVersion) -> bool {
    match ver {
        SampVersion::R1 => !client_api::samp::v037::netgame().is_null(),
        SampVersion::R3 => !client_api::samp::v037r3::netgame().is_null(),
        SampVersion::Dl => false,
    }
}

/// Text to Windows-1252 (SA-MP's chat encoding); umlauts are Latin-1 there.
fn to_ansi(text: &str) -> Vec<u8> {
    let mut out: Vec<u8> = text
        .chars()
        .map(|c| {
            let code = c as u32;
            if code == 0 {
                b' '
            } else if code < 0x80 || (0xA0..=0xFF).contains(&code) {
                code as u8
            } else {
                b'?'
            }
        })
        .collect();
    out.truncate(143);
    out.push(0);
    out
}

/// One chat line, local only. Color as 0xAARRGGBB, `{RRGGBB}` inside the text works.
pub fn chat_line(ver: SampVersion, color: u32, text: &str) {
    let Some(chat) = chat(ver) else {
        crate::log!("Chat nicht bereit: {text}");
        return;
    };

    let (_, add_message) = ver.chat_offsets();
    let bytes = to_ansi(text);

    let add_message: extern "thiscall" fn(*mut c_void, u32, *const c_char) =
        unsafe { std::mem::transmute(base() + add_message) };

    add_message(chat as *mut c_void, color, bytes.as_ptr() as *const c_char);
}

pub type CmdProc = unsafe extern "C" fn(*const c_char);

/// Registers a client command with CInput::AddCommand. Returns false if CInput is
/// missing, the name is already registered or the table is full.
pub fn register_command(ver: SampVersion, name: &'static CStr, proc_: CmdProc) -> bool {
    let Some(input_ptr) = input(ver) else {
        return false;
    };
    let Some((_, add_command)) = ver.input_offsets() else {
        return false;
    };

    // CInput layout from samp-client-api (inputs.rs, same for R1 and R3)
    let Some(input) = client_api::samp::inputs::Input::get()
        .map(|input| input as *mut client_api::samp::inputs::Input)
    else {
        return false;
    };

    if input as usize != input_ptr {
        crate::log!("CInput: Zeiger passt nicht ({:#x} / {:#x})", input as usize, input_ptr);
        return false;
    }

    let count = || unsafe { std::ptr::addr_of!((*input).m_nCommandCount).read_unaligned() };
    let count_before = count();

    if !(0..144).contains(&count_before) {
        crate::log!("CInput: unerwartete Befehlsanzahl {count_before}");
        return false;
    }

    for i in 0..count_before as usize {
        let entry = unsafe { std::ptr::addr_of!((*input).m_szCommandName[i]).read_unaligned() };
        let len = entry.iter().position(|&c| c == 0).unwrap_or(entry.len());
        let existing: Vec<u8> = entry[..len].iter().map(|&c| c as u8).collect();

        if existing.eq_ignore_ascii_case(name.to_bytes()) {
            crate::log!("Befehl /{} ist schon registriert", name.to_string_lossy());
            return true;
        }
    }

    let add: extern "thiscall" fn(*mut c_void, *const c_char, Option<CmdProc>) =
        unsafe { std::mem::transmute(base() + add_command) };

    add(input_ptr as *mut c_void, name.as_ptr(), Some(proc_));

    let count_after = count();
    crate::log!("CInput::AddCommand: Befehle vorher {count_before}, nachher {count_after}");
    count_after == count_before + 1
}

/// GTA ped (CPed*) of the local SA-MP player, falls back to CWorld::Players[0].
pub fn local_ped(ver: SampVersion) -> Option<usize> {
    let from_samp = unsafe {
        match ver {
            SampVersion::R1 => client_api::samp::v037::local_player().and_then(|local| {
                let ped = local.m_pPed;
                if ped.is_null() || !readable(ped as usize, 0x2A8) {
                    None
                } else {
                    Some(std::ptr::addr_of!((*ped).m_pGamePed).read_unaligned() as usize)
                }
            }),
            SampVersion::R3 => client_api::samp::v037r3::local_player().and_then(|local| {
                let ped = local.m_pPed;
                if ped.is_null() || !readable(ped as usize, 0x2A8) {
                    None
                } else {
                    Some(std::ptr::addr_of!((*ped).m_pGamePed).read_unaligned() as usize)
                }
            }),
            SampVersion::Dl => None,
        }
    };

    from_samp
        .filter(|&p| p != 0)
        .or_else(crate::game::world_player0_ped)
}

/// SA-MP player id of the local player (CPlayerPool::m_localInfo.m_nId).
pub fn local_id() -> Option<u16> {
    client_api::samp::players::local_player()?
        .id()
        .and_then(|id| u16::try_from(id).ok())
}

/// GTA peds (CPed*) of all streamed remote players, with their SA-MP player id.
pub fn remote_peds() -> Vec<(u16, usize)> {
    let mut out = Vec::new();

    if let Some(players) = client_api::samp::players::players() {
        for player in players {
            let id = player.remote_player().map(|remote| remote.id()).unwrap_or(u16::MAX);

            if let Some(ped) = player.gta_ped() {
                let ped = ped as *const _ as usize;

                if ped != 0 {
                    out.push((id, ped));
                }
            }
        }
    }

    out
}
