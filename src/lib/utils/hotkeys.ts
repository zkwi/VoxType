export function formatHotkey(value: string) {
  return value
    .split("+")
    .map((part) => {
      const normalized = part.trim().toLowerCase();
      if (normalized === "ctrl" || normalized === "control") return "Ctrl";
      if (normalized === "alt") return "Alt";
      if (normalized === "shift") return "Shift";
      if (normalized === "win" || normalized === "meta") return "Win";
      if (normalized === "space") return "Space";
      if (normalized === "enter") return "Enter";
      if (normalized === "tab") return "Tab";
      return part.trim().toUpperCase();
    })
    .filter(Boolean)
    .join(" + ");
}

export function hotkeyFromKeyboardEvent(event: KeyboardEvent) {
  const key = normalizedHotkeyMainKey(event.key);
  if (!key) return "";
  const modifiers = [];
  if (event.ctrlKey) modifiers.push("Ctrl");
  if (event.altKey) modifiers.push("Alt");
  if (event.shiftKey) modifiers.push("Shift");
  if (event.metaKey) modifiers.push("Win");
  if (modifiers.length === 0) return "";
  return [...modifiers, key].join(" + ");
}

export function normalizedHotkeyMainKey(key: string) {
  if (/^[a-z]$/i.test(key)) return key.toUpperCase();
  if (/^[0-9]$/.test(key)) return key;
  if (/^F([1-9]|1[0-2])$/i.test(key)) return key.toUpperCase();
  if (key === " " || key.toLowerCase() === "space" || key === "Spacebar") return "Space";
  if (key.toLowerCase() === "tab") return "Tab";
  if (key.toLowerCase() === "enter") return "Enter";
  return "";
}

export function validateHotkeyText(
  value: string,
  messages: { required: string; needsModifier: string; unsupported: string },
) {
  const parts = value
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean);
  if (parts.length === 0) return messages.required;
  const modifiers = parts.slice(0, -1).map((part) => part.toLowerCase());
  const key = parts[parts.length - 1];
  const hasModifier = modifiers.some((part) => ["ctrl", "control", "alt", "shift", "win", "meta"].includes(part));
  if (!hasModifier) return messages.needsModifier;
  if (!normalizedHotkeyMainKey(key)) return messages.unsupported;
  return "";
}
