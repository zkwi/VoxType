use serde::{Deserialize, Serialize};
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
    pub typing: TypingConfig,
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
    #[serde(default = "default_true")]
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
    #[serde(default = "default_true")]
    pub enable_recent_context: bool,
    #[serde(default = "default_recent_context_rounds")]
    pub recent_context_rounds: usize,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub hotwords: Vec<String>,
    #[serde(default)]
    pub prompt_context: Vec<TextContext>,
    #[serde(default)]
    pub recent_context: Vec<TextContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContext {
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingConfig {
    #[serde(default = "default_paste_delay_ms")]
    pub paste_delay_ms: u64,
    #[serde(default = "default_paste_method")]
    pub paste_method: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    #[serde(default = "default_true")]
    pub print_transcript_to_console: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadedConfig {
    pub path: String,
    pub exists: bool,
    pub data: AppConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey(),
            auth: AuthConfig::default(),
            audio: AudioConfig::default(),
            request: RequestConfig::default(),
            context: ContextConfig::default(),
            typing: TypingConfig::default(),
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
            mute_system_volume_while_recording: true,
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
            enable_recent_context: true,
            recent_context_rounds: default_recent_context_rounds(),
            image_url: None,
            hotwords: Vec::new(),
            prompt_context: Vec::new(),
            recent_context: Vec::new(),
        }
    }
}

impl Default for TypingConfig {
    fn default() -> Self {
        Self {
            paste_delay_ms: default_paste_delay_ms(),
            paste_method: default_paste_method(),
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
        }
    }
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            print_transcript_to_console: true,
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
    let data =
        toml::from_str::<AppConfig>(&text).map_err(|err| format!("解析配置失败: {}", err))?;
    Ok(LoadedConfig {
        path: path.display().to_string(),
        exists: true,
        data,
    })
}

pub fn save_config(config: AppConfig) -> Result<LoadedConfig, String> {
    let path = resolve_config_path();
    let text = toml::to_string_pretty(&config).map_err(|err| format!("序列化配置失败: {}", err))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| format!("创建配置目录失败: {}", err))?;
    }
    std::fs::write(&path, text).map_err(|err| format!("写入配置失败: {}", err))?;
    load_config()
}

pub fn remember_recent_context(text: &str) -> Result<(), String> {
    let mut loaded = load_config()?;
    if !loaded.data.context.enable_recent_context {
        return Ok(());
    }
    let cleaned = sanitize_recent_context_text(text);
    if cleaned.is_empty() {
        return Ok(());
    }
    loaded
        .data
        .context
        .recent_context
        .retain(|item| item.text != cleaned);
    loaded
        .data
        .context
        .recent_context
        .insert(0, TextContext { text: cleaned });
    loaded
        .data
        .context
        .recent_context
        .truncate(loaded.data.context.recent_context_rounds);
    save_config(loaded.data).map(|_| ())
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
