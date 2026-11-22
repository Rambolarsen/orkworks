// "Soft graphite" terminal palette — a lifted graphite base, muted ANSI hues
// and the ork-lime cursor, so the terminal reads as part of the app rather
// than a raw black box. The base colors mirror the --term-* tokens in
// styles/tokens.css; bright* variants are slightly lighter takes on the same
// hues. Terminals stay dark in both app themes, so these values are
// theme-independent.
export const orkworksTerminalTheme = {
  background: "#111319",
  foreground: "#c9ced6",
  cursor: "#9dc520",
  cursorAccent: "#111319",
  selectionBackground: "rgba(130, 170, 240, 0.22)",
  black: "#2b303a",
  red: "#e8837c",
  green: "#8fcf9c",
  yellow: "#e6cc82",
  blue: "#82aaf0",
  magenta: "#c99bea",
  cyan: "#7fcfd6",
  white: "#dfe3e8",
  brightBlack: "#6b7280",
  brightRed: "#f09a94",
  brightGreen: "#a6dcb1",
  brightYellow: "#eed89d",
  brightBlue: "#9cbcf4",
  brightMagenta: "#d6b0f0",
  brightCyan: "#9adbe0",
  brightWhite: "#f4f6f8",
} as const;

// Shared look for live and historical terminals.
export const orkworksTerminalOptions = {
  theme: orkworksTerminalTheme,
  fontFamily: "'SF Mono', 'SFMono-Regular', ui-monospace, 'Menlo', 'Monaco', monospace",
  fontSize: 13,
  lineHeight: 1.2,
  cursorStyle: "bar",
  cursorWidth: 2,
} as const;
