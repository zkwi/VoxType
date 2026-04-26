use crate::config::{self, AppConfig};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

const MAX_ENTRY_CHARS: usize = 2_000;

#[derive(Debug, Clone, Serialize)]
pub struct AutoHotwordStatus {
    pub enabled: bool,
    pub path: String,
    pub entry_count: usize,
    pub total_chars: usize,
    pub max_history_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HotwordHistoryEntry {
    created_at: String,
    text: String,
}

pub fn append_transcript(text: &str) -> Result<(), String> {
    let loaded = config::load_config()?;
    let config_path = PathBuf::from(&loaded.path);
    append_transcript_with_config(&config_path, &loaded.data, text)
}

pub fn clear_history() -> Result<(), String> {
    let config_path = config::resolve_config_path();
    let path = history_path_for_config(&config_path);
    clear_history_at(&path)
}

pub fn status() -> Result<AutoHotwordStatus, String> {
    let loaded = config::load_config()?;
    let config_path = PathBuf::from(&loaded.path);
    Ok(status_at(&config_path, &loaded.data))
}

pub fn load_recent_text(max_chars: usize) -> Result<String, String> {
    let config_path = config::resolve_config_path();
    let path = history_path_for_config(&config_path);
    Ok(load_recent_text_from_path(&path, max_chars))
}

fn append_transcript_with_config(
    config_path: &Path,
    config: &AppConfig,
    text: &str,
) -> Result<(), String> {
    if !config.auto_hotwords.enabled {
        return Ok(());
    }

    let text = sanitize_history_text(text);
    if text.is_empty() {
        return Ok(());
    }

    let path = history_path_for_config(config_path);
    let mut entries = load_entries_from_path(&path);
    entries.push(HotwordHistoryEntry {
        created_at: Utc::now().to_rfc3339(),
        text,
    });
    let entries = trim_entries_to_max_chars(entries, config.auto_hotwords.max_history_chars);
    write_entries_to_path(&path, &entries)
}

fn clear_history_at(path: &Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_file(path).map_err(|err| format!("清空自动热词采集文本失败: {}", err))?;
    }
    Ok(())
}

fn status_at(config_path: &Path, config: &AppConfig) -> AutoHotwordStatus {
    let path = history_path_for_config(config_path);
    let entries = load_entries_from_path(&path);
    AutoHotwordStatus {
        enabled: config.auto_hotwords.enabled,
        path: path.display().to_string(),
        entry_count: entries.len(),
        total_chars: entries.iter().map(|entry| entry.text.chars().count()).sum(),
        max_history_chars: config.auto_hotwords.max_history_chars,
    }
}

fn load_recent_text_from_path(path: &Path, max_chars: usize) -> String {
    let entries = load_entries_from_path(path);
    let entries = trim_entries_to_max_chars(entries, max_chars);
    entries
        .into_iter()
        .map(|entry| entry.text)
        .collect::<Vec<_>>()
        .join("\n")
}

fn sanitize_history_text(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_ENTRY_CHARS)
        .collect::<String>()
        .trim()
        .to_string()
}

fn history_path_for_config(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("context")
        .join("hotword_history.jsonl")
}

fn load_entries_from_path(path: &Path) -> Vec<HotwordHistoryEntry> {
    std::fs::read_to_string(path)
        .ok()
        .map(|text| {
            text.lines()
                .filter_map(|line| serde_json::from_str::<HotwordHistoryEntry>(line).ok())
                .filter_map(|mut entry| {
                    entry.text = sanitize_history_text(&entry.text);
                    if entry.text.is_empty() {
                        None
                    } else {
                        Some(entry)
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn write_entries_to_path(path: &Path, entries: &[HotwordHistoryEntry]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("创建自动热词历史目录失败: {}", err))?;
    }
    let mut file =
        std::fs::File::create(path).map_err(|err| format!("写入自动热词历史失败: {}", err))?;
    for entry in entries {
        let line = serde_json::to_string(entry)
            .map_err(|err| format!("序列化自动热词历史失败: {}", err))?;
        writeln!(file, "{}", line).map_err(|err| format!("写入自动热词历史失败: {}", err))?;
    }
    Ok(())
}

fn trim_entries_to_max_chars(
    entries: Vec<HotwordHistoryEntry>,
    max_chars: usize,
) -> Vec<HotwordHistoryEntry> {
    if max_chars == 0 {
        return Vec::new();
    }

    let mut total_chars = 0usize;
    let mut kept = Vec::new();
    for mut entry in entries.into_iter().rev() {
        let entry_chars = entry.text.chars().count();
        if total_chars + entry_chars <= max_chars {
            total_chars += entry_chars;
            kept.push(entry);
            continue;
        }

        let remaining = max_chars.saturating_sub(total_chars);
        if remaining > 0 {
            entry.text = entry.text.chars().take(remaining).collect::<String>();
            if !entry.text.trim().is_empty() {
                kept.push(entry);
            }
        }
        break;
    }
    kept.reverse();
    kept
}

#[cfg(test)]
mod tests {
    use super::{
        append_transcript_with_config, clear_history_at, history_path_for_config,
        load_recent_text_from_path, status_at,
    };
    use crate::config::AppConfig;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_config_path(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "voxtype_hotword_history_{}_{}_{}",
            name,
            std::process::id(),
            suffix
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("config.toml")
    }

    #[test]
    fn disabled_config_does_not_create_history_file() {
        let config_path = temp_config_path("disabled");
        let config = AppConfig::default();

        append_transcript_with_config(&config_path, &config, "VoxType 测试文本").unwrap();

        assert!(!history_path_for_config(&config_path).exists());
    }

    #[test]
    fn enabled_config_writes_history_file() {
        let config_path = temp_config_path("enabled");
        let mut config = AppConfig::default();
        config.auto_hotwords.enabled = true;

        append_transcript_with_config(&config_path, &config, "  VoxType   自动 热词  ").unwrap();
        let path = history_path_for_config(&config_path);
        let text = std::fs::read_to_string(path).unwrap();

        assert!(text.contains("VoxType 自动 热词"));
    }

    #[test]
    fn history_is_trimmed_to_max_chars() {
        let config_path = temp_config_path("trim");
        let mut config = AppConfig::default();
        config.auto_hotwords.enabled = true;
        config.auto_hotwords.max_history_chars = 10;

        append_transcript_with_config(&config_path, &config, "一二三四五六").unwrap();
        append_transcript_with_config(&config_path, &config, "七八九十十一").unwrap();
        let text = load_recent_text_from_path(&history_path_for_config(&config_path), 10);

        assert!(text.replace('\n', "").chars().count() <= 10);
        assert!(text.contains("七八九十"));
    }

    #[test]
    fn clear_history_removes_file() {
        let config_path = temp_config_path("clear");
        let mut config = AppConfig::default();
        config.auto_hotwords.enabled = true;

        append_transcript_with_config(&config_path, &config, "VoxType").unwrap();
        let path = history_path_for_config(&config_path);
        assert!(path.exists());

        clear_history_at(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn status_reports_counts_without_text() {
        let config_path = temp_config_path("status");
        let mut config = AppConfig::default();
        config.auto_hotwords.enabled = true;

        append_transcript_with_config(&config_path, &config, "VoxType").unwrap();
        let status = status_at(&config_path, &config);

        assert!(status.enabled);
        assert_eq!(status.entry_count, 1);
        assert_eq!(status.total_chars, 7);
        assert!(status.path.ends_with("hotword_history.jsonl"));
    }
}
