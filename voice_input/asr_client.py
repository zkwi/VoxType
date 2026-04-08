import asyncio
import json
import logging
import threading
import uuid
from collections.abc import Callable
from typing import Any

import aiohttp

from voice_input.protocol import build_audio_request, build_full_request, parse_response


logger = logging.getLogger(__name__)


class DoubaoAsrClient:
    """流式语音识别客户端（基于豆包 ASR 服务）。

    通过 WebSocket 将麦克风音频流发送到豆包 ASR 服务，
    实时接收识别结果（一遍流式 + 二遍精修）。
    """

    def __init__(self, config: dict[str, Any]) -> None:
        self.config = config
        self.seq = 1  # WebSocket 消息序号，每发一包递增

    def _build_context_payload(self) -> str | None:
        context_config = self.config.get("context", {})
        hotwords = [word for word in context_config.get("hotwords", []) if str(word).strip()]
        prompt_context = context_config.get("prompt_context") or []
        recent_context = context_config.get("recent_context") or []
        image_url = context_config.get("image_url")

        payload: dict[str, Any] = {}
        if hotwords:
            payload["hotwords"] = [{"word": word} for word in hotwords]

        context_data: list[dict[str, Any]] = []
        for item in recent_context:
            text = str(item.get("text", "")).strip()
            if text:
                context_data.append({"text": text})
        if image_url:
            context_data.append({"image_url": image_url})
        for item in prompt_context:
            text = str(item.get("text", "")).strip()
            if text:
                context_data.append({"text": text})

        if context_data:
            # 文档要求 context_data 按从新到旧排列，最多 20 条。
            payload["context_type"] = "dialog_ctx"
            payload["context_data"] = context_data[:20]
            logger.debug(
                "发送豆包上下文: recent=%s, prompt=%s, total=%s",
                len(recent_context),
                len(prompt_context),
                len(payload["context_data"]),
            )

        if not payload:
            return None
        return json.dumps(payload, ensure_ascii=False)

    def _build_request_payload(self) -> dict[str, Any]:
        request_config = self.config["request"]
        audio_config = self.config["audio"]
        context_payload = self._build_context_payload()

        request_payload: dict[str, Any] = {
            "model_name": request_config.get("model_name", "bigmodel"),
            "enable_nonstream": request_config.get("enable_nonstream", True),
            "enable_itn": request_config.get("enable_itn", True),
            "enable_punc": request_config.get("enable_punc", True),
            "enable_ddc": request_config.get("enable_ddc", False),
            "show_utterances": request_config.get("show_utterances", True),
            "result_type": request_config.get("result_type", "full"),
        }
        for key in [
            "enable_accelerate_text",
            "accelerate_score",
            "end_window_size",
            "force_to_speech_time",
        ]:
            value = request_config.get(key)
            if value is not None:
                request_payload[key] = value
        if context_payload:
            request_payload["corpus"] = {"context": context_payload}

        return {
            "user": {"uid": "desktop-input"},
            "audio": {
                "format": "pcm",
                "codec": "raw",
                "rate": audio_config.get("sample_rate", 16000),
                "bits": 16,
                "channel": audio_config.get("channels", 1),
            },
            "request": request_payload,
        }

    def _build_headers(self) -> dict[str, str]:
        auth = self.config["auth"]
        return {
            "X-Api-App-Key": auth["app_key"],
            "X-Api-Access-Key": auth["access_key"],
            "X-Api-Resource-Id": auth.get("resource_id", "volc.seedasr.sauc.duration"),
            "X-Api-Connect-Id": str(uuid.uuid4()),
        }

    @staticmethod
    def _extract_display_text(payload_msg: dict[str, Any] | None) -> str:
        if not payload_msg:
            return ""
        result = payload_msg.get("result") or {}
        if isinstance(result, dict):
            return str(result.get("text") or "")
        return ""

    @staticmethod
    def _extract_definite_segments(payload_msg: dict[str, Any] | None) -> list[dict[str, Any]]:
        if not payload_msg:
            return []
        result = payload_msg.get("result") or {}
        if not isinstance(result, dict):
            return []
        utterances = result.get("utterances") or []
        if not isinstance(utterances, list):
            return []

        definite_segments: list[dict[str, Any]] = []
        for item in utterances:
            if not isinstance(item, dict):
                continue
            if not item.get("definite"):
                continue
            text = str(item.get("text") or "").strip()
            if not text:
                continue
            definite_segments.append(
                {
                    "start_time": int(item.get("start_time") or 0),
                    "end_time": int(item.get("end_time") or 0),
                    "text": text,
                }
            )
        return definite_segments

    @staticmethod
    def _post_process_final_text(text: str) -> str:
        result = text.strip()
        if result.endswith("。") or result.endswith("."):
            result = result[:-1].rstrip()
        return result

    async def run(
        self,
        audio_chunks,
        on_partial: Callable[[str], None],
        stop_event: threading.Event | None = None,
        final_result_timeout_seconds: float | None = None,
    ) -> str:
        """执行一次完整的语音识别会话。

        同时运行 sender（发音频）和 receiver（收结果）两个协程。
        sender 在 stop_event 触发或音频耗尽后发送结束包；
        receiver 持续接收，直到收到 is_last_package 或超时。
        最终优先返回二遍 definite 结果（更准确）。
        """
        final_text = ""
        definitive_text = ""
        definitive_end_time = -1
        self.seq = 1
        ws_url = self.config["request"]["ws_url"]
        if final_result_timeout_seconds is None:
            final_result_timeout_seconds = float(
                self.config["request"].get("final_result_timeout_seconds", 15)
            )

        timeout = aiohttp.ClientTimeout(total=None, connect=30, sock_read=None)
        async with aiohttp.ClientSession(timeout=timeout) as session:
            async with session.ws_connect(ws_url, headers=self._build_headers()) as ws:
                await ws.send_bytes(build_full_request(self._build_request_payload(), self.seq))
                self.seq += 1

                initial_msg = await ws.receive()
                if initial_msg.type == aiohttp.WSMsgType.BINARY:
                    parse_response(initial_msg.data)

                async def sender() -> None:
                    """持续读取麦克风音频块并发送到 WebSocket。"""
                    while True:
                        if stop_event and stop_event.is_set():
                            break
                        chunk = await asyncio.to_thread(next, audio_chunks, None)
                        if chunk is None:
                            break
                        await ws.send_bytes(build_audio_request(self.seq, chunk, is_last=False))
                        self.seq += 1
                    await ws.send_bytes(build_audio_request(self.seq, b"", is_last=True))

                async def receiver() -> None:
                    """接收并解析服务端返回的识别结果，跟踪 definite 片段。"""
                    nonlocal final_text, definitive_text, definitive_end_time
                    waiting_for_final_after_stop = False
                    final_deadline: float | None = None
                    while True:
                        try:
                            message = await asyncio.wait_for(ws.receive(), timeout=1.0)
                        except TimeoutError:
                            if stop_event and stop_event.is_set() and not waiting_for_final_after_stop:
                                waiting_for_final_after_stop = True
                                final_deadline = asyncio.get_running_loop().time() + final_result_timeout_seconds
                            if waiting_for_final_after_stop and final_deadline is not None:
                                if asyncio.get_running_loop().time() >= final_deadline:
                                    logger.warning("等待二遍最终结果超时，结束本次识别")
                                    break
                            continue

                        if message.type == aiohttp.WSMsgType.CLOSED:
                            break
                        if message.type == aiohttp.WSMsgType.ERROR:
                            raise RuntimeError("豆包 websocket 连接异常")
                        if message.type != aiohttp.WSMsgType.BINARY:
                            continue

                        parsed = parse_response(message.data)
                        if parsed.code and parsed.code != 20000000:
                            raise RuntimeError(f"豆包识别失败，错误码: {parsed.code}")

                        text = self._extract_display_text(parsed.payload_msg)
                        if text:
                            final_text = text
                            on_partial(text)

                        definite_segments = self._extract_definite_segments(parsed.payload_msg)
                        if definite_segments:
                            merged_segments: list[str] = []
                            for item in definite_segments:
                                end_time = item["end_time"]
                                if end_time <= definitive_end_time:
                                    continue
                                definitive_end_time = end_time
                                merged_segments.append(item["text"])
                            if merged_segments:
                                definitive_text += "".join(merged_segments)

                        if parsed.is_last_package:
                            break

                await asyncio.gather(sender(), receiver())

        if definitive_text.strip():
            return self._post_process_final_text(definitive_text)
        logger.warning("未提取到二遍 definite 结果，本次不输出最终文本")
        return ""
