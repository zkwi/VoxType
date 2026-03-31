"""
豆包 ASR WebSocket 二进制协议的编解码。

协议格式（每个消息）：
  [4字节 header][4字节 sequence (可选)][4字节 payload_size (可选)][payload]

Header 各字节含义：
  byte0: 高4位=协议版本(1), 低4位=header大小(单位4字节, 固定1=4字节)
  byte1: 高4位=消息类型, 低4位=标志位(正序号/负序号等)
  byte2: 高4位=序列化类型(JSON=1), 低4位=压缩类型(GZIP=1)
  byte3: 保留(0x00)

参考: 豆包语音识别流式 API 文档
"""

import gzip
import json
import struct
from dataclasses import dataclass
from typing import Any


class ProtocolVersion:
    V1 = 0b0001


class MessageType:
    CLIENT_FULL_REQUEST = 0b0001     # 客户端首包（携带完整配置 + 音频参数）
    CLIENT_AUDIO_ONLY_REQUEST = 0b0010  # 客户端后续音频包
    SERVER_FULL_RESPONSE = 0b1001    # 服务端识别结果
    SERVER_ERROR_RESPONSE = 0b1111   # 服务端错误


class MessageTypeSpecificFlags:
    NO_SEQUENCE = 0b0000
    POS_SEQUENCE = 0b0001        # 正序号，表示还有后续数据
    NEG_SEQUENCE = 0b0010        # 负序号标记
    NEG_WITH_SEQUENCE = 0b0011   # 负序号 + 序号值，表示最后一包


class SerializationType:
    JSON = 0b0001


class CompressionType:
    GZIP = 0b0001


def gzip_compress(data: bytes) -> bytes:
    return gzip.compress(data)


def gzip_decompress(data: bytes) -> bytes:
    return gzip.decompress(data)


def build_header(message_type: int, flags: int) -> bytes:
    """构建 4 字节协议头。"""
    header = bytearray()
    header.append((ProtocolVersion.V1 << 4) | 1)  # 版本 + header 大小
    header.append((message_type << 4) | flags)
    header.append((SerializationType.JSON << 4) | CompressionType.GZIP)
    header.append(0x00)
    return bytes(header)


def build_full_request(payload: dict[str, Any], seq: int) -> bytes:
    """构建首包请求：包含音频格式、模型参数、热词/上下文等完整配置。"""
    payload_bytes = gzip_compress(json.dumps(payload, ensure_ascii=False).encode("utf-8"))
    request = bytearray()
    request.extend(build_header(MessageType.CLIENT_FULL_REQUEST, MessageTypeSpecificFlags.POS_SEQUENCE))
    request.extend(struct.pack(">i", seq))
    request.extend(struct.pack(">I", len(payload_bytes)))
    request.extend(payload_bytes)
    return bytes(request)


def build_audio_request(seq: int, audio_chunk: bytes, is_last: bool) -> bytes:
    """构建音频数据包。is_last=True 时发送负序号，通知服务端音频结束。"""
    flags = MessageTypeSpecificFlags.NEG_WITH_SEQUENCE if is_last else MessageTypeSpecificFlags.POS_SEQUENCE
    signed_seq = -seq if is_last else seq
    payload = gzip_compress(audio_chunk)

    request = bytearray()
    request.extend(build_header(MessageType.CLIENT_AUDIO_ONLY_REQUEST, flags))
    request.extend(struct.pack(">i", signed_seq))
    request.extend(struct.pack(">I", len(payload)))
    request.extend(payload)
    return bytes(request)


@dataclass
class ParsedResponse:
    code: int = 0
    is_last_package: bool = False
    sequence: int = 0
    payload_size: int = 0
    payload_msg: dict[str, Any] | None = None


def parse_response(message: bytes) -> ParsedResponse:
    """解析服务端返回的二进制消息，提取识别结果或错误码。"""
    response = ParsedResponse()
    header_size = message[0] & 0x0F
    message_type = message[1] >> 4
    flags = message[1] & 0x0F
    serialization = message[2] >> 4
    compression = message[2] & 0x0F
    payload = message[header_size * 4 :]

    if flags & 0x01:
        response.sequence = struct.unpack(">i", payload[:4])[0]
        payload = payload[4:]
    if flags & 0x02:
        response.is_last_package = True

    if message_type == MessageType.SERVER_FULL_RESPONSE:
        response.payload_size = struct.unpack(">I", payload[:4])[0]
        payload = payload[4:]
    elif message_type == MessageType.SERVER_ERROR_RESPONSE:
        response.code = struct.unpack(">i", payload[:4])[0]
        response.payload_size = struct.unpack(">I", payload[4:8])[0]
        payload = payload[8:]

    if not payload:
        return response
    if compression == CompressionType.GZIP:
        payload = gzip_decompress(payload)
    if serialization == SerializationType.JSON:
        response.payload_msg = json.loads(payload.decode("utf-8"))
    return response
