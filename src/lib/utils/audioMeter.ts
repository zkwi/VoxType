export function clampAudioLevel(value: number) {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(1, value));
}

export function micBarHeight(recording: boolean, audioLevel: number, index: number) {
  const level = recording ? clampAudioLevel(audioLevel * 3.2) : 0;
  const quietHeights = [5, 7, 9, 11, 9, 7];
  const activeHeights = [7, 11, 15, 19, 16, 12];
  const threshold = 0.08 + index * 0.105;
  const target = level >= threshold ? activeHeights[index] : quietHeights[index];
  return `${target}px`;
}

export function micBarOpacity(recording: boolean, audioLevel: number, index: number) {
  if (!recording) return "0.45";
  const level = clampAudioLevel(audioLevel * 3.2);
  return level >= 0.08 + index * 0.105 ? "1" : "0.38";
}
