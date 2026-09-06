export interface BackendRetryGuard {
  begin(): number;
  isCurrent(token: number): boolean;
}

export function createBackendRetryGuard(): BackendRetryGuard {
  let latest = 0;

  return {
    begin(): number {
      latest += 1;
      return latest;
    },
    isCurrent(token: number): boolean {
      return token === latest;
    },
  };
}
