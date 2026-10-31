//! Farben für Kleidungsteile (ab 0.9.0): `@RRGGBB` hinter einem Teil in `kleidung:spieler`, und wie ein Teil damit
//! gefärbt wird. Hier steht nur Rechnen auf Pixeln, ohne Aufrufe ins Spiel; die Texturen holt faerben.rs.
//!
//! Färben: Helligkeit des Pixels mal Zielfarbe. Damit helle Grundteile die volle Farbe ergeben, wird die Helligkeit
//! auf das 95. Perzentil der gefärbten Pixel bezogen (das hellste Weiß eines T-Shirts ergibt genau die Zielfarbe,
//! Falten und Schatten werden dunkler). Sehr dunkle Zielfarben (Schwarz) werden auf eine Mindesthelligkeit angehoben,
//! sonst wäre alles einfach Schwarz ohne Schattierung. Alpha bleibt, wie es ist.

/// Eine Farbe aus `@RRGGBB`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Farbe {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Farbe {
    /// Genau sechs Hex-Ziffern (groß oder klein), ohne `@`.
    pub fn parse(text: &str) -> Option<Farbe> {
        if text.len() != 6 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }

        let wert = u32::from_str_radix(text, 16).ok()?;
        Some(Farbe {
            r: (wert >> 16) as u8,
            g: (wert >> 8) as u8,
            b: wert as u8,
        })
    }

    pub fn hex(self) -> String {
        format!("{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

/// Helligkeit 0 bis 255 nach ITU-R BT.601 (wie die Graustufen von Bildprogrammen).
pub fn helligkeit(r: u8, g: u8, b: u8) -> u32 {
    (299 * r as u32 + 587 * g as u32 + 114 * b as u32 + 500) / 1000
}

/// Zielfarben dunkler als das werden gleichmäßig auf diese Helligkeit angehoben: Schwarz ergibt #303030 an den hellsten
/// Stellen, Schatten und Falten bleiben sichtbar. Dunkelblau bleibt blau (es kommt nur Grau dazu).
pub const DUNKEL_MIN: u32 = 0x30;

/// Bezugshelligkeit höchstens um den Faktor 4 verstärken: Ein dunkles Grundteil (schwarze Hose) ergibt dann eine
/// dunklere Farbe, statt das Rauschen der Textur hochzuziehen.
pub const BEZUG_MIN: u32 = 64;

/// Anteil der gefärbten Pixel, die heller als der Bezug sein dürfen (die werden voll zur Zielfarbe).
const PERZENTIL: u64 = 95;

/// Zielfarbe nach dem Anheben dunkler Farben (siehe DUNKEL_MIN).
pub fn angehoben(farbe: Farbe) -> Farbe {
    let h = helligkeit(farbe.r, farbe.g, farbe.b);
    if h >= DUNKEL_MIN {
        return farbe;
    }
    let plus = (DUNKEL_MIN - h) as u8;
    Farbe {
        r: farbe.r.saturating_add(plus),
        g: farbe.g.saturating_add(plus),
        b: farbe.b.saturating_add(plus),
    }
}

/// Ein Kanal: Zielkanal mal Helligkeit / Bezug, höchstens der Zielkanal.
fn kanal(ziel: u8, h: u32, bezug: u32) -> u8 {
    let h = h.min(bezug);
    ((ziel as u32 * h + bezug / 2) / bezug) as u8
}

/// Welche Pixel zum Kleidungsstück gehören (bei Oberteil und Hose: Pixel, an denen die Kleidung über der Haut liegt).
#[derive(Clone, Debug, PartialEq)]
pub struct Maske {
    pub breite: usize,
    pub hoehe: usize,
    bits: Vec<u64>,
}

impl Maske {
    pub fn neu(breite: usize, hoehe: usize) -> Maske {
        Maske {
            breite,
            hoehe,
            bits: vec![0; (breite * hoehe).div_ceil(64)],
        }
    }

    fn setzen(&mut self, x: usize, y: usize) {
        let i = y * self.breite + x;
        self.bits[i / 64] |= 1 << (i % 64);
    }

    pub fn gesetzt(&self, x: usize, y: usize) -> bool {
        let i = y * self.breite + x;
        self.bits[i / 64] & (1 << (i % 64)) != 0
    }

    pub fn anzahl(&self) -> usize {
        self.bits.iter().map(|b| b.count_ones() as usize).sum()
    }

    /// Wie CClothesBuilder::PlaceTextureOnTopOfTexture (gta-reversed ClothesBuilder.cpp, 0x5A57B0) entscheidet, welche
    /// Pixel der Kleidung über die Haut kommen: das oberste Byte des 32-Bit-Pixels (Alpha) ist nicht 0.
    pub fn aus_pixeln(pixel: &[u8], breite: usize, hoehe: usize, stride: usize) -> Option<Maske> {
        if !passt(pixel.len(), breite, hoehe, stride) {
            return None;
        }

        let mut maske = Maske::neu(breite, hoehe);
        for y in 0..hoehe {
            let zeile = &pixel[y * stride..y * stride + breite * 4];
            for (x, p) in zeile.chunks_exact(4).enumerate() {
                if p[3] != 0 {
                    maske.setzen(x, y);
                }
            }
        }
        Some(maske)
    }

    /// Ungefährer Speicher in Bytes (für /kleidunginfo).
    pub fn bytes(&self) -> usize {
        self.bits.len() * 8
    }
}

/// Puffer groß genug für `hoehe` Zeilen mit `stride` Bytes, jede mit `breite` Pixeln zu 4 Bytes.
fn passt(laenge: usize, breite: usize, hoehe: usize, stride: usize) -> bool {
    breite > 0
        && hoehe > 0
        && breite.checked_mul(4).is_some_and(|zeile| stride >= zeile)
        && stride
            .checked_mul(hoehe - 1)
            .and_then(|v| v.checked_add(breite * 4))
            .is_some_and(|noetig| laenge >= noetig)
}

/// Ergebnis von `faerben`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gefaerbt {
    /// so viele Pixel wurden umgefärbt
    pub pixel: usize,
    /// Helligkeit, die die volle Zielfarbe ergibt
    pub bezug: u32,
}

/// Färbt 32-Bit-Pixel im Speicher von D3DFMT_A8R8G8B8/X8R8G8B8 (Bytes B, G, R, A).
/// `mit_alpha`: Pixel mit Alpha 0 sind unsichtbar und bleiben, wie sie sind (bei X8R8G8B8 false, das vierte Byte ist
/// dann kein Alpha). `maske`: nur diese Pixel; sie muss genau so groß sein wie das Bild.
pub fn faerben(
    pixel: &mut [u8],
    breite: usize,
    hoehe: usize,
    stride: usize,
    mit_alpha: bool,
    farbe: Farbe,
    maske: Option<&Maske>,
) -> Result<Gefaerbt, String> {
    if !passt(pixel.len(), breite, hoehe, stride) {
        return Err(format!("Puffer passt nicht zu {breite}x{hoehe}, Zeile {stride} Bytes"));
    }
    if let Some(m) = maske {
        if m.breite != breite || m.hoehe != hoehe {
            return Err(format!("Maske {}x{} passt nicht zur Textur {breite}x{hoehe}", m.breite, m.hoehe));
        }
    }

    let dabei = |p: &[u8], x: usize, y: usize| (!mit_alpha || p[3] != 0) && maske.is_none_or(|m| m.gesetzt(x, y));

    // 1. Häufigkeit jeder Helligkeit unter den Pixeln, die gefärbt werden
    let mut verteilung = [0u64; 256];
    let mut anzahl = 0u64;
    for y in 0..hoehe {
        let zeile = &pixel[y * stride..y * stride + breite * 4];
        for (x, p) in zeile.chunks_exact(4).enumerate() {
            if dabei(p, x, y) {
                verteilung[helligkeit(p[2], p[1], p[0]) as usize] += 1;
                anzahl += 1;
            }
        }
    }

    if anzahl == 0 {
        return Err("keine sichtbaren Pixel zum Färben".to_owned());
    }

    // 2. Bezug: das 95. Perzentil der Helligkeit, mindestens BEZUG_MIN
    let grenze = (anzahl * PERZENTIL).div_ceil(100);
    let mut summe = 0u64;
    let mut bezug = 255u32;
    for (h, &n) in verteilung.iter().enumerate() {
        summe += n;
        if summe >= grenze {
            bezug = h as u32;
            break;
        }
    }
    let bezug = bezug.max(BEZUG_MIN);

    // 3. Färben
    let ziel = angehoben(farbe);
    for y in 0..hoehe {
        let zeile = &mut pixel[y * stride..y * stride + breite * 4];
        for (x, p) in zeile.chunks_exact_mut(4).enumerate() {
            if dabei(&*p, x, y) {
                let h = helligkeit(p[2], p[1], p[0]);
                p[0] = kanal(ziel.b, h, bezug);
                p[1] = kanal(ziel.g, h, bezug);
                p[2] = kanal(ziel.r, h, bezug);
            }
        }
    }

    Ok(Gefaerbt {
        pixel: anzahl as usize,
        bezug,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bild(pixel: &[(u8, u8, u8, u8)]) -> Vec<u8> {
        pixel.iter().flat_map(|&(r, g, b, a)| [b, g, r, a]).collect()
    }

    fn rgb(puffer: &[u8], i: usize) -> (u8, u8, u8) {
        (puffer[i * 4 + 2], puffer[i * 4 + 1], puffer[i * 4])
    }

    #[test]
    fn farbe_lesen() {
        assert_eq!(Farbe::parse("FFDC00"), Some(Farbe { r: 0xFF, g: 0xDC, b: 0 }));
        assert_eq!(Farbe::parse("be00ff"), Some(Farbe { r: 0xBE, g: 0, b: 0xFF }));
        assert_eq!(Farbe::parse("000000"), Some(Farbe { r: 0, g: 0, b: 0 }));
        for falsch in ["", "FFF", "FFDC0", "FFDC000", "GGDC00", "+FDC00", " FDC00", "FFDC0\u{e4}"] {
            assert_eq!(Farbe::parse(falsch), None, "{falsch:?}");
        }
        assert_eq!(Farbe { r: 0xBE, g: 0, b: 0xFF }.hex(), "BE00FF");
    }

    #[test]
    fn weiss_wird_genau_die_zielfarbe() {
        let mut p = bild(&[(255, 255, 255, 255); 20]);
        let lila = Farbe { r: 0xBE, g: 0, b: 0xFF };
        let r = faerben(&mut p, 20, 1, 80, true, lila, None).unwrap();
        assert_eq!(r.pixel, 20);
        for i in 0..20 {
            assert_eq!(rgb(&p, i), (0xBE, 0, 0xFF));
        }
    }

    #[test]
    fn schattierung_bleibt() {
        // 18 helle Pixel, eine Falte (halb so hell) und ein Schatten
        let mut pixel = vec![(240, 240, 240, 255); 18];
        pixel.push((120, 120, 120, 255));
        pixel.push((30, 30, 30, 255));
        let mut p = bild(&pixel);
        let gelb = Farbe { r: 0xFF, g: 0xDC, b: 0 };
        let r = faerben(&mut p, 20, 1, 80, true, gelb, None).unwrap();
        assert_eq!(r.bezug, 240);
        assert_eq!(rgb(&p, 0), (0xFF, 0xDC, 0), "hell = volle Farbe");
        let (fr, fg, fb) = rgb(&p, 18);
        assert_eq!((fr, fg, fb), (128, 110, 0), "Falte halb so hell, gleicher Farbton");
        let (sr, sg, _) = rgb(&p, 19);
        assert!(sr < fr && sg < fg && sr > 0, "Schatten dunkler als die Falte");
    }

    #[test]
    fn schwarz_ist_dunkel_mit_schattierung() {
        let mut pixel = vec![(250, 250, 250, 255); 19];
        pixel.push((125, 125, 125, 255));
        let mut p = bild(&pixel);
        faerben(&mut p, 20, 1, 80, true, Farbe { r: 0, g: 0, b: 0 }, None).unwrap();
        assert_eq!(rgb(&p, 0), (0x30, 0x30, 0x30));
        assert_eq!(rgb(&p, 19), (0x18, 0x18, 0x18));
    }

    #[test]
    fn dunkelblau_bleibt_blau() {
        let z = angehoben(Farbe { r: 0, g: 0, b: 0x80 });
        assert!(z.b > z.r + 0x60 && z.r == z.g);
        assert!(helligkeit(z.r, z.g, z.b) >= DUNKEL_MIN - 1);
        // helle Farben bleiben unverändert
        assert_eq!(angehoben(Farbe { r: 0xFF, g: 0xDC, b: 0 }), Farbe { r: 0xFF, g: 0xDC, b: 0 });
    }

    #[test]
    fn dunkles_grundteil_wird_nicht_ueberverstaerkt() {
        let mut p = bild(&[(20, 20, 20, 255); 10]);
        let r = faerben(&mut p, 10, 1, 40, true, Farbe { r: 0xFF, g: 0, b: 0 }, None).unwrap();
        assert_eq!(r.bezug, BEZUG_MIN);
        let (rot, _, _) = rgb(&p, 0);
        assert_eq!(rot as u32, (255 * 20 + 32) / 64);
    }

    #[test]
    fn alpha_bleibt_und_unsichtbares_bleibt() {
        let mut p = bild(&[(200, 200, 200, 128), (200, 10, 10, 0), (200, 200, 200, 255)]);
        faerben(&mut p, 3, 1, 12, true, Farbe { r: 0, g: 0xFF, b: 0 }, None).unwrap();
        assert_eq!(p[3], 128);
        assert_eq!(&p[4..8], &[10, 10, 200, 0], "Alpha 0 bleibt, wie es ist");
        assert_eq!(p[11], 255);
        assert_eq!(rgb(&p, 0), (0, 0xFF, 0));
    }

    #[test]
    fn ohne_alpha_zaehlt_jedes_pixel() {
        let mut p = bild(&[(200, 200, 200, 0), (200, 200, 200, 0)]);
        let r = faerben(&mut p, 2, 1, 8, false, Farbe { r: 0xFF, g: 0, b: 0 }, None).unwrap();
        assert_eq!(r.pixel, 2);
        assert_eq!(rgb(&p, 1), (0xFF, 0, 0));
        assert_eq!(p[7], 0, "viertes Byte bleibt");
    }

    #[test]
    fn maske_aus_kleidung() {
        // 2x2, Zeile 12 Bytes (ein Pixel Abstand am Ende): Kleidung nur links oben und rechts unten
        let mut kleidung = vec![0u8; 24];
        kleidung[3] = 255;
        kleidung[12 + 4 + 3] = 1;
        let maske = Maske::aus_pixeln(&kleidung, 2, 2, 12).unwrap();
        assert!(maske.gesetzt(0, 0) && !maske.gesetzt(1, 0) && !maske.gesetzt(0, 1) && maske.gesetzt(1, 1));
        assert_eq!(maske.anzahl(), 2);

        // Haut bleibt, Kleidung wird rot
        let mut p = vec![0u8; 24];
        for (i, (r, g, b)) in [(0usize, (255u8, 255u8, 255u8)), (4, (180, 140, 110)), (12, (180, 140, 110)), (16, (255, 255, 255))] {
            p[i..i + 4].copy_from_slice(&[b, g, r, 255]);
        }
        let r = faerben(&mut p, 2, 2, 12, true, Farbe { r: 0xFF, g: 0, b: 0 }, Some(&maske)).unwrap();
        assert_eq!(r.pixel, 2);
        assert_eq!(&p[0..4], &[0, 0, 255, 255]);
        assert_eq!(&p[4..8], &[110, 140, 180, 255], "Haut unverändert");
        assert_eq!(&p[16..20], &[0, 0, 255, 255]);
    }

    #[test]
    fn falsche_groessen() {
        let mut p = vec![255u8; 16];
        assert!(faerben(&mut p, 2, 2, 7, true, Farbe { r: 1, g: 2, b: 3 }, None).is_err(), "Zeile zu kurz");
        assert!(faerben(&mut p, 3, 2, 12, true, Farbe { r: 1, g: 2, b: 3 }, None).is_err(), "Puffer zu klein");
        let maske = Maske::neu(4, 4);
        assert!(faerben(&mut p, 2, 2, 8, true, Farbe { r: 1, g: 2, b: 3 }, Some(&maske)).is_err());
        assert!(Maske::aus_pixeln(&p, 0, 2, 8).is_none());
        let mut leer = bild(&[(1, 2, 3, 0)]);
        assert!(faerben(&mut leer, 1, 1, 4, true, Farbe { r: 1, g: 2, b: 3 }, None).is_err(), "nichts sichtbar");
    }
}
