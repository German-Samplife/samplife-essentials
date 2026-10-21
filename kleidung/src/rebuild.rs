//! Rebuilding a ped with its clothes. Three variants, tried in this order until one
//! works; the working one is used first from then on:
//! - Nachgebaut: the steps of RebuildPlayer with the game's inner functions
//!   (game::rebuild_player_manual), always a full build (no "old clothes"). This is
//!   what worked in the live test with 0.3.0 (SA-MP puts a `ret` at 0x5A82C0).
//! - Normal: CClothes::RebuildPlayer (0x5A82C0)
//! - Originalbytes: RebuildPlayer while all differences found by diag::scan are
//!   reverted to the bytes of gta_sa.exe, restored right after the call
//!
//! "Works" means: ConstructPedModel gave model 0 a new clump (CPedModelInfo::SetClump),
//! i.e. the clothes builder really built something, and the figure now holds a clone
//! of exactly that clump (same geometry).
//!
//! Every build makes new geometry, materials and textures (gta-reversed
//! ClothesBuilder.cpp: ConstructGeometryArray, CopyTexture -> RwRasterCreate/
//! RwTextureCreate, RpMaterialCreate). Dress clones the model clump; the clone shares
//! that geometry (refcounted), and when the next build replaces the model clump the
//! figure keeps its own. So building and dressing one figure at a time gives every
//! figure its own clothes.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use crate::diag;
use crate::game::{self, Ped};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variante {
    Normal,
    Nachgebaut,
    Originalbytes,
}

impl Variante {
    pub fn name(self) -> &'static str {
        match self {
            Variante::Normal => "RebuildPlayer",
            Variante::Nachgebaut => "nachgebaut",
            Variante::Originalbytes => "Originalbytes",
        }
    }
}

static WIRKT: Mutex<Option<Variante>> = Mutex::new(None);
static FEHLSCHLAEGE: AtomicU32 = AtomicU32::new(0);
const AUFGEBEN_NACH: u32 = 3;

pub fn wirksame_variante() -> Option<Variante> {
    WIRKT.lock().ok().and_then(|v| *v)
}

/// No variant worked several times in a row: stop rebuilding.
pub fn aufgegeben() -> bool {
    FEHLSCHLAEGE.load(Ordering::SeqCst) >= AUFGEBEN_NACH
}

pub fn zuruecksetzen() {
    FEHLSCHLAEGE.store(0, Ordering::SeqCst);
}

pub struct Ergebnis {
    pub wirksam: Option<Variante>,
    pub ms: f32,
    pub rw_nachher: usize,
}

fn ausfuehren(variante: Variante, ped: usize, desc_ptr: usize) -> Option<()> {
    match variante {
        Variante::Normal => {
            game::forget_player_clothes();
            game::rebuild_player(ped);
            Some(())
        }
        Variante::Nachgebaut => {
            game::rebuild_player_manual(ped, desc_ptr, true);
            Some(())
        }
        Variante::Originalbytes => {
            let patches = diag::last().unwrap_or_else(|| diag::scan("vor Originalbytes"));
            game::forget_player_clothes();
            diag::with_original_code(&patches, || game::rebuild_player(ped))
        }
    }
}

/// Game thread. `desc_ptr` already holds the wanted clothes.
pub fn umbauen(ped: usize, desc_ptr: usize, wer: &str) -> Ergebnis {
    let start_all = Instant::now();
    let mut reihenfolge = vec![Variante::Nachgebaut, Variante::Normal, Variante::Originalbytes];

    if let Some(v) = wirksame_variante() {
        reihenfolge.retain(|&x| x != v);
        reihenfolge.insert(0, v);
    }

    let (image, entries) = game::player_img_state();
    let mut wirksam = None;

    for variante in reihenfolge {
        let view = Ped(ped);

        if !view.valid() || view.rebuild_blocker().is_some() {
            crate::log!("  {} nicht mehr umbaubar, Abbruch vor {}", wer, variante.name());
            break;
        }

        let rw_vorher = view.rw_object();
        let (geo_vorher, tex_vorher) = game::clump_parts(rw_vorher);
        let mi_vorher = game::model0_clump();
        let start = Instant::now();
        let lief = ausfuehren(variante, ped, desc_ptr);
        let ms = start.elapsed().as_secs_f32() * 1000.0;
        let rw_nachher = if view.valid() { view.rw_object() } else { 0 };
        let (geo_nachher, tex_nachher) = game::clump_parts(rw_nachher);
        let mi_nachher = game::model0_clump();
        let (geo_modell, _) = game::clump_parts(mi_nachher);
        let ok = lief.is_some()
            && mi_nachher != 0
            && mi_nachher != mi_vorher
            && geo_nachher != 0
            && geo_nachher == geo_modell;

        crate::log!(
            "  {wer}: Variante {} {} in {ms:.1} ms | Kleidungsdaten {desc_ptr:#x} | Figur-Clump {rw_vorher:#x} -> {rw_nachher:#x} | Geometrie {geo_vorher:#x} -> {geo_nachher:#x} | Textur {tex_vorher:#x} -> {tex_nachher:#x} | Modell-0-Clump {mi_vorher:#x} -> {mi_nachher:#x} (Geometrie {geo_modell:#x}) | player.img Bild {image}, {entries} Einträge -> {}",
            variante.name(),
            if lief.is_some() { "ausgeführt" } else { "übersprungen (nichts wiederherzustellen)" },
            if ok { "WIRKT" } else { "wirkt nicht" }
        );

        if ok {
            wirksam = Some(variante);
            break;
        }
    }

    match wirksam {
        Some(v) => {
            FEHLSCHLAEGE.store(0, Ordering::SeqCst);
            if let Ok(mut w) = WIRKT.lock() {
                if *w != Some(v) {
                    crate::log!("Wirksame Variante: {}", v.name());
                }
                *w = Some(v);
            }
        }
        None => {
            let n = FEHLSCHLAEGE.fetch_add(1, Ordering::SeqCst) + 1;
            crate::log!("Keine Variante hat Modell 0 neu gebaut ({n}. Mal in Folge)");
            if let Ok(mut w) = WIRKT.lock() {
                *w = None;
            }
        }
    }

    Ergebnis {
        wirksam,
        ms: start_all.elapsed().as_secs_f32() * 1000.0,
        rw_nachher: if Ped(ped).valid() { Ped(ped).rw_object() } else { 0 },
    }
}
