use chrono::Local;
use std::io::Write;
use std::path::PathBuf;

pub fn log_path() -> PathBuf {
    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("voice_input.log"));
        candidates.push(cwd.join("..").join("voice_input.log"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("voice_input.log"));
            candidates.push(dir.join("..").join("..").join("..").join("voice_input.log"));
        }
    }
    candidates
        .into_iter()
        .find(|path| path.exists())
        .unwrap_or_else(|| PathBuf::from("voice_input.log"))
}

pub fn info(message: impl AsRef<str>) {
    write_line("INFO", message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    write_line("WARNING", message.as_ref());
}

fn write_line(level: &str, message: &str) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    let _ = writeln!(
        file,
        "{} {} {}",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        level,
        message
    );
}
