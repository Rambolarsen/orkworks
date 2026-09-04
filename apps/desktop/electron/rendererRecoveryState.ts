// Chromium net error code for ERR_ABORTED: a navigation superseded by another
// in-flight load — not a genuine failure of the original document.
export const ERR_ABORTED = -3;

export function isSupersededNavigation(errorCode: number): boolean {
  return errorCode === ERR_ABORTED;
}

export function createRecoveryDocumentGuard(originalUrl: string) {
  let recoveryDocumentLoaded = false;

  return {
    beginOriginalDocumentNavigation(url: string): void {
      if (url === originalUrl) recoveryDocumentLoaded = false;
    },
    finishOriginalDocumentLoad(url: string): void {
      if (url === originalUrl) recoveryDocumentLoaded = false;
    },
    beginRecoveryDocumentLoad(): boolean {
      if (recoveryDocumentLoaded) return false;
      recoveryDocumentLoaded = true;
      return true;
    },
    recoveryDocumentLoadFailed(): void {
      recoveryDocumentLoaded = false;
    },
  };
}
