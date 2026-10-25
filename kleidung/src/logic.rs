//! Clothes logic, runs once per frame on the game thread (after CGame::Process).
//!
//! The server sends the outfit of every player (netz.rs). Until the first outfit
//! arrives in a connection, nothing is touched: on servers without the clothes system
//! CJ stays as SA-MP shows him. After that every ped with model 0 wears the outfit of
//! its player, or the standard CJ if the server hasn't sent one.

use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::{c_char, CStr};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::anim::Anims;
use crate::game::{self, ClothesDesc, Ped, Skip, MODEL_PLAYER};
use crate::hooks;
use crate::netz::{self, Nachricht};
use crate::outfit::OutfitKeys;
use crate::rebuild;
use crate::samp::{self, SampVersion};

const PREFIX: &str = "{007DFF}[Kleidung]{FFFFFF} ";
const WHITE: u32 = 0xFFFF_FFFF;
const CHECK_INTERVAL: Duration = Duration::from_millis(1000);
const INIT_INTERVAL: Duration = Duration::from_millis(500);
const COMMAND: &CStr = c"kleidunginfo";
const REGISTER_TRIES: u32 = 10;

static STATE: Mutex<Option<State>> = Mutex::new(None);
static INFO_PENDING: AtomicU32 = AtomicU32::new(0);

// read by the watchdog thread
static SAMP_DETECTED: AtomicU32 = AtomicU32::new(0);
static COMMAND_STATE: AtomicU32 = AtomicU32::new(0);
static COMMAND_INPUT: AtomicUsize = AtomicUsize::new(0);
static OUTFITS: AtomicU32 = AtomicU32::new(0);
const CMD_WAITING: u32 = 1;
const CMD_OK: u32 = 2;
const CMD_FAILED: u32 = 3;

pub fn samp_text() -> &'static str {
    match SAMP_DETECTED.load(Ordering::SeqCst) {
        0 => "noch nicht",
        1 => "0.3.7 R1",
        2 => "0.3.7 R3",
        3 => "0.3.DL (nicht unterstützt)",
        _ => "unbekannt (nicht unterstützt)",
    }
}

pub fn command_text() -> String {
    let input = COMMAND_INPUT.load(Ordering::SeqCst);

    match COMMAND_STATE.load(Ordering::SeqCst) {
        0 => "noch nicht versucht".to_owned(),
        CMD_WAITING => "warte auf CChat/CInput/CNetGame".to_owned(),
        CMD_OK => format!("registriert (CInput {input:#x})"),
        _ => format!("fehlgeschlagen (CInput {input:#x})"),
    }
}

pub fn outfit_text() -> String {
    format!(
        "{} Outfits, Ereignisse {} (verworfen {}), hallo {}x",
        OUTFITS.load(Ordering::Relaxed),
        netz::EMPFANGEN.load(Ordering::Relaxed),
        netz::VERWORFEN.load(Ordering::Relaxed),
        netz::HALLO_GESENDET.load(Ordering::Relaxed)
    )
}

/// CMDPROC for /kleidunginfo, called by SA-MP on the game thread. Only queues; the
/// work happens in `frame`.
unsafe extern "C" fn cmd_kleidunginfo(_params: *const c_char) {
    INFO_PENDING.fetch_add(1, Ordering::SeqCst);
}

pub fn frame() {
    let mut guard = match STATE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    guard.get_or_insert_with(State::new).frame();
}

enum Phase {
    Warten,
    Bereit(SampVersion),
    NichtUnterstuetzt,
}

#[derive(Clone, Copy)]
struct Entry {
    /// RwObject (clump) we last confirmed for this ped
    rw: usize,
    /// geometry of that clump, the clothes this figure really wears
    geo: usize,
    applied: Option<OutfitKeys>,
}

#[derive(Clone, Copy)]
struct CjPed {
    ped: usize,
    local: bool,
    /// SA-MP player id, None if unknown (local player before the pool is ready)
    id: Option<u16>,
    rw: usize,
    desc: Option<usize>,
}

#[derive(Default, Clone, PartialEq)]
struct ScanResult {
    local: u32,
    remote: u32,
    remote_ids: Vec<u16>,
    with_desc: u32,
    distinct_desc: u32,
    ok: u32,
    queued: u32,
    wait_vehicle: u32,
    wait_dead: u32,
    no_model: u32,
    no_data: u32,
}

#[derive(Default, Clone, Copy)]
struct Stats {
    rebuilds: u32,
    adopted: u32,
    last_ms: f32,
    max_ms: f32,
}

struct State {
    phase: Phase,
    frames: u64,
    last_init_try: Option<Instant>,
    init_tries: u32,
    register_tries: u32,
    last_wait_text: String,
    started: Instant,
    /// the server has sent at least one outfit in this connection
    aktiv: bool,
    outfits: HashMap<u16, OutfitKeys>,
    standard: Option<OutfitKeys>,
    peds: HashMap<usize, Entry>,
    queue: VecDeque<usize>,
    last_scan: Option<Instant>,
    force_scan: bool,
    /// what the shared clump of model 0 (CPedModelInfo) shows since our last rebuild
    model_shows: Option<OutfitKeys>,
    /// clumps of CJ peds that existed at our last rebuild (cloned from an older model clump)
    stale_rws: HashSet<usize>,
    /// peds whose rebuild in a vehicle didn't work: rebuilt again once they're out
    nach_aussteigen: HashSet<usize>,
    stats: Stats,
    last_result: ScanResult,
    diagnosed: bool,
    aufgabe_gemeldet: bool,
    /// poses (partial animations) per player, independent of the clothes
    anims: Anims,
}

impl State {
    fn new() -> State {
        State {
            phase: Phase::Warten,
            frames: 0,
            last_init_try: None,
            init_tries: 0,
            register_tries: 0,
            last_wait_text: String::new(),
            started: Instant::now(),
            aktiv: false,
            outfits: HashMap::new(),
            standard: None,
            peds: HashMap::new(),
            queue: VecDeque::new(),
            last_scan: None,
            force_scan: false,
            model_shows: None,
            stale_rws: HashSet::new(),
            nach_aussteigen: HashSet::new(),
            stats: Stats::default(),
            last_result: ScanResult::default(),
            diagnosed: false,
            aufgabe_gemeldet: false,
            anims: Anims::new(),
        }
    }

    fn frame(&mut self) {
        self.frames += 1;

        if self.frames == 1 {
            crate::log!(
                "Spiel-Thread erreicht (erster Aufruf über Einstieg {})",
                if hooks::LOGIC_FROM_PROCESS.load(Ordering::Relaxed) > 0 { "A, CGame::Process" } else { "B, Present" }
            );
        }

        match self.phase {
            Phase::Warten => self.try_init(),
            Phase::Bereit(ver) => self.run(ver),
            Phase::NichtUnterstuetzt => {
                netz::take();
                INFO_PENDING.store(0, Ordering::SeqCst);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Start

    fn try_init(&mut self) {
        let now = Instant::now();

        if self
            .last_init_try
            .is_some_and(|last| now.duration_since(last) < INIT_INTERVAL)
        {
            return;
        }

        self.last_init_try = Some(now);

        if !samp::loaded() {
            return;
        }

        self.init_tries += 1;

        let detected = samp::detect();
        SAMP_DETECTED.store(
            match detected {
                Some(SampVersion::R1) => 1,
                Some(SampVersion::R3) => 2,
                Some(SampVersion::Dl) => 3,
                None => 4,
            },
            Ordering::SeqCst,
        );

        match detected {
            None => {
                // version() re-reads the file version each time while it's unknown
                if self.init_tries >= 20 {
                    crate::log!("Unbekannte SA-MP-Version (weder 0.3.7 R1 noch R3). Die Kleidung bleibt aus.");
                    COMMAND_STATE.store(CMD_FAILED, Ordering::SeqCst);
                    self.phase = Phase::NichtUnterstuetzt;
                }
            }

            Some(SampVersion::Dl) => {
                if samp::chat(SampVersion::Dl).is_some() && self.started.elapsed() > Duration::from_secs(5) {
                    samp::chat_line(
                        SampVersion::Dl,
                        WHITE,
                        &format!("{PREFIX}SA-MP 0.3.DL wird nicht unterstützt, nur 0.3.7 R1 und R3. Die Kleidung bleibt aus."),
                    );
                    crate::log!("SA-MP 0.3.DL erkannt, nicht unterstützt");
                    COMMAND_STATE.store(CMD_FAILED, Ordering::SeqCst);
                    self.phase = Phase::NichtUnterstuetzt;
                } else if self.started.elapsed() > Duration::from_secs(120) {
                    crate::log!("SA-MP 0.3.DL erkannt, nicht unterstützt (Chat nicht gefunden)");
                    COMMAND_STATE.store(CMD_FAILED, Ordering::SeqCst);
                    self.phase = Phase::NichtUnterstuetzt;
                }
            }

            Some(ver) => {
                let chat = samp::chat(ver);
                let input = samp::input(ver);
                let netgame = samp::netgame_ready(ver);
                let waiting = format!(
                    "SA-MP {}: CChat {:#x}, CInput {:#x}, CNetGame {}",
                    ver.name(),
                    chat.unwrap_or(0),
                    input.unwrap_or(0),
                    if netgame { "da" } else { "fehlt" }
                );

                if waiting != self.last_wait_text {
                    crate::log!("{waiting}");
                    self.last_wait_text = waiting;
                }

                if chat.is_none() || input.is_none() || !netgame {
                    COMMAND_STATE.store(CMD_WAITING, Ordering::SeqCst);
                    return;
                }

                COMMAND_INPUT.store(input.unwrap_or(0), Ordering::SeqCst);
                self.register_tries += 1;

                // /kleidunginfo is only for support; the clothes work without it
                if samp::register_command(ver, COMMAND, cmd_kleidunginfo) {
                    crate::log!(
                        "Befehl /kleidunginfo registriert (CInput {:#x}, Versuch {})",
                        input.unwrap_or(0),
                        self.register_tries
                    );
                    COMMAND_STATE.store(CMD_OK, Ordering::SeqCst);
                    self.phase = Phase::Bereit(ver);
                } else if self.register_tries >= REGISTER_TRIES {
                    crate::log!("CInput::AddCommand hat /kleidunginfo nicht angelegt, weiter ohne den Befehl");
                    COMMAND_STATE.store(CMD_FAILED, Ordering::SeqCst);
                    self.phase = Phase::Bereit(ver);
                }

                if !netz::plugin_aktiv() && matches!(self.phase, Phase::Bereit(_)) {
                    crate::log!(
                        "Nicht als samp-cef-Plugin geladen (oder samp-cef ohne cef/plugin-ereignisse.patch): \
                         ohne Daten vom Server bleibt die Kleidung aus. Die Datei gehört nach GTA/cef/plugins/."
                    );
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Main loop

    fn run(&mut self, ver: SampVersion) {
        for nachricht in netz::take() {
            self.handle(ver, nachricht);
        }

        if INFO_PENDING.swap(0, Ordering::SeqCst) > 0 {
            self.info(ver);
        }

        // poses work without clothes from the server
        self.anims.frame(ver);

        if !self.aktiv {
            return;
        }

        let now = Instant::now();

        if self.force_scan
            || self
                .last_scan
                .is_none_or(|last| now.duration_since(last) >= CHECK_INTERVAL)
        {
            self.scan(ver, now);
        }

        // at most one rebuild per frame
        self.rebuild_next(ver);
    }

    fn handle(&mut self, ver: SampVersion, nachricht: Nachricht) {
        match nachricht {
            Nachricht::Spieler(id, teile) => {
                let keys = OutfitKeys::from_teile(&teile);

                if !self.aktiv {
                    self.aktiv = true;
                    self.aufgabe_gemeldet = false;
                    rebuild::zuruecksetzen();
                    crate::log!("Erstes Outfit vom Server: Kleidung aktiv");
                }

                if self.outfits.insert(id, keys) != Some(keys) {
                    crate::log!("Outfit Spieler-ID {id}: {}", keys.kurz());
                    self.force_scan = true;
                }
            }

            Nachricht::Weg(id) => {
                if self.outfits.remove(&id).is_some() {
                    crate::log!("Outfit Spieler-ID {id} vergessen");
                    self.force_scan = true;
                }
            }

            Nachricht::AnimStart(id, name) => self.anims.start(ver, id, name),

            Nachricht::AnimStopp(id) => self.anims.stopp(ver, id),

            Nachricht::Getrennt => {
                self.anims.getrennt(ver);
                self.aktiv = false;
                self.outfits.clear();
                self.peds.clear();
                self.queue.clear();
                self.model_shows = None;
                self.stale_rws.clear();
                self.nach_aussteigen.clear();
            }
        }

        OUTFITS.store(self.outfits.len() as u32, Ordering::Relaxed);
    }

    fn standard(&mut self) -> OutfitKeys {
        *self.standard.get_or_insert_with(|| {
            let keys = OutfitKeys::standard();
            crate::log!("Schlüssel Standard-CJ: {:08x?}", keys.parts);
            keys
        })
    }

    fn target_for(&mut self, cj: &CjPed) -> Option<OutfitKeys> {
        if !self.aktiv {
            return None;
        }

        match cj.id.and_then(|id| self.outfits.get(&id).copied()) {
            Some(keys) => Some(keys),
            None => Some(self.standard()),
        }
    }

    fn write_target(&self, target: OutfitKeys, desc_ptr: usize) {
        let mut desc = unsafe { (desc_ptr as *const ClothesDesc).read_unaligned() };
        target.apply(&mut desc);
        unsafe { (desc_ptr as *mut ClothesDesc).write_unaligned(desc) };
    }

    // -----------------------------------------------------------------------
    // Finding CJ peds

    fn survey(&self, ver: SampVersion) -> Vec<CjPed> {
        let mut out: Vec<CjPed> = Vec::new();
        let local = samp::local_ped(ver);

        let mut add = |ped: usize, is_local: bool, id: Option<u16>| {
            let view = Ped(ped);

            if out.iter().any(|cj| cj.ped == ped) || !view.valid() || view.model() != MODEL_PLAYER {
                return;
            }

            out.push(CjPed {
                ped,
                local: is_local,
                id,
                rw: view.rw_object(),
                desc: view.clothes_desc_ptr(),
            });
        };

        if let Some(ped) = local {
            add(ped, true, samp::local_id());
        }

        for (id, ped) in samp::remote_peds() {
            if Some(ped) != local {
                add(ped, false, (id != u16::MAX).then_some(id));
            }
        }

        out
    }

    fn scan(&mut self, ver: SampVersion, now: Instant) {
        self.last_scan = Some(now);
        self.force_scan = false;

        let cjs = self.survey(ver);
        let mut result = count(&cjs);
        let seen: HashSet<usize> = cjs.iter().map(|cj| cj.ped).collect();

        // (ped, geometry, target) of every CJ, to find figures that share clothes
        let targets: Vec<(usize, usize, Option<OutfitKeys>)> = cjs
            .iter()
            .map(|cj| (cj.ped, game::clump_parts(cj.rw).0, self.target_for(cj)))
            .collect();

        for cj in &cjs {
            let entry = *self.peds.entry(cj.ped).or_insert(Entry {
                rw: 0,
                geo: 0,
                applied: None,
            });

            let Some(target) = self.target_for(cj) else {
                continue;
            };

            if cj.rw == 0 {
                result.no_model += 1;
                if let Some(entry) = self.peds.get_mut(&cj.ped) {
                    entry.rw = 0;
                    entry.applied = None;
                }
                continue;
            }

            let Some(desc_ptr) = cj.desc else {
                result.no_data += 1;
                continue;
            };

            let desc = unsafe { (desc_ptr as *const ClothesDesc).read_unaligned() };
            let desc_ok = target.matches(&desc);

            // the clump must still carry the geometry we built for this figure, and no
            // figure with a different outfit may share it
            let (geo_now, _) = game::clump_parts(cj.rw);
            let conflict = geo_now != 0
                && targets
                    .iter()
                    .any(|&(other, other_geo, other_target)| other != cj.ped && other_geo == geo_now && other_target != Some(target));

            if conflict {
                crate::log!(
                    "{} {:#x}: Geometrie {geo_now:#x} geteilt mit einer Figur mit anderem Outfit, wird neu gebaut",
                    who(cj),
                    cj.ped
                );
            }

            if entry.rw == cj.rw
                && entry.applied == Some(target)
                && desc_ok
                && !conflict
                && (entry.geo == 0 || entry.geo == geo_now)
            {
                result.ok += 1;
                continue;
            }

            // A clump we haven't confirmed yet that was created after our last rebuild
            // is a clone of the model-0 clump, which already shows the target: no rebuild.
            if entry.rw != cj.rw
                && self.model_shows == Some(target)
                && !self.stale_rws.contains(&cj.rw)
                && !conflict
            {
                if !desc_ok {
                    self.write_target(target, desc_ptr);
                }

                if let Some(entry) = self.peds.get_mut(&cj.ped) {
                    entry.rw = cj.rw;
                    entry.geo = geo_now;
                    entry.applied = Some(target);
                }

                self.stats.adopted += 1;
                result.ok += 1;
                crate::log!(
                    "Übernommen ohne Umbau: {} {:#x} (Clump {:#x}) zeigt schon {}",
                    who(cj),
                    cj.ped,
                    cj.rw,
                    target.kurz()
                );
                continue;
            }

            if self.nach_aussteigen.contains(&cj.ped) {
                if Ped(cj.ped).in_vehicle() {
                    result.wait_vehicle += 1;
                    continue;
                }
                self.nach_aussteigen.remove(&cj.ped);
            }

            match Ped(cj.ped).rebuild_blocker() {
                Some(Skip::InVehicle) => result.wait_vehicle += 1,
                Some(Skip::Dead) => result.wait_dead += 1,
                Some(Skip::NoModel) => result.no_model += 1,
                Some(Skip::NoClothesData) => result.no_data += 1,
                None => {
                    if !self.queue.contains(&cj.ped) {
                        self.queue.push_back(cj.ped);
                    }
                    result.queued += 1;
                }
            }
        }

        self.peds.retain(|ped, _| seen.contains(ped));
        self.queue.retain(|ped| seen.contains(ped));
        self.nach_aussteigen.retain(|ped| seen.contains(ped));

        if result != self.last_result {
            crate::log!(
                "Prüfung: eigene {}, fremde {} {:?}, mit Kleidungsdaten {} ({} verschiedene), passt {}, umbauen {}, warten Fahrzeug {} tot {} ohne Daten {}, ohne Modell {}",
                result.local,
                result.remote,
                result.remote_ids,
                result.with_desc,
                result.distinct_desc,
                result.ok,
                result.queued,
                result.wait_vehicle,
                result.wait_dead,
                result.no_data,
                result.no_model
            );
        }

        self.last_result = result;
    }

    // -----------------------------------------------------------------------
    // Rebuild

    fn rebuild_next(&mut self, ver: SampVersion) {
        if rebuild::aufgegeben() {
            self.queue.clear();
            return;
        }

        while let Some(ped_addr) = self.queue.pop_front() {
            let ped = Ped(ped_addr);

            if !ped.valid() || ped.model() != MODEL_PLAYER || ped.rebuild_blocker().is_some() {
                continue;
            }

            let Some(cj) = self.survey(ver).into_iter().find(|cj| cj.ped == ped_addr) else {
                continue;
            };
            let Some(target) = self.target_for(&cj) else {
                continue;
            };
            let Some(desc_ptr) = ped.clothes_desc_ptr() else {
                continue;
            };

            if !self.diagnosed {
                self.diagnosed = true;
                crate::diag::scan("vor dem ersten Umbau");
            }

            self.write_target(target, desc_ptr);

            let wer = who(&cj);
            crate::log!(
                "Umbau {wer} {ped_addr:#x}{} -> {}",
                if ped.seated_in_vehicle() { " (im Fahrzeug)" } else { "" },
                target.kurz()
            );
            let ergebnis = rebuild::umbauen(ped_addr, desc_ptr, &wer);
            let ms = ergebnis.ms;

            self.stats.rebuilds += 1;
            self.stats.last_ms = ms;
            self.stats.max_ms = self.stats.max_ms.max(ms);

            let (geo, _) = game::clump_parts(ergebnis.rw_nachher);
            let im_fahrzeug_fehlgeschlagen = ergebnis.im_fahrzeug && ergebnis.wirksam.is_none();

            if let Some(entry) = self.peds.get_mut(&ped_addr) {
                entry.rw = ergebnis.rw_nachher;
                entry.geo = geo;
                entry.applied = if im_fahrzeug_fehlgeschlagen { None } else { Some(target) };
            }

            if im_fahrzeug_fehlgeschlagen {
                self.nach_aussteigen.insert(ped_addr);
            }

            if ergebnis.wirksam.is_some() {
                // the model-0 clump now shows `target`; every other existing CJ clump is older
                self.model_shows = Some(target);
                self.stale_rws = self
                    .survey(ver)
                    .iter()
                    .filter(|cj| cj.ped != ped_addr && cj.rw != 0)
                    .map(|cj| cj.rw)
                    .collect();
            } else {
                self.model_shows = None;
            }

            if rebuild::aufgegeben() {
                self.queue.clear();

                if !self.aufgabe_gemeldet {
                    self.aufgabe_gemeldet = true;
                    samp::chat_line(
                        ver,
                        WHITE,
                        &format!("{PREFIX}Die Kleidung lässt sich nicht anzeigen. Details: /kleidunginfo und samplife-kleidung.log"),
                    );
                }
            }

            break;
        }
    }

    // -----------------------------------------------------------------------
    // /kleidunginfo

    fn info(&mut self, ver: SampVersion) {
        let cjs = self.survey(ver);
        let result = count(&cjs);

        let zustand = if !netz::plugin_aktiv() {
            "nicht als samp-cef-Plugin geladen"
        } else if !netz::verbunden() {
            "nicht mit dem CEF-Server verbunden"
        } else if !self.aktiv {
            "verbunden, der Server hat keine Kleidung gesendet"
        } else {
            "aktiv"
        };

        samp::chat_line(
            ver,
            WHITE,
            &format!("{PREFIX}Version {}, SA-MP {}, {zustand}", crate::ASI_VERSION, ver.name()),
        );

        let ids: Vec<String> = result.remote_ids.iter().take(8).map(|id| id.to_string()).collect();
        let ids = if ids.is_empty() {
            String::new()
        } else {
            format!(", IDs {}", ids.join(" "))
        };

        samp::chat_line(
            ver,
            WHITE,
            &format!(
                "CJ-Figuren: {} (eigene {}, fremde {}{ids}), Outfits vom Server: {}, eigene ID {}",
                result.local + result.remote,
                result.local,
                result.remote,
                self.outfits.len(),
                samp::local_id().map(|id| id.to_string()).unwrap_or_else(|| "?".to_owned())
            ),
        );

        let last = &self.last_result;

        samp::chat_line(
            ver,
            WHITE,
            &format!(
                "Umgebaut: {} (letzter {:.0} ms, längster {:.0} ms), ohne Umbau: {}, warten: Fahrzeug {}, tot {}, ohne Daten {}",
                self.stats.rebuilds,
                self.stats.last_ms,
                self.stats.max_ms,
                self.stats.adopted,
                last.wait_vehicle,
                last.wait_dead,
                last.no_data
            ),
        );

        samp::chat_line(
            ver,
            WHITE,
            &format!(
                "Posen: {}, aktiv bei {} Spielern, gestartet {}x",
                self.anims.zustand(),
                self.anims.aktive_posen(),
                self.anims.gestartet
            ),
        );

        let patches = crate::diag::scan("/kleidunginfo");
        let variante = match (rebuild::wirksame_variante(), rebuild::aufgegeben(), self.stats.rebuilds) {
            (Some(v), _, _) => format!("wirkt: {}", v.name()),
            (None, true, _) => "keine wirkt".to_owned(),
            (None, false, 0) => "noch kein Umbau".to_owned(),
            (None, false, _) => "bisher keine wirksam".to_owned(),
        };

        samp::chat_line(
            ver,
            WHITE,
            &format!(
                "Kleidungs-Code gepatcht: {}, Umbau {variante}, Ereignisse {} (verworfen {})",
                if patches.is_empty() { "nein".to_owned() } else { format!("ja, {} Stellen", patches.len()) },
                netz::EMPFANGEN.load(Ordering::Relaxed),
                netz::VERWORFEN.load(Ordering::Relaxed)
            ),
        );

        let mut parts: Vec<(Option<OutfitKeys>, usize, usize)> = Vec::new();

        for cj in cjs.iter() {
            let desc = Ped(cj.ped).clothes_desc();
            let (geo, tex) = game::clump_parts(cj.rw);
            let soll = self.peds.get(&cj.ped).and_then(|e| e.applied);
            crate::log!(
                "Info: {} Ped {:#x} Clump {:#x} Geometrie {geo:#x} Textur {tex:#x} Kleidungsdaten {:#x} Oberteil-Textur {:08x} Outfit {}",
                who(cj),
                cj.ped,
                cj.rw,
                cj.desc.unwrap_or(0),
                desc.map(|d| d.texture_keys[0]).unwrap_or(0),
                soll.map(|k| k.kurz()).unwrap_or_else(|| "keins".to_owned())
            );
            parts.push((soll, geo, tex));
        }

        let shared = |pick: fn(&(Option<OutfitKeys>, usize, usize)) -> usize| {
            parts
                .iter()
                .enumerate()
                .any(|(i, a)| pick(a) != 0 && parts.iter().skip(i + 1).any(|b| pick(b) == pick(a) && b.0 != a.0))
        };

        if shared(|p| p.1) || shared(|p| p.2) {
            samp::chat_line(
                ver,
                WHITE,
                "Figuren mit verschiedenem Outfit teilen Geometrie oder Texturen (Fehler, Details im Log)",
            );
        }
    }
}

fn who(cj: &CjPed) -> String {
    match (cj.local, cj.id) {
        (true, _) => "eigene Figur".to_owned(),
        (false, Some(id)) => format!("Spieler-ID {id}"),
        (false, None) => "fremde Figur".to_owned(),
    }
}

fn count(cjs: &[CjPed]) -> ScanResult {
    let mut result = ScanResult::default();
    let mut descs: HashSet<usize> = HashSet::new();

    for cj in cjs {
        if cj.local {
            result.local += 1;
        } else {
            result.remote += 1;
            result.remote_ids.extend(cj.id);
        }

        if let Some(desc) = cj.desc {
            result.with_desc += 1;
            descs.insert(desc);
        }
    }

    result.distinct_desc = descs.len() as u32;
    result
}
