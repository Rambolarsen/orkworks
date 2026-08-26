export const BACKEND_UNAVAILABLE_MESSAGE = "The OrkWorks sidecar is unavailable.";

export function sanitizeBackendLifecycleFailure(_error: unknown): string {
  return BACKEND_UNAVAILABLE_MESSAGE;
}
