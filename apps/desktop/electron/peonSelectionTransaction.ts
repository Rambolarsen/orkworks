import { peonSelectionMatchesAppliedState, type PeonAppliedState, type PeonProviderVerificationResponse, type PeonSelection, type ProviderId } from "./providerTypes.ts";
import { normalizeProviderSettings } from "./settingsMemory.ts";

export function normalizePeonSelectionInput(value: unknown, fallbackOllamaBaseUrl?: string): PeonSelection {
  const candidate = value && typeof value === "object" && !Array.isArray(value)
    ? value as Record<string, unknown>
    : undefined;
  const selection = candidate && !Array.isArray(candidate)
    && candidate.provider === "ollama" && candidate.ollamaBaseUrl == null && fallbackOllamaBaseUrl
    ? { ...candidate, ollamaBaseUrl: fallbackOllamaBaseUrl }
    : candidate;
  const normalized = normalizeProviderSettings({
    version: 2,
    revision: 0,
    peonSelection: selection,
    providers: [],
  }).peonSelection;
  if (!normalized) throw new Error("Invalid Peon provider selection.");
  return normalized;
}

export interface PeonVerificationRequest {
  provider: ProviderId;
  ollamaBaseUrl?: string;
  generation: number;
  readyPort?: number;
  signal?: AbortSignal;
}

export interface PeonApplyRequest {
  selection: PeonSelection;
  generation: number;
  readyPort?: number;
  signal?: AbortSignal;
}

export interface PeonSelectionTransport {
  verify(request: PeonVerificationRequest): Promise<PeonProviderVerificationResponse>;
  apply(request: PeonApplyRequest): Promise<PeonAppliedState>;
  getApplied(signal?: AbortSignal): Promise<PeonAppliedState>;
}

export type PeonSelectionSaveResult =
  | { ok: true }
  | { ok: false; error: string };

interface VerifiedSelection {
  generation: number;
  provider: ProviderId;
  ollamaBaseUrl: string | null;
}

interface AppliedSelection {
  generation: number;
  state: PeonAppliedState;
}

export function createPeonSelectionTransaction(transport: PeonSelectionTransport) {
  let generation = 0;
  let verified: VerifiedSelection | null = null;
  let applied: AppliedSelection | null = null;
  let mutationTail: Promise<void> = Promise.resolve();

  function enqueueMutation<T>(operation: () => Promise<T>): Promise<T> {
    const result = mutationTail.then(operation, operation);
    mutationTail = result.then(() => undefined, () => undefined);
    return result;
  }

  function nextGeneration(): number {
    generation += 1;
    verified = null;
    applied = null;
    return generation;
  }

  function matchesVerified(selection: PeonSelection, candidate: VerifiedSelection | null): boolean {
    return candidate !== null
      && candidate.provider === selection.provider
      && (selection.provider !== "ollama" || candidate.ollamaBaseUrl === selection.ollamaBaseUrl);
  }

  async function verify(
    provider: ProviderId,
    ollamaBaseUrl?: string,
    signal?: AbortSignal,
    readyPort?: number,
  ): Promise<PeonProviderVerificationResponse> {
    const requestGeneration = nextGeneration();
    const result = await transport.verify({ provider, ollamaBaseUrl, generation: requestGeneration, readyPort, signal });
    if (requestGeneration !== generation || result.generation !== requestGeneration) {
      throw new Error("Peon provider verification was superseded.");
    }
    verified = {
      generation: requestGeneration,
      provider: result.provider,
      ollamaBaseUrl: result.ollamaBaseUrl,
    };
    return result;
  }

  async function discover(provider: ProviderId, ollamaBaseUrl?: string): Promise<string[]> {
    const result = await verify(provider, ollamaBaseUrl);
    if (generation !== result.generation) throw new Error("Peon model discovery was superseded.");
    verified = null;
    return result.models;
  }

  async function apply(selection: PeonSelection, signal?: AbortSignal, readyPort?: number): Promise<PeonAppliedState> {
    return enqueueMutation(async () => {
      const candidate = verified;
      if (!matchesVerified(selection, candidate)) {
        throw new Error("A matching successful Peon provider verification is required before Apply.");
      }
      const result = await transport.apply({ selection, generation: candidate!.generation, readyPort, signal });
      if (!matchesVerified(selection, verified)) {
        throw new Error("The Peon provider Apply was superseded.");
      }
      if (!peonSelectionMatchesAppliedState(selection, result)) {
        throw new Error("The sidecar applied a different Peon selection.");
      }
      applied = { generation: candidate!.generation, state: result };
      return result;
    });
  }

  async function syncPersistedSelection(
    selection: PeonSelection,
    signal?: AbortSignal,
    readyPort?: number,
  ): Promise<PeonAppliedState> {
    await verify(selection.provider, selection.ollamaBaseUrl, signal, readyPort);
    return apply(selection, signal, readyPort);
  }

  async function save(
    selection: PeonSelection,
    persist: () => void | Promise<void>,
  ): Promise<PeonSelectionSaveResult> {
    return enqueueMutation(async () => {
      const candidate = verified;
      const localApplied = applied;
      if (!matchesVerified(selection, candidate)
        || localApplied === null
        || localApplied.generation !== candidate!.generation
        || !peonSelectionMatchesAppliedState(selection, localApplied.state)) {
        return { ok: false, error: "Save requires a matching successful Apply." };
      }
      let current: PeonAppliedState;
      try {
        current = await transport.getApplied();
      } catch (error) {
        return { ok: false, error: error instanceof Error ? error.message : "Couldn't confirm the applied Peon provider." };
      }
      if (!matchesVerified(selection, verified)
        || applied !== localApplied
        || localApplied.generation !== verified!.generation
        || !peonSelectionMatchesAppliedState(selection, localApplied.state)) {
        return { ok: false, error: "Save was superseded by a newer Peon selection." };
      }
      if (!peonSelectionMatchesAppliedState(selection, current)) {
        return { ok: false, error: "Save requires the sidecar's applied provider, model, and URL to match." };
      }
      await persist();
      return { ok: true };
    });
  }

  async function getApplied(signal?: AbortSignal): Promise<PeonAppliedState> {
    return transport.getApplied(signal);
  }

  return { verify, discover, apply, syncPersistedSelection, save, getApplied };
}

export type PeonSelectionTransaction = ReturnType<typeof createPeonSelectionTransaction>;
