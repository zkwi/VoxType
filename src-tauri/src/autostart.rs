use crate::{app_log, config::StartupConfig};

const APP_NAME: &str = "VoxType";

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn apply(config: &StartupConfig) -> Result<(), String> {
    if config.launch_on_startup {
        enable()
    } else {
        disable()
    }
}

#[cfg(windows)]
fn enable() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|err| format!("获取程序路径失败: {}", err))?;
    let command = format!("\"{}\"", exe.display());
    let output = reg_command()
        .args([
            "add",
            run_key(),
            "/v",
            APP_NAME,
            "/t",
            "REG_SZ",
            "/d",
            &command,
            "/f",
        ])
        .output()
        .map_err(|err| format!("写入开机启动失败: {}", err))?;
    if output.status.success() {
        app_log::info(format!("开机自启动已启用: {}", exe.display()));
        Ok(())
    } else {
        Err(command_error("启用开机自启动失败", &output))
    }
}

#[cfg(windows)]
fn disable() -> Result<(), String> {
    let exists = reg_command()
        .args(["query", run_key(), "/v", APP_NAME])
        .output()
        .map_err(|err| format!("读取开机启动状态失败: {}", err))?;
    if !exists.status.success() {
        app_log::info("开机自启动未启用，无需关闭。");
        return Ok(());
    }

    let output = reg_command()
        .args(["delete", run_key(), "/v", APP_NAME, "/f"])
        .output()
        .map_err(|err| format!("关闭开机启动失败: {}", err))?;
    if output.status.success() {
        app_log::info("开机自启动已关闭。");
        Ok(())
    } else {
        Err(command_error("关闭开机启动失败", &output))
    }
}

#[cfg(windows)]
fn run_key() -> &'static str {
    r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run"
}

#[cfg(windows)]
fn reg_command() -> std::process::Command {
    use std::os::windows::process::CommandExt;

    let mut command = std::process::Command::new("reg");
    // GUI 应用启动控制台程序时会闪黑框；注册表同步应完全后台执行。
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

#[cfg(windows)]
fn command_error(prefix: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() { stderr } else { stdout };
    if detail.is_empty() {
        format!("{}: reg.exe 退出码 {:?}", prefix, output.status.code())
    } else {
        format!("{}: {}", prefix, detail)
    }
}

#[cfg(not(windows))]
fn enable() -> Result<(), String> {
    app_log::warn("当前平台暂不支持开机自启动。");
    Ok(())
}

#[cfg(not(windows))]
fn disable() -> Result<(), String> {
    Ok(())
}
