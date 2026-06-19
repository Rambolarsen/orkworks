const modifierKeys = new Set(["Meta", "Control", "Alt", "Shift"]);

const keyNameMap: Record<string, string> = {
  " ": "Space",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  Escape: "Esc",
};

export function acceleratorFromKeyboardEvent(event: KeyboardEvent): string | null {
  if (modifierKeys.has(event.key)) {
    return null;
  }

  const key = normalizeKey(event.key);
  if (!key) {
    return null;
  }

  const parts: string[] = [];
  if (event.metaKey || event.ctrlKey) parts.push("CmdOrCtrl");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  parts.push(key);

  return parts.join("+");
}

function normalizeKey(key: string): string | null {
  const mapped = keyNameMap[key] ?? key;
  if (mapped.length === 1) {
    return mapped.toUpperCase();
  }
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(mapped)) {
    return mapped;
  }
  if (/^[A-Za-z]+$/.test(mapped)) {
    return mapped[0].toUpperCase() + mapped.slice(1);
  }
  return null;
}
