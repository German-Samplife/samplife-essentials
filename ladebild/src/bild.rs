//! Das Ladebild `samplife-ladebild.jpg` aus dem GTA-Verzeichnis, als BGRA-Pixel für eine
//! Direct3D-Textur (D3DFMT_A8R8G8B8). Liegt die Datei nicht da, bleibt das Ladebild aus.

use std::sync::OnceLock;

use crate::log::spielordner;

pub const DATEI: &str = "samplife-ladebild.jpg";

pub struct Bild {
    pub breite: u32,
    pub hoehe: u32,
    /// Zeile für Zeile, je Pixel B, G, R, A
    pub bgra: Vec<u8>,
}

static BILD: OnceLock<Bild> = OnceLock::new();

pub fn bild() -> Option<&'static Bild> {
    BILD.get()
}

/// Liest und entpackt das Bild. Rückgabe: Breite und Höhe.
pub fn laden() -> Result<(u32, u32), String> {
    let pfad = spielordner().join(DATEI);
    let daten = std::fs::read(&pfad).map_err(|fehler| format!("{} nicht lesbar ({fehler})", pfad.display()))?;
    let mut dekoder = jpeg_decoder::Decoder::new(daten.as_slice());
    let pixel = dekoder.decode().map_err(|fehler| format!("{DATEI} ist kein lesbares JPEG ({fehler})"))?;
    let info = dekoder.info().ok_or("JPEG ohne Angaben zur Größe")?;
    let (breite, hoehe) = (info.width as u32, info.height as u32);
    if breite == 0 || hoehe == 0 || breite > 4096 || hoehe > 4096 {
        return Err(format!("{DATEI} hat eine unpassende Größe ({breite} x {hoehe})"));
    }

    let anzahl = (breite * hoehe) as usize;
    let mut bgra = Vec::with_capacity(anzahl * 4);
    match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            for rgb in pixel.chunks_exact(3) {
                bgra.extend_from_slice(&[rgb[2], rgb[1], rgb[0], 255]);
            }
        }
        jpeg_decoder::PixelFormat::L8 => {
            for &grau in &pixel {
                bgra.extend_from_slice(&[grau, grau, grau, 255]);
            }
        }
        andere => return Err(format!("{DATEI}: Farbformat {andere:?} wird nicht unterstützt")),
    }
    if bgra.len() != anzahl * 4 {
        return Err(format!("{DATEI}: unerwartete Pixelzahl"));
    }

    let _ = BILD.set(Bild { breite, hoehe, bgra });
    Ok((breite, hoehe))
}
