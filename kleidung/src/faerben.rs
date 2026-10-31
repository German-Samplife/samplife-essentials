//! Kleidung färben (ab 0.9.0): nach einem Umbau die Texturen der Teile mit `@RRGGBB` umfärben, nur für diese Figur.
//!
//! Welche Texturen das sind (gta-reversed ClothesBuilder.cpp, CreateSkinnedClump 0x5A69D0 und ConstructTextures
//! 0x5A6040): Vor jedem Bau zerstört CreateSkinnedClump alle Texturen im Wörterbuch von Modell 0 und legt sie mit
//! ConstructTextures neu an. Jede ist eine eigene Kopie (CopyTexture 0x5A5730: RwRasterCreate, RwTextureCreate), die
//! Quellen aus player.img (die TXD des T-Shirts, des Huts, der Brille ...) werden danach wieder entladen
//! (CStreaming::RemoveModel). Die Materialien der Figur zeigen auf diese Kopien unter festen Namen ("torso", "head",
//! "legs", "feet", "necklace", "watch", "glasses", "hat", "extra1"; die Hände nehmen "torso"). Die Kopien teilen sich nur
//! Modell 0 und die gerade gebaute Figur (CPed::Dress klont das Modell, der Klon teilt die Geometrie samt Materialien).
//! Beim nächsten Bau verlieren Modell und Wörterbuch ihre Referenz, die Figur behält ihre Texturen, bis das Spiel sie
//! mit ihr freigibt. Deshalb färbt das Plugin direkt im gebauten Raster: Es legt keine RenderWare-Objekte an, andere
//! Figuren (auch mit demselben Teil) haben eigene Texturen, und freigegeben wird, was das Spiel freigibt.
//! Neu erscheinende Figuren, die das Modell kopieren, tragen die Farben mit; logic.rs übernimmt sie nur, wenn ihr Outfit
//! samt Farben gleich ist, sonst baut es sie neu.
//!
//! Oberteil und Hose sind zusammengesetzt: ConstructTextures nimmt die Haut (mit Fett, Muskeln und Tattoos) und legt
//! die Kleidung mit PlaceTextureOnTopOfTexture (0x5A57B0) darüber, an jedem Pixel, dessen Alpha in der Kleidung nicht 0
//! ist. Damit nur die Kleidung die Farbe bekommt und nicht die Arme oder Beine, lädt das Plugin die Kleidungstextur
//! einmal selbst (wie der Baukasten: RequestTexture 0x5A4220, CStreaming::LoadAllRequestedModels, danach RemoveModel)
//! und merkt sich daraus die Maske. Kopf (Haare samt Gesicht), Schuhe und Zubehör sind reine Kopien und werden ganz
//! gefärbt.
//!
//! Gefärbt wird nur, was der Baukasten selbst lesen kann: 32 Bit je Pixel, D3DFMT_A8R8G8B8 (RenderWare 8888) oder
//! D3DFMT_X8R8G8B8 (888), ohne Palette, ohne Kompression, ohne eigene Mipmap-Stufen. Alles andere bleibt ungefärbt und
//! steht im Log.

use std::collections::HashMap;
use std::ffi::{c_char, CStr};
use std::time::Instant;

use client_api::gta::rw::rwcore::RwRaster;

use crate::farbe::{Farbe, Maske};
use crate::game::{self, read_ptr, read_u32, readable, writable};
use crate::outfit::OutfitKeys;

// gta-reversed RenderWare/rw/rwcore.cpp
const RW_RASTER_LOCK: usize = 0x7FB2D0; // RwUInt8* RwRasterLock(RwRaster*, RwUInt8 level, RwInt32 lockMode)
const RW_RASTER_UNLOCK: usize = 0x7FAEC0; // RwRaster* RwRasterUnlock(RwRaster*)
const RW_TEX_DICTIONARY_FIND_NAMED_TEXTURE: usize = 0x7F39F0;
const RW_TEX_DICTIONARY_FOR_ALL_TEXTURES: usize = 0x7F3730;
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

// gta-reversed ClothesBuilder.cpp: RH_ScopedInstall(RequestTexture, 0x5A4220), int32 RequestTexture(uint32 txdNameKey)
const CCLOTHESBUILDER_REQUEST_TEXTURE: usize = 0x5A4220;
// gta-reversed Streaming.cpp: LoadAllRequestedModels 0x40EA10, RemoveModel 0x4089A0
const CSTREAMING_LOAD_ALL_REQUESTED_MODELS: usize = 0x40EA10;
const CSTREAMING_REMOVE_MODEL: usize = 0x4089A0;
// gta-reversed Streaming.h: RESOURCE_ID_TXD (TXDToModelId)
const RESOURCE_ID_TXD: i32 = 20000;

/// Teil (eClothesTexturePart) -> Name der Textur im Wörterbuch von Modell 0 (BODY_PART_TEX_NAMES in CreateSkinnedClump)
const TEXTUREN: &[(usize, &CStr)] = &[
    (0, c"torso"),
    (1, c"head"),
    (2, c"legs"),
    (3, c"feet"),
    (13, c"necklace"),
    (14, c"watch"),
    (15, c"glasses"),
    (16, c"hat"),
    (17, c"extra1"),
];

/// Oberteil und Hose liegen auf der Haut: nur die Pixel der Kleidung färben
fn mit_haut(part: usize) -> bool {
    part == game::TEX_TORSO || part == game::TEX_LEGS
}

/// So viele Masken (je Kleidungstextur ein Bit pro Pixel, 8 KB bei 256x256) bleiben im Speicher.
const MASKEN_MAX: usize = 32;

pub struct Faerber {
    /// Maske je Texturschlüssel der Kleidung (Oberteil, Hose)
    masken: HashMap<u32, Maske>,
    pub gefaerbt: u32,
    pub fehler: u32,
    letzter_fehler: String,
    letzte_ms: f32,
}

impl Faerber {
    pub fn new() -> Faerber {
        Faerber {
            masken: HashMap::new(),
            gefaerbt: 0,
            fehler: 0,
            letzter_fehler: String::new(),
            letzte_ms: 0.0,
        }
    }

    /// Beim Trennen: Masken freigeben (die Zähler bleiben für /kleidunginfo).
    pub fn vergessen(&mut self) {
        self.masken.clear();
    }

    pub fn zustand(&self) -> String {
        let kb: usize = self.masken.values().map(Maske::bytes).sum::<usize>().div_ceil(1024);
        let mut text = format!(
            "{} Teile gefärbt (zuletzt {:.1} ms), {} nicht, Masken {} ({kb} KB)",
            self.gefaerbt,
            self.letzte_ms,
            self.fehler,
            self.masken.len()
        );
        if !self.letzter_fehler.is_empty() {
            let grund: String = self.letzter_fehler.chars().take(50).collect();
            text += &format!(", zuletzt: {grund}");
        }
        text
    }

    /// Game thread, direkt nach einem Umbau, der gewirkt hat: `clump` ist der Clump der gebauten Figur.
    /// Jeder Fehler lässt das Teil ungefärbt und steht im Log.
    pub fn anwenden(&mut self, wer: &str, clump: usize, keys: &OutfitKeys) {
        if keys.gefaerbt() == 0 {
            return;
        }

        let start = Instant::now();
        let dict = game::model0_txd();
        let figur = game::clump_textures(clump);

        for &(part, name) in TEXTUREN {
            let Some(farbe) = keys.farben[part] else {
                continue;
            };
            let name_text = name.to_str().unwrap_or("?");

            let ergebnis = match dict {
                None => Err("Texturen von Modell 0 nicht gefunden".to_owned()),
                Some(dict) => self.teil(dict, &figur, part, name, farbe, keys.parts[part].0),
            };

            match ergebnis {
                Ok(text) => {
                    self.gefaerbt += 1;
                    crate::log!("  {wer}: Teil {part} ({name_text}) #{} gefärbt, {text}", farbe.hex());
                }
                Err(grund) => {
                    self.fehler += 1;
                    crate::log!("  {wer}: Teil {part} ({name_text}) #{} nicht gefärbt: {grund}", farbe.hex());
                    self.letzter_fehler = format!("Teil {part}: {grund}");
                }
            }
        }

        self.letzte_ms = start.elapsed().as_secs_f32() * 1000.0;
    }

    fn teil(&mut self, dict: usize, figur: &[usize], part: usize, name: &CStr, farbe: Farbe, key: u32) -> Result<String, String> {
        let find: extern "C" fn(usize, *const c_char) -> usize =
            unsafe { std::mem::transmute(RW_TEX_DICTIONARY_FIND_NAMED_TEXTURE) };
        let textur = find(dict, name.as_ptr());

        if textur == 0 || !readable(textur, TEXTURE_NAME + 32) {
            return Err("der Baukasten hat diese Textur nicht gebaut (Teil verdeckt?)".to_owned());
        }
        // nur eine Textur, die im Wörterbuch von Modell 0 liegt und die die Figur wirklich trägt
        if read_ptr(textur + TEXTURE_DICT) != Some(dict) {
            return Err(format!("Textur {textur:#x} gehört nicht zu Modell 0"));
        }
        if !figur.contains(&textur) {
            return Err(format!("die Figur trägt die Textur {textur:#x} nicht"));
        }

        let raster = read_ptr(textur).ok_or("Textur ohne Raster")?;
        let info = RasterInfo::lesen(raster)?;
        let mit_alpha = info.faerbbar()?;

        let maske = if mit_haut(part) {
            if key == 0 {
                return Err("ohne Kleidung".to_owned());
            }
            Some(self.maske(key)?)
        } else {
            None
        };

        let ms_start = Instant::now();
        let mut gesperrt = Gesperrt::neu(raster, LOCK_READWRITE, true)?;
        let stride = gesperrt.stride;
        let ergebnis = crate::farbe::faerben(gesperrt.pixel(), info.breite, info.hoehe, stride, mit_alpha, farbe, maske)?;
        drop(gesperrt);

        Ok(format!(
            "{} von {} Pixeln, Bezug {}, {}x{} {}{}, {:.1} ms",
            ergebnis.pixel,
            info.breite * info.hoehe,
            ergebnis.bezug,
            info.breite,
            info.hoehe,
            info.d3d_name(),
            if maske.is_some() { ", nur die Kleidung" } else { "" },
            ms_start.elapsed().as_secs_f32() * 1000.0
        ))
    }

    fn maske(&mut self, key: u32) -> Result<&Maske, String> {
        if !self.masken.contains_key(&key) {
            if self.masken.len() >= MASKEN_MAX {
                self.masken.clear();
            }
            let start = Instant::now();
            let maske = maske_laden(key)?;
            crate::log!(
                "  Maske der Kleidung {key:08x}: {}x{}, {} Pixel Kleidung, geladen in {:.1} ms",
                maske.breite,
                maske.hoehe,
                maske.anzahl(),
                start.elapsed().as_secs_f32() * 1000.0
            );
            self.masken.insert(key, maske);
        }
        self.masken.get(&key).ok_or_else(|| "Maske fehlt".to_owned())
    }
}

/// Lädt die Kleidungstextur `key` aus player.img in einen der Plätze des Baukastens, liest daraus die Maske und entlädt
/// sie wieder; dieselben Schritte wie GetTextureFromTxdAndLoadNextTxd (0x5A5F70).
fn maske_laden(key: u32) -> Result<Maske, String> {
    let request: extern "C" fn(u32) -> i32 = unsafe { std::mem::transmute(CCLOTHESBUILDER_REQUEST_TEXTURE) };
    let load_all: extern "C" fn(bool) = unsafe { std::mem::transmute(CSTREAMING_LOAD_ALL_REQUESTED_MODELS) };
    let remove: extern "C" fn(i32) = unsafe { std::mem::transmute(CSTREAMING_REMOVE_MODEL) };

    let slot = request(key);
    if slot < 0 {
        return Err(format!("Kleidungstextur {key:08x} nicht angefordert"));
    }

    load_all(true);
    let ergebnis = maske_aus_txd(slot);
    remove(slot + RESOURCE_ID_TXD);
    ergebnis.map_err(|grund| format!("Maske der Kleidung {key:08x}: {grund}"))
}

extern "C" fn erste_textur_cb(textur: usize, data: *mut usize) -> usize {
    unsafe { *data = textur };
    0
}

fn maske_aus_txd(slot: i32) -> Result<Maske, String> {
    let dict = game::get_txd(slot);
    if dict == 0 || !readable(dict, 0x10) {
        return Err(format!("TXD {slot} nicht geladen"));
    }

    // GetFirstTexture (gta-reversed RenderWare/rw/rwtexdict.cpp): RwTexDictionaryForAllTextures bis zur ersten
    let for_all: extern "C" fn(usize, extern "C" fn(usize, *mut usize) -> usize, *mut usize) -> usize =
        unsafe { std::mem::transmute(RW_TEX_DICTIONARY_FOR_ALL_TEXTURES) };
    let mut textur = 0usize;
    for_all(dict, erste_textur_cb, &mut textur);

    let raster = read_ptr(textur).ok_or("TXD ohne Textur")?;
    let info = RasterInfo::lesen(raster)?;
    info.faerbbar()?;

    let mut gesperrt = Gesperrt::neu(raster, LOCK_READ, false)?;
    let stride = gesperrt.stride;
    Maske::aus_pixeln(gesperrt.pixel(), info.breite, info.hoehe, stride).ok_or_else(|| "Raster zu klein".to_owned())
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
