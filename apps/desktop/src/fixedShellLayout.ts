import type { ShellPreferences } from "./orkworksWindow";

export const SHELL_SEPARATOR_WIDTH = 6;
const clamp = (value: number, min: number, max: number) => Math.max(min, Math.min(max, value));

/** Width is measured in CSS pixels, so Electron zoom naturally uses the same breakpoints. */
export function computeShellLayout(width: number, preferences: ShellPreferences, inspectorOpen: boolean) {
  const mode = width < 860 ? "compact" : width < 1180 ? "medium" : "wide";
  const sessionsVisible = mode !== "compact" && preferences.sessionsVisible;
  let sessionsWidth = sessionsVisible ? clamp(preferences.sessionsWidth, 200, Math.min(320, width - 566)) : 0;
  let inspectorWidth = inspectorOpen && mode === "wide" ? clamp(preferences.inspectorWidth, 280, 420) : 0;
  if (inspectorWidth) {
    const separators = SHELL_SEPARATOR_WIDTH * (sessionsVisible ? 2 : 1);
    const budget = width - 560 - separators;
    inspectorWidth = Math.max(280, Math.min(inspectorWidth, budget - sessionsWidth));
    if (sessionsVisible) sessionsWidth = Math.max(200, Math.min(sessionsWidth, budget - inspectorWidth));
    if (sessionsWidth + inspectorWidth > budget) inspectorWidth = 0;
  }
  const separators = SHELL_SEPARATOR_WIDTH * (sessionsVisible ? 2 : 1);
  const sessionsMaximum = Math.min(320, width - 560 - SHELL_SEPARATOR_WIDTH - (inspectorWidth ? inspectorWidth + SHELL_SEPARATOR_WIDTH : 0));
  const inspectorMaximum = Math.min(420, width - 560 - separators - sessionsWidth);
  return { mode, sessionsVisible, sessionsWidth, inspectorWidth, sessionsMaximum, inspectorMaximum,
    inspectorPage: inspectorOpen && inspectorWidth === 0 };
}
