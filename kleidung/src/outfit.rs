//! Outfits: the clothes the server sends per player (`kleidung:spieler`) and the
//! standard CJ for players the server hasn't sent anything for. Texture/model names
//! are the pairs from models/player.img (MTA:SA, CClientPlayerClothes.cpp,
//! m_DefaultClothes: index = eClothesTexturePart).
//! Ab 0.9.0 darf jedes Teil außer den Tattoos eine Farbe haben (`textur/modell@RRGGBB`, siehe farbe.rs und faerben.rs).
//! Ab 0.9.1 färbt das Plugin davon nur Oberteil und Hose (farbe::faerbbar); Farben an anderen Stellen werden weiter
//! angenommen, damit das Outfit nicht verworfen wird, aber nicht angewendet.

use crate::farbe::{self, Farbe};
use crate::game::{
    self, ClothesDesc, CLOTHES_TEXTURE_TOTAL, TEX_HEAD, TEX_LEGS, TEX_SHOES, TEX_TORSO,
};

/// Longest texture or model name in player.img is well below this; the game keys
/// names with CKeyGen, so length only guards against garbage.
const NAME_MAX: usize = 24;

/// One outfit by name: (texture, model) per texture part, None = nothing worn there.
/// Tattoos (parts 4 to 12) have no model; their model name is empty.
pub type Teile = [Option<(String, String)>; CLOTHES_TEXTURE_TOTAL];

/// Farbe je Teil aus `@RRGGBB`, None = so, wie GTA es zeigt.
pub type Farben = [Option<Farbe>; CLOTHES_TEXTURE_TOTAL];

/// Tattoo-Stellen 4 bis 12 (eClothesTexturePart): keine Farbe, sie liegen auf der Haut
pub fn ist_tattoo(part: usize) -> bool {
    game::texture_dependency(part).is_none()
}

/// Stellen mit Farbe, die das Plugin nicht färbt (alles außer Oberteil und Hose), für das Log.
pub fn ignorierte_farben(farben: &Farben) -> Vec<usize> {
    (0..CLOTHES_TEXTURE_TOTAL)
        .filter(|&part| farben[part].is_some() && !farbe::faerbbar(part))
        .collect()
}

/// CJ at the start of the single player game: white vest, jeans, sneakers, short hair
/// (the clothes main.scm gives the player before the first rebuild).
const STANDARD: &[(usize, &str, &str)] = &[
    (TEX_TORSO, "vest", "vest"),
    (TEX_HEAD, "player_face", "head"),
    (TEX_LEGS, "jeansdenim", "jeans"),
    (TEX_SHOES, "sneakerbincblk", "sneaker"),
];

fn name_ok(name: &str) -> bool {
    !name.is_empty() && name.len() <= NAME_MAX && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Parses the 18 parts of `kleidung:spieler`: `textur/modell`, `textur` (tattoos, no
/// model) or `-` (nothing). Anything else makes the whole outfit invalid.
/// Ab 0.9.0 darf hinter jedem Teil außer den Tattoos `@RRGGBB` stehen (`bandred/bandana@FFDC00`).
pub fn parse_teile<'a>(tokens: impl Iterator<Item = &'a str>) -> Result<(Teile, Farben), String> {
    let mut teile: Teile = Default::default();
    let mut farben: Farben = [None; CLOTHES_TEXTURE_TOTAL];
    let mut count = 0;

    for (part, token) in tokens.enumerate() {
        if part >= CLOTHES_TEXTURE_TOTAL {
            return Err(format!("mehr als {CLOTHES_TEXTURE_TOTAL} Teile"));
        }

        count += 1;

        if token == "-" {
            continue;
        }

        let (teil, farbe) = match token.split_once('@') {
            None => (token, None),
            Some(_) if ist_tattoo(part) => return Err(format!("Teil {part} ist ein Tattoo, ohne Farbe: {token:?}")),
            Some(("-", _)) => return Err(format!("Teil {part}: Farbe ohne Kleidung: {token:?}")),
            Some((teil, hex)) => match Farbe::parse(hex) {
                Some(farbe) => (teil, Some(farbe)),
                None => return Err(format!("Teil {part}: Farbe ungültig (RRGGBB): {token:?}")),
            },
        };

        let (texture, model) = teil.split_once('/').unwrap_or((teil, ""));

        if !name_ok(texture) || (teil.contains('/') && !name_ok(model)) {
            return Err(format!("Teil {part} ungültig: {token:?}"));
        }

        teile[part] = Some((texture.to_ascii_lowercase(), model.to_ascii_lowercase()));
        farben[part] = farbe;
    }

    if count != CLOTHES_TEXTURE_TOTAL {
        return Err(format!("{count} statt {CLOTHES_TEXTURE_TOTAL} Teile"));
    }

    Ok((teile, farben))
}

/// Höchstwert von Fett und Muskeln wie die Statistiken von GTA (STAT_FAT, STAT_MUSCLE)
pub const KOERPER_MAX: u16 = 1000;

/// `kleidung:koerper <id> <fett> <muskeln>`: zwei ganze Zahlen von 0 bis KOERPER_MAX
pub fn parse_koerper<'a>(mut tokens: impl Iterator<Item = &'a str>) -> Result<(u16, u16), String> {
    let mut wert = |name: &str| -> Result<u16, String> {
        let t = tokens.next().ok_or_else(|| format!("{name} fehlt"))?;
        t.parse::<u16>()
            .ok()
            .filter(|&v| v <= KOERPER_MAX)
            .ok_or_else(|| format!("{name} ungültig: {t}"))
    };
    let fett = wert("Fett")?;
    let muskeln = wert("Muskeln")?;
    if tokens.next().is_some() {
        return Err("zu viele Argumente".to_owned());
    }
    Ok((fett, muskeln))
}

/// Keys of one outfit, computed with the game's own CKeyGen::GetUppercaseKey.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OutfitKeys {
    /// (texture key, model key) per texture part, 0/0 = nothing
    pub parts: [(u32, u32); CLOTHES_TEXTURE_TOTAL],
    /// (Fett, Muskeln) von 0 bis KOERPER_MAX aus `kleidung:koerper`; None: der Körper bleibt, wie er ist
    pub koerper: Option<(u16, u16)>,
    /// Farbe je Teil (`@RRGGBB`); gehört zum Outfit, damit eine Figur mit anderer Farbe neu gebaut wird
    pub farben: Farben,
}

fn key(name: &str) -> u32 {
    if name.is_empty() {
        0
    } else {
        game::uppercase_key(name)
    }
}

impl OutfitKeys {
    /// Game thread only (calls into gta_sa.exe).
    pub fn standard() -> OutfitKeys {
        let mut parts = [(0u32, 0u32); CLOTHES_TEXTURE_TOTAL];

        for &(part, texture, model) in STANDARD {
            parts[part] = (key(texture), key(model));
        }

        OutfitKeys { parts, koerper: None, farben: [None; CLOTHES_TEXTURE_TOTAL] }
    }

    /// Game thread only (calls into gta_sa.exe).
    pub fn from_teile(teile: &Teile, farben: &Farben) -> OutfitKeys {
        let mut parts = [(0u32, 0u32); CLOTHES_TEXTURE_TOTAL];

        for (part, teil) in teile.iter().enumerate() {
            if let Some((texture, model)) = teil {
                parts[part] = (key(texture), key(model));
            }
        }

        // eine Farbe nur an einem Teil, das getragen wird und gefärbt werden kann; die anderen gehören nicht zum Outfit,
        // damit sie keinen Umbau auslösen
        let mut farben = *farben;
        for (part, f) in farben.iter_mut().enumerate() {
            if parts[part].0 == 0 || ist_tattoo(part) || !farbe::faerbbar(part) {
                *f = None;
            }
        }

        OutfitKeys { parts, koerper: None, farben }
    }

    /// Teile mit Farbe
    pub fn gefaerbt(&self) -> usize {
        self.farben.iter().filter(|f| f.is_some()).count()
    }

    pub fn mit_koerper(mut self, koerper: Option<(u16, u16)>) -> OutfitKeys {
        self.koerper = koerper;
        self
    }

    /// Writes the outfit into a desc: every texture part (also tattoos) and every
    /// model part that depends on a texture is set, everything else (hands) stays as it is.
    /// Fett und Muskeln nur, wenn der Server sie geschickt hat (die Muskeln liest der Baukasten aus der Statistik, siehe
    /// rebuild::umbauen; in der Beschreibung stehen sie für den Abgleich).
    pub fn apply(&self, desc: &mut ClothesDesc) {
        for (texture_part, &(texture, model)) in self.parts.iter().enumerate() {
            desc.set_texture_and_model(texture, model, texture_part);
        }
        if let Some((fett, muskeln)) = self.koerper {
            desc.fat_stat = fett as f32;
            desc.muscle_stat = muskeln as f32;
        }
    }

    pub fn matches(&self, desc: &ClothesDesc) -> bool {
        let koerper_ok = self
            .koerper
            .is_none_or(|(fett, muskeln)| desc.fat_stat == fett as f32 && desc.muscle_stat == muskeln as f32);
        koerper_ok
            && self.parts.iter().enumerate().all(|(texture_part, &(texture, model))| {
                desc.texture_keys[texture_part] == texture
                    && game::texture_dependency(texture_part)
                        .map(|model_part| desc.model_keys[model_part] == model)
                        .unwrap_or(true)
            })
    }

    /// Short form for the log: torso texture key and how many parts are worn.
    pub fn kurz(&self) -> String {
        let worn = self.parts.iter().filter(|&&(texture, _)| texture != 0).count();
        let mut text = format!("Oberteil {:08x}, {worn} Teile", self.parts[TEX_TORSO].0);
        if let Some((fett, muskeln)) = self.koerper {
            text += &format!(", Fett {fett}, Muskeln {muskeln}");
        }
        let farben: Vec<String> = self
            .farben
            .iter()
            .enumerate()
            .filter_map(|(part, farbe)| farbe.map(|f| format!("{part}:{}", f.hex())))
            .collect();
        if !farben.is_empty() {
            text += &format!(", Farben {}", farben.join(" "));
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn achtzehn_teile() {
        let text = "suit1red/suit1 bald/head suit1trred/suit1tr shoedressblk/shoe - - - - - - - - - - watchgno/watch - cowboy/cowboy -";
        let (teile, farben) = parse_teile(text.split_whitespace()).unwrap();
        assert_eq!(teile[0], Some(("suit1red".into(), "suit1".into())));
        assert_eq!(teile[4], None);
        assert_eq!(teile[16], Some(("cowboy".into(), "cowboy".into())));
        assert!(farben.iter().all(|f| f.is_none()), "ohne @ keine Farbe");
    }

    #[test]
    fn tattoo_ohne_modell() {
        let mut parts = vec!["-"; 18];
        parts[4] = "4weed";
        let (teile, _) = parse_teile(parts.into_iter()).unwrap();
        assert_eq!(teile[4], Some(("4weed".into(), String::new())));
    }

    #[test]
    fn farben() {
        let text = "tshirtwhite/tshirt@BE00FF player_face/head jeansdenim/jeans@000000 sneakerbincblk/sneaker - - - - - - - - - neckgold/neck2@ffdc00 - - bandred/bandana@FFDC00 -";
        let (teile, farben) = parse_teile(text.split_whitespace()).unwrap();
        assert_eq!(teile[0], Some(("tshirtwhite".into(), "tshirt".into())));
        assert_eq!(farben[0], Some(Farbe { r: 0xBE, g: 0, b: 0xFF }));
        assert_eq!(farben[1], None);
        assert_eq!(farben[2], Some(Farbe { r: 0, g: 0, b: 0 }));
        assert_eq!(farben[13], Some(Farbe { r: 0xFF, g: 0xDC, b: 0 }));
        assert_eq!(teile[16], Some(("bandred".into(), "bandana".into())));
        assert_eq!(farben[16], Some(Farbe { r: 0xFF, g: 0xDC, b: 0 }));
        assert_eq!(farben.iter().filter(|f| f.is_some()).count(), 4);
        // ab 0.9.1 färbt das Plugin nur Oberteil und Hose
        assert_eq!(ignorierte_farben(&farben), vec![13, 16]);
    }

    #[test]
    fn falsche_farben() {
        for (stelle, falsch) in [
            (0, "tshirtwhite/tshirt@"),
            (0, "tshirtwhite/tshirt@FFF"),
            (0, "tshirtwhite/tshirt@FFDC000"),
            (0, "tshirtwhite/tshirt@XXDC00"),
            (0, "tshirtwhite/tshirt@#FFDC00"),
            (0, "tshirtwhite/tshirt@FFDC00@FFDC00"),
            (0, "@FFDC00"),
            (0, "-@FFDC00"),
            (0, "tshirtwhite@FFDC00/tshirt"),
            (4, "4weed@FFDC00"),
            (12, "12bullet@FFDC00"),
        ] {
            let mut parts = vec!["-"; 18];
            parts[stelle] = falsch;
            assert!(parse_teile(parts.into_iter()).is_err(), "{falsch}");
        }
    }

    #[test]
    fn falsche_anzahl() {
        assert!(parse_teile(vec!["-"; 17].into_iter()).is_err());
        assert!(parse_teile(vec!["-"; 19].into_iter()).is_err());
    }

    #[test]
    fn koerper() {
        assert_eq!(parse_koerper("400 750".split_whitespace()), Ok((400, 750)));
        assert_eq!(parse_koerper("0 1000".split_whitespace()), Ok((0, 1000)));
        assert!(parse_koerper("1001 0".split_whitespace()).is_err());
        assert!(parse_koerper("-1 0".split_whitespace()).is_err());
        assert!(parse_koerper("400".split_whitespace()).is_err());
        assert!(parse_koerper("400 500 600".split_whitespace()).is_err());
        assert!(parse_koerper("viel 500".split_whitespace()).is_err());
    }

    #[test]
    fn ungueltige_namen() {
        for falsch in ["vest/../x", "/vest", "vest/", "Wes te"] {
            let mut parts = vec!["-"; 18];
            parts[0] = falsch;
            assert!(parse_teile(parts.into_iter()).is_err(), "{falsch}");
        }
    }
}
