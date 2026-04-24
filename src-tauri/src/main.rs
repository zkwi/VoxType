// Windows 下调试版也不需要额外控制台，日志已写入 voice_input.log。
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    voxtype_lib::run()
}
