//! Zeichnet das Ladebild direkt vor jedem Present des Geräts, mit dem GTA rendert.
//!
//! Wie samplife-kleidung (Einstieg B in kleidung/src/hooks.rs): Das Gerät steht im
//! RenderWare-Global RwD3D9Device. In dessen vtable ersetzen wir den Eintrag Present und rufen
//! danach den vorigen Eintrag auf, so laufen SA-MP, samp-cef und andere Hooks weiter.
//!
//! Solange das Spiel lädt, liegt das Bild bildschirmfüllend über allem (zugeschnitten wie
//! „cover“ im Web, die Mitte bleibt). Sobald gGameState auf „läuft“ steht, blendet es in einer
//! halben Sekunde aus; danach gibt es die Textur frei und reicht Present nur noch durch.
//! Zustand des Geräts: vor dem Zeichnen per State-Block gesichert und danach zurückgesetzt.

use std::ffi::c_void;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use winapi::shared::d3d9::{IDirect3DDevice9, IDirect3DDevice9Vtbl, IDirect3DStateBlock9, IDirect3DSurface9, IDirect3DTexture9};
use winapi::shared::d3d9types::*;
use winapi::shared::windef::{HWND, RECT};
use winapi::um::memoryapi::VirtualProtect;
use winapi::um::wingdi::RGNDATA;
use winapi::um::winnt::{HRESULT, PAGE_READWRITE};

use crate::bild;
use crate::log;
use crate::spiel::{lesbar, lesen_u32, lesen_zeiger, spiel_laeuft, RW_D3D9_DEVICE};

type PresentFn = unsafe extern "system" fn(*mut IDirect3DDevice9, *const RECT, *const RECT, HWND, *const RGNDATA) -> HRESULT;

const AUSBLENDEN_MS: f32 = 500.0;

static VORHER: AtomicUsize = AtomicUsize::new(0);
static EINGEHAKT: AtomicBool = AtomicBool::new(false);
static FERTIG: AtomicBool = AtomicBool::new(false);
static FEHLER_GEMELDET: AtomicBool = AtomicBool::new(false);

struct Zustand {
    textur: usize, // *mut IDirect3DTexture9
    ausblenden_ab: Option<Instant>,
    bilder: u32,
}

static ZUSTAND: Mutex<Zustand> = Mutex::new(Zustand { textur: 0, ausblenden_ab: None, bilder: 0 });

pub fn eingehakt() -> bool {
    EINGEHAKT.load(Ordering::SeqCst)
}

pub fn fertig() -> bool {
    FERTIG.load(Ordering::SeqCst)
}

fn hook_adresse() -> usize {
    present_hook as PresentFn as usize
}

/// Hängt sich in Present ein, sobald es das Gerät gibt. Rückgabe: Text fürs Protokoll, wenn sich
/// etwas getan hat.
pub fn einhaken() -> Option<String> {
    if eingehakt() {
        return None;
    }
    if spiel_laeuft() {
        // Zu spät: Das Spiel läuft schon, es gibt nichts mehr zu überdecken
        FERTIG.store(true, Ordering::SeqCst);
        return Some("Das Spiel lief schon, bevor das Gerät bereit war; kein Ladebild".to_owned());
    }
    let geraet = lesen_zeiger(RW_D3D9_DEVICE)?;
    let vtable = lesen_zeiger(geraet)?;
    let eintrag = vtable + std::mem::offset_of!(IDirect3DDevice9Vtbl, Present);
    let aktuell = lesen_u32(eintrag)? as usize;
    if aktuell == hook_adresse() {
        return None;
    }
    if aktuell == 0 || !lesbar(aktuell, 1) {
        return Some(format!("Present-Eintrag {aktuell:#x} ungültig"));
    }

    VORHER.store(aktuell, Ordering::SeqCst);
    let ok = unsafe {
        let mut alt = 0;
        if VirtualProtect(eintrag as *mut c_void as _, 4, PAGE_READWRITE, &mut alt) == 0 {
            false
        } else {
            // ausgerichtetes 4-Byte-Schreiben, für den Spielfaden in einem Schritt
            (*(eintrag as *const AtomicUsize)).store(hook_adresse(), Ordering::SeqCst);
            let mut egal = 0;
            VirtualProtect(eintrag as *mut c_void as _, 4, alt, &mut egal);
            true
        }
    };
    if !ok {
        return Some(format!("Speicherschutz an {eintrag:#x} ließ sich nicht ändern"));
    }
    EINGEHAKT.store(true, Ordering::SeqCst);
    Some(format!("Present eingehakt: Gerät {geraet:#x}, vtable {vtable:#x}, vorher {aktuell:#x}"))
}

unsafe extern "system" fn present_hook(
    geraet: *mut IDirect3DDevice9,
    quelle: *const RECT,
    ziel: *const RECT,
    fenster: HWND,
    bereich: *const RGNDATA,
) -> HRESULT {
    if !FERTIG.load(Ordering::SeqCst) && !geraet.is_null() {
        let ergebnis = std::panic::catch_unwind(|| unsafe { vor_present(&*geraet) });
        if ergebnis.is_err() {
            // Nie das Spiel mitreißen: Bei einem Fehler bleibt das Ladebild aus
            FERTIG.store(true, Ordering::SeqCst);
            log::zeile("Fehler beim Zeichnen, das Ladebild ist aus");
        }
    }
    let vorher = VORHER.load(Ordering::SeqCst);
    if vorher == 0 || vorher == hook_adresse() {
        return 0;
    }
    let vorher: PresentFn = std::mem::transmute(vorher);
    vorher(geraet, quelle, ziel, fenster, bereich)
}

/// Im Spielfaden vor jedem Present, solange das Ladebild nicht fertig ist.
unsafe fn vor_present(geraet: &IDirect3DDevice9) {
    let Ok(mut zustand) = ZUSTAND.lock() else { return };

    let deckkraft = if spiel_laeuft() {
        let ab = *zustand.ausblenden_ab.get_or_insert_with(|| {
            log::zeile(&format!("Spiel läuft, blende aus (nach {} Bildern)", zustand_bilder()));
            Instant::now()
        });
        let anteil = ab.elapsed().as_secs_f32() * 1000.0 / AUSBLENDEN_MS;
        if anteil >= 1.0 {
            textur_freigeben(&mut zustand);
            FERTIG.store(true, Ordering::SeqCst);
            log::zeile("Ladebild ausgeblendet");
            return;
        }
        1.0 - anteil
    } else {
        1.0
    };

    if zustand.textur == 0 {
        match textur_anlegen(geraet) {
            Ok(textur) => {
                zustand.textur = textur as usize;
                log::zeile("Textur angelegt, Ladebild wird gezeigt");
            }
            Err(fehler) => {
                if !FEHLER_GEMELDET.swap(true, Ordering::SeqCst) {
                    log::zeile(&format!("Textur ließ sich nicht anlegen ({fehler}), das Ladebild bleibt aus"));
                }
                FERTIG.store(true, Ordering::SeqCst);
                return;
            }
        }
    }
    zustand.bilder = zustand.bilder.wrapping_add(1);
    BILDER.store(zustand.bilder as usize, Ordering::Relaxed);
    zeichne(geraet, zustand.textur as *mut IDirect3DTexture9, deckkraft);
}

static BILDER: AtomicUsize = AtomicUsize::new(0);

fn zustand_bilder() -> usize {
    BILDER.load(Ordering::Relaxed)
}

unsafe fn textur_freigeben(zustand: &mut Zustand) {
    if zustand.textur != 0 {
        (*(zustand.textur as *mut IDirect3DTexture9)).Release();
        zustand.textur = 0;
    }
}

/// Textur aus dem Bild, D3DPOOL_MANAGED: übersteht ein Reset des Geräts (Alt+Tab).
unsafe fn textur_anlegen(geraet: &IDirect3DDevice9) -> Result<*mut IDirect3DTexture9, String> {
    let bild = bild::bild().ok_or("kein Bild")?;
    let mut textur: *mut IDirect3DTexture9 = null_mut();
    let hr = geraet.CreateTexture(bild.breite, bild.hoehe, 1, 0, D3DFMT_A8R8G8B8, D3DPOOL_MANAGED, &mut textur, null_mut());
    if hr < 0 || textur.is_null() {
        return Err(format!("CreateTexture {hr:#x}"));
    }
    let mut gesperrt: D3DLOCKED_RECT = std::mem::zeroed();
    let hr = (*textur).LockRect(0, &mut gesperrt, null_mut(), 0);
    if hr < 0 || gesperrt.pBits.is_null() {
        (*textur).Release();
        return Err(format!("LockRect {hr:#x}"));
    }
    let zeile = (bild.breite * 4) as usize;
    for y in 0..bild.hoehe as usize {
        let von = &bild.bgra[y * zeile..(y + 1) * zeile];
        let nach = (gesperrt.pBits as *mut u8).add(y * gesperrt.Pitch as usize);
        std::ptr::copy_nonoverlapping(von.as_ptr(), nach, zeile);
    }
    (*textur).UnlockRect(0);
    Ok(textur)
}

#[repr(C)]
struct Ecke {
    x: f32,
    y: f32,
    z: f32,
    rhw: f32,
    farbe: u32,
    u: f32,
    v: f32,
}

const FVF: u32 = D3DFVF_XYZRHW | D3DFVF_DIFFUSE | D3DFVF_TEX1;

/// Bildschirmfüllendes Rechteck mit dem Bild, Mitte bleibt, Ränder werden abgeschnitten.
unsafe fn zeichne(geraet: &IDirect3DDevice9, textur: *mut IDirect3DTexture9, deckkraft: f32) {
    let Some((breite, hoehe)) = bildschirm(geraet) else { return };
    let Some(bild) = bild::bild() else { return };

    let mut sicherung: *mut IDirect3DStateBlock9 = null_mut();
    if geraet.CreateStateBlock(D3DSBT_ALL, &mut sicherung) < 0 || sicherung.is_null() {
        return;
    }
    (*sicherung).Capture();

    // „cover“: Seitenverhältnis des Bildschirms gegen das des Bildes
    let schirm = breite / hoehe;
    let eigen = bild.breite as f32 / bild.hoehe as f32;
    let (mut u0, mut u1, mut v0, mut v1) = (0.0f32, 1.0f32, 0.0f32, 1.0f32);
    if schirm < eigen {
        let sichtbar = schirm / eigen;
        u0 = (1.0 - sichtbar) / 2.0;
        u1 = 1.0 - u0;
    } else {
        let sichtbar = eigen / schirm;
        v0 = (1.0 - sichtbar) / 2.0;
        v1 = 1.0 - v0;
    }
    let alpha = (deckkraft.clamp(0.0, 1.0) * 255.0) as u32;
    let farbe = (alpha << 24) | 0x00FF_FFFF;
    // halbes Pixel versetzt, damit Texel und Pixel unter Direct3D 9 genau aufeinanderliegen
    let (l, o, r, u) = (-0.5f32, -0.5f32, breite - 0.5, hoehe - 0.5);
    let ecken = [
        Ecke { x: l, y: o, z: 0.0, rhw: 1.0, farbe, u: u0, v: v0 },
        Ecke { x: r, y: o, z: 0.0, rhw: 1.0, farbe, u: u1, v: v0 },
        Ecke { x: l, y: u, z: 0.0, rhw: 1.0, farbe, u: u0, v: v1 },
        Ecke { x: r, y: u, z: 0.0, rhw: 1.0, farbe, u: u1, v: v1 },
    ];

    if geraet.BeginScene() >= 0 {
        geraet.SetVertexShader(null_mut());
        geraet.SetPixelShader(null_mut());
        geraet.SetFVF(FVF);
        geraet.SetTexture(0, textur as *mut _);
        geraet.SetRenderState(D3DRS_ZENABLE, 0);
        geraet.SetRenderState(D3DRS_ZWRITEENABLE, 0);
        geraet.SetRenderState(D3DRS_LIGHTING, 0);
        geraet.SetRenderState(D3DRS_FOGENABLE, 0);
        geraet.SetRenderState(D3DRS_CULLMODE, D3DCULL_NONE);
        geraet.SetRenderState(D3DRS_ALPHATESTENABLE, 0);
        geraet.SetRenderState(D3DRS_STENCILENABLE, 0);
        geraet.SetRenderState(D3DRS_SCISSORTESTENABLE, 0);
        geraet.SetRenderState(D3DRS_COLORWRITEENABLE, 0xF);
        geraet.SetRenderState(D3DRS_ALPHABLENDENABLE, 1);
        geraet.SetRenderState(D3DRS_SRCBLEND, D3DBLEND_SRCALPHA);
        geraet.SetRenderState(D3DRS_DESTBLEND, D3DBLEND_INVSRCALPHA);
        geraet.SetTextureStageState(0, D3DTSS_COLOROP, D3DTOP_MODULATE);
        geraet.SetTextureStageState(0, D3DTSS_COLORARG1, D3DTA_TEXTURE);
        geraet.SetTextureStageState(0, D3DTSS_COLORARG2, D3DTA_DIFFUSE);
        geraet.SetTextureStageState(0, D3DTSS_ALPHAOP, D3DTOP_SELECTARG1);
        geraet.SetTextureStageState(0, D3DTSS_ALPHAARG1, D3DTA_DIFFUSE);
        geraet.SetTextureStageState(1, D3DTSS_COLOROP, D3DTOP_DISABLE);
        geraet.SetTextureStageState(1, D3DTSS_ALPHAOP, D3DTOP_DISABLE);
        geraet.SetSamplerState(0, D3DSAMP_MINFILTER, D3DTEXF_LINEAR);
        geraet.SetSamplerState(0, D3DSAMP_MAGFILTER, D3DTEXF_LINEAR);
        geraet.SetSamplerState(0, D3DSAMP_MIPFILTER, D3DTEXF_NONE);
        geraet.SetSamplerState(0, D3DSAMP_ADDRESSU, D3DTADDRESS_CLAMP);
        geraet.SetSamplerState(0, D3DSAMP_ADDRESSV, D3DTADDRESS_CLAMP);
        geraet.DrawPrimitiveUP(D3DPT_TRIANGLESTRIP, 2, ecken.as_ptr() as *const winapi::ctypes::c_void, std::mem::size_of::<Ecke>() as u32);
        geraet.EndScene();
    }

    (*sicherung).Apply();
    (*sicherung).Release();
}

/// Größe des Backbuffers in Pixeln.
unsafe fn bildschirm(geraet: &IDirect3DDevice9) -> Option<(f32, f32)> {
    let mut flaeche: *mut IDirect3DSurface9 = null_mut();
    if geraet.GetBackBuffer(0, 0, D3DBACKBUFFER_TYPE_MONO, &mut flaeche) < 0 || flaeche.is_null() {
        return None;
    }
    let mut beschreibung: D3DSURFACE_DESC = std::mem::zeroed();
    let ok = (*flaeche).GetDesc(&mut beschreibung) >= 0;
    (*flaeche).Release();
    if !ok || beschreibung.Width == 0 || beschreibung.Height == 0 {
        return None;
    }
    Some((beschreibung.Width as f32, beschreibung.Height as f32))
}
