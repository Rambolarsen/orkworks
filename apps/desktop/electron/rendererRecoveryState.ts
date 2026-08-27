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
