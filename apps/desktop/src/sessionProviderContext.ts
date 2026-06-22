import type { SessionInfo } from "./api.ts";
import type { ProviderEffectiveState } from "./providerTypes.ts";

export function sessionProviderContext(session: SessionInfo): {
  provider: string;
  model: string;
  state: ProviderEffectiveState;
} {
  return {
    provider: session.provider ?? "—",
    model: session.providerModel ?? "—",
    state: session.providerState ?? "unknown",
  };
}
