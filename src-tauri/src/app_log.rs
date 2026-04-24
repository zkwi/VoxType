use chrono::Local;
use std::io::Write;
use std::path::{Path, PathBuf};

const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_ARCHIVE_FILES: usize = 3;
const MAX_MESSAGE_CHARS: usize = 2000;
const REDACTED: &str = "[redacted]";
const SENSITIVE_KEYS: &[&str] = &[
    "access_key",
    "app_key",
    "api_key",
    "authorization",
    "bearer",
    "client_secret",
    "password",
    "secret",
    "secret_key",
    "token",
];

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
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|dir| dir.join("voice_input.log")))
        })
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
    rotate_if_needed(&path);
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    let message = sanitize_message(message);
    let _ = writeln!(
        file,
        "{} {} {}",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        level,
        message
    );
}

fn rotate_if_needed(path: &PathBuf) {
    let Ok(metadata) = std::fs::metadata(path) else {
        return;
    };
    if metadata.len() < MAX_LOG_BYTES {
        return;
    }

    let oldest = archive_path(path, MAX_ARCHIVE_FILES);
    let _ = std::fs::remove_file(oldest);
    for index in (1..MAX_ARCHIVE_FILES).rev() {
        let from = archive_path(path, index);
        let to = archive_path(path, index + 1);
        if from.exists() {
            let _ = std::fs::rename(from, to);
        }
    }
    let _ = std::fs::rename(path, archive_path(path, 1));
}

fn archive_path(path: &Path, index: usize) -> PathBuf {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return path.with_extension(format!("log.{}", index));
    };
    path.with_file_name(format!("{}.{}", file_name, index))
}

fn sanitize_message(message: &str) -> String {
    let flattened = message
        .chars()
        .map(|ch| if ch == '\r' || ch == '\n' { ' ' } else { ch })
        .collect::<String>();
    let mut redacted = redact_prefixed_token(&flattened, "bearer ");
    for key in SENSITIVE_KEYS {
        redacted = redact_key_values(&redacted, key);
    }
    redacted = redact_openai_style_keys(&redacted);

    let mut limited = redacted.chars().take(MAX_MESSAGE_CHARS).collect::<String>();
    if redacted.chars().count() > MAX_MESSAGE_CHARS {
        limited.push_str("...");
    }
    limited
}

fn redact_key_values(message: &str, key: &str) -> String {
    let mut output = message.to_string();
    let mut search_start = 0;
    loop {
        let lower = output.to_ascii_lowercase();
        let Some(relative_pos) = lower[search_start..].find(key) else {
            break;
        };
        let pos = search_start + relative_pos;
        let bytes = output.as_bytes();
        let mut cursor = pos + key.len();
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() || !matches!(bytes[cursor], b':' | b'=') {
            search_start = pos + key.len();
            continue;
        }
        cursor += 1;
        while cursor < output.len() && output.as_bytes()[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        let quote = output.as_bytes().get(cursor).copied().and_then(|byte| {
            if byte == b'"' || byte == b'\'' {
                Some(byte)
            } else {
                None
            }
        });
        if quote.is_some() {
            cursor += 1;
        }
        let value_start = cursor;
        while cursor < output.len() {
            let byte = output.as_bytes()[cursor];
            if let Some(quote) = quote {
                if byte == quote {
                    break;
                }
            } else if byte.is_ascii_whitespace() || matches!(byte, b',' | b';' | b'}' | b']') {
                break;
            }
            cursor += 1;
        }
        if cursor > value_start {
            output.replace_range(value_start..cursor, REDACTED);
            search_start = value_start + REDACTED.len();
        } else {
            search_start = cursor;
        }
    }
    output
}

fn redact_prefixed_token(message: &str, prefix: &str) -> String {
    let mut output = message.to_string();
    let mut search_start = 0;
    loop {
        let lower = output.to_ascii_lowercase();
        let Some(relative_pos) = lower[search_start..].find(prefix) else {
            break;
        };
        let value_start = search_start + relative_pos + prefix.len();
        let mut cursor = value_start;
        while cursor < output.len()
            && !output.as_bytes()[cursor].is_ascii_whitespace()
            && !matches!(output.as_bytes()[cursor], b',' | b';' | b'}' | b']')
        {
            cursor += 1;
        }
        if cursor > value_start {
            output.replace_range(value_start..cursor, REDACTED);
            search_start = value_start + REDACTED.len();
        } else {
            search_start = cursor;
        }
    }
    output
}

fn redact_openai_style_keys(message: &str) -> String {
    let mut output = message.to_string();
    let mut search_start = 0;
    while let Some(relative_pos) = output[search_start..].find("sk-") {
        let value_start = search_start + relative_pos;
        let mut cursor = value_start;
        while cursor < output.len()
            && !output.as_bytes()[cursor].is_ascii_whitespace()
            && !matches!(
                output.as_bytes()[cursor],
                b',' | b';' | b'}' | b']' | b'"' | b'\''
            )
        {
            cursor += 1;
        }
        if cursor.saturating_sub(value_start) >= 12 {
            output.replace_range(value_start..cursor, REDACTED);
            search_start = value_start + REDACTED.len();
        } else {
            search_start = cursor;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::sanitize_message;

    #[test]
    fn redacts_common_secret_shapes() {
        let openai_like_key = ["sk", "1234567890abcdef"].join("-");
        let message = sanitize_message(&format!(
            "access_key=\"abc123456789\" api_key={} Authorization: Bearer tokenvalue",
            openai_like_key
        ));
        assert!(!message.contains("abc123456789"));
        assert!(!message.contains(&openai_like_key),);
        assert!(!message.contains("tokenvalue"));
    }

    #[test]
    fn flattens_multiline_messages() {
        let message = sanitize_message("first\nsecond\r\nthird");
        assert_eq!(message, "first second  third");
    }
}
