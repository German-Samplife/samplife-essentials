//! Diagnosis: compares the code of the clothes path in memory with gta_sa.exe on
//! disk and can put the original bytes back for the duration of one call.
//!
//! Addresses (GTA SA 1.0 US) with sources:
//! - gta-reversed ClothesBuilder.cpp: LoadCdDirectory 0x5A4190 ... CreateSkinnedClump 0x5A69D0
//! - gta-reversed PedClothesDesc.cpp: Initialise 0x5A78F0 ... SetTextureAndModel 0x5A8080
//! - gta-reversed Clothes.cpp: Init 0x5A80D0, ConstructPedModel 0x5A81E0,
//!   RebuildPlayer 0x5A82C0, RebuildPlayerIfNeeded 0x5A8390
//! - gta-reversed Ped.cpp: Dress 0x5E0130, SetModelIndex 0x5E4880, DeleteRwObject 0x5DEBF0
//! - gta-reversed Models/PedModelInfo.cpp: DeleteRwObject 0x4C6C50, SetClump 0x4C7340
//! - gta-reversed Models/ClumpModelInfo.cpp: SetClump 0x4C4F70, CreateInstance 0x4C5140
//! - gta-reversed Streaming.cpp: RequestFile 0x40A080, LoadAllRequestedModels 0x40EA10
//! - gta-reversed Timer.cpp: Suspend 0x5619D0, Resume 0x561A00
//! - gta-reversed World.cpp: Add 0x563220, Remove 0x563280
//! - gta-reversed RpAnimBlend.cpp: RpAnimBlendClumpExtractAssociations 0x4D6BE0,
//!   RpAnimBlendClumpGiveAssociations 0x4D6C30
//! - gta-reversed Core/KeyGen.cpp: GetUppercaseKey 0x53CF30

use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

use winapi::shared::minwindef::HMODULE;
use winapi::um::libloaderapi::{
    GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
};
use winapi::um::memoryapi::VirtualProtect;
use winapi::um::processthreadsapi::{FlushInstructionCache, GetCurrentProcess};
use winapi::um::winnt::PAGE_EXECUTE_READWRITE;

use crate::game::readable;

/// (start, length, name). Lengths are generous; a difference may also belong to the
/// next function.
const RANGES: &[(usize, usize, &str)] = &[
    (0x5A4190, 0x5A8400 - 0x5A4190, "CClothesBuilder/CPedClothesDesc/CClothes (0x5A4190-0x5A8400)"),
    (0x5E0130, 0x40, "CPed::Dress"),
    (0x5E4880, 0x100, "CPed::SetModelIndex"),
    (0x5DEBF0, 0x60, "CPed::DeleteRwObject"),
    (0x4C6C50, 0x80, "CPedModelInfo::DeleteRwObject"),
    (0x4C7340, 0x100, "CPedModelInfo::SetClump"),
    (0x4C4F70, 0x100, "CClumpModelInfo::SetClump"),
    (0x4C5140, 0x80, "CClumpModelInfo::CreateInstance"),
    (0x40A080, 0x40, "CStreaming::RequestFile"),
    (0x40EA10, 0x40, "CStreaming::LoadAllRequestedModels"),
    (0x5619D0, 0x30, "CTimer::Suspend"),
    (0x561A00, 0x30, "CTimer::Resume"),
    (0x563220, 0x40, "CWorld::Add"),
    (0x563280, 0x40, "CWorld::Remove"),
    (0x4D6BE0, 0x40, "RpAnimBlendClumpExtractAssociations"),
    (0x4D6C30, 0x40, "RpAnimBlendClumpGiveAssociations"),
    (0x53CF30, 0x30, "CKeyGen::GetUppercaseKey"),
];

const IMAGE_BASE: usize = 0x400000;

#[derive(Clone)]
pub struct Patch {
    pub addr: usize,
    pub original: Vec<u8>,
    pub memory: Vec<u8>,
    pub range: &'static str,
    pub note: String,
}

static EXE: OnceLock<Option<Vec<u8>>> = OnceLock::new();
static LAST: Mutex<Option<Vec<Patch>>> = Mutex::new(None);

fn exe_bytes() -> Option<&'static Vec<u8>> {
    EXE.get_or_init(|| {
        let path = std::env::current_exe().ok()?;
        match std::fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) => {
                crate::log!("Diagnose: {} nicht lesbar: {error}", path.display());
                None
            }
        }
    })
    .as_ref()
}

fn u16_at(data: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?) as usize)
}

fn u32_at(data: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?) as usize)
}

/// File offset of a virtual address, from the PE section table.
fn file_offset(exe: &[u8], va: usize) -> Option<usize> {
    let pe = u32_at(exe, 0x3C)?;
    if exe.get(pe..pe + 4)? != b"PE\0\0" {
        return None;
    }
    let sections = u16_at(exe, pe + 6)?;
    let optional = u16_at(exe, pe + 20)?;
    let table = pe + 24 + optional;
    let rva = va.checked_sub(IMAGE_BASE)?;

    for i in 0..sections {
        let s = table + i * 40;
        let virtual_size = u32_at(exe, s + 8)?;
        let virtual_address = u32_at(exe, s + 12)?;
        let raw_size = u32_at(exe, s + 16)?;
        let raw_pointer = u32_at(exe, s + 20)?;
        let size = virtual_size.max(raw_size);

        if rva >= virtual_address && rva < virtual_address + size {
            let inside = rva - virtual_address;
            return (inside < raw_size).then_some(raw_pointer + inside);
        }
    }

    None
}

pub fn module_of(addr: usize) -> String {
    let mut module: HMODULE = std::ptr::null_mut();
    let ok = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            addr as *const u16,
            &mut module,
        )
    };

    if ok == 0 || module.is_null() {
        return "kein Modul".to_owned();
    }

    let mut buffer = vec![0u16; 512];
    let len = unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    let path = String::from_utf16_lossy(&buffer[..len.min(buffer.len())]);
    path.rsplit(['\\', '/']).next().unwrap_or(&path).to_owned()
}

/// What a patched byte run probably is: call/jmp into which module, ret, nop.
fn describe(addr: usize, mem: &[u8], whole: &dyn Fn(usize) -> Option<u8>) -> String {
    let rel_target = |p: usize| -> Option<usize> {
        let mut rel = [0u8; 4];
        for (i, b) in rel.iter_mut().enumerate() {
            *b = whole(p + 1 + i)?;
        }
        Some((p + 5).wrapping_add(i32::from_le_bytes(rel) as isize as usize))
    };

    for p in [addr.wrapping_sub(1), addr] {
        match whole(p) {
            Some(0xE8) | Some(0xE9) => {
                if let Some(target) = rel_target(p) {
                    let kind = if whole(p) == Some(0xE8) { "call" } else { "jmp" };
                    return format!("{kind} {target:#x} in {}", module_of(target));
                }
            }
            Some(0xFF) if whole(p + 1) == Some(0x25) => {
                let mut abs = [0u8; 4];
                for (i, b) in abs.iter_mut().enumerate() {
                    *b = whole(p + 2 + i).unwrap_or(0);
                }
                let slot = u32::from_le_bytes(abs) as usize;
                let target = crate::game::read_u32(slot).unwrap_or(0) as usize;
                return format!("jmp [{slot:#x}] -> {target:#x} in {}", module_of(target));
            }
            _ => {}
        }
    }

    match mem.first() {
        Some(0xC3) | Some(0xC2) => "ret".to_owned(),
        Some(0x90) => "nop".to_owned(),
        Some(0x33) | Some(0x31) => "xor (Rückgabe 0?)".to_owned(),
        _ => String::new(),
    }
}

/// Compares all ranges, logs every difference and remembers them for `restore`.
pub fn scan(reason: &str) -> Vec<Patch> {
    let Some(exe) = exe_bytes() else {
        return Vec::new();
    };

    let mut patches = Vec::new();

    for &(start, len, name) in RANGES {
        if !readable(start, len) {
            crate::log!("Diagnose: {name} nicht lesbar");
            continue;
        }

        let Some(offset) = file_offset(exe, start) else {
            crate::log!("Diagnose: {name} ({start:#x}) nicht in der EXE gefunden");
            continue;
        };

        let Some(original) = exe.get(offset..offset + len) else {
            continue;
        };

        let memory = unsafe { std::slice::from_raw_parts(start as *const u8, len) }.to_vec();
        let whole = |addr: usize| -> Option<u8> {
            addr.checked_sub(start).and_then(|i| memory.get(i).copied())
        };

        let mut i = 0;
        while i < len {
            if original[i] == memory[i] {
                i += 1;
                continue;
            }

            // merge differences closer than 4 bytes
            let first = i;
            let mut last = i;
            let mut j = i + 1;
            while j < len && j <= last + 4 {
                if original[j] != memory[j] {
                    last = j;
                }
                j += 1;
            }

            let addr = start + first;
            let mem = memory[first..=last].to_vec();
            let note = describe(addr, &mem, &whole);

            patches.push(Patch {
                addr,
                original: original[first..=last].to_vec(),
                memory: mem,
                range: name,
                note,
            });

            i = last + 1;
        }
    }

    crate::log!("Diagnose ({reason}): {} Abweichung(en) gegenüber gta_sa.exe", patches.len());

    for p in &patches {
        crate::log!(
            "  {:#x} [{}] Original {} | Speicher {} {}",
            p.addr,
            p.range,
            hex(&p.original),
            hex(&p.memory),
            if p.note.is_empty() { String::new() } else { format!("| {}", p.note) }
        );
    }

    if let Ok(mut last) = LAST.lock() {
        *last = Some(patches.clone());
    }

    patches
}

pub fn last() -> Option<Vec<Patch>> {
    LAST.lock().ok().and_then(|last| last.clone())
}

fn hex(bytes: &[u8]) -> String {
    let shown: Vec<String> = bytes.iter().take(16).map(|b| format!("{b:02X}")).collect();
    let more = if bytes.len() > 16 { format!(" ... ({} Bytes)", bytes.len()) } else { String::new() };
    format!("{}{more}", shown.join(" "))
}

fn write_code(addr: usize, bytes: &[u8]) -> bool {
    unsafe {
        let mut old = 0;
        if VirtualProtect(addr as *mut c_void as _, bytes.len(), PAGE_EXECUTE_READWRITE, &mut old) == 0 {
            return false;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), addr as *mut u8, bytes.len());
        let mut ignored = 0;
        VirtualProtect(addr as *mut c_void as _, bytes.len(), old, &mut ignored);
        FlushInstructionCache(GetCurrentProcess(), addr as *const c_void as _, bytes.len());
    }
    true
}

/// Game thread only: puts the original bytes of all differences back, runs `f`, and
/// writes back exactly what was in memory before. Returns None if there was nothing
/// to restore.
pub fn with_original_code<R>(patches: &[Patch], f: impl FnOnce() -> R) -> Option<R> {
    if patches.is_empty() {
        return None;
    }

    let mut saved: Vec<(usize, Vec<u8>)> = Vec::new();

    for p in patches {
        let current = unsafe { std::slice::from_raw_parts(p.addr as *const u8, p.original.len()) }.to_vec();
        if write_code(p.addr, &p.original) {
            saved.push((p.addr, current));
        }
    }

    let result = f();

    for (addr, bytes) in saved.iter().rev() {
        write_code(*addr, bytes);
    }

    Some(result)
}
