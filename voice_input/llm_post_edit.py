import logging
import asyncio
from typing import Any

from openai import OpenAI


logger = logging.getLogger(__name__)


class AliyunLlmPostEditor:
    """通过 OpenAI 兼容接口调用大模型，对 ASR 识别结果做轻度润色。

    默认使用阿里云百炼（DashScope），也可替换为任何 OpenAI 兼容的接口。
    润色失败时静默回退到原文，不影响正常使用。
    """

    def __init__(self, config: dict[str, Any]) -> None:
        self.config = config

    async def polish(
        self,
        text: str,
        hotwords: list[str] | None = None,
        prompt_contexts: list[str] | None = None,
    ) -> str:
        settings = self.config.get("llm_post_edit", {})
        if not settings.get("enabled", False):
            return text
        min_chars = int(settings.get("min_chars", 100) or 0)
        if len(text.strip()) < min_chars:
            return text

        api_key = str(settings.get("api_key", "")).strip()
        base_url = str(settings.get("base_url", "")).rstrip("/")
        model = str(settings.get("model", "")).strip()
        if not api_key or not base_url or not model:
            logger.warning("阿里云润色已启用，但配置不完整，跳过润色")
            return text

        system_prompt = str(settings.get("system_prompt", "")).strip()
        user_prompt_template = str(settings.get("user_prompt_template", "{text}"))
        hotword_block = ""
        if hotwords:
            hotword_block = "\n\n用户词典：\n" + "\n".join(hotwords)
        prompt_context_block = ""
        if prompt_contexts:
            prompt_context_block = "\n\n场景与偏好上下文：\n" + "\n".join(prompt_contexts)
        user_prompt = user_prompt_template.format(text=text) + hotword_block + prompt_context_block

        timeout_seconds = float(settings.get("timeout_seconds", 30))

        try:
            data = await asyncio.wait_for(
                asyncio.to_thread(
                    self._call_openai_compatible_api,
                    base_url=base_url,
                    api_key=api_key,
                    model=model,
                    system_prompt=system_prompt,
                    user_prompt=user_prompt,
                    enable_thinking=bool(settings.get("enable_thinking", False)),
                    timeout_seconds=timeout_seconds,
                ),
                timeout=timeout_seconds + 5,
            )
        except Exception as exc:
            logger.warning("阿里云润色失败，回退原文: %r", exc)
            return text

        content = data.choices[0].message.content if data.choices else ""
        polished = str(content).strip()
        return polished or text

    @staticmethod
    def _call_openai_compatible_api(
        *,
        base_url: str,
        api_key: str,
        model: str,
        system_prompt: str,
        user_prompt: str,
        enable_thinking: bool,
        timeout_seconds: float,
    ):
        client = OpenAI(
            api_key=api_key,
            base_url=base_url,
            timeout=timeout_seconds,
        )
        return client.chat.completions.create(
            model=model,
            messages=[
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt},
            ],
            extra_body={"enable_thinking": enable_thinking},
        )
