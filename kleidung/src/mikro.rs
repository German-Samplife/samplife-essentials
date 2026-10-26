//! Voice level of the local player for the lips (lippen.rs).
//!
//! While the server says a talk key is held (`lippen:mikro 1`, sent only when SampVoice
//! found a microphone), the plugin reads the default recording device itself, the one
//! SampVoice records from, through waveIn (winmm; Windows shares the device), and tells
//! the server when speech starts and stops (`lippen:pegel 1|0`). Only this yes/no leaves
//! the client, no audio.
//!
//! Polled on the game thread every frame, without callback or thread: 4 buffers of
//! 40 ms, 16 kHz mono. Speech starts after two buffers in a row at least `ABSTAND` dB
//! above the noise floor and above `MIN_DB`, and ends `NACHLAUF` after the last loud
//! buffer. The device stays open `OFFEN_NACH` after the key is released, so the next
//! press doesn't open it again (opening blocks the game thread briefly). If it can't be
//! opened, every press counts as speech (key only).
//!
//! waveIn from mmeapi.h/mmsystem.h; WAVEFORMATEX from mmreg.h (packed, 18 bytes),
//! WAVEHDR 32 bytes on x86.

use std::time::{Duration, Instant};

const RATE: u32 = 16_000;
const SAMPLES: usize = 640; // 40 ms
const PUFFER: usize = 4;

const WAVE_MAPPER: u32 = u32::MAX;
const CALLBACK_NULL: u32 = 0;
const WAVE_FORMAT_PCM: u16 = 1;
const WHDR_DONE: u32 = 0x1;
const MMSYSERR_NOERROR: u32 = 0;

/// speech: this many dB above the noise floor ...
const ABSTAND: f32 = 10.0;
/// ... and at least this loud (dBFS)
const MIN_DB: f32 = -55.0;
/// the noise floor starts here at most and rises by this much per buffer (2.5 dB/s)
const BODEN_START: f32 = -45.0;
const BODEN_ANSTIEG: f32 = 0.1;
/// loud buffers in a row before speech starts (a key click is shorter)
const ANSCHLAG: u32 = 2;
const NACHLAUF: Duration = Duration::from_millis(300);
const OFFEN_NACH: Duration = Duration::from_secs(30);

#[allow(dead_code)]
#[repr(C, packed)]
struct WaveFormat {
    tag: u16,
    channels: u16,
    rate: u32,
    bytes_per_sec: u32,
    block_align: u16,
    bits: u16,
    cb_size: u16,
}

#[allow(dead_code)]
#[repr(C)]
struct WaveHdr {
    data: *mut i16,
    length: u32,
    recorded: u32,
    user: usize,
    flags: u32,
    loops: u32,
    next: *mut WaveHdr,
    reserved: usize,
}

#[link(name = "winmm")]
extern "system" {
    fn waveInOpen(handle: *mut usize, device: u32, format: *const WaveFormat, callback: usize, instance: usize, flags: u32) -> u32;
    fn waveInPrepareHeader(handle: usize, hdr: *mut WaveHdr, size: u32) -> u32;
    fn waveInUnprepareHeader(handle: usize, hdr: *mut WaveHdr, size: u32) -> u32;
    fn waveInAddBuffer(handle: usize, hdr: *mut WaveHdr, size: u32) -> u32;
    fn waveInStart(handle: usize) -> u32;
    fn waveInReset(handle: usize) -> u32;
    fn waveInClose(handle: usize) -> u32;
}

const HDR_SIZE: u32 = std::mem::size_of::<WaveHdr>() as u32;

/// RMS of one buffer in dBFS, without DC offset.
pub fn pegel_dbfs(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return -120.0;
    }

    let n = samples.len() as f64;
    let mittel = samples.iter().map(|&s| s as f64).sum::<f64>() / n;
    let quadrat = samples.iter().map(|&s| (s as f64 - mittel).powi(2)).sum::<f64>() / n;
    let rms = quadrat.sqrt() / 32768.0;

    (20.0 * rms.max(1e-6).log10()) as f32
}

/// Speech detection over the buffer levels, see the module docs.
pub struct Stimme {
    boden: Option<f32>,
    laut_in_folge: u32,
    zuletzt_laut: Option<Instant>,
    pub an: bool,
}

impl Stimme {
    pub fn new() -> Stimme {
        Stimme {
            boden: None,
            laut_in_folge: 0,
            zuletzt_laut: None,
            an: false,
        }
    }

    pub fn boden(&self) -> Option<f32> {
        self.boden
    }

    /// Learns the noise floor from every buffer, also while no key is held.
    pub fn puffer(&mut self, db: f32, jetzt: Instant) {
        let boden = self.boden.unwrap_or(db.min(BODEN_START));
        let laut = db >= MIN_DB && db >= boden + ABSTAND;

        self.boden = Some(if db < boden { db } else { boden + BODEN_ANSTIEG });

        if laut {
            self.laut_in_folge += 1;

            if self.laut_in_folge >= ANSCHLAG {
                self.an = true;
                self.zuletzt_laut = Some(jetzt);
            } else if self.an {
                self.zuletzt_laut = Some(jetzt);
            }
        } else {
            self.laut_in_folge = 0;
        }

        self.nachlauf(jetzt);
    }

    pub fn nachlauf(&mut self, jetzt: Instant) {
        if self.an && self.zuletzt_laut.is_none_or(|t| jetzt.duration_since(t) >= NACHLAUF) {
            self.an = false;
        }
    }

    /// Key released: speech over, the noise floor stays.
    pub fn ruhe(&mut self) {
        self.an = false;
        self.laut_in_folge = 0;
        self.zuletzt_laut = None;
    }
}

struct Aufnahme {
    handle: usize,
    kopf: Box<[WaveHdr; PUFFER]>,
    // the device writes here while a buffer is queued; never moved, freed after Reset
    _daten: Box<[[i16; SAMPLES]; PUFFER]>,
}

// only ever touched on the game thread; the Mutex around State needs Send
unsafe impl Send for Aufnahme {}

impl Aufnahme {
    fn oeffnen() -> Result<Aufnahme, String> {
        let format = WaveFormat {
            tag: WAVE_FORMAT_PCM,
            channels: 1,
            rate: RATE,
            bytes_per_sec: RATE * 2,
            block_align: 2,
            bits: 16,
            cb_size: 0,
        };

        let mut handle = 0usize;
        let ergebnis = unsafe { waveInOpen(&mut handle, WAVE_MAPPER, &format, 0, 0, CALLBACK_NULL) };

        if ergebnis != MMSYSERR_NOERROR || handle == 0 {
            return Err(format!("waveInOpen: Fehler {ergebnis}"));
        }

        let mut daten = Box::new([[0i16; SAMPLES]; PUFFER]);
        let kopf: Box<[WaveHdr; PUFFER]> = Box::new(std::array::from_fn(|i| WaveHdr {
            data: daten[i].as_mut_ptr(),
            length: (SAMPLES * 2) as u32,
            recorded: 0,
            user: 0,
            flags: 0,
            loops: 0,
            next: std::ptr::null_mut(),
            reserved: 0,
        }));

        let mut aufnahme = Aufnahme { handle, kopf, _daten: daten };

        for i in 0..PUFFER {
            let hdr: *mut WaveHdr = &mut aufnahme.kopf[i];

            unsafe {
                let ergebnis = waveInPrepareHeader(handle, hdr, HDR_SIZE);
                if ergebnis != MMSYSERR_NOERROR {
                    return Err(format!("waveInPrepareHeader: Fehler {ergebnis}"));
                }
                let ergebnis = waveInAddBuffer(handle, hdr, HDR_SIZE);
                if ergebnis != MMSYSERR_NOERROR {
                    return Err(format!("waveInAddBuffer: Fehler {ergebnis}"));
                }
            }
        }

        let ergebnis = unsafe { waveInStart(handle) };

        if ergebnis != MMSYSERR_NOERROR {
            return Err(format!("waveInStart: Fehler {ergebnis}"));
        }

        Ok(aufnahme)
    }

    /// Levels of all buffers the device has filled since the last call; each goes back
    /// into the queue.
    fn fertige(&mut self) -> Vec<f32> {
        let mut pegel = Vec::new();

        for hdr in self.kopf.iter_mut() {
            // the device sets WHDR_DONE from its own thread
            let flags = unsafe { std::ptr::read_volatile(&hdr.flags) };

            if flags & WHDR_DONE == 0 {
                continue;
            }

            let anzahl = ((hdr.recorded as usize) / 2).min(SAMPLES);
            let samples = unsafe { std::slice::from_raw_parts(hdr.data, anzahl) };
            pegel.push(pegel_dbfs(samples));

            hdr.recorded = 0;
            hdr.flags = flags & !WHDR_DONE;
            unsafe { waveInAddBuffer(self.handle, hdr, HDR_SIZE) };
        }

        pegel
    }
}

impl Drop for Aufnahme {
    fn drop(&mut self) {
        unsafe {
            // Reset hands every queued buffer back; only then may they be freed
            waveInReset(self.handle);

            for hdr in self.kopf.iter_mut() {
                waveInUnprepareHeader(self.handle, hdr, HDR_SIZE);
            }

            waveInClose(self.handle);
        }
    }
}

pub struct Mikro {
    aufnahme: Option<Aufnahme>,
    /// opening failed in this connection: key only
    fehler: bool,
    taste: bool,
    losgelassen: Option<Instant>,
    stimme: Stimme,
    /// what the server knows (`lippen:pegel`)
    gemeldet: bool,
    letzter_pegel: Option<f32>,
    pub meldungen: u32,
}

impl Mikro {
    pub fn new() -> Mikro {
        Mikro {
            aufnahme: None,
            fehler: false,
            taste: false,
            losgelassen: None,
            stimme: Stimme::new(),
            gemeldet: false,
            letzter_pegel: None,
            meldungen: 0,
        }
    }

    pub fn zustand(&self) -> String {
        let geraet = match (&self.aufnahme, self.fehler) {
            (Some(_), _) => "offen",
            (None, true) => "ließ sich nicht öffnen, nur Taste",
            (None, false) => "zu",
        };
        let pegel = self.letzter_pegel.map_or("-".to_owned(), |p| format!("{p:.0} dB"));
        let boden = self.stimme.boden().map_or("-".to_owned(), |b| format!("{b:.0} dB"));

        format!(
            "Mikrofon {geraet}, Taste {}, Stimme {}, Pegel {pegel}, Grundrauschen {boden}, gemeldet {}x",
            if self.taste { "gedrückt" } else { "los" },
            if self.stimme.an { "ja" } else { "nein" },
            self.meldungen
        )
    }

    /// `lippen:mikro`: the server says a talk key is held (or no longer).
    pub fn taste(&mut self, an: bool) {
        if self.taste == an {
            return;
        }

        self.taste = an;

        if !an {
            // the server forgets the level itself when the key is released
            self.losgelassen = Some(Instant::now());
            self.stimme.ruhe();
            self.gemeldet = false;
        }
    }

    pub fn getrennt(&mut self) {
        self.taste = false;
        self.aufnahme = None;
        self.fehler = false;
        self.losgelassen = None;
        self.stimme = Stimme::new();
        self.gemeldet = false;
        self.letzter_pegel = None;
    }

    pub fn frame(&mut self) {
        let jetzt = Instant::now();

        if self.taste && self.aufnahme.is_none() && !self.fehler {
            match Aufnahme::oeffnen() {
                Ok(aufnahme) => {
                    crate::log!("Lippen: Mikrofon geöffnet");
                    self.aufnahme = Some(aufnahme);
                }
                Err(grund) => {
                    crate::log!("Lippen: Mikrofon lässt sich nicht öffnen ({grund}), Lippen nur nach der Taste");
                    self.fehler = true;
                }
            }
        }

        if let Some(aufnahme) = self.aufnahme.as_mut() {
            for db in aufnahme.fertige() {
                self.letzter_pegel = Some(db);
                self.stimme.puffer(db, jetzt);
            }
        }

        self.stimme.nachlauf(jetzt);

        if !self.taste {
            if self.aufnahme.is_some() && self.losgelassen.is_some_and(|t| jetzt.duration_since(t) >= OFFEN_NACH) {
                crate::log!("Lippen: Mikrofon geschlossen");
                self.aufnahme = None;
            }
            return;
        }

        let sprechen = self.fehler || self.stimme.an;

        if sprechen != self.gemeldet && crate::netz::lippen_pegel(sprechen) {
            self.gemeldet = sprechen;
            self.meldungen += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pegel() {
        assert!(pegel_dbfs(&[0; 640]) < -100.0);
        let voll: Vec<i16> = (0..640).map(|i| if i % 2 == 0 { 32767 } else { -32767 }).collect();
        assert!(pegel_dbfs(&voll) > -0.1);
        // DC offset alone is no sound
        assert!(pegel_dbfs(&[5000; 640]) < -100.0);
    }

    #[test]
    fn stimme_mit_nachlauf() {
        let mut stimme = Stimme::new();
        let start = Instant::now();
        let t = |ms: u64| start + Duration::from_millis(ms);

        // quiet room
        for i in 0..10 {
            stimme.puffer(-60.0, t(i * 40));
        }
        assert!(!stimme.an);

        // one loud buffer (key click) is not speech
        stimme.puffer(-25.0, t(400));
        stimme.puffer(-60.0, t(440));
        assert!(!stimme.an);

        // speech
        stimme.puffer(-25.0, t(480));
        stimme.puffer(-25.0, t(520));
        assert!(stimme.an);

        // short pause between words keeps it
        stimme.puffer(-60.0, t(560));
        stimme.puffer(-60.0, t(600));
        assert!(stimme.an);

        // 300 ms quiet ends it
        stimme.nachlauf(t(820));
        assert!(!stimme.an);
    }

    #[test]
    fn lauter_raum() {
        let mut stimme = Stimme::new();
        let start = Instant::now();

        // steady noise at -40 dB is learned and doesn't count as speech
        for i in 0..200 {
            stimme.puffer(-40.0, start + Duration::from_millis(i * 40));
        }
        assert!(!stimme.an);
        assert!(stimme.boden().unwrap() >= -40.5);
    }
}
