//! Outfits: the clothes the server sends per player (`kleidung:spieler`) and the
//! standard CJ for players the server hasn't sent anything for. Texture/model names
//! are the pairs from models/player.img (MTA:SA, CClientPlayerClothes.cpp,
//! m_DefaultClothes: index = eClothesTexturePart).

use crate::game::{
    self, ClothesDesc, CLOTHES_TEXTURE_TOTAL, TEX_HEAD, TEX_LEGS, TEX_SHOES, TEX_TORSO,
};

/// Longest texture or model name in player.img is well below this; the game keys
/// names with CKeyGen, so length only guards against garbage.
const NAME_MAX: usize = 24;

/// One outfit by name: (texture, model) per texture part, None = nothing worn there.
/// Tattoos (parts 4 to 12) have no model; their model name is empty.
pub type Teile = [Option<(String, String)>; CLOTHES_TEXTURE_TOTAL];

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
pub fn parse_teile<'a>(tokens: impl Iterator<Item = &'a str>) -> Result<Teile, String> {
    let mut teile: Teile = Default::default();
    let mut count = 0;

    for (part, token) in tokens.enumerate() {
        if part >= CLOTHES_TEXTURE_TOTAL {
            return Err(format!("mehr als {CLOTHES_TEXTURE_TOTAL} Teile"));
        }

        count += 1;

        if token == "-" {
            continue;
        }

        let (texture, model) = token.split_once('/').unwrap_or((token, ""));

        if !name_ok(texture) || (token.contains('/') && !name_ok(model)) {
            return Err(format!("Teil {part} ungültig: {token:?}"));
        }

        teile[part] = Some((texture.to_ascii_lowercase(), model.to_ascii_lowercase()));
    }

    if count != CLOTHES_TEXTURE_TOTAL {
        return Err(format!("{count} statt {CLOTHES_TEXTURE_TOTAL} Teile"));
    }

    Ok(teile)
}

/// Keys of one outfit, computed with the game's own CKeyGen::GetUppercaseKey.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct OutfitKeys {
    /// (texture key, model key) per texture part, 0/0 = nothing
    pub parts: [(u32, u32); CLOTHES_TEXTURE_TOTAL],
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

        OutfitKeys { parts }
    }

    /// Game thread only (calls into gta_sa.exe).
    pub fn from_teile(teile: &Teile) -> OutfitKeys {
        let mut parts = [(0u32, 0u32); CLOTHES_TEXTURE_TOTAL];

        for (part, teil) in teile.iter().enumerate() {
            if let Some((texture, model)) = teil {
                parts[part] = (key(texture), key(model));
            }
        }

        OutfitKeys { parts }
    }

    /// Writes the outfit into a desc: every texture part (also tattoos) and every
    /// model part that depends on a texture is set, everything else (hands, fat,
    /// muscle) stays as it is.
    pub fn apply(&self, desc: &mut ClothesDesc) {
        for (texture_part, &(texture, model)) in self.parts.iter().enumerate() {
            desc.set_texture_and_model(texture, model, texture_part);
        }
    }

    pub fn matches(&self, desc: &ClothesDesc) -> bool {
        self.parts.iter().enumerate().all(|(texture_part, &(texture, model))| {
            desc.texture_keys[texture_part] == texture
                && game::texture_dependency(texture_part)
                    .map(|model_part| desc.model_keys[model_part] == model)
                    .unwrap_or(true)
        })
    }

    /// Short form for the log: torso texture key and how many parts are worn.
    pub fn kurz(&self) -> String {
        let worn = self.parts.iter().filter(|&&(texture, _)| texture != 0).count();
        format!("Oberteil {:08x}, {worn} Teile", self.parts[TEX_TORSO].0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn achtzehn_teile() {
        let text = "suit1red/suit1 bald/head suit1trred/suit1tr shoedressblk/shoe - - - - - - - - - - watchgno/watch - cowboy/cowboy -";
        let teile = parse_teile(text.split_whitespace()).unwrap();
        assert_eq!(teile[0], Some(("suit1red".into(), "suit1".into())));
        assert_eq!(teile[4], None);
        assert_eq!(teile[16], Some(("cowboy".into(), "cowboy".into())));
    }

    #[test]
    fn tattoo_ohne_modell() {
        let mut parts = vec!["-"; 18];
        parts[4] = "4weed";
        let teile = parse_teile(parts.into_iter()).unwrap();
        assert_eq!(teile[4], Some(("4weed".into(), String::new())));
    }

    #[test]
    fn falsche_anzahl() {
        assert!(parse_teile(vec!["-"; 17].into_iter()).is_err());
        assert!(parse_teile(vec!["-"; 19].into_iter()).is_err());
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
