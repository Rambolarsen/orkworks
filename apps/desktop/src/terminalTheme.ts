// "Phosphor" terminal palette — cool-graphite base with a lime cursor.
// The base colors mirror the --term-* tokens in styles/tokens.css; the
// bright* variants are derived here (xterm needs a full 16-color set, so
// they reuse the normal hues with a brighter white). Terminals stay dark
// in both app themes, so these values are theme-independent.
export const orkworksTerminalTheme = {
  background: "#0c0d10",
  foreground: "#d4d8de",
  cursor: "#9dc520",
  cursorAccent: "#0c0d10",
  selectionBackground: "#243a52",
  black: "#000000",
  red: "#ff6b63",
  green: "#66e08a",
  yellow: "#f3d35a",
  blue: "#66b3ff",
  magenta: "#ff79c6",
  cyan: "#8be9fd",
  white: "#f1f1f0",
  brightBlack: "#686868",
  brightRed: "#ff6b63",
  brightGreen: "#66e08a",
  brightYellow: "#f3d35a",
  brightBlue: "#66b3ff",
  brightMagenta: "#ff79c6",
  brightCyan: "#8be9fd",
  brightWhite: "#ffffff",
} as const;
