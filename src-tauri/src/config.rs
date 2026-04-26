use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub audio: AudioConfig,
    #[serde(default)]
    pub request: RequestConfig,
    #[serde(default)]
    pub context: ContextConfig,
    #[serde(default)]
    pub triggers: TriggerConfig,
    #[serde(default)]
    pub typing: TypingConfig,
    #[serde(default)]
    pub startup: StartupConfig,
    #[serde(default)]
    pub update: UpdateConfig,
    #[serde(default)]
    pub llm_post_edit: LlmPostEditConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub tray: TrayConfig,
    #[serde(default)]
    pub debug: DebugConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub app_key: String,
    #[serde(default)]
    pub access_key: String,
    #[serde(default = "default_resource_id")]
    pub resource_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    #[serde(default = "default_channels")]
    pub channels: u16,
    #[serde(default = "default_segment_ms")]
    pub segment_ms: u64,
    #[serde(default = "default_max_record_seconds")]
    pub max_record_seconds: u64,
    #[serde(default = "default_stop_grace_ms")]
    pub stop_grace_ms: u64,
    #[serde(default)]
    pub mute_system_volume_while_recording: bool,
    #[serde(default)]
    pub input_device: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestConfig {
    #[serde(default = "default_ws_url")]
    pub ws_url: String,
    #[serde(default = "default_model_name")]
    pub model_name: String,
    #[serde(default = "default_true")]
    pub enable_nonstream: bool,
    #[serde(default = "default_true")]
    pub enable_itn: bool,
    #[serde(default = "default_true")]
    pub enable_punc: bool,
    #[serde(default = "default_true")]
    pub enable_ddc: bool,
    #[serde(default = "default_true")]
    pub show_utterances: bool,
    #[serde(default = "default_result_type")]
    pub result_type: String,
    #[serde(default)]
    pub enable_accelerate_text: Option<bool>,
    #[serde(default)]
    pub accelerate_score: Option<i64>,
    #[serde(default)]
    pub end_window_size: Option<u64>,
    #[serde(default)]
    pub force_to_speech_time: Option<u64>,
    #[serde(default = "default_final_timeout")]
    pub final_result_timeout_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    #[serde(default)]
    pub enable_recent_context: bool,
    #[serde(default = "default_recent_context_rounds")]
    pub recent_context_rounds: usize,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub hotwords: Vec<String>,
    #[serde(default)]
    pub prompt_context: Vec<TextContext>,
    #[serde(default, skip_serializing)]
    pub recent_context: Vec<TextContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContext {
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConfig {
    #[serde(default = "default_true")]
    pub hotkey_enabled: bool,
    #[serde(default)]
    pub middle_mouse_enabled: bool,
    #[serde(default)]
    pub right_alt_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingConfig {
    #[serde(default = "default_paste_delay_ms")]
    pub paste_delay_ms: u64,
    #[serde(default = "default_paste_method")]
    pub paste_method: String,
    #[serde(default = "default_true")]
    pub restore_clipboard_after_paste: bool,
    #[serde(default = "default_clipboard_open_retry_count")]
    pub clipboard_open_retry_count: u32,
    #[serde(default = "default_clipboard_open_retry_interval_ms")]
    pub clipboard_open_retry_interval_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StartupConfig {
    #[serde(default)]
    pub launch_on_startup: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    #[serde(default = "default_true")]
    pub auto_check_on_startup: bool,
    #[serde(default = "default_update_github_repo")]
    pub github_repo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPostEditConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_min_chars")]
    pub min_chars: usize,
    #[serde(default = "default_llm_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default = "default_llm_timeout")]
    pub timeout_seconds: f64,
    #[serde(default)]
    pub enable_thinking: bool,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default = "default_user_prompt_template")]
    pub user_prompt_template: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_ui_width")]
    pub width: u32,
    #[serde(default = "default_ui_height")]
    pub height: u32,
    #[serde(default = "default_ui_margin_bottom")]
    pub margin_bottom: u32,
    #[serde(default = "default_ui_opacity")]
    pub opacity: f64,
    #[serde(default = "default_scroll_interval_ms")]
    pub scroll_interval_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayConfig {
    #[serde(default = "default_true")]
    pub show_startup_message: bool,
    #[serde(default = "default_startup_message_timeout_ms")]
    pub startup_message_timeout_ms: u64,
    #[serde(default = "default_close_behavior")]
    pub close_behavior: String,
    #[serde(default)]
    pub close_to_tray_notice_shown: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugConfig {
    #[serde(default)]
    pub print_transcript_to_console: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadedConfig {
    pub path: String,
    pub exists: bool,
    pub data: AppConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigValidationError {
    pub field: String,
    pub message: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey(),
            auth: AuthConfig::default(),
            audio: AudioConfig::default(),
            request: RequestConfig::default(),
            context: ContextConfig::default(),
            triggers: TriggerConfig::default(),
            typing: TypingConfig::default(),
            startup: StartupConfig::default(),
            update: UpdateConfig::default(),
            llm_post_edit: LlmPostEditConfig::default(),
            ui: UiConfig::default(),
            tray: TrayConfig::default(),
            debug: DebugConfig::default(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            app_key: String::new(),
            access_key: String::new(),
            resource_id: default_resource_id(),
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: default_sample_rate(),
            channels: default_channels(),
            segment_ms: default_segment_ms(),
            max_record_seconds: default_max_record_seconds(),
            stop_grace_ms: default_stop_grace_ms(),
            mute_system_volume_while_recording: false,
            input_device: None,
        }
    }
}

impl Default for RequestConfig {
    fn default() -> Self {
        Self {
            ws_url: default_ws_url(),
            model_name: default_model_name(),
            enable_nonstream: true,
            enable_itn: true,
            enable_punc: true,
            enable_ddc: true,
            show_utterances: true,
            result_type: default_result_type(),
            enable_accelerate_text: None,
            accelerate_score: None,
            end_window_size: None,
            force_to_speech_time: None,
            final_result_timeout_seconds: default_final_timeout(),
        }
    }
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            enable_recent_context: false,
            recent_context_rounds: default_recent_context_rounds(),
            image_url: None,
            hotwords: Vec::new(),
            prompt_context: Vec::new(),
            recent_context: Vec::new(),
        }
    }
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            hotkey_enabled: true,
            middle_mouse_enabled: false,
            right_alt_enabled: false,
        }
    }
}

impl Default for TypingConfig {
    fn default() -> Self {
        Self {
            paste_delay_ms: default_paste_delay_ms(),
            paste_method: default_paste_method(),
            restore_clipboard_after_paste: true,
            clipboard_open_retry_count: default_clipboard_open_retry_count(),
            clipboard_open_retry_interval_ms: default_clipboard_open_retry_interval_ms(),
        }
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            auto_check_on_startup: true,
            github_repo: default_update_github_repo(),
        }
    }
}

impl Default for LlmPostEditConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_chars: default_min_chars(),
            base_url: default_llm_base_url(),
            api_key: String::new(),
            model: default_llm_model(),
            timeout_seconds: default_llm_timeout(),
            enable_thinking: false,
            system_prompt: String::new(),
            user_prompt_template: default_user_prompt_template(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            width: default_ui_width(),
            height: default_ui_height(),
            margin_bottom: default_ui_margin_bottom(),
            opacity: default_ui_opacity(),
            scroll_interval_ms: default_scroll_interval_ms(),
        }
    }
}

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            show_startup_message: true,
            startup_message_timeout_ms: default_startup_message_timeout_ms(),
            close_behavior: default_close_behavior(),
            close_to_tray_notice_shown: false,
        }
    }
}

pub fn resolve_config_path() -> PathBuf {
    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("config.toml"));
        candidates.push(cwd.join("..").join("config.toml"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("config.toml"));
            candidates.push(dir.join("..").join("..").join("..").join("config.toml"));
            candidates.push(
                dir.join("..")
                    .join("..")
                    .join("..")
                    .join("..")
                    .join("..")
                    .join("config.toml"),
            );
        }
    }

    for candidate in &candidates {
        if candidate.exists() {
            return normalize_path(candidate);
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        if looks_like_project_root(&cwd) {
            return normalize_path(cwd.join("config.toml"));
        }
        if let Some(parent) = cwd.parent() {
            if looks_like_project_root(parent) {
                return normalize_path(parent.join("config.toml"));
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for ancestor in dir.ancestors() {
                if looks_like_project_root(ancestor) {
                    return normalize_path(ancestor.join("config.toml"));
                }
            }
            return normalize_path(dir.join("config.toml"));
        }
    }

    normalize_path(PathBuf::from("config.toml"))
}

pub fn load_config() -> Result<LoadedConfig, String> {
    let path = resolve_config_path();
    if !path.exists() {
        return Ok(LoadedConfig {
            path: path.display().to_string(),
            exists: false,
            data: AppConfig::default(),
        });
    }

    let text = std::fs::read_to_string(&path).map_err(|err| format!("读取配置失败: {}", err))?;
    let mut data =
        toml::from_str::<AppConfig>(&text).map_err(|err| format!("解析配置失败: {}", err))?;
    let legacy_recent_context = data.context.recent_context.clone();
    if data.context.enable_recent_context {
        data.context.recent_context = load_recent_context_entries(
            &path,
            legacy_recent_context.clone(),
            data.context.recent_context_rounds,
        );
        if !legacy_recent_context.is_empty() && !recent_context_path(&path).exists() {
            write_recent_context_entries(&path, &data.context.recent_context)?;
        }
    } else {
        data.context.recent_context.clear();
    }
    if contains_legacy_recent_context(&text) {
        let mut cleaned = data.clone();
        cleaned.context.recent_context.clear();
        write_config_file(&path, &cleaned)?;
    }
    Ok(LoadedConfig {
        path: path.display().to_string(),
        exists: true,
        data,
    })
}

pub fn save_config(config: AppConfig) -> Result<LoadedConfig, String> {
    validate_config(&config).map_err(format_validation_errors)?;
    let path = resolve_config_path();
    write_config_file(&path, &config)?;
    load_config()
}

pub fn remember_recent_context(text: &str) -> Result<(), String> {
    let loaded = load_config()?;
    if !loaded.data.context.enable_recent_context {
        return Ok(());
    }
    let cleaned = sanitize_recent_context_text(text);
    if cleaned.is_empty() {
        return Ok(());
    }
    let path = PathBuf::from(&loaded.path);
    let mut entries = load_recent_context_entries(
        &path,
        loaded.data.context.recent_context.clone(),
        loaded.data.context.recent_context_rounds,
    );
    entries.retain(|item| item.text != cleaned);
    entries.insert(0, TextContext { text: cleaned });
    entries.truncate(loaded.data.context.recent_context_rounds);
    write_recent_context_entries(&path, &entries)
}

pub fn clear_recent_context() -> Result<(), String> {
    let config_path = resolve_config_path();
    let path = recent_context_path(&config_path);
    remove_recent_context_file(&path)?;
    if config_path.exists() {
        let mut loaded = load_config()?;
        loaded.data.context.recent_context.clear();
        save_config(loaded.data)?;
        remove_recent_context_file(&path)?;
    }
    Ok(())
}

pub fn recent_context_count() -> usize {
    let path = resolve_config_path();
    load_recent_context_entries(&path, Vec::new(), usize::MAX).len()
}

pub fn validate_config(config: &AppConfig) -> Result<(), Vec<ConfigValidationError>> {
    let mut errors = Vec::new();

    validate_u32_range(
        &mut errors,
        "audio.sample_rate",
        config.audio.sample_rate,
        8_000,
        96_000,
        "采样率需在 8000 到 96000 之间。",
    );
    validate_u16_range(
        &mut errors,
        "audio.channels",
        config.audio.channels,
        1,
        2,
        "声道数只能填写 1 或 2。",
    );
    validate_u64_range(
        &mut errors,
        "audio.segment_ms",
        config.audio.segment_ms,
        20,
        2_000,
        "分片毫秒需在 20 到 2000 之间。",
    );
    validate_u64_range(
        &mut errors,
        "audio.max_record_seconds",
        config.audio.max_record_seconds,
        1,
        3_600,
        "最长录音秒数需在 1 到 3600 之间。",
    );
    validate_u64_range(
        &mut errors,
        "audio.stop_grace_ms",
        config.audio.stop_grace_ms,
        0,
        10_000,
        "停止收尾毫秒需在 0 到 10000 之间。",
    );
    validate_u64_range(
        &mut errors,
        "typing.paste_delay_ms",
        config.typing.paste_delay_ms,
        0,
        5_000,
        "粘贴延迟需在 0 到 5000 毫秒之间。",
    );
    validate_f64_range(
        &mut errors,
        "request.final_result_timeout_seconds",
        config.request.final_result_timeout_seconds,
        1.0,
        120.0,
        "最终结果等待时间需在 1 到 120 秒之间。",
    );
    validate_f64_range(
        &mut errors,
        "ui.opacity",
        config.ui.opacity,
        0.05,
        1.0,
        "悬浮窗透明度需大于 0 且不超过 1。",
    );
    validate_u32_range(
        &mut errors,
        "ui.width",
        config.ui.width,
        160,
        1_200,
        "悬浮窗宽度需在 160 到 1200 之间。",
    );
    validate_u32_range(
        &mut errors,
        "ui.height",
        config.ui.height,
        40,
        400,
        "悬浮窗高度需在 40 到 400 之间。",
    );
    validate_f64_range(
        &mut errors,
        "llm_post_edit.timeout_seconds",
        config.llm_post_edit.timeout_seconds,
        1.0,
        300.0,
        "大模型超时时间需在 1 到 300 秒之间。",
    );

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_u32_range(
    errors: &mut Vec<ConfigValidationError>,
    field: &str,
    value: u32,
    min: u32,
    max: u32,
    message: &str,
) {
    if value < min || value > max {
        push_validation_error(errors, field, message);
    }
}

fn validate_u16_range(
    errors: &mut Vec<ConfigValidationError>,
    field: &str,
    value: u16,
    min: u16,
    max: u16,
    message: &str,
) {
    if value < min || value > max {
        push_validation_error(errors, field, message);
    }
}

fn validate_u64_range(
    errors: &mut Vec<ConfigValidationError>,
    field: &str,
    value: u64,
    min: u64,
    max: u64,
    message: &str,
) {
    if value < min || value > max {
        push_validation_error(errors, field, message);
    }
}

fn validate_f64_range(
    errors: &mut Vec<ConfigValidationError>,
    field: &str,
    value: f64,
    min: f64,
    max: f64,
    message: &str,
) {
    if !value.is_finite() || value < min || value > max {
        push_validation_error(errors, field, message);
    }
}

fn push_validation_error(errors: &mut Vec<ConfigValidationError>, field: &str, message: &str) {
    errors.push(ConfigValidationError {
        field: field.to_string(),
        message: message.to_string(),
    });
}

fn format_validation_errors(errors: Vec<ConfigValidationError>) -> String {
    let summary = errors
        .iter()
        .map(|error| format!("{}: {}", error.field, error.message))
        .collect::<Vec<_>>()
        .join("; ");
    format!("配置存在不合法字段，请修改后再保存。{}", summary)
}

fn write_config_file(path: &Path, config: &AppConfig) -> Result<(), String> {
    let mut clean_config = config.clone();
    clean_config.context.recent_context.clear();
    let text =
        toml::to_string_pretty(&clean_config).map_err(|err| format!("序列化配置失败: {}", err))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| format!("创建配置目录失败: {}", err))?;
    }
    std::fs::write(path, text).map_err(|err| format!("写入配置失败: {}", err))
}

fn contains_legacy_recent_context(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        line == "[[context.recent_context]]"
            || line.starts_with("recent_context =")
            || line.starts_with("recent_context=")
    })
}

fn sanitize_recent_context_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(160)
        .collect::<String>()
        .trim()
        .to_string()
}

fn recent_context_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("context")
        .join("recent_context.jsonl")
}

fn load_recent_context_entries(
    config_path: &Path,
    fallback: Vec<TextContext>,
    max_rounds: usize,
) -> Vec<TextContext> {
    let path = recent_context_path(config_path);
    let entries = std::fs::read_to_string(&path)
        .ok()
        .map(|text| {
            text.lines()
                .filter_map(|line| serde_json::from_str::<TextContext>(line).ok())
                .map(|item| sanitize_recent_context_text(&item.text))
                .filter(|text| !text.is_empty())
                .map(|text| TextContext { text })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut entries = if entries.is_empty() {
        fallback
    } else {
        entries
    };
    entries.retain(|item| !item.text.trim().is_empty());
    entries.truncate(max_rounds);
    entries
}

fn write_recent_context_entries(config_path: &Path, entries: &[TextContext]) -> Result<(), String> {
    let path = recent_context_path(config_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("创建最近上下文目录失败: {}", err))?;
    }
    let mut file =
        std::fs::File::create(&path).map_err(|err| format!("写入最近上下文失败: {}", err))?;
    for item in entries {
        let line =
            serde_json::to_string(item).map_err(|err| format!("序列化最近上下文失败: {}", err))?;
        writeln!(file, "{}", line).map_err(|err| format!("写入最近上下文失败: {}", err))?;
    }
    Ok(())
}

fn remove_recent_context_file(path: &Path) -> Result<(), String> {
    if path.exists() {
        std::fs::remove_file(path).map_err(|err| format!("清除最近上下文失败: {}", err))?;
    }
    Ok(())
}

fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    dunce::simplified(path.as_ref()).to_path_buf()
}

fn looks_like_project_root(path: &Path) -> bool {
    path.join("package.json").exists() && path.join("src-tauri").is_dir()
}

fn default_hotkey() -> String {
    "ctrl+q".to_string()
}
fn default_resource_id() -> String {
    "volc.seedasr.sauc.duration".to_string()
}
fn default_sample_rate() -> u32 {
    16000
}
fn default_channels() -> u16 {
    1
}
fn default_segment_ms() -> u64 {
    200
}
fn default_max_record_seconds() -> u64 {
    300
}
fn default_stop_grace_ms() -> u64 {
    500
}
fn default_true() -> bool {
    true
}
fn default_ws_url() -> String {
    "wss://openspeech.bytedance.com/api/v3/sauc/bigmodel_async".to_string()
}
fn default_model_name() -> String {
    "bigmodel".to_string()
}
fn default_result_type() -> String {
    "full".to_string()
}
fn default_final_timeout() -> f64 {
    15.0
}
fn default_recent_context_rounds() -> usize {
    5
}
fn default_paste_delay_ms() -> u64 {
    120
}
fn default_paste_method() -> String {
    "ctrl_v".to_string()
}
fn default_clipboard_open_retry_count() -> u32 {
    5
}
fn default_clipboard_open_retry_interval_ms() -> u64 {
    50
}
fn default_update_github_repo() -> String {
    "zkwi/VoxType".to_string()
}
fn default_min_chars() -> usize {
    40
}
fn default_llm_base_url() -> String {
    "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string()
}
fn default_llm_model() -> String {
    "qwen3.5-plus".to_string()
}
fn default_llm_timeout() -> f64 {
    30.0
}
fn default_user_prompt_template() -> String {
    "{text}".to_string()
}
fn default_ui_width() -> u32 {
    350
}
fn default_ui_height() -> u32 {
    64
}
fn default_ui_margin_bottom() -> u32 {
    52
}
fn default_ui_opacity() -> f64 {
    0.9
}
fn default_scroll_interval_ms() -> u64 {
    1200
}
fn default_startup_message_timeout_ms() -> u64 {
    6000
}
fn default_close_behavior() -> String {
    "close_to_tray".to_string()
}

#[cfg(test)]
mod tests {
    use super::{contains_legacy_recent_context, validate_config, AppConfig, TextContext};

    #[test]
    fn defaults_are_conservative_for_consumer_use() {
        let config = AppConfig::default();
        assert!(config.triggers.hotkey_enabled);
        assert!(!config.triggers.middle_mouse_enabled);
        assert!(!config.triggers.right_alt_enabled);
        assert!(!config.audio.mute_system_volume_while_recording);
        assert!(!config.context.enable_recent_context);
        assert!(!config.debug.print_transcript_to_console);
        assert!(config.typing.restore_clipboard_after_paste);
        assert_eq!(config.typing.clipboard_open_retry_count, 5);
        assert_eq!(config.typing.clipboard_open_retry_interval_ms, 50);
    }

    #[test]
    fn validates_obviously_invalid_fields() {
        let mut config = AppConfig::default();
        config.audio.sample_rate = 0;
        config.audio.channels = 0;
        config.typing.paste_delay_ms = 9_999;
        config.request.final_result_timeout_seconds = 0.0;
        config.ui.opacity = 2.0;
        config.llm_post_edit.timeout_seconds = f64::NAN;

        let errors = validate_config(&config).expect_err("invalid config should fail");
        let fields = errors
            .iter()
            .map(|error| error.field.as_str())
            .collect::<Vec<_>>();

        assert!(fields.contains(&"audio.sample_rate"));
        assert!(fields.contains(&"audio.channels"));
        assert!(fields.contains(&"typing.paste_delay_ms"));
        assert!(fields.contains(&"request.final_result_timeout_seconds"));
        assert!(fields.contains(&"ui.opacity"));
        assert!(fields.contains(&"llm_post_edit.timeout_seconds"));
    }

    #[test]
    fn accepts_default_config() {
        assert!(validate_config(&AppConfig::default()).is_ok());
    }

    #[test]
    fn recent_context_is_not_serialized_to_config() {
        let mut config = AppConfig::default();
        config.context.enable_recent_context = true;
        config.context.recent_context = vec![TextContext {
            text: "private words".to_string(),
        }];

        let text = toml::to_string_pretty(&config).unwrap();
        assert!(!text.contains("context.recent_context"));
        assert!(!text.contains("private words"));
    }

    #[test]
    fn detects_legacy_recent_context_shapes() {
        assert!(contains_legacy_recent_context(
            "[context]\nenable_recent_context = true\nrecent_context = []\n"
        ));
        assert!(contains_legacy_recent_context(
            "[context]\nrecent_context=[]\n"
        ));
        assert!(contains_legacy_recent_context(
            "[[context.recent_context]]\ntext = \"private words\"\n"
        ));
        assert!(!contains_legacy_recent_context(
            "[context]\nenable_recent_context = true\nrecent_context_rounds = 5\n"
        ));
    }
}
