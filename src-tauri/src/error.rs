use std::fmt::Display;

/// 为底层错误添加用户可读上下文。
///
/// Tauri command 边界当前仍使用 `String`，这里先集中处理上下文拼接，
/// 避免各模块散落重复的 `map_err(|err| format!(...))`。
pub(crate) fn context<E>(message: &'static str) -> impl FnOnce(E) -> String
where
    E: Display,
{
    move |err| format!("{message}: {err}")
}
