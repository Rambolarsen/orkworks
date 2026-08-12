import type { HarnessConfig } from "./harnessTypes.ts";

export interface NewSessionDraft {
  harnessId: string;
  model: string;
}

export function selectableHarnesses(harnesses: HarnessConfig[]): HarnessConfig[] {
  return harnesses.filter((harness) => !harness.retired);
}

export function activeNewSessionHarnesses(
  harnesses: HarnessConfig[],
  activeHarnessIds: string[],
): HarnessConfig[] {
  if (activeHarnessIds.length === 0) {
    return harnesses.filter((harness) => harness.id === "generic-shell");
  }

  const selectable = selectableHarnesses(harnesses);
  const active = selectable.filter(
    (harness) => harness.id === "generic-shell" || activeHarnessIds.includes(harness.id),
  );
  const hasRetiredSelection = activeHarnessIds.some(
    (id) => harnesses.some((harness) => harness.id === id && harness.retired),
  );
  const antigravity = selectable.find((harness) => harness.id === "antigravity");
  return hasRetiredSelection && antigravity && !active.includes(antigravity)
    ? [antigravity, ...active]
    : active;
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
