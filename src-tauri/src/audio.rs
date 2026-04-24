use crate::app_log;
use crate::config::AudioConfig;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, SampleRate, Stream, StreamConfig, SupportedStreamConfig};
use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct AudioCaptureInfo {
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub chunks: usize,
    pub pcm_bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioDeviceInfo {
    pub index: u32,
    pub name: String,
}

#[derive(Clone)]
struct CaptureCounters {
    chunks: Arc<AtomicUsize>,
    pcm_bytes: Arc<AtomicUsize>,
}

pub struct AudioCapture {
    stop_tx: mpsc::Sender<()>,
    join_handle: Option<JoinHandle<()>>,
    device_name: String,
    sample_rate: u32,
    channels: u16,
    counters: CaptureCounters,
}

impl AudioCapture {
    pub fn info(&self) -> AudioCaptureInfo {
        AudioCaptureInfo {
            device_name: self.device_name.clone(),
            sample_rate: self.sample_rate,
            channels: self.channels,
            chunks: self.counters.chunks.load(Ordering::Relaxed),
            pcm_bytes: self.counters.pcm_bytes.load(Ordering::Relaxed),
        }
    }
}

pub fn start_capture(
    audio: &AudioConfig,
    chunk_tx: Option<mpsc::Sender<Vec<u8>>>,
) -> Result<AudioCapture, String> {
    let audio = audio.clone();
    let counters = CaptureCounters {
        chunks: Arc::new(AtomicUsize::new(0)),
        pcm_bytes: Arc::new(AtomicUsize::new(0)),
    };
    let worker_counters = counters.clone();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (stop_tx, stop_rx) = mpsc::channel();

    let join_handle = thread::spawn(move || {
        let (stream, device_name, sample_rate, channels) =
            match start_capture_in_thread(&audio, worker_counters, chunk_tx) {
                Ok(result) => result,
                Err(err) => {
                    let _ = ready_tx.send(Err(err));
                    return;
                }
            };
        if ready_tx
            .send(Ok((device_name, sample_rate, channels)))
            .is_err()
        {
            return;
        }
        let _ = stop_rx.recv();
        drop(stream);
    });

    let (device_name, sample_rate, channels) = ready_rx
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "启动麦克风采集超时".to_string())??;

    Ok(AudioCapture {
        stop_tx,
        join_handle: Some(join_handle),
        device_name,
        sample_rate,
        channels,
        counters,
    })
}

pub fn list_input_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    let host = cpal::default_host();
    let devices = host
        .input_devices()
        .map_err(|err| format!("枚举输入设备失败: {}", err))?;
    Ok(devices
        .enumerate()
        .map(|(index, device)| AudioDeviceInfo {
            index: index as u32,
            name: device
                .name()
                .unwrap_or_else(|_| format!("Input device {}", index)),
        })
        .collect())
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        let _ = self.stop_tx.send(());
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

fn start_capture_in_thread(
    audio: &AudioConfig,
    counters: CaptureCounters,
    chunk_tx: Option<mpsc::Sender<Vec<u8>>>,
) -> Result<(Stream, String, u32, u16), String> {
    let host = cpal::default_host();
    let device = select_input_device(&host, audio.input_device)?;
    let device_name = device
        .name()
        .unwrap_or_else(|_| "Unknown input device".to_string());
    let supported = select_input_config(&device, audio)?;
    let sample_format = supported.sample_format();
    let stream_config = StreamConfig {
        channels: supported.channels(),
        sample_rate: supported.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };
    let target_chunk_bytes = target_chunk_bytes(&stream_config, audio.segment_ms);
    let err_fn = |err| app_log::warn(format!("audio input stream error: {}", err));
    let stream = match sample_format {
        SampleFormat::I16 => build_i16_stream(
            &device,
            &stream_config,
            target_chunk_bytes,
            counters.clone(),
            chunk_tx,
            err_fn,
        )?,
        SampleFormat::U16 => build_u16_stream(
            &device,
            &stream_config,
            target_chunk_bytes,
            counters.clone(),
            chunk_tx,
            err_fn,
        )?,
        SampleFormat::U8 => build_u8_stream(
            &device,
            &stream_config,
            target_chunk_bytes,
            counters.clone(),
            chunk_tx,
            err_fn,
        )?,
        SampleFormat::F32 => build_f32_stream(
            &device,
            &stream_config,
            target_chunk_bytes,
            counters.clone(),
            chunk_tx,
            err_fn,
        )?,
        other => return Err(format!("暂不支持的输入采样格式: {:?}", other)),
    };
    stream
        .play()
        .map_err(|err| format!("启动麦克风采集失败: {}", err))?;
    Ok((
        stream,
        device_name,
        stream_config.sample_rate.0,
        stream_config.channels,
    ))
}

fn select_input_device(host: &cpal::Host, input_device: Option<u32>) -> Result<Device, String> {
    if let Some(index) = input_device {
        return host
            .input_devices()
            .map_err(|err| format!("枚举输入设备失败: {}", err))?
            .nth(index as usize)
            .ok_or_else(|| format!("找不到配置中的输入设备: {}", index));
    }
    host.default_input_device()
        .ok_or_else(|| "未找到默认麦克风输入设备".to_string())
}

fn select_input_config(
    device: &Device,
    audio: &AudioConfig,
) -> Result<SupportedStreamConfig, String> {
    let target_rate = SampleRate(audio.sample_rate);
    let mut fallback = None;
    for range in device
        .supported_input_configs()
        .map_err(|err| format!("读取麦克风采样配置失败: {}", err))?
    {
        if fallback.is_none() {
            fallback = Some(range.with_max_sample_rate());
        }
        if range.channels() == audio.channels
            && range.min_sample_rate() <= target_rate
            && target_rate <= range.max_sample_rate()
        {
            return Ok(range.with_sample_rate(target_rate));
        }
    }
    fallback
        .or_else(|| device.default_input_config().ok())
        .ok_or_else(|| "麦克风没有可用采样配置".to_string())
}

fn target_chunk_bytes(config: &StreamConfig, segment_ms: u64) -> usize {
    let frames = ((config.sample_rate.0 as u64 * segment_ms.max(1)) / 1000).max(1);
    frames as usize * config.channels.max(1) as usize * 2
}

fn send_segmented_bytes(tx: &mpsc::Sender<Vec<u8>>, pending: &mut Vec<u8>, target_bytes: usize) {
    while pending.len() >= target_bytes {
        let chunk = pending.drain(..target_bytes).collect::<Vec<_>>();
        let _ = tx.send(chunk);
    }
}

fn build_i16_stream(
    device: &Device,
    config: &StreamConfig,
    target_chunk_bytes: usize,
    counters: CaptureCounters,
    chunk_tx: Option<mpsc::Sender<Vec<u8>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, String> {
    let channels = config.channels.max(1) as usize;
    let mut pending = Vec::new();
    device
        .build_input_stream(
            config,
            move |data: &[i16], _| {
                let frame_count = data.len() / channels;
                counters.chunks.fetch_add(1, Ordering::Relaxed);
                counters
                    .pcm_bytes
                    .fetch_add(frame_count * channels * 2, Ordering::Relaxed);
                if let Some(tx) = &chunk_tx {
                    for sample in data {
                        pending.extend(sample.to_le_bytes());
                    }
                    send_segmented_bytes(tx, &mut pending, target_chunk_bytes);
                }
            },
            err_fn,
            None,
        )
        .map_err(|err| format!("创建麦克风采集流失败: {}", err))
}

fn build_u16_stream(
    device: &Device,
    config: &StreamConfig,
    target_chunk_bytes: usize,
    counters: CaptureCounters,
    chunk_tx: Option<mpsc::Sender<Vec<u8>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, String> {
    let channels = config.channels.max(1) as usize;
    let mut pending = Vec::new();
    device
        .build_input_stream(
            config,
            move |data: &[u16], _| {
                let frame_count = data.len() / channels;
                counters.chunks.fetch_add(1, Ordering::Relaxed);
                counters
                    .pcm_bytes
                    .fetch_add(frame_count * channels * 2, Ordering::Relaxed);
                if let Some(tx) = &chunk_tx {
                    for sample in data {
                        let value = (*sample as i32 - 32768) as i16;
                        pending.extend(value.to_le_bytes());
                    }
                    send_segmented_bytes(tx, &mut pending, target_chunk_bytes);
                }
            },
            err_fn,
            None,
        )
        .map_err(|err| format!("创建麦克风采集流失败: {}", err))
}

fn build_u8_stream(
    device: &Device,
    config: &StreamConfig,
    target_chunk_bytes: usize,
    counters: CaptureCounters,
    chunk_tx: Option<mpsc::Sender<Vec<u8>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, String> {
    let channels = config.channels.max(1) as usize;
    let mut pending = Vec::new();
    device
        .build_input_stream(
            config,
            move |data: &[u8], _| {
                let frame_count = data.len() / channels;
                counters.chunks.fetch_add(1, Ordering::Relaxed);
                counters
                    .pcm_bytes
                    .fetch_add(frame_count * channels * 2, Ordering::Relaxed);
                if let Some(tx) = &chunk_tx {
                    for sample in data {
                        let value = (*sample as i16 - 128) << 8;
                        pending.extend(value.to_le_bytes());
                    }
                    send_segmented_bytes(tx, &mut pending, target_chunk_bytes);
                }
            },
            err_fn,
            None,
        )
        .map_err(|err| format!("创建麦克风采集流失败: {}", err))
}

fn build_f32_stream(
    device: &Device,
    config: &StreamConfig,
    target_chunk_bytes: usize,
    counters: CaptureCounters,
    chunk_tx: Option<mpsc::Sender<Vec<u8>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<Stream, String> {
    let channels = config.channels.max(1) as usize;
    let mut pending = Vec::new();
    device
        .build_input_stream(
            config,
            move |data: &[f32], _| {
                let frame_count = data.len() / channels;
                counters.chunks.fetch_add(1, Ordering::Relaxed);
                counters
                    .pcm_bytes
                    .fetch_add(frame_count * channels * 2, Ordering::Relaxed);
                if let Some(tx) = &chunk_tx {
                    for sample in data {
                        let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                        pending.extend(value.to_le_bytes());
                    }
                    send_segmented_bytes(tx, &mut pending, target_chunk_bytes);
                }
            },
            err_fn,
            None,
        )
        .map_err(|err| format!("创建麦克风采集流失败: {}", err))
}
