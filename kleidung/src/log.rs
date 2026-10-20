//! Small log file `samplife-kleidung.log` next to gta_sa.exe.
//! Only called from the game thread, never from DllMain.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static STARTED: AtomicBool = AtomicBool::new(false);

fn path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
        .join("samplife-kleidung.log")
}

pub fn write(line: &str) {
    let first = !STARTED.swap(true, Ordering::SeqCst);

    // a new file per game start, appended afterwards
    let file = if first {
        OpenOptions::new().create(true).write(true).truncate(true).open(path())
    } else {
        OpenOptions::new().create(true).append(true).open(path())
    };

    if let Ok(mut file) = file {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{secs}] {line}");
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::write(&format!($($arg)*))
    };
}
