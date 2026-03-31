import queue
import threading
import time

import sounddevice as sd


class MicrophoneRecorder:
    """麦克风录音器，使用 sounddevice 回调方式采集 PCM 音频。

    通过 iter_chunks() 迭代获取音频块，供 ASR 客户端消费。
    """
    def __init__(
        self,
        sample_rate: int,
        channels: int,
        max_record_seconds: int,
        input_device: int | None = None,
        block_ms: int = 200,
    ) -> None:
        self.sample_rate = sample_rate
        self.channels = channels
        self.max_record_seconds = max_record_seconds
        self.input_device = input_device
        self.block_ms = block_ms
        self.blocksize = max(1, int(sample_rate * block_ms / 1000))
        self._queue: queue.Queue[bytes | None] = queue.Queue()
        self._stop_event = threading.Event()
        self._stream: sd.RawInputStream | None = None
        self.started_at = 0.0

    def start(self) -> None:
        self.started_at = time.time()
        self._stop_event.clear()
        self._queue = queue.Queue()

        def callback(indata, frames, time_info, status) -> None:
            if self._stop_event.is_set():
                return
            if time.time() - self.started_at >= self.max_record_seconds:
                self._stop_event.set()
                return
            self._queue.put(bytes(indata))

        self._stream = sd.RawInputStream(
            samplerate=self.sample_rate,
            channels=self.channels,
            dtype="int16",
            blocksize=self.blocksize,
            device=self.input_device,
            callback=callback,
        )
        self._stream.start()

    def stop(self) -> None:
        self._stop_event.set()
        if self._stream is not None:
            self._stream.stop()
            self._stream.close()
            self._stream = None
        self._queue.put(None)

    def iter_chunks(self):
        while True:
            chunk = self._queue.get()
            if chunk is None:
                break
            yield chunk

    @property
    def reached_limit(self) -> bool:
        if not self.started_at:
            return False
        return (time.time() - self.started_at) >= self.max_record_seconds
