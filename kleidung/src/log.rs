//! Log file `samplife-kleidung.log`, written from the watchdog thread and the game
//! thread. First choice is the folder of gta_sa.exe. If that can't be written (for
//! example under C:\Program Files without admin rights), the log goes to
//! "Documents\GTA San Andreas User Files", then to %TEMP%.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const NAME: &str = "samplife-kleidung.log";

static FILE: Mutex<Option<(File, PathBuf)>> = Mutex::new(None);

pub fn game_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn candidates() -> Vec<PathBuf> {
    let mut list = vec![game_dir().join(NAME)];

    if let Some(profile) = std::env::var_os("USERPROFILE") {
        list.push(
            PathBuf::from(profile)
                .join("Documents")
                .join("GTA San Andreas User Files")
                .join(NAME),
        );
    }

    list.push(std::env::temp_dir().join(NAME));
    list
}

fn open() -> Option<(File, PathBuf)> {
    for path in candidates() {
        // a new file per game start
        if let Ok(file) = OpenOptions::new().create(true).write(true).truncate(true).open(&path) {
            return Some((file, path));
        }
    }

    None
}

/// Where the log is written (opens it if needed).
pub fn path() -> Option<PathBuf> {
    let mut guard = FILE.lock().unwrap_or_else(|p| p.into_inner());

    if guard.is_none() {
        *guard = open();
    }

    guard.as_ref().map(|(_, path)| path.clone())
}

pub fn write(line: &str) {
    let mut guard = FILE.lock().unwrap_or_else(|p| p.into_inner());

    if guard.is_none() {
        *guard = open();
    }

    if let Some((file, _)) = guard.as_mut() {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let _ = writeln!(file, "[{}.{:03}] {line}", millis / 1000, millis % 1000);
        let _ = file.flush();
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::write(&format!($($arg)*))
    };
}
