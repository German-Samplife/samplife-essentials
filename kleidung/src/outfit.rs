//! Test outfits. Texture/model names are the pairs MTA:SA ships for CJ
//! (multitheftauto/mtasa-blue, Client/mods/deathmatch/logic/CClientPlayerClothes.cpp,
//! m_DefaultClothes: index = eClothesTexturePart), all from models/player.img.

use crate::game::{
    self, ClothesDesc, CLOTHES_TEXTURE_TOTAL, TEX_BRACELET, TEX_GLASSES, TEX_HATS, TEX_HEAD,
    TEX_LEGS, TEX_NECKLACE, TEX_SHOES, TEX_TORSO,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum OutfitId {
    Eins,
    Zwei,
    Standard,
}

pub struct Part {
    pub texture_part: usize,
    pub texture: &'static str,
    pub model: &'static str,
}

const fn part(texture_part: usize, texture: &'static str, model: &'static str) -> Part {
    Part { texture_part, texture, model }
}

/// Outfit 1: black vest, afro, green camo work trousers, cowboy boots, gold chain, dark glasses
const EINS: &[Part] = &[
    part(TEX_TORSO, "vestblack", "vest"),
    part(TEX_HEAD, "afro", "afro"),
    part(TEX_LEGS, "worktrcamogrn", "worktr"),
    part(TEX_SHOES, "cowboyboot", "biker"),
    part(TEX_NECKLACE, "neckgold", "neck2"),
    part(TEX_GLASSES, "glasses04dark", "glasses04"),
];

/// Outfit 2: red suit, bald, red suit trousers, black dress shoes, watch, cowboy hat
const ZWEI: &[Part] = &[
    part(TEX_TORSO, "suit1red", "suit1"),
    part(TEX_HEAD, "bald", "head"),
    part(TEX_LEGS, "suit1trred", "suit1tr"),
    part(TEX_SHOES, "shoedressblk", "shoe"),
    part(TEX_BRACELET, "watchgno", "watch"),
    part(TEX_HATS, "cowboy", "cowboy"),
];

/// CJ at the start of the single player game: white vest, jeans, sneakers, short hair
/// (the clothes main.scm gives the player before the first rebuild).
const STANDARD: &[Part] = &[
    part(TEX_TORSO, "vest", "vest"),
    part(TEX_HEAD, "player_face", "head"),
    part(TEX_LEGS, "jeansdenim", "jeans"),
    part(TEX_SHOES, "sneakerbincblk", "sneaker"),
];

impl OutfitId {
    pub fn parts(self) -> &'static [Part] {
        match self {
            OutfitId::Eins => EINS,
            OutfitId::Zwei => ZWEI,
            OutfitId::Standard => STANDARD,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            OutfitId::Eins => "Outfit 1",
            OutfitId::Zwei => "Outfit 2",
            OutfitId::Standard => "Standard-CJ",
        }
    }

    pub fn beschreibung(self) -> &'static str {
        match self {
            OutfitId::Eins => "schwarzes Unterhemd, Afro, Tarnhose, Cowboystiefel, Goldkette, Sonnenbrille",
            OutfitId::Zwei => "roter Anzug, Glatze, schwarze Schuhe, Uhr, Cowboyhut",
            OutfitId::Standard => "weißes Unterhemd, Jeans, Turnschuhe",
        }
    }
}

/// Keys of one outfit, computed with the game's own CKeyGen::GetUppercaseKey.
#[derive(Clone, Copy)]
pub struct OutfitKeys {
    /// (texture key, model key) per texture part, 0/0 = nothing
    pub parts: [(u32, u32); CLOTHES_TEXTURE_TOTAL],
}

impl OutfitKeys {
    pub fn compute(id: OutfitId) -> OutfitKeys {
        let mut parts = [(0u32, 0u32); CLOTHES_TEXTURE_TOTAL];

        for p in id.parts() {
            parts[p.texture_part] = (game::uppercase_key(p.texture), game::uppercase_key(p.model));
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
}
