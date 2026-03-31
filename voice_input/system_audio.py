import logging
from dataclasses import dataclass

from pycaw.pycaw import AudioUtilities


logger = logging.getLogger(__name__)


@dataclass
class VolumeState:
    level_scalar: float
    muted: bool


class SystemVolumeController:
    def __init__(self) -> None:
        self._endpoint = None
        self._saved_state: VolumeState | None = None

    def _get_endpoint(self):
        if self._endpoint is None:
            devices = AudioUtilities.GetSpeakers()
            self._endpoint = devices.EndpointVolume
        return self._endpoint

    def mute_and_save(self) -> None:
        endpoint = self._get_endpoint()
        self._saved_state = VolumeState(
            level_scalar=float(endpoint.GetMasterVolumeLevelScalar()),
            muted=bool(endpoint.GetMute()),
        )
        endpoint.SetMute(1, None)

    def restore(self) -> None:
        if self._saved_state is None:
            return
        endpoint = self._get_endpoint()
        endpoint.SetMasterVolumeLevelScalar(self._saved_state.level_scalar, None)
        endpoint.SetMute(1 if self._saved_state.muted else 0, None)
        self._saved_state = None

    def safe_mute_and_save(self) -> bool:
        try:
            self.mute_and_save()
            return True
        except Exception as exc:
            logger.warning("系统静音失败: %s", exc)
            return False

    def safe_restore(self) -> None:
        try:
            self.restore()
        except Exception as exc:
            logger.warning("恢复系统音量失败: %s", exc)
