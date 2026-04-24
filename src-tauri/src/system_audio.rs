use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};

#[derive(Debug, Clone)]
pub struct VolumeState {
    level_scalar: f32,
    muted: bool,
}

pub fn safe_mute_and_save() -> Option<VolumeState> {
    match mute_and_save() {
        Ok(state) => Some(state),
        Err(err) => {
            crate::app_log::warn(format!("系统静音失败: {}", err));
            None
        }
    }
}

pub fn safe_restore(state: Option<VolumeState>) {
    let Some(state) = state else {
        return;
    };
    if let Err(err) = restore(state) {
        crate::app_log::warn(format!("恢复系统音量失败: {}", err));
    }
}

fn mute_and_save() -> Result<VolumeState, String> {
    with_endpoint(|endpoint| unsafe {
        let state = VolumeState {
            level_scalar: endpoint
                .GetMasterVolumeLevelScalar()
                .map_err(|err| format!("读取系统音量失败: {}", err))?,
            muted: endpoint
                .GetMute()
                .map_err(|err| format!("读取静音状态失败: {}", err))?
                .as_bool(),
        };
        endpoint
            .SetMute(true, std::ptr::null())
            .map_err(|err| format!("设置系统静音失败: {}", err))?;
        Ok(state)
    })
}

fn restore(state: VolumeState) -> Result<(), String> {
    with_endpoint(|endpoint| unsafe {
        endpoint
            .SetMasterVolumeLevelScalar(state.level_scalar, std::ptr::null())
            .map_err(|err| format!("恢复系统音量失败: {}", err))?;
        endpoint
            .SetMute(state.muted, std::ptr::null())
            .map_err(|err| format!("恢复静音状态失败: {}", err))
    })
}

fn with_endpoint<T>(
    f: impl FnOnce(&IAudioEndpointVolume) -> Result<T, String>,
) -> Result<T, String> {
    let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let com_initialized = hr.is_ok();
    let result = unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|err| format!("创建音频设备枚举器失败: {}", err))?;
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|err| format!("获取默认播放设备失败: {}", err))?;
        let endpoint: IAudioEndpointVolume = device
            .Activate(CLSCTX_ALL, None)
            .map_err(|err| format!("激活系统音量接口失败: {}", err))?;
        f(&endpoint)
    };
    if com_initialized {
        unsafe { CoUninitialize() };
    }
    result
}
