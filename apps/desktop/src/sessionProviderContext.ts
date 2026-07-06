import type { SessionInfo } from "./api.ts";
import type { ProviderEffectiveState } from "./providerTypes.ts";

/**
 * The one place that knows which session fields name the coding tool and in
 * what order — sessions may carry `harness` or `harnessId` depending on the
 * payload, both holding the backend harness id.
 */
export function sessionCodingTool(session: SessionInfo): string | undefined {
  return session.harness ?? session.harnessId;
}

/**
 * Resolve a harness id to its registry display name ("gemini" → "Gemini CLI")
 * for user-facing surfaces — raw ids must never reach the UI. Unknown ids
 * pass through unchanged so custom harnesses still show something.
 */
export function harnessDisplayName(
  harnesses: readonly { id: string; name: string }[],
  idOrName: string | undefined,
): string | undefined {
  if (!idOrName) return undefined;
  return harnesses.find((h) => h.id === idOrName)?.name ?? idOrName;
}

export function sessionProviderContext(session: SessionInfo): {
  codingTool: string;
  modelProvider: string;
  model: string;
  providerState: ProviderEffectiveState;
} {
  return {
    codingTool: sessionCodingTool(session) ?? "Unknown",
    modelProvider: session.provider ?? session.modelProviderId ?? "Unknown",
    model: session.model ?? session.modelId ?? "Unknown",
    providerState: session.providerState ?? "unknown",
  };
}
