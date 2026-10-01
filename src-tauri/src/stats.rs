use chrono::{DateTime, Duration, Local};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Write;
use std::path::PathBuf;

const STATS_FILE_NAME: &str = "voice_input_stats.jsonl";

#[derive(Debug, Clone, Serialize)]
pub struct UsageStats {
    pub session_count: u32,
    pub total_seconds: f64,
    pub total_chars: u32,
    pub total_minutes_int: u32,
    pub avg_chars_per_minute: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyUsageStats {
    pub day: String,
    pub stats: UsageStats,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsSnapshot {
    pub path: String,
    pub recent_24h: UsageStats,
    pub recent_7d: UsageStats,
    pub by_day: Vec<DailyUsageStats>,
    pub history: Vec<HistoryEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryEvent {
    pub created_at: String,
    pub duration_seconds: f64,
    pub text_chars: u32,
}

#[derive(Debug, Deserialize)]
struct RawEvent {
    created_at: String,
    duration_seconds: Option<f64>,
    text_chars: Option<u32>,
}

#[derive(Debug, Clone)]
struct ParsedEvent {
    created_at: DateTime<Local>,
    duration_seconds: f64,
    text_chars: u32,
}

impl UsageStats {
    fn empty() -> Self {
        Self {
            session_count: 0,
            total_seconds: 0.0,
            total_chars: 0,
            total_minutes_int: 0,
            avg_chars_per_minute: 0.0,
        }
    }

    fn from_events<'a>(events: impl Iterator<Item = &'a ParsedEvent>) -> Self {
        let mut stats = Self::empty();
        for event in events {
            stats.session_count += 1;
            stats.total_seconds += event.duration_seconds.max(0.0);
            stats.total_chars += event.text_chars;
        }
        stats.total_minutes_int = (stats.total_seconds / 60.0).floor() as u32;
        stats.avg_chars_per_minute = if stats.total_seconds <= 0.0 {
            0.0
        } else {
            stats.total_chars as f64 / (stats.total_seconds / 60.0)
        };
        stats
    }
}

pub fn stats_path() -> PathBuf {
    let candidates = stats_path_candidates();
    for candidate in &candidates {
        if candidate.exists() {
            return dunce::simplified(candidate).to_path_buf();
        }
    }
    let fallback = new_stats_file_path(
        crate::config::is_development_layout(),
        candidates.first().cloned(),
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|dir| dir.to_path_buf())),
    );
    dunce::simplified(&fallback).to_path_buf()
}

/// 还没有统计文件时，新文件建在哪里。
///
/// 安装版被开机自启动拉起时，工作目录是系统目录，往那里写会一直失败，统计也就永远记不上。
/// 所以安装版固定建在程序所在目录，和从快捷方式启动时的位置一致；开发时仍放在工作目录。
fn new_stats_file_path(
    development_layout: bool,
    working_dir_candidate: Option<PathBuf>,
    exe_dir: Option<PathBuf>,
) -> PathBuf {
    let beside_exe = exe_dir.map(|dir| dir.join(STATS_FILE_NAME));
    let preferred = if development_layout {
        working_dir_candidate.or(beside_exe)
    } else {
        beside_exe.or(working_dir_candidate)
    };
    preferred.unwrap_or_else(|| PathBuf::from(STATS_FILE_NAME))
}

fn stats_path_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(STATS_FILE_NAME));
        candidates.push(cwd.join("..").join(STATS_FILE_NAME));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(STATS_FILE_NAME));
            candidates.push(dir.join("..").join("..").join("..").join(STATS_FILE_NAME));
            candidates.push(
                dir.join("..")
                    .join("..")
                    .join("..")
                    .join("..")
                    .join("..")
                    .join(STATS_FILE_NAME),
            );
        }
    }
    candidates
}

pub fn load_stats_snapshot() -> StatsSnapshot {
    let path = stats_path();
    let events = read_events(&path);
    let now = Local::now();
    let recent_24h_start = now - Duration::hours(24);
    let recent_7d_start = now - Duration::days(7);
    let recent_24h = UsageStats::from_events(
        events
            .iter()
            .filter(|event| event.created_at >= recent_24h_start),
    );
    let recent_7d = UsageStats::from_events(
        events
            .iter()
            .filter(|event| event.created_at >= recent_7d_start),
    );

    let mut by_day = Vec::new();
    for offset in (0..7).rev() {
        let day = (now - Duration::days(offset)).date_naive();
        let day_key = day.to_string();
        let stats = UsageStats::from_events(
            events
                .iter()
                .filter(|event| event.created_at.date_naive() == day),
        );
        by_day.push(DailyUsageStats {
            day: day_key,
            stats,
        });
    }

    let history = events
        .iter()
        .rev()
        .take(30)
        .map(|event| HistoryEvent {
            created_at: event.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            duration_seconds: event.duration_seconds,
            text_chars: event.text_chars,
        })
        .collect();

    StatsSnapshot {
        path: path.display().to_string(),
        recent_24h,
        recent_7d,
        by_day,
        history,
    }
}

pub fn stats_event_count() -> usize {
    read_events(&stats_path()).len()
}

pub fn clear_stats() -> Result<(), String> {
    clear_stats_paths(&stats_path_candidates())
}

fn clear_stats_paths(paths: &[PathBuf]) -> Result<(), String> {
    let mut cleared = Vec::new();
    for path in paths {
        let path = dunce::simplified(path).to_path_buf();
        if cleared
            .iter()
            .any(|cleared_path: &PathBuf| cleared_path == &path)
        {
            continue;
        }
        if path.exists() {
            std::fs::remove_file(&path).map_err(|err| format!("清空统计数据失败: {}", err))?;
        }
        cleared.push(path);
    }
    Ok(())
}

pub fn append_event(text: &str, duration_seconds: f64) -> Result<(), String> {
    let path = stats_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| format!("创建统计目录失败: {}", err))?;
    }
    let event = json!({
        "created_at": Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        "duration_seconds": (duration_seconds.max(0.0) * 100.0).round() / 100.0,
        "text_chars": count_text_chars(text),
    });
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|err| format!("打开统计文件失败: {}", err))?;
    writeln!(file, "{}", event).map_err(|err| format!("写入统计文件失败: {}", err))
}

fn count_text_chars(text: &str) -> u32 {
    text.split_whitespace().collect::<String>().chars().count() as u32
}

fn read_events(path: &PathBuf) -> Vec<ParsedEvent> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    text.lines()
        .filter_map(|line| serde_json::from_str::<RawEvent>(line).ok())
        .filter_map(|raw| {
            let created_at =
                chrono::NaiveDateTime::parse_from_str(&raw.created_at, "%Y-%m-%dT%H:%M:%S")
                    .ok()
                    .and_then(|value| value.and_local_timezone(Local).single())?;
            Some(ParsedEvent {
                created_at,
                duration_seconds: raw.duration_seconds.unwrap_or(0.0),
                text_chars: raw.text_chars.unwrap_or(0),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{clear_stats_paths, new_stats_file_path};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn installed_app_creates_stats_beside_the_executable() {
        // 回归：开机自启动拉起时工作目录是系统目录，此前新统计文件会落到那里而写入失败。
        let path = new_stats_file_path(
            false,
            Some(PathBuf::from(
                r"C:\Windows\System32\voice_input_stats.jsonl",
            )),
            Some(PathBuf::from(r"C:\Users\Alice\AppData\Local\VoxType")),
        );

        assert_eq!(
            path,
            PathBuf::from(r"C:\Users\Alice\AppData\Local\VoxType\voice_input_stats.jsonl")
        );
    }

    #[test]
    fn development_run_keeps_stats_in_the_working_directory() {
        let path = new_stats_file_path(
            true,
            Some(PathBuf::from(r"C:\repo\src-tauri\voice_input_stats.jsonl")),
            Some(PathBuf::from(r"C:\repo\src-tauri\target\debug")),
        );

        assert_eq!(
            path,
            PathBuf::from(r"C:\repo\src-tauri\voice_input_stats.jsonl")
        );
    }

    #[test]
    fn stats_path_still_resolves_when_no_directory_is_known() {
        assert_eq!(
            new_stats_file_path(false, None, None),
            PathBuf::from("voice_input_stats.jsonl")
        );
    }

    struct TempStatsDir {
        dir: PathBuf,
    }

    impl TempStatsDir {
        fn new() -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!(
                "voxtype_stats_clear_{}_{}",
                std::process::id(),
                suffix
            ));
            std::fs::create_dir_all(&dir).unwrap();
            Self { dir }
        }

        fn path(&self, name: &str) -> PathBuf {
            self.dir.join(name)
        }
    }

    impl Drop for TempStatsDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn clear_stats_removes_all_existing_candidate_files() {
        let fixture = TempStatsDir::new();
        let primary = fixture.path("primary.jsonl");
        let fallback = fixture.path("fallback.jsonl");
        std::fs::write(&primary, "{}\n").unwrap();
        std::fs::write(&fallback, "{}\n").unwrap();

        clear_stats_paths(&[primary.clone(), fallback.clone(), primary.clone()]).unwrap();

        assert!(!primary.exists());
        assert!(!fallback.exists());
    }
}
