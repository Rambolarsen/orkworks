import type { HarnessConfig } from "./harnessTypes.ts";

export interface NewSessionDraft {
  harnessId: string;
  model: string;
}

export function syncDraftWithHarnesses(
  draft: NewSessionDraft,
  harnesses: HarnessConfig[],
  savedDraft?: NewSessionDraft | null,
): NewSessionDraft {
  if (harnesses.length === 0) {
    return draft;
  }

  if (harnesses.some((harness) => harness.id === draft.harnessId)) {
    return draft;
  }

  if (savedDraft && harnesses.some((harness) => harness.id === savedDraft.harnessId)) {
    return savedDraft;
  }

  const fallback = harnesses[0];
  return {
    harnessId: fallback.id,
    model: draft.model || fallback.defaultModel,
  };
}

export function canStartNewSession(harnesses: HarnessConfig[], harnessId: string): boolean {
  return harnesses.length === 0 || harnesses.some((harness) => harness.id === harnessId);
}
