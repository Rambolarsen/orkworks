import type { SessionInfo } from "./api.ts";
import type { ProviderEffectiveState } from "./providerTypes.ts";

export function sessionProviderContext(session: SessionInfo): {
  codingTool: string;
  modelProvider: string;
  model: string;
  providerState: ProviderEffectiveState;
} {
  return {
    codingTool: session.harness ?? session.harnessId ?? "Unknown",
    modelProvider: session.provider ?? session.modelProviderId ?? "Unknown",
    model: session.model ?? session.modelId ?? "Unknown",
    providerState: session.providerState ?? "unknown",
  };
}
