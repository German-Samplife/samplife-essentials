//! Kleidung färben (ab 0.9.0; ab 0.9.2 nur Oberteil und Hose und ohne eigenes Laden): nach einem Umbau die Textur des
//! Teils mit `@RRGGBB` umfärben, nur für diese Figur.
//!
//! Welche Texturen das sind (gta-reversed ClothesBuilder.cpp, CreateSkinnedClump 0x5A69D0; ConstructTextures 0x5A6040
//! ist dort nicht nachgebaut, der Ablauf stammt aus der Rekonstruktion in pedrokilo/ReSA): Vor jedem Bau zerstört
//! CreateSkinnedClump alle Texturen im Wörterbuch von Modell 0, ConstructTextures legt sie neu an. Oberteil und Hose
//! sind dabei eigene Raster (CopyTexture 0x5A5730 der Haut mit Fett, Muskeln und Tattoos), auf die
//! GetTextureFromTxdAndLoadNextTxd (0x5A5F70) mit PlaceTextureOnTopOfTexture (0x5A57B0) die Kleidung legt, an jedem
//! Pixel, dessen Alpha in der Kleidung nicht 0 ist. Die Kopien teilen sich nur Modell 0 und die gerade gebaute Figur
//! (CPed::Dress klont das Modell, der Klon teilt die Geometrie samt Materialien). Beim nächsten Bau verlieren Modell und
//! Wörterbuch ihre Referenz, die Figur behält ihre Texturen, bis das Spiel sie mit ihr freigibt. Deshalb färbt das
//! Plugin direkt im gebauten Raster: Es legt keine RenderWare-Objekte an, und freigegeben wird, was das Spiel freigibt.
//!
//! Die Maske (welche Pixel Kleidung sind) greift das Plugin ab 0.9.2 im Baukasten selbst ab, statt die Kleidung noch
//! einmal zu laden: Nur während eines eigenen Umbaus mit Farben leitet es den Aufruf von PlaceTextureOnTopOfTexture in
//! GetTextureFromTxdAndLoadNextTxd auf `platzieren` um. Das liest die Kleidung (die Quelle) nur, merkt sich Maske und
//! Prüfsumme samt Ziel und ruft dann die Funktion des Spiels mit denselben Werten auf. Direkt nach dem Umbau kommt der
//! ursprüngliche Aufruf zurück. Das Plugin lädt und entlädt dabei keine TXD und ruft nichts von CStreaming auf
//! (0.9.1 hat die Kleidung mit RequestTexture, LoadAllRequestedModels und RemoveModel selbst geladen; im Spieltest mit
//! modloader/std.stream stürzte danach der Baukasten mit einem leeren Wörterbuch ab).
//!
//! Gefärbt wird nur ein Ziel, das die Figur trägt, das im Wörterbuch von Modell 0 liegt und unter der Maske noch genau
//! die Pixel der Kleidung zeigt (Prüfsumme). Welche Stelle es ist, sagt die Zahl der Materialien, die es benutzen:
//! CreateSkinnedClump (in gta-reversed nachgebaut) gibt die Textur „torso“ dem Oberkörper und den Händen, „legs“ nur
//! den Beinen. Zwei Materialien sind also das Oberteil, eins die Hose. Ist das nicht eindeutig, bleibt das Teil
//! ungefärbt. Gefärbt werden nur die Pixel unter der Maske; Arme, Hände, Beine und Tattoos bleiben.
//!
//! Kopf, Schuhe und Zubehör färbt das Plugin nicht: Im Spieltest mit 0.9.0 lag die Textur „hat“ aus dem Wörterbuch von
//! Modell 0 an den Füßen, und das Kopftuch verschwand. Für diese reinen Kopien gibt es keine Maske, an der sich die
//! richtige Textur beweisen ließe.
//!
//! Gefärbt wird nur, was der Baukasten selbst lesen kann: 32 Bit je Pixel, D3DFMT_A8R8G8B8 (RenderWare 8888) oder
//! D3DFMT_X8R8G8B8 (888), ohne Palette, ohne Kompression, ohne eigene Mipmap-Stufen. Alles andere bleibt ungefärbt und
//! steht im Log.

use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use client_api::gta::rw::rwcore::RwRaster;

use crate::farbe::{Farbe, Maske};
use crate::game::{self, read_ptr, read_u32, readable, writable};
use crate::outfit::OutfitKeys;

// gta-reversed RenderWare/rw/rwcore.cpp
const RW_RASTER_LOCK: usize = 0x7FB2D0; // RwUInt8* RwRasterLock(RwRaster*, RwUInt8 level, RwInt32 lockMode)
const RW_RASTER_UNLOCK: usize = 0x7FAEC0; // RwRaster* RwRasterUnlock(RwRaster*)
// rwcore.h: rwRASTERLOCKWRITE 1, rwRASTERLOCKREAD 2
const LOCK_READ: i32 = 0x02;
const LOCK_READWRITE: i32 = 0x03;
// rwcore.h RwRasterFormat
const FORMAT_PIXEL_MASK: u32 = 0x0F00;
const FORMAT_8888: u32 = 0x0500;
const FORMAT_888: u32 = 0x0600;
const FORMAT_AUTOMIPMAP: u32 = 0x1000;
const FORMAT_PAL8: u32 = 0x2000;
const FORMAT_PAL4: u32 = 0x4000;
const FORMAT_MIPMAP: u32 = 0x8000;
// rwcore.h struct RwTexture: raster, dict, lInDictionary (RwLLLink, 8 Bytes), name[32]
const TEXTURE_DICT: usize = 0x4;
const TEXTURE_NAME: usize = 0x10;

// gta-reversed app/platform/win/WindowedMode.cpp: _RwD3D9RasterExtOffset (0xB4E9E0) und struct _rwD3D9RasterExt:
// texture, palette, alpha, cube:4/face:4, automipmapgen:4/compressed:4 (+0xA), lockedMipLevel, lockedSurface,
// lockedRect (8 Bytes), d3dFormat (+0x18)
const RW_D3D9_RASTER_EXT_OFFSET: usize = 0xB4E9E0;
const EXT_PALETTE: usize = 0x4;
const EXT_FLAGS: usize = 0xA;
const EXT_D3D_FORMAT: usize = 0x18;
const D3DFMT_A8R8G8B8: u32 = 21;
const D3DFMT_X8R8G8B8: u32 = 22;

// gta-reversed ClothesBuilder.cpp: RH_ScopedInstall(PlaceTextureOnTopOfTexture, 0x5A57B0),
// void PlaceTextureOnTopOfTexture(RwTexture* dst, RwTexture* src); aufgerufen aus GetTextureFromTxdAndLoadNextTxd
// (0x5A5F70), das ConstructTextures (0x5A6040 bis 0x5A6530) benutzt
const CCLOTHESBUILDER_PLACE_TEXTURE_ON_TOP: usize = 0x5A57B0;
const AUFRUFE_VON: usize = 0x5A5F70;
const AUFRUFE_BIS: usize = 0x5A6530;

/// Gefärbte Stellen (eClothesTexturePart), Name für das Log und wie viele Materialien der Figur ihre Textur benutzen
/// (CreateSkinnedClump, BODY_PART_TEX_NAMES: „torso“ für Oberkörper und Hände, „legs“ für die Beine).
const STELLEN: &[(usize, &str, usize)] = &[(game::TEX_TORSO, "Oberteil", 2), (game::TEX_LEGS, "Hose", 1)];

/// Was `platzieren` bei einem Aufruf von PlaceTextureOnTopOfTexture gesehen hat
pub struct Platzierung {
    /// Ziel (RwTexture*), auf das die Kleidung kam
    ziel: usize,
    /// Name der Quelltextur (Kleidung) für das Log
    quelle: String,
    /// Maske und Prüfsumme der Quelle, oder warum nicht
    maske: Result<Maske, String>,
}

static ABGREIFEN: AtomicBool = AtomicBool::new(false);
static PLATZIERUNGEN: Mutex<Vec<Platzierung>> = Mutex::new(Vec::new());

/// Ersatz für den Aufruf von PlaceTextureOnTopOfTexture, nur während eines eigenen Umbaus mit Farben. Liest die Quelle,
/// ändert nichts und ruft danach immer die Funktion des Spiels mit denselben Werten auf.
extern "C" fn platzieren(ziel: usize, quelle: usize) {
    if ABGREIFEN.load(Ordering::SeqCst) {
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let platzierung = Platzierung {
                ziel,
                quelle: textur_name(quelle),
                maske: maske_aus_quelle(quelle),
            };
            if let Ok(mut liste) = PLATZIERUNGEN.lock() {
                if liste.len() < 32 {
                    liste.push(platzierung);
                }
            }
        }));
    }

    let original: extern "C" fn(usize, usize) = unsafe { std::mem::transmute(CCLOTHESBUILDER_PLACE_TEXTURE_ON_TOP) };
    original(ziel, quelle);
}

fn maske_aus_quelle(quelle: usize) -> Result<Maske, String> {
    let raster = read_ptr(quelle).ok_or("Kleidung ohne Raster")?;
    let info = RasterInfo::lesen(raster)?;
    info.faerbbar()?;
    let mut gesperrt = Gesperrt::neu(raster, LOCK_READ, false)?;
    let stride = gesperrt.stride;
    Maske::aus_pixeln(gesperrt.pixel(), info.breite, info.hoehe, stride).ok_or_else(|| "Raster zu klein".to_owned())
}

/// Umgeleitete Aufrufe während eines Umbaus. Beim Beenden (auch durch Drop) kommen die ursprünglichen Ziele zurück.
pub struct Abgriff {
    stellen: Vec<(usize, [u8; 4])>,
}

impl Abgriff {
    /// Game thread, direkt vor rebuild::umbauen. Ohne Fund (z. B. ein anderer Mod hat den Aufruf schon umgeleitet)
    /// bleibt der Code, wie er ist; dann wird nicht gefärbt.
    pub fn starten() -> Result<Abgriff, String> {
        if let Ok(mut liste) = PLATZIERUNGEN.lock() {
            liste.clear();
        }

        let laenge = AUFRUFE_BIS - AUFRUFE_VON;
        if !readable(AUFRUFE_VON, laenge + 4) {
            return Err("Code des Baukastens nicht lesbar".to_owned());
        }
        let code = unsafe { std::slice::from_raw_parts(AUFRUFE_VON as *const u8, laenge + 4) };

        // erst alle Aufrufe suchen (call rel32 mit Ziel PlaceTextureOnTopOfTexture), dann umleiten
        let mut funde: Vec<(usize, [u8; 4])> = Vec::new();
        for i in 0..laenge {
            if code[i] != 0xE8 {
                continue;
            }
            let rel = [code[i + 1], code[i + 2], code[i + 3], code[i + 4]];
            let stelle = AUFRUFE_VON + i;
            let ziel = (stelle + 5).wrapping_add(i32::from_le_bytes(rel) as isize as usize);
            if ziel == CCLOTHESBUILDER_PLACE_TEXTURE_ON_TOP {
                funde.push((stelle, rel));
            }
        }

        let mut abgriff = Abgriff { stellen: Vec::new() };
        let neu = (platzieren as extern "C" fn(usize, usize)) as usize;

        for (stelle, rel) in funde {
            let neu_rel = (neu.wrapping_sub(stelle + 5) as u32).to_le_bytes();
            if !crate::diag::write_code(stelle + 1, &neu_rel) {
                return Err(format!("Aufruf bei {stelle:#x} nicht umzuleiten"));
            }
            abgriff.stellen.push((stelle + 1, rel));
        }

        if abgriff.stellen.is_empty() {
            return Err("kein Aufruf von PlaceTextureOnTopOfTexture gefunden (von einem anderen Mod umgeleitet?)".to_owned());
        }

        ABGREIFEN.store(true, Ordering::SeqCst);
        Ok(abgriff)
    }

    /// Game thread, direkt nach rebuild::umbauen: Aufrufe zurück, was gesehen wurde.
    pub fn beenden(mut self) -> Vec<Platzierung> {
        self.zurueck();
        match PLATZIERUNGEN.lock() {
            Ok(mut liste) => liste.drain(..).collect(),
            Err(_) => Vec::new(),
        }
    }

    fn zurueck(&mut self) {
        ABGREIFEN.store(false, Ordering::SeqCst);
        for (adresse, rel) in self.stellen.drain(..) {
            if !crate::diag::write_code(adresse, &rel) {
                crate::log!("Aufruf bei {:#x} nicht zurückgesetzt", adresse - 1);
            }
        }
    }
}

impl Drop for Abgriff {
    fn drop(&mut self) {
        self.zurueck();
    }
}

pub struct Faerber {
    pub gefaerbt: u32,
    pub fehler: u32,
    letzter_fehler: String,
    letzte_ms: f32,
}

impl Faerber {
    pub fn new() -> Faerber {
        Faerber {
            gefaerbt: 0,
            fehler: 0,
            letzter_fehler: String::new(),
            letzte_ms: 0.0,
        }
    }

    pub fn zustand(&self) -> String {
        let mut text = format!(
            "{} Teile gefärbt (zuletzt {:.1} ms), {} nicht",
            self.gefaerbt, self.letzte_ms, self.fehler
        );
        if !self.letzter_fehler.is_empty() {
            let grund: String = self.letzter_fehler.chars().take(60).collect();
            text += &format!(", zuletzt: {grund}");
        }
        text
    }

    fn fehler(&mut self, wer: &str, part: usize, name: &str, farbe: Farbe, grund: &str) {
        self.fehler += 1;
        crate::log!("  {wer}: Teil {part} ({name}) #{} nicht gefärbt: {grund}", farbe.hex());
        self.letzter_fehler = format!("Teil {part}: {grund}");
    }

    /// Ohne Abgriff (Umleitung nicht möglich): jedes Teil mit Farbe bleibt ungefärbt.
    pub fn ohne_abgriff(&mut self, wer: &str, keys: &OutfitKeys, grund: &str) {
        for &(part, name, _) in STELLEN {
            if let Some(farbe) = keys.farben[part] {
                self.fehler(wer, part, name, farbe, grund);
            }
        }
    }

    /// Game thread, direkt nach einem Umbau, der gewirkt hat: `clump` ist der Clump der gebauten Figur, `platzierungen`
    /// was der Abgriff dabei gesehen hat. Jeder Fehler lässt das Teil ungefärbt und steht im Log.
    pub fn anwenden(&mut self, wer: &str, clump: usize, keys: &OutfitKeys, platzierungen: &[Platzierung]) {
        if keys.gefaerbt() == 0 {
            return;
        }

        let start = Instant::now();
        let dict = game::model0_txd();
        let figur = game::clump_textures(clump);
        materialien_loggen(wer, dict, &figur);

        // Ziele, die die Figur trägt, die in Modell 0 liegen und unter der Maske noch genau die Kleidung zeigen
        let mut kandidaten: Vec<(&Platzierung, &Maske, usize)> = Vec::new();
        for p in platzierungen {
            let Ok(maske) = &p.maske else {
                continue;
            };
            let materialien = figur.iter().filter(|&&t| t == p.ziel).count();
            if materialien == 0 || dict.is_none() || read_ptr(p.ziel + TEXTURE_DICT) != dict {
                continue;
            }
            if kandidaten.iter().any(|(k, _, _)| k.ziel == p.ziel) {
                continue;
            }
            if pruefsumme_passt(p.ziel, maske) {
                kandidaten.push((p, maske, materialien));
            }
        }

        let gesehen: Vec<String> = platzierungen
            .iter()
            .map(|p| match &p.maske {
                Ok(m) => format!("{:#x} „{}“ {}x{} {} Pixel", p.ziel, p.quelle, m.breite, m.hoehe, m.anzahl()),
                Err(grund) => format!("{:#x} „{}“: {grund}", p.ziel, p.quelle),
            })
            .collect();
        crate::log!("  {wer}: Kleidung über der Haut: {}", gesehen.join(" | "));

        for &(part, name, materialien) in STELLEN {
            let Some(farbe) = keys.farben[part] else {
                continue;
            };

            let passend: Vec<&(&Platzierung, &Maske, usize)> = kandidaten.iter().filter(|k| k.2 == materialien).collect();
            let ergebnis = match passend.as_slice() {
                [(p, maske, _)] => faerben_im_raster(p.ziel, maske, farbe).map(|text| format!("„{}“ auf {:#x}, {text}", p.quelle, p.ziel)),
                [] => Err(format!("keine gebaute Textur mit dieser Kleidung und {materialien} Material(ien) gefunden")),
                viele => Err(format!("{} Texturen passen, nicht eindeutig", viele.len())),
            };

            match ergebnis {
                Ok(text) => {
                    self.gefaerbt += 1;
                    crate::log!("  {wer}: Teil {part} ({name}) #{} gefärbt, {text}", farbe.hex());
                }
                Err(grund) => self.fehler(wer, part, name, farbe, &grund),
            }
        }

        self.letzte_ms = start.elapsed().as_secs_f32() * 1000.0;
    }
}

/// Zeigt die Textur unter der Maske noch genau die Pixel der Kleidung?
fn pruefsumme_passt(textur: usize, maske: &Maske) -> bool {
    let Some(raster) = read_ptr(textur) else {
        return false;
    };
    let Ok(info) = RasterInfo::lesen(raster) else {
        return false;
    };
    if info.breite != maske.breite || info.hoehe != maske.hoehe || info.faerbbar().is_err() {
        return false;
    }
    let Ok(mut gesperrt) = Gesperrt::neu(raster, LOCK_READ, false) else {
        return false;
    };
    let stride = gesperrt.stride;
    crate::farbe::pruefsumme(gesperrt.pixel(), info.breite, info.hoehe, stride, maske) == Some(maske.pruefsumme)
}

fn faerben_im_raster(textur: usize, maske: &Maske, farbe: Farbe) -> Result<String, String> {
    let raster = read_ptr(textur).ok_or("Textur ohne Raster")?;
    let info = RasterInfo::lesen(raster)?;
    let mit_alpha = info.faerbbar()?;

    let start = Instant::now();
    let mut gesperrt = Gesperrt::neu(raster, LOCK_READWRITE, true)?;
    let stride = gesperrt.stride;
    let ergebnis = crate::farbe::faerben(gesperrt.pixel(), info.breite, info.hoehe, stride, mit_alpha, farbe, Some(maske))?;
    drop(gesperrt);

    Ok(format!(
        "{} von {} Pixeln, Bezug {}, {}x{} {}, nur die Kleidung, {:.1} ms",
        ergebnis.pixel,
        info.breite * info.hoehe,
        ergebnis.bezug,
        info.breite,
        info.hoehe,
        info.d3d_name(),
        start.elapsed().as_secs_f32() * 1000.0
    ))
}

/// Name einer Textur (RwTexture::name, höchstens 32 Bytes)
fn textur_name(textur: usize) -> String {
    if !readable(textur + TEXTURE_NAME, 32) {
        return "?".to_owned();
    }
    let bytes = unsafe { std::slice::from_raw_parts((textur + TEXTURE_NAME) as *const u8, 32) };
    let ende = bytes.iter().position(|&b| b == 0).unwrap_or(32);
    String::from_utf8_lossy(&bytes[..ende]).into_owned()
}

/// Für die Fehlersuche: jede Textur der Figur mit Name, Größe, Format und ob sie im Wörterbuch von Modell 0 liegt,
/// in der Reihenfolge der Materialien.
fn materialien_loggen(wer: &str, dict: Option<usize>, figur: &[usize]) {
    let teile: Vec<String> = figur
        .iter()
        .enumerate()
        .map(|(i, &textur)| {
            let raster = read_ptr(textur).unwrap_or(0);
            let groesse = match RasterInfo::lesen(raster) {
                Ok(info) => format!("{}x{} {}", info.breite, info.hoehe, info.d3d_name()),
                Err(_) => "?".to_owned(),
            };
            let modell0 = if dict.is_some() && read_ptr(textur + TEXTURE_DICT) == dict { "" } else { ", fremd" };
            format!("{i}: {textur:#x} „{}“ {groesse}{modell0}", textur_name(textur))
        })
        .collect();
    crate::log!("  {wer}: Materialien {}", teile.join(" | "));
}

struct RasterInfo {
    breite: usize,
    hoehe: usize,
    tiefe: i32,
    format: u32,
    d3d_format: u32,
    komprimiert: bool,
    palette: bool,
}

impl RasterInfo {
    fn lesen(raster: usize) -> Result<RasterInfo, String> {
        const _: () = assert!(std::mem::size_of::<RwRaster>() == 0x34);

        if !readable(raster, std::mem::size_of::<RwRaster>()) {
            return Err(format!("Raster {raster:#x} nicht lesbar"));
        }
        let r = unsafe { (raster as *const RwRaster).read_unaligned() };

        let offset = read_u32(RW_D3D9_RASTER_EXT_OFFSET).unwrap_or(0) as usize;
        let ext = raster + offset;
        if offset < std::mem::size_of::<RwRaster>() || offset > 0x100 || !readable(ext, EXT_D3D_FORMAT + 4) {
            return Err(format!("D3D9-Teil des Rasters nicht gefunden (Abstand {offset:#x})"));
        }
        let flags = unsafe { ((ext + EXT_FLAGS) as *const u8).read() };

        Ok(RasterInfo {
            breite: r.width.max(0) as usize,
            hoehe: r.height.max(0) as usize,
            tiefe: r.depth,
            format: (r.cFormat as u32) << 8,
            d3d_format: read_u32(ext + EXT_D3D_FORMAT).unwrap_or(0),
            komprimiert: flags >> 4 != 0,
            palette: read_u32(ext + EXT_PALETTE).unwrap_or(0) != 0,
        })
    }

    /// Ok(mit Alpha) für Formate, die sich färben lassen
    fn faerbbar(&self) -> Result<bool, String> {
        let name = self.d3d_name();

        if self.format & (FORMAT_PAL4 | FORMAT_PAL8) != 0 || self.palette {
            return Err(format!("Format {name} mit Palette wird nicht gefärbt"));
        }
        if self.komprimiert {
            return Err(format!("komprimiertes Format {name} wird nicht gefärbt"));
        }
        if self.format & FORMAT_MIPMAP != 0 && self.format & FORMAT_AUTOMIPMAP == 0 {
            return Err(format!("Format {name} mit eigenen Mipmap-Stufen wird nicht gefärbt"));
        }
        if self.breite == 0 || self.hoehe == 0 || self.breite > 2048 || self.hoehe > 2048 {
            return Err(format!("Größe {}x{} ungültig", self.breite, self.hoehe));
        }

        let pixel = self.format & FORMAT_PIXEL_MASK;
        match (self.d3d_format, self.tiefe, pixel) {
            (D3DFMT_A8R8G8B8, 32, FORMAT_8888) => Ok(true),
            (D3DFMT_X8R8G8B8, 32, FORMAT_888) => Ok(false),
            _ => Err(format!("Format {name} ({} Bit, RenderWare {:#06x}) wird nicht gefärbt", self.tiefe, self.format)),
        }
    }

    fn d3d_name(&self) -> String {
        match self.d3d_format {
            20 => "R8G8B8".to_owned(),
            D3DFMT_A8R8G8B8 => "A8R8G8B8".to_owned(),
            D3DFMT_X8R8G8B8 => "X8R8G8B8".to_owned(),
            23 => "R5G6B5".to_owned(),
            25 => "A1R5G5B5".to_owned(),
            26 => "A4R4G4B4".to_owned(),
            41 => "P8".to_owned(),
            50 => "L8".to_owned(),
            f => {
                // FourCC wie DXT1 bis DXT5
                let b = f.to_le_bytes();
                if b.iter().all(|c| c.is_ascii_alphanumeric()) {
                    String::from_utf8_lossy(&b).into_owned()
                } else {
                    format!("D3DFMT {f}")
                }
            }
        }
    }
}

/// Ein gesperrtes Raster; wird beim Verlassen entsperrt, auch nach einem Fehler.
struct Gesperrt {
    raster: usize,
    pixel: usize,
    laenge: usize,
    stride: usize,
}

impl Gesperrt {
    fn neu(raster: usize, modus: i32, schreiben: bool) -> Result<Gesperrt, String> {
        let lock: extern "C" fn(usize, u8, i32) -> usize = unsafe { std::mem::transmute(RW_RASTER_LOCK) };
        let pixel = lock(raster, 0, modus);
        if pixel == 0 {
            return Err("RwRasterLock fehlgeschlagen".to_owned());
        }

        // RwRasterLock setzt stride (gta-reversed rwcore.cpp)
        let r = unsafe { (raster as *const RwRaster).read_unaligned() };
        let mut gesperrt = Gesperrt {
            raster,
            pixel,
            laenge: 0,
            stride: r.stride.max(0) as usize,
        };

        let (breite, hoehe) = (r.width.max(0) as usize, r.height.max(0) as usize);
        let laenge = gesperrt.stride * hoehe.saturating_sub(1) + breite * 4;
        let zugang = if schreiben { writable(pixel, laenge) } else { readable(pixel, laenge) };
        if gesperrt.stride < breite * 4 || !zugang {
            return Err(format!("Pixel {pixel:#x} nicht zugänglich (Zeile {} Bytes)", gesperrt.stride));
        }

        gesperrt.laenge = laenge;
        Ok(gesperrt)
    }

    fn pixel(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.pixel as *mut u8, self.laenge) }
    }
}

impl Drop for Gesperrt {
    fn drop(&mut self) {
        let unlock: extern "C" fn(usize) -> usize = unsafe { std::mem::transmute(RW_RASTER_UNLOCK) };
        unlock(self.raster);
    }
}
