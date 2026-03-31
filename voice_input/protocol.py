import gzip
import json
import struct
from dataclasses import dataclass
from typing import Any


class ProtocolVersion:
    V1 = 0b0001


class MessageType:
    CLIENT_FULL_REQUEST = 0b0001
    CLIENT_AUDIO_ONLY_REQUEST = 0b0010
    SERVER_FULL_RESPONSE = 0b1001
    SERVER_ERROR_RESPONSE = 0b1111


class MessageTypeSpecificFlags:
    NO_SEQUENCE = 0b0000
    POS_SEQUENCE = 0b0001
    NEG_SEQUENCE = 0b0010
    NEG_WITH_SEQUENCE = 0b0011


class SerializationType:
    JSON = 0b0001


class CompressionType:
    GZIP = 0b0001


def gzip_compress(data: bytes) -> bytes:
    return gzip.compress(data)


def gzip_decompress(data: bytes) -> bytes:
    return gzip.decompress(data)


def build_header(message_type: int, flags: int) -> bytes:
    header = bytearray()
    header.append((ProtocolVersion.V1 << 4) | 1)
    header.append((message_type << 4) | flags)
    header.append((SerializationType.JSON << 4) | CompressionType.GZIP)
    header.append(0x00)
    return bytes(header)


def build_full_request(payload: dict[str, Any], seq: int) -> bytes:
    payload_bytes = gzip_compress(json.dumps(payload, ensure_ascii=False).encode("utf-8"))
    request = bytearray()
    request.extend(build_header(MessageType.CLIENT_FULL_REQUEST, MessageTypeSpecificFlags.POS_SEQUENCE))
    request.extend(struct.pack(">i", seq))
    request.extend(struct.pack(">I", len(payload_bytes)))
    request.extend(payload_bytes)
    return bytes(request)


def build_audio_request(seq: int, audio_chunk: bytes, is_last: bool) -> bytes:
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
