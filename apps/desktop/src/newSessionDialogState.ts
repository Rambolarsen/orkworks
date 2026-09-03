import type { HarnessConfig } from "./harnessTypes.ts";

export interface NewSessionDraft {
  harnessId: string;
  model: string;
}

export function selectableHarnesses<T extends HarnessConfig>(harnesses: T[]): T[] {
  return harnesses.filter((harness) => !harness.retired);
}

export function normalizeActiveHarnessIds(
  harnesses: HarnessConfig[],
  activeHarnessIds: string[],
): string[] {
  const antigravityAvailable = selectableHarnesses(harnesses).some(
    (harness) => harness.id === "antigravity",
  );
  return activeHarnessIds.flatMap((id) => {
    const harness = harnesses.find((entry) => entry.id === id);
    if (!harness?.retired) return [id];
    return id === "gemini" && antigravityAvailable ? ["antigravity"] : [];
  });
}

export function activeNewSessionHarnesses(
  harnesses: HarnessConfig[],
  activeHarnessIds: string[],
): HarnessConfig[] {
  if (activeHarnessIds.length === 0) {
    return harnesses.filter((harness) => harness.id === "generic-shell");
  }

  const selectable = selectableHarnesses(harnesses);
  const normalizedActiveHarnessIds = normalizeActiveHarnessIds(harnesses, activeHarnessIds);
  const active = selectable.filter(
    (harness) => harness.id === "generic-shell" || normalizedActiveHarnessIds.includes(harness.id),
  );
  return active;
}

export function syncDraftWithHarnesses(
  draft: NewSessionDraft,
  harnesses: HarnessConfig[],
  savedDraft?: NewSessionDraft | null,
): NewSessionDraft {
  const selectable = selectableHarnesses(harnesses);
  if (selectable.length === 0) {
    return draft;
  }

  if (selectable.some((harness) => harness.id === draft.harnessId)) {
    return draft;
  }

  if (savedDraft && selectable.some((harness) => harness.id === savedDraft.harnessId)) {
    return savedDraft;
  }

  const fallback = selectable[0];
  return {
    harnessId: fallback.id,
    model: harnesses.some((harness) => harness.id === draft.harnessId && harness.retired)
      ? fallback.defaultModel || ""
      : draft.model || fallback.defaultModel || "",
  };
}

export function canStartNewSession(harnesses: HarnessConfig[], harnessId: string): boolean {
  const selectable = selectableHarnesses(harnesses);
  return selectable.length === 0 || selectable.some((harness) => harness.id === harnessId);
}
