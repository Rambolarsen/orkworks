import type { SessionInfo } from "./api.ts";

export function sessionProviderContext(session: SessionInfo): {
  provider: string;
  model: string;
  state: string;
} {
  return {
    provider: session.provider ?? "—",
    model: session.providerModel ?? "—",
    state: session.providerState ?? "unknown",
  };
}
