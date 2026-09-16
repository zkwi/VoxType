use crate::{app_log, config, main_window};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

pub const SETUP_GUIDE_URL: &str = "https://github.com/zkwi/VoxType/wiki/Setup-Guide";
pub const DOUBAO_ASR_DOCS_URL: &str = "https://www.volcengine.com/docs/6561/1354869?lang=zh";
pub const DOUBAO_AGENT_PLAN_ASR_DOCS_URL: &str =
    "https://www.volcengine.com/docs/82379/2516286?lang=zh";
pub const DOUBAO_API_KEY_ASR_DOCS_URL: &str =
    "https://docs.volcengine.com/docs/6561/2630027?lang=zh";
pub const ALIYUN_ASR_DOCS_URL: &str =
    "https://help.aliyun.com/zh/model-studio/fun-asr-realtime-websocket-api";

pub fn open(app: &AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(SETUP_GUIDE_URL, None::<&str>)
        .map_err(|err| format!("打开配置指南失败: {}", err))
}

pub fn doubao_asr_docs_url(auth_mode: &str) -> &'static str {
    match auth_mode.trim() {
        config::DOUBAO_AUTH_MODE_AGENT_PLAN => DOUBAO_AGENT_PLAN_ASR_DOCS_URL,
        config::DOUBAO_AUTH_MODE_API_KEY => DOUBAO_API_KEY_ASR_DOCS_URL,
        _ => DOUBAO_ASR_DOCS_URL,
    }
}

pub fn open_doubao_asr_docs(app: &AppHandle, auth_mode: &str) -> Result<(), String> {
    app.opener()
        .open_url(doubao_asr_docs_url(auth_mode), None::<&str>)
        .map_err(|err| format!("打开豆包帮助文档失败: {}", err))
}

pub fn open_aliyun_asr_docs(app: &AppHandle) -> Result<(), String> {
    app.opener()
        .open_url(ALIYUN_ASR_DOCS_URL, None::<&str>)
        .map_err(|err| format!("打开阿里云帮助文档失败: {}", err))
}

pub fn open_if_config_missing(app: &AppHandle) {
    app_log::info("配置文件检查开始。");
    let Ok(loaded) = config::load_config() else {
        app_log::warn("读取配置文件失败，跳过自动打开配置指南。");
        return;
    };
    if loaded.exists {
        app_log::info(format!("配置文件已存在: {}", loaded.path));
        return;
    }

    main_window::show_existing(app, "缺少配置文件");
    match open(app) {
        Ok(()) => app_log::info("未找到 config.toml，已打开配置指南。"),
        Err(err) => app_log::warn(err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_docs_url_per_auth_mode() {
        assert_eq!(
            doubao_asr_docs_url(config::DOUBAO_AUTH_MODE_AGENT_PLAN),
            "https://www.volcengine.com/docs/82379/2516286?lang=zh"
        );
        assert_eq!(
            doubao_asr_docs_url(config::DOUBAO_AUTH_MODE_API_KEY),
            "https://docs.volcengine.com/docs/6561/2630027?lang=zh"
        );
        assert_eq!(
            doubao_asr_docs_url(config::DOUBAO_AUTH_MODE_APP_ACCESS),
            DOUBAO_ASR_DOCS_URL
        );
    }
}
