import { approveInferenceAdapter, readInferenceTrust, revokeInferenceAdapter, type TrustContext } from './inferenceTrust.ts';
import { parseTaskmasterModels, taskmasterAdviceRequest, taskmasterRequest, TaskmasterHttpError } from './taskmasterSettings.ts';

type AdviceResource = Parameters<typeof taskmasterAdviceRequest>[2];

export type TaskmasterAcceptOptions = {
  sessionId?: string;
  prompt?: string;
  packetRevision?: number;
  evidenceFingerprint?: string;
  idempotencyKey?: string;
};

export type TaskmasterIpcDependencies = {
  handle(name: string, handler: (event: unknown, ...args: unknown[]) => Promise<unknown>): void;
  withReady<T>(operation: (port: number, token: string, signal: AbortSignal) => Promise<T>): Promise<T>;
  workspacePath(): string | null;
  getGeneration(): number;
  getReadiness(): Promise<number>;
  getToken(): string;
  setKnowledgeUpdatesEnabled(enabled: boolean): void;
  getKnowledgeUpdateStatus(): unknown;
  refreshKnowledge(): void;
  enqueueSettingsWrite<T>(operation: () => Promise<T>): Promise<T>;
  confirmInferenceApproval(detail: string): Promise<boolean>;
  fetcher?: (url: string, init?: RequestInit) => Promise<Response>;
};

export function normalizeRecommendationAcceptOptions(value: unknown): TaskmasterAcceptOptions {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error('Invalid recommendation handoff.');
  }
  const input = value as {
    sessionId?: unknown;
    prompt?: unknown;
    packetRevision?: unknown;
    evidenceFingerprint?: unknown;
    idempotencyKey?: unknown;
  };
  if (input.sessionId !== undefined && (typeof input.sessionId !== 'string' || !input.sessionId)) {
    throw new Error('Invalid recommendation handoff session.');
  }
  if (input.prompt !== undefined && typeof input.prompt !== 'string') {
    throw new Error('Invalid recommendation handoff prompt.');
  }
  if (input.packetRevision !== undefined
    && (!Number.isInteger(input.packetRevision) || (input.packetRevision as number) <= 0)) {
    throw new Error('Invalid completion packet revision.');
  }
  if (input.evidenceFingerprint !== undefined
    && (typeof input.evidenceFingerprint !== 'string' || !/^[0-9a-f]{64}$/i.test(input.evidenceFingerprint))) {
    throw new Error('Invalid completion packet evidence fingerprint.');
  }
  if (input.idempotencyKey !== undefined
    && (typeof input.idempotencyKey !== 'string'
      || input.idempotencyKey.length === 0
      || input.idempotencyKey.length > 128
      || /[\s\x00-\x1f\x7f]/.test(input.idempotencyKey))) {
    throw new Error('Invalid completion packet idempotency key.');
  }
  return {
    ...(input.sessionId === undefined ? {} : { sessionId: input.sessionId as string }),
    ...(input.prompt === undefined ? {} : { prompt: input.prompt }),
    ...(input.packetRevision === undefined ? {} : { packetRevision: input.packetRevision as number }),
    ...(input.evidenceFingerprint === undefined ? {} : { evidenceFingerprint: input.evidenceFingerprint as string }),
    ...(input.idempotencyKey === undefined ? {} : { idempotencyKey: input.idempotencyKey as string }),
  };
}

/** Single main-process IPC owner for Taskmaster advice (protocol v3), settings, models, and inference trust. */
export function installTaskmasterIpc(deps: TaskmasterIpcDependencies): void {
  const id = (value: unknown): string => {
    if (typeof value !== 'string' || !value || value.length > 512) throw Error('Invalid recommendation ID');
    return value;
  };
  const adviceRequest = async (
    expectedWorkspace: unknown,
    resource: AdviceResource,
    payload?: unknown,
    recordId?: string,
    expectedBackend?: unknown,
  ) => {
    const current = () => {
      if (typeof expectedWorkspace !== 'string' || !expectedWorkspace || expectedWorkspace !== deps.workspacePath()) {
        throw Error('Workspace changed; reload Recommendations');
      }
    };
    current();
    return deps.withReady(async (port, token, signal) => {
      current();
      // An equality selector only: the supplied URL never becomes a fetch target.
      if (expectedBackend !== undefined && expectedBackend !== `http://127.0.0.1:${port}`) {
        throw Error('Workspace changed; reload Recommendations');
      }
      let value: Record<string, unknown>;
      try {
        value = await taskmasterAdviceRequest(port, token, resource, payload, recordId, deps.fetcher, signal, current);
      } catch (error) {
        if (!(error instanceof TaskmasterHttpError)) throw error;
        // Custom Error properties do not survive ipcRenderer.invoke. Carry only
        // a fixed HTTP status, never a provider/model error message.
        value = { protocolVersion: 3, requestError: { status: error.status } };
      }
      current();
      return value;
    });
  };
  const backendAdviceRequest = (workspace: unknown, expectedBackend: unknown, resource: AdviceResource, recordId?: string) => {
    if (typeof expectedBackend !== 'string') throw Error('Workspace changed; reload Recommendations');
    return adviceRequest(workspace, resource, undefined, recordId, expectedBackend);
  };

  async function readTaskmasterSettings(payload?: unknown): Promise<Record<string, unknown>> {
    const generation = deps.getGeneration();
    const port = await deps.getReadiness();
    if (generation !== deps.getGeneration()) throw new Error('Workspace changed; reopen Recommendations settings');
    const result = await taskmasterRequest(port, deps.getToken(), 'settings', payload, deps.fetcher);
    if (generation !== deps.getGeneration()) throw new Error('Workspace changed; reopen Recommendations settings');
    const settings = result.settings as { automaticKnowledgeUpdates?: boolean } | undefined;
    deps.setKnowledgeUpdatesEnabled(settings?.automaticKnowledgeUpdates !== false);
    return { ...result, knowledgeUpdate: deps.getKnowledgeUpdateStatus() };
  }

  async function refreshTaskmasterModels(provider: unknown, ollamaBaseUrl?: unknown): Promise<{ id: string; displayName: string }[]> {
    if (provider !== 'codex' && provider !== 'ollama') {
      throw new Error('Taskmaster model refresh is unavailable for this provider');
    }
    if (ollamaBaseUrl !== undefined && typeof ollamaBaseUrl !== 'string') throw new Error('Invalid Ollama URL');
    const generation = deps.getGeneration();
    const port = await deps.getReadiness();
    if (generation !== deps.getGeneration()) throw new Error('Workspace changed; retry model discovery');
    const result = await taskmasterRequest(port, deps.getToken(), 'models', {
      provider,
      ...(ollamaBaseUrl === undefined ? {} : { ollamaBaseUrl }),
    }, deps.fetcher);
    if (generation !== deps.getGeneration()) throw new Error('Workspace changed; retry model discovery');
    return parseTaskmasterModels(result);
  }

  async function inferenceTrustContext(): Promise<TrustContext> {
    const generation = deps.getGeneration();
    const port = await deps.getReadiness();
    if (generation !== deps.getGeneration()) throw new Error('Sidecar changed. Refresh executable approvals.');
    return {
      port,
      token: deps.getToken(),
      isCurrent: () => generation === deps.getGeneration(),
      confirm: deps.confirmInferenceApproval,
      ...(deps.fetcher ? { fetcher: deps.fetcher } : {}),
    };
  }

  for (const [channel, resource] of [
    ['get-advice-readiness-status', 'readiness'],
    ['get-advice-improvement-status', 'improvement'],
    ['request-advice-improvement', 'start-improvement'],
    ['request-advice-readiness-assessment', 'assess-readiness'],
    ['request-advice-readiness-suggestion', 'assess-and-suggest'],
    ['get-advice-run-status', 'run-status'],
    ['request-advice-analysis', 'analyze'],
    ['request-advice-assessment', 'assess-workflow'],
  ] as const) {
    deps.handle(channel, (_event, workspace) => adviceRequest(workspace, resource));
  }
  deps.handle('cancel-advice-improvement', (_event, workspace, runId) => {
    if (typeof runId !== 'string' || !/^[A-Za-z0-9-]{1,128}$/.test(runId)) throw Error('Invalid improvement run ID');
    return adviceRequest(workspace, 'cancel-improvement', { runId });
  });
  deps.handle('get-advice-recommendations', (_event, workspace, expectedBackend) =>
    backendAdviceRequest(workspace, expectedBackend, 'recommendations'));
  deps.handle('get-advice-recommendation', (_event, workspace, expectedBackend, recordId) =>
    backendAdviceRequest(workspace, expectedBackend, 'recommendation', id(recordId)));
  deps.handle('run-advice-audit', (_event, workspace, expectedBackend) =>
    backendAdviceRequest(workspace, expectedBackend, 'audit-recommendations'));
  deps.handle('dismiss-advice-recommendation', (_event, workspace, recordId, reason) => {
    if (reason !== undefined && (typeof reason !== 'string' || [...reason].length > 500)) {
      throw Error('Invalid dismissal reason');
    }
    return adviceRequest(workspace, 'dismiss-recommendation', reason === undefined ? {} : { reason }, id(recordId));
  });
  deps.handle('accept-advice-recommendation', (_event, workspace, recordId, options) =>
    adviceRequest(workspace, 'accept-recommendation', normalizeRecommendationAcceptOptions(options), id(recordId)));

  deps.handle('get-taskmaster-settings', () => readTaskmasterSettings());
  deps.handle('save-taskmaster-settings', (_event, payload) => deps.enqueueSettingsWrite(async () => {
    const saved = await readTaskmasterSettings(payload);
    deps.refreshKnowledge();
    return saved;
  }));
  deps.handle('refresh-taskmaster-models', (_event, provider, ollamaBaseUrl) =>
    refreshTaskmasterModels(provider, ollamaBaseUrl));
  deps.handle('get-inference-trust', async () => readInferenceTrust(await inferenceTrustContext()));
  deps.handle('approve-inference-adapter', (_event, request) =>
    deps.enqueueSettingsWrite(async () => approveInferenceAdapter(request, await inferenceTrustContext())));
  deps.handle('revoke-inference-adapter', (_event, request) =>
    deps.enqueueSettingsWrite(async () => revokeInferenceAdapter(request, await inferenceTrustContext())));
}
