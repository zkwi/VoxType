use crate::{audio, config::AppConfig};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

// 头部保护并入第一包真实音频，避免独立短包破坏豆包推荐的发包节奏。
const INITIAL_AUDIO_SILENCE_PADDING_MS: u64 = 50;
// 建连和屏幕 OCR 会让本地先攒下几包音频；落后时按豆包建议区间的下限追赶，分片大小不变。
const CATCH_UP_SEND_INTERVAL: Duration = Duration::from_millis(100);
// 录音已结束时剩下的都是本地积压，不再需要按说话节奏发，实测服务端最终文本一致。
const TAIL_FLUSH_SEND_INTERVAL: Duration = Duration::from_millis(50);
// 队列里还剩这么多包就算落后，需要追赶。
const CATCH_UP_BACKLOG_PACKETS: usize = 2;

/// 发包节奏：只影响发送间隔，不改变音频分片大小。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SendPace {
    Realtime,
    CatchUp,
    TailFlush,
}

pub(super) struct AudioSendPacer {
    next_send_at: Option<Instant>,
}

pub(super) struct AsrAudioQueue {
    pending_packets: VecDeque<Vec<u8>>,
    buffered: Vec<u8>,
    leading_padding: Vec<u8>,
    first_packet_bytes: usize,
    regular_packet_bytes: usize,
    real_audio_seen: bool,
    first_packet_sent: bool,
    closed: bool,
}

impl AsrAudioQueue {
    pub(super) fn new(config: &AppConfig) -> Self {
        let regular_packet_bytes =
            asr_pcm_bytes_for_ms(audio::effective_asr_segment_ms(config.audio.segment_ms)).max(2)
                as usize;
        let first_packet_bytes = regular_packet_bytes;
        Self {
            pending_packets: VecDeque::new(),
            buffered: Vec::new(),
            leading_padding: vec![
                0;
                asr_pcm_bytes_for_ms(INITIAL_AUDIO_SILENCE_PADDING_MS) as usize
            ],
            first_packet_bytes,
            regular_packet_bytes,
            real_audio_seen: false,
            first_packet_sent: false,
            closed: false,
        }
    }

    pub(super) fn push_real_audio(&mut self, chunk: Vec<u8>) {
        if chunk.is_empty() || self.closed {
            return;
        }
        if !self.real_audio_seen {
            self.real_audio_seen = true;
            if !self.leading_padding.is_empty() {
                self.buffered
                    .extend(std::mem::take(&mut self.leading_padding));
            }
        }
        self.buffered.extend(chunk);
        self.drain_complete_packets();
    }

    pub(super) fn close_input(&mut self) {
        if self.closed {
            return;
        }
        self.closed = true;
        self.drain_complete_packets();
        if !self.buffered.is_empty() {
            self.pending_packets
                .push_back(std::mem::take(&mut self.buffered));
        }
    }

    pub(super) fn pop_front(&mut self) -> Option<Vec<u8>> {
        self.pending_packets.pop_front()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.pending_packets.is_empty()
    }

    /// 取出一包后调用：按剩余积压决定下一包的发送节奏。
    pub(super) fn send_pace(&self) -> SendPace {
        if self.closed {
            SendPace::TailFlush
        } else if self.pending_packets.len() >= CATCH_UP_BACKLOG_PACKETS {
            SendPace::CatchUp
        } else {
            SendPace::Realtime
        }
    }

    fn drain_complete_packets(&mut self) {
        loop {
            let target = self.next_packet_bytes();
            if self.buffered.len() < target {
                break;
            }
            let packet = self.buffered.drain(..target).collect::<Vec<_>>();
            self.pending_packets.push_back(packet);
            self.first_packet_sent = true;
        }
    }

    fn next_packet_bytes(&self) -> usize {
        if self.first_packet_sent {
            self.regular_packet_bytes
        } else {
            self.first_packet_bytes
        }
    }
}

impl AudioSendPacer {
    pub(super) fn new() -> Self {
        Self { next_send_at: None }
    }

    fn interval_for_audio_bytes(byte_len: usize) -> Duration {
        Duration::from_millis(
            asr_pcm_duration_ms_for_bytes(byte_len)
                .clamp(audio::ASR_MIN_SEGMENT_MS, audio::ASR_MAX_SEGMENT_MS),
        )
    }

    pub(super) fn ready_to_send(&self) -> bool {
        self.next_send_at
            .map(|next_send_at| Instant::now() >= next_send_at)
            .unwrap_or(true)
    }

    fn response_poll_timeout(&self, default_timeout: Duration) -> Duration {
        let Some(next_send_at) = self.next_send_at else {
            return default_timeout;
        };
        let wait = next_send_at
            .checked_duration_since(Instant::now())
            .unwrap_or_default();
        if wait.is_zero() {
            Duration::from_millis(1)
        } else {
            wait.min(default_timeout)
        }
    }

    pub(super) fn mark_sent_bytes(&mut self, byte_len: usize, pace: SendPace) {
        let interval = match pace {
            SendPace::Realtime => Self::interval_for_audio_bytes(byte_len),
            SendPace::CatchUp => CATCH_UP_SEND_INTERVAL,
            SendPace::TailFlush => TAIL_FLUSH_SEND_INTERVAL,
        };
        self.next_send_at = Some(Instant::now() + interval);
    }
}

pub(super) fn websocket_response_poll_timeout(
    audio_finished: bool,
    audio_pacer: &AudioSendPacer,
    default_timeout: Duration,
) -> Duration {
    if audio_finished {
        default_timeout
    } else {
        audio_pacer.response_poll_timeout(default_timeout)
    }
}

pub(super) fn silent_test_audio(config: &AppConfig) -> Vec<u8> {
    let bytes_per_second =
        audio::ASR_OUTPUT_SAMPLE_RATE as usize * audio::ASR_OUTPUT_CHANNELS as usize * 2;
    let requested = bytes_per_second
        .saturating_mul(audio::effective_asr_segment_ms(config.audio.segment_ms) as usize)
        / 1000;
    let byte_len = requested.clamp(3_200, 32_000);
    vec![0; byte_len]
}

fn asr_pcm_bytes_for_ms(duration_ms: u64) -> u64 {
    if duration_ms == 0 {
        return 0;
    }
    audio::ASR_OUTPUT_SAMPLE_RATE as u64 * audio::ASR_OUTPUT_CHANNELS as u64 * 2 * duration_ms
        / 1000
}

fn asr_pcm_duration_ms_for_bytes(byte_len: usize) -> u64 {
    let bytes_per_second =
        audio::ASR_OUTPUT_SAMPLE_RATE as u64 * audio::ASR_OUTPUT_CHANNELS as u64 * 2;
    (byte_len as u64)
        .saturating_mul(1000)
        .div_ceil(bytes_per_second)
}

#[cfg(test)]
mod tests {
    use super::{
        asr_pcm_bytes_for_ms, silent_test_audio, websocket_response_poll_timeout, AsrAudioQueue,
        AudioSendPacer, SendPace,
    };
    use crate::config::AppConfig;
    use std::time::{Duration, Instant};

    const RESPONSE_POLL_TIMEOUT: Duration = Duration::from_millis(20);

    #[test]
    fn silent_test_audio_is_small_and_non_empty() {
        let config = AppConfig::default();
        let audio = silent_test_audio(&config);
        assert!(!audio.is_empty());
        assert!(audio.len() >= 3_200);
        assert!(audio.len() <= 32_000);
        assert!(audio.iter().all(|value| *value == 0));
    }

    #[test]
    fn initial_audio_padding_keeps_first_packet_at_configured_segment_size() {
        let mut config = AppConfig::default();
        config.audio.segment_ms = 200;
        let real_audio = vec![7; asr_pcm_bytes_for_ms(200) as usize];
        let mut queue = AsrAudioQueue::new(&config);

        queue.push_real_audio(real_audio);
        let first = queue.pop_front().unwrap();
        queue.close_input();
        let tail = queue.pop_front().unwrap();

        assert_eq!(first.len(), asr_pcm_bytes_for_ms(200) as usize);
        assert!(first[..asr_pcm_bytes_for_ms(50) as usize]
            .iter()
            .all(|byte| *byte == 0));
        assert!(first[asr_pcm_bytes_for_ms(50) as usize..]
            .iter()
            .all(|byte| *byte == 7));
        assert_eq!(tail.len(), asr_pcm_bytes_for_ms(50) as usize);
        assert!(tail.iter().all(|byte| *byte == 7));
        assert!(queue.pop_front().is_none());
    }

    #[test]
    fn initial_audio_padding_does_not_repeat_between_real_packets() {
        let config = AppConfig::default();
        let mut queue = AsrAudioQueue::new(&config);

        queue.push_real_audio(vec![7; asr_pcm_bytes_for_ms(200) as usize]);
        let first = queue.pop_front().unwrap();
        queue.push_real_audio(vec![9; asr_pcm_bytes_for_ms(200) as usize]);
        let second = queue.pop_front().unwrap();
        queue.close_input();
        let tail = queue.pop_front().unwrap();

        assert_eq!(first.len(), asr_pcm_bytes_for_ms(200) as usize);
        assert_eq!(second.len(), asr_pcm_bytes_for_ms(200) as usize);
        assert_eq!(tail.len(), asr_pcm_bytes_for_ms(50) as usize);
        assert!(first[..asr_pcm_bytes_for_ms(50) as usize]
            .iter()
            .all(|byte| *byte == 0));
        assert!(second.iter().all(|byte| *byte == 7 || *byte == 9));
        assert!(tail.iter().all(|byte| *byte == 9));
    }

    #[test]
    fn closing_without_real_audio_does_not_send_padding() {
        let config = AppConfig::default();
        let mut queue = AsrAudioQueue::new(&config);

        queue.close_input();

        assert!(queue.pop_front().is_none());
    }

    #[test]
    fn audio_send_pacer_uses_actual_packet_duration_with_documented_bounds() {
        assert_eq!(
            AudioSendPacer::interval_for_audio_bytes(640),
            Duration::from_millis(100)
        );
        assert_eq!(
            AudioSendPacer::interval_for_audio_bytes(3_200),
            Duration::from_millis(100)
        );
        assert_eq!(
            AudioSendPacer::interval_for_audio_bytes(5_120),
            Duration::from_millis(160)
        );
        assert_eq!(
            AudioSendPacer::interval_for_audio_bytes(16_000),
            Duration::from_millis(200)
        );
    }

    #[test]
    fn audio_send_pacer_does_not_block_response_polling_until_next_packet() {
        let mut pacer = AudioSendPacer::new();

        assert!(pacer.ready_to_send());
        assert_eq!(
            pacer.response_poll_timeout(RESPONSE_POLL_TIMEOUT),
            RESPONSE_POLL_TIMEOUT
        );

        pacer.mark_sent_bytes(3_200, SendPace::Realtime);

        assert!(!pacer.ready_to_send());
        assert!(pacer.response_poll_timeout(RESPONSE_POLL_TIMEOUT) <= RESPONSE_POLL_TIMEOUT);
    }

    #[test]
    fn send_pace_follows_backlog_and_input_state() {
        let config = AppConfig::default();
        let mut queue = AsrAudioQueue::new(&config);
        let packet = vec![7; asr_pcm_bytes_for_ms(200) as usize];

        queue.push_real_audio(packet.clone());
        queue.pop_front().unwrap();
        assert_eq!(queue.send_pace(), SendPace::Realtime);

        // 建连/OCR 期间攒下的积压：取走一包后仍有两包待发，应转入追赶。
        for _ in 0..3 {
            queue.push_real_audio(packet.clone());
        }
        queue.pop_front().unwrap();
        assert_eq!(queue.send_pace(), SendPace::CatchUp);

        queue.close_input();
        assert_eq!(queue.send_pace(), SendPace::TailFlush);
    }

    #[test]
    fn catch_up_and_tail_flush_shorten_send_interval_without_changing_segment_size() {
        let mut pacer = AudioSendPacer::new();
        let realtime_bytes = asr_pcm_bytes_for_ms(200) as usize;

        // 机器负载高时下一次发送时间可能已经过去，用 checked 版本避免断言变成 panic。
        let remaining = |pacer: &AudioSendPacer| {
            pacer
                .next_send_at
                .unwrap()
                .checked_duration_since(Instant::now())
                .unwrap_or_default()
        };

        pacer.mark_sent_bytes(realtime_bytes, SendPace::Realtime);
        let realtime_wait = remaining(&pacer);
        pacer.mark_sent_bytes(realtime_bytes, SendPace::CatchUp);
        let catch_up_wait = remaining(&pacer);
        pacer.mark_sent_bytes(realtime_bytes, SendPace::TailFlush);
        let tail_wait = remaining(&pacer);

        assert!(realtime_wait > catch_up_wait);
        assert!(catch_up_wait > tail_wait);
        // 追赶间隔仍不低于豆包建议区间下限 100ms。
        assert!(catch_up_wait >= Duration::from_millis(90));
    }

    #[test]
    fn final_wait_uses_default_response_poll_timeout_after_audio_finished() {
        let pacer = AudioSendPacer {
            next_send_at: Some(Instant::now() - Duration::from_millis(1)),
        };

        assert_eq!(
            pacer.response_poll_timeout(RESPONSE_POLL_TIMEOUT),
            Duration::from_millis(1)
        );
        assert_eq!(
            websocket_response_poll_timeout(true, &pacer, RESPONSE_POLL_TIMEOUT),
            RESPONSE_POLL_TIMEOUT
        );
    }
}
