import { taskmasterAdviceRequest, TaskmasterHttpError } from './taskmasterSettings.ts';

type Resource = Parameters<typeof taskmasterAdviceRequest>[2];
type Dependencies = {
  handle(name: string, handler: (event: unknown, ...args: unknown[]) => Promise<unknown>): void;
  withReady<T>(operation: (port: number, token: string, signal: AbortSignal) => Promise<T>): Promise<T>;
  workspacePath(): string | null;
  normalizeAccept(value: unknown): Record<string, unknown>;
  fetcher?: (url: string, init?: RequestInit) => Promise<Response>;
};

/** New channels own the fixed protocol. Legacy channels remain separately registered. */
export function installAdviceIpc(deps: Dependencies): void {
  const id = (value: unknown): string => {
    if (typeof value !== 'string' || !value || value.length > 512) throw Error('Invalid recommendation ID');
    return value;
  };
  const request = async (expectedWorkspace: unknown, resource: Resource, payload?: unknown, recordId?: string, expectedBackend?: unknown) => {
    const current = () => {
      if (typeof expectedWorkspace !== 'string' || !expectedWorkspace || expectedWorkspace !== deps.workspacePath()) throw Error('Workspace changed; reload Recommendations');
    };
    current();
    return deps.withReady(async (port, token, signal) => {
      current();
      // An equality selector only: the supplied URL never becomes a fetch target.
      if (expectedBackend !== undefined && expectedBackend !== `http://127.0.0.1:${port}`) throw Error('Workspace changed; reload Recommendations');
      let value: Record<string, unknown>;
      try {
        value = await taskmasterAdviceRequest(port, token, resource, payload, recordId, deps.fetcher, signal, current);
      } catch (error) {
        if (!(error instanceof TaskmasterHttpError)) throw error;
        // Custom Error properties do not survive ipcRenderer.invoke. Carry only
        // a fixed HTTP status, never a provider/model error message.
        value = {protocolVersion:3,requestError:{status:error.status}};
      }
      current();
      return value;
    });
  };
  const backendRequest = (workspace: unknown, expectedBackend: unknown, resource: Resource, recordId?: string) => {
    if (typeof expectedBackend !== 'string') throw Error('Workspace changed; reload Recommendations');
    return request(workspace, resource, undefined, recordId, expectedBackend);
  };

  for (const [channel, resource] of [
    ['get-advice-readiness-status', 'readiness'],
    ['get-advice-improvement-status', 'improvement'],
    ['request-advice-improvement', 'start-improvement'],
    ['request-advice-readiness-assessment', 'assess-readiness'],
    ['request-advice-readiness-suggestion', 'assess-and-suggest'],
    ['get-advice-run-status', 'run-status'],
    ['request-advice-analysis', 'analyze'],
    ['request-advice-assessment', 'assess-workflow'],
  ] as const) deps.handle(channel, (_event, workspace) => request(workspace, resource));
  deps.handle('cancel-advice-improvement', (_event, workspace, runId) => {
    if (typeof runId !== 'string' || !/^[A-Za-z0-9-]{1,128}$/.test(runId)) throw Error('Invalid improvement run ID');
    return request(workspace, 'cancel-improvement', {runId});
  });
  deps.handle('get-advice-recommendations', (_event, workspace, expectedBackend) => backendRequest(workspace, expectedBackend, 'recommendations'));
  deps.handle('get-advice-recommendation', (_event, workspace, expectedBackend, recordId) => backendRequest(workspace, expectedBackend, 'recommendation', id(recordId)));
  deps.handle('run-advice-audit', (_event, workspace, expectedBackend) => backendRequest(workspace, expectedBackend, 'audit-recommendations'));
  deps.handle('dismiss-advice-recommendation', (_event, workspace, recordId, reason) => {
    if (reason !== undefined && (typeof reason !== 'string' || [...reason].length > 500)) throw Error('Invalid dismissal reason');
    return request(workspace, 'dismiss-recommendation', reason === undefined ? {} : {reason}, id(recordId));
  });
  deps.handle('accept-advice-recommendation', (_event, workspace, recordId, options) => request(workspace, 'accept-recommendation', deps.normalizeAccept(options), id(recordId)));
}
