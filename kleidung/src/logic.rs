//! Test logic, runs once per frame on the game thread (after CGame::Process).

use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::{c_char, CStr};
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::game::{self, ClothesDesc, Ped, Skip, MODEL_PLAYER};
use crate::hooks;
use crate::outfit::{OutfitId, OutfitKeys};
use crate::samp::{self, SampVersion};

const PREFIX: &str = "{007DFF}[Kleidung]{FFFFFF} ";
const WHITE: u32 = 0xFFFF_FFFF;
const CHECK_INTERVAL: Duration = Duration::from_millis(1000);
const INIT_INTERVAL: Duration = Duration::from_millis(500);
const RESTORE_TIMEOUT: Duration = Duration::from_secs(30);
const COMMAND: &CStr = c"kleidungtest";

static STATE: Mutex<Option<State>> = Mutex::new(None);
static PENDING: Mutex<Vec<Command>> = Mutex::new(Vec::new());

// read by the watchdog thread
static SAMP_DETECTED: AtomicU32 = AtomicU32::new(0);
static COMMAND_STATE: AtomicU32 = AtomicU32::new(0);
static COMMAND_INPUT: AtomicUsize = AtomicUsize::new(0);
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

enum Command {
    Outfit(OutfitId),
    Aus,
    Info,
    Hilfe,
}

/// CMDPROC for /kleidungtest, called by SA-MP on the game thread. Only queues the
/// command; the work happens in `frame`.
unsafe extern "C" fn cmd_kleidungtest(params: *const c_char) {
    let _ = std::panic::catch_unwind(|| {
        let text = if params.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(params) }
                .to_string_lossy()
                .trim()
                .to_lowercase()
        };

        let command = match text.as_str() {
            "" | "1" => Command::Outfit(OutfitId::Eins),
            "2" => Command::Outfit(OutfitId::Zwei),
            "aus" | "0" => Command::Aus,
            "info" => Command::Info,
            _ => Command::Hilfe,
        };

        if let Ok(mut pending) = PENDING.lock() {
            pending.push(command);
        }
    });
}

pub fn frame() {
    let mut guard = match STATE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    guard.get_or_insert_with(State::new).frame();
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Target {
    Outfit(OutfitId),
    /// clothes the ped had before the test
    Original,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Aus,
    Aktiv(OutfitId),
    Wiederherstellen,
}

enum Phase {
    Warten,
    Bereit(SampVersion),
    NichtUnterstuetzt,
}

struct Entry {
    /// RwObject (clump) we last confirmed for this ped
    rw: usize,
    applied: Option<Target>,
    original: Option<ClothesDesc>,
}

#[derive(Clone, Copy)]
struct CjPed {
    ped: usize,
    local: bool,
    id: u16,
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

impl ScanResult {
    fn waiting(&self) -> u32 {
        self.wait_vehicle + self.wait_dead + self.no_data
    }
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
    mode: Mode,
    mode_since: Instant,
    keys: HashMap<OutfitId, OutfitKeys>,
    peds: HashMap<usize, Entry>,
    queue: VecDeque<usize>,
    last_scan: Option<Instant>,
    force_scan: bool,
    /// what the shared clump of model 0 (CPedModelInfo) shows since our last rebuild
    model_shows: Option<Target>,
    /// clumps of CJ peds that existed at our last rebuild (cloned from an older model clump)
    stale_rws: HashSet<usize>,
    stats: Stats,
    stats_at_command: Stats,
    last_result: ScanResult,
    report_pending: bool,
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
            mode: Mode::Aus,
            mode_since: Instant::now(),
            keys: HashMap::new(),
            peds: HashMap::new(),
            queue: VecDeque::new(),
            last_scan: None,
            force_scan: false,
            model_shows: None,
            stale_rws: HashSet::new(),
            stats: Stats::default(),
            stats_at_command: Stats::default(),
            last_result: ScanResult::default(),
            report_pending: false,
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
                if let Ok(mut pending) = PENDING.lock() {
                    pending.clear();
                }
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
                    crate::log!("Unbekannte SA-MP-Version (weder 0.3.7 R1 noch R3). Das ASI bleibt inaktiv.");
                    COMMAND_STATE.store(CMD_FAILED, Ordering::SeqCst);
                    self.phase = Phase::NichtUnterstuetzt;
                }
            }

            Some(SampVersion::Dl) => {
                if samp::chat(SampVersion::Dl).is_some() && self.started.elapsed() > Duration::from_secs(5) {
                    samp::chat_line(
                        SampVersion::Dl,
                        WHITE,
                        &format!("{PREFIX}SA-MP 0.3.DL wird nicht unterstützt, nur 0.3.7 R1 und R3. /kleidungtest gibt es hier nicht."),
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

                if samp::register_command(ver, COMMAND, cmd_kleidungtest) {
                    crate::log!(
                        "Befehl /kleidungtest registriert (CInput {:#x}, Versuch {})",
                        input.unwrap_or(0),
                        self.register_tries
                    );
                    COMMAND_STATE.store(CMD_OK, Ordering::SeqCst);
                    self.phase = Phase::Bereit(ver);
                } else {
                    crate::log!(
                        "CInput::AddCommand hat /kleidungtest nicht angelegt (CInput {:#x}, Versuch {})",
                        input.unwrap_or(0),
                        self.register_tries
                    );

                    if self.register_tries >= 10 {
                        samp::chat_line(
                            ver,
                            WHITE,
                            &format!("{PREFIX}Der Befehl /kleidungtest konnte nicht angelegt werden (siehe samplife-kleidung.log)."),
                        );
                        COMMAND_STATE.store(CMD_FAILED, Ordering::SeqCst);
                        self.phase = Phase::NichtUnterstuetzt;
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Main loop

    fn run(&mut self, ver: SampVersion) {
        let commands: Vec<Command> = match PENDING.lock() {
            Ok(mut pending) => pending.drain(..).collect(),
            Err(_) => Vec::new(),
        };

        for command in commands {
            self.handle(ver, command);
        }

        if self.mode == Mode::Aus {
            return;
        }

        let now = Instant::now();

        if self.force_scan
            || self
                .last_scan
                .is_none_or(|last| now.duration_since(last) >= CHECK_INTERVAL)
        {
            self.scan(ver, now);
            self.after_scan(ver);
        }

        // at most one RebuildPlayer per frame
        self.rebuild_next(ver);
    }

    fn handle(&mut self, ver: SampVersion, command: Command) {
        match command {
            Command::Outfit(id) => {
                crate::log!("Befehl: {}", id.name());
                self.set_mode(Mode::Aktiv(id));
                samp::chat_line(ver, WHITE, &format!("{PREFIX}{} aktiv: {}", id.name(), id.beschreibung()));
                samp::chat_line(
                    ver,
                    WHITE,
                    &format!("{PREFIX}Gilt für alle Figuren mit Skin 0 in deiner Nähe. Beenden mit /kleidungtest aus"),
                );
            }

            Command::Aus => {
                if self.mode == Mode::Aus && self.peds.is_empty() {
                    samp::chat_line(ver, WHITE, &format!("{PREFIX}Der Test ist nicht aktiv."));
                    return;
                }

                crate::log!("Befehl: aus");
                self.set_mode(Mode::Wiederherstellen);
                samp::chat_line(ver, WHITE, &format!("{PREFIX}Standard-CJ wird wiederhergestellt ..."));
            }

            Command::Info => self.info(ver),

            Command::Hilfe => {
                samp::chat_line(ver, WHITE, &format!("{PREFIX}Benutze: /kleidungtest [1 | 2 | aus | info]"));
            }
        }
    }

    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.mode_since = Instant::now();
        self.queue.clear();
        self.force_scan = true;
        self.report_pending = true;
        self.stats_at_command = self.stats;
    }

    fn keys(&mut self, id: OutfitId) -> OutfitKeys {
        *self.keys.entry(id).or_insert_with(|| {
            let keys = OutfitKeys::compute(id);
            crate::log!("Schlüssel {}: {:08x?}", id.name(), keys.parts);
            keys
        })
    }

    fn target_for(&self, entry: &Entry) -> Option<Target> {
        match self.mode {
            Mode::Aus => None,
            Mode::Aktiv(id) => Some(Target::Outfit(id)),
            Mode::Wiederherstellen => Some(match entry.original {
                Some(original) if !original.is_empty() => Target::Original,
                _ => Target::Outfit(OutfitId::Standard),
            }),
        }
    }

    fn desc_matches(&mut self, target: Target, desc: &ClothesDesc, entry: &Entry) -> bool {
        match target {
            Target::Outfit(id) => self.keys(id).matches(desc),
            Target::Original => entry.original.is_some_and(|original| same_clothes(&original, desc)),
        }
    }

    fn write_target(&mut self, target: Target, desc_ptr: usize, original: Option<ClothesDesc>) {
        let mut desc = unsafe { (desc_ptr as *const ClothesDesc).read_unaligned() };

        match target {
            Target::Outfit(id) => self.keys(id).apply(&mut desc),
            Target::Original => {
                if let Some(original) = original {
                    desc = original;
                }
            }
        }

        unsafe { (desc_ptr as *mut ClothesDesc).write_unaligned(desc) };
    }

    // -----------------------------------------------------------------------
    // Finding CJ peds

    fn survey(&self, ver: SampVersion) -> Vec<CjPed> {
        let mut out: Vec<CjPed> = Vec::new();
        let local = samp::local_ped(ver);

        let mut add = |ped: usize, is_local: bool, id: u16| {
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
            add(ped, true, u16::MAX);
        }

        for (id, ped) in samp::remote_peds() {
            if Some(ped) != local {
                add(ped, false, id);
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

        for cj in &cjs {
            let entry = self.peds.entry(cj.ped).or_insert_with(|| {
                let original = Ped(cj.ped).clothes_desc();
                Entry {
                    rw: 0,
                    applied: None,
                    original,
                }
            });

            // don't take our own outfits for the original clothes (shared descs)
            if let Some(original) = entry.original {
                let ours = [OutfitId::Eins, OutfitId::Zwei]
                    .iter()
                    .any(|&id| self.keys.get(&id).is_some_and(|k| k.matches(&original)));
                if ours {
                    entry.original = None;
                }
            }

            let entry_rw = entry.rw;
            let entry_applied = entry.applied;
            let entry_snapshot = Entry {
                rw: entry.rw,
                applied: entry.applied,
                original: entry.original,
            };

            let Some(target) = self.target_for(&entry_snapshot) else {
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
            let desc_ok = self.desc_matches(target, &desc, &entry_snapshot);

            if entry_rw == cj.rw && entry_applied == Some(target) && desc_ok {
                result.ok += 1;
                continue;
            }

            // nothing was rebuilt in this game session: every clump is still SA-MP's own
            if self.mode == Mode::Wiederherstellen && entry_applied.is_none() && self.model_shows.is_none() {
                result.ok += 1;
                continue;
            }

            // A clump we haven't confirmed yet that was created after our last rebuild
            // is a clone of the model-0 clump, which already shows the target: no rebuild.
            if entry_rw != cj.rw
                && self.model_shows == Some(target)
                && target != Target::Original
                && !self.stale_rws.contains(&cj.rw)
            {
                if !desc_ok {
                    self.write_target(target, desc_ptr, entry_snapshot.original);
                }

                if let Some(entry) = self.peds.get_mut(&cj.ped) {
                    entry.rw = cj.rw;
                    entry.applied = Some(target);
                }

                self.stats.adopted += 1;
                result.ok += 1;
                crate::log!(
                    "Übernommen ohne Umbau: {} {:#x} (Clump {:#x}) zeigt schon {:?}",
                    who(cj),
                    cj.ped,
                    cj.rw,
                    target
                );
                continue;
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

    fn after_scan(&mut self, ver: SampVersion) {
        let result = self.last_result.clone();

        if !self.queue.is_empty() || result.queued > 0 {
            return;
        }

        if self.report_pending {
            self.report_pending = false;

            let total = result.local + result.remote;
            let rebuilt = self.stats.rebuilds - self.stats_at_command.rebuilds;
            let adopted = self.stats.adopted - self.stats_at_command.adopted;
            let mut text = format!(
                "{PREFIX}{total} CJ-Figuren (eigene {}, fremde {}): {rebuilt} umgebaut, {adopted} ohne Umbau",
                result.local, result.remote
            );

            if result.waiting() > 0 {
                text.push_str(&format!(", {} warten (Fahrzeug/tot)", result.waiting()));
            }

            samp::chat_line(ver, WHITE, &text);
        }

        if self.mode == Mode::Wiederherstellen {
            let timeout = self.mode_since.elapsed() > RESTORE_TIMEOUT;

            if result.waiting() == 0 || timeout {
                if result.waiting() > 0 {
                    samp::chat_line(
                        ver,
                        WHITE,
                        &format!(
                            "{PREFIX}{} Figuren konnten nicht zurückgesetzt werden (Fahrzeug/tot). Test beendet.",
                            result.waiting()
                        ),
                    );
                } else {
                    samp::chat_line(ver, WHITE, &format!("{PREFIX}Standard-CJ wiederhergestellt. Test beendet."));
                }

                crate::log!("Wiederherstellung fertig, wartend: {}", result.waiting());
                self.mode = Mode::Aus;
                self.peds.clear();
                self.queue.clear();
            }
        }
    }

    // -----------------------------------------------------------------------
    // Rebuild

    fn rebuild_next(&mut self, ver: SampVersion) {
        while let Some(ped_addr) = self.queue.pop_front() {
            let ped = Ped(ped_addr);

            if !ped.valid() || ped.model() != MODEL_PLAYER || ped.rebuild_blocker().is_some() {
                continue;
            }

            let Some(entry) = self.peds.get(&ped_addr) else {
                continue;
            };
            let snapshot = Entry {
                rw: entry.rw,
                applied: entry.applied,
                original: entry.original,
            };
            let Some(target) = self.target_for(&snapshot) else {
                continue;
            };
            let Some(desc_ptr) = ped.clothes_desc_ptr() else {
                continue;
            };

            self.write_target(target, desc_ptr, snapshot.original);

            let start = Instant::now();
            game::rebuild_player(ped_addr);
            let ms = start.elapsed().as_secs_f32() * 1000.0;

            self.stats.rebuilds += 1;
            self.stats.last_ms = ms;
            self.stats.max_ms = self.stats.max_ms.max(ms);

            let rw = if ped.valid() { ped.rw_object() } else { 0 };

            if let Some(entry) = self.peds.get_mut(&ped_addr) {
                entry.rw = rw;
                entry.applied = Some(target);
            }

            // the model-0 clump now shows `target`; every other existing CJ clump is older
            self.model_shows = Some(target);
            self.stale_rws = self
                .survey(ver)
                .iter()
                .filter(|cj| cj.ped != ped_addr && cj.rw != 0)
                .map(|cj| cj.rw)
                .collect();

            crate::log!(
                "RebuildPlayer {:#x} -> {:?} in {:.1} ms (Clump jetzt {:#x})",
                ped_addr,
                target,
                ms,
                rw
            );
            break;
        }
    }

    // -----------------------------------------------------------------------
    // /kleidungtest info

    fn info(&mut self, ver: SampVersion) {
        let cjs = self.survey(ver);
        let result = count(&cjs);

        let mode = match self.mode {
            Mode::Aus => "aus".to_owned(),
            Mode::Aktiv(id) => format!("{} aktiv", id.name()),
            Mode::Wiederherstellen => "Wiederherstellung läuft".to_owned(),
        };

        samp::chat_line(
            ver,
            WHITE,
            &format!("{PREFIX}ASI {}, SA-MP {}, Test: {mode}", crate::ASI_VERSION, ver.name()),
        );

        let ids: Vec<String> = result.remote_ids.iter().take(8).map(|id| id.to_string()).collect();
        let ids = if ids.is_empty() {
            String::new()
        } else {
            format!(", IDs {}", ids.join(" "))
        };

        let shared = if result.distinct_desc < result.with_desc {
            ", teilweise GETEILT"
        } else {
            ""
        };

        samp::chat_line(
            ver,
            WHITE,
            &format!(
                "CJ-Figuren gefunden: {} (eigene {}, fremde {}{ids}), Kleidungsdaten: {} verschiedene{shared}",
                result.local + result.remote,
                result.local,
                result.remote,
                result.distinct_desc
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

        for cj in cjs.iter().take(6) {
            let desc = Ped(cj.ped).clothes_desc();
            crate::log!(
                "Info: {} Ped {:#x} Clump {:#x} Kleidungsdaten {:#x} Oberteil-Textur {:08x}",
                who(cj),
                cj.ped,
                cj.rw,
                cj.desc.unwrap_or(0),
                desc.map(|d| d.texture_keys[0]).unwrap_or(0)
            );
        }
    }
}

fn who(cj: &CjPed) -> String {
    if cj.local {
        "eigene Figur".to_owned()
    } else {
        format!("Spieler-ID {}", cj.id)
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
            result.remote_ids.push(cj.id);
        }

        if let Some(desc) = cj.desc {
            result.with_desc += 1;
            descs.insert(desc);
        }
    }

    result.distinct_desc = descs.len() as u32;
    result
}

/// Texture keys and the model keys that belong to them (hands are filled in by the game).
fn same_clothes(a: &ClothesDesc, b: &ClothesDesc) -> bool {
    a.texture_keys == b.texture_keys
        && (0..a.texture_keys.len()).all(|part| {
            game::texture_dependency(part)
                .map(|model| a.model_keys[model] == b.model_keys[model])
                .unwrap_or(true)
        })
}
