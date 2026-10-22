//! Protokoll `samplife-ladebild.log` im GTA-Verzeichnis, sonst in
//! „Dokumente\GTA San Andreas User Files“, sonst in %TEMP%. Wird bei jedem Start neu angelegt.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use winapi::um::minwinbase::SYSTEMTIME;
use winapi::um::sysinfoapi::GetLocalTime;

const NAME: &str = "samplife-ladebild.log";

static DATEI: Mutex<Option<File>> = Mutex::new(None);

pub fn spielordner() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|ordner| ordner.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn kandidaten() -> Vec<PathBuf> {
    let mut liste = vec![spielordner().join(NAME)];
    if let Some(profil) = std::env::var_os("USERPROFILE") {
        liste.push(PathBuf::from(profil).join("Documents").join("GTA San Andreas User Files").join(NAME));
    }
    liste.push(std::env::temp_dir().join(NAME));
    liste
}

pub fn zeile(text: &str) {
    let Ok(mut datei) = DATEI.lock() else { return };
    if datei.is_none() {
        *datei = kandidaten()
            .into_iter()
            .find_map(|pfad| OpenOptions::new().create(true).write(true).truncate(true).open(pfad).ok());
    }
    if let Some(datei) = datei.as_mut() {
        let mut zeit: SYSTEMTIME = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut zeit) };
        let _ = writeln!(datei, "{:02}:{:02}:{:02}.{:03}  {text}", zeit.wHour, zeit.wMinute, zeit.wSecond, zeit.wMilliseconds);
        let _ = datei.flush();
    }
}
