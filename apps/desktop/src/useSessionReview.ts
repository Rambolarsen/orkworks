import { useCallback, useEffect, useRef, useState } from "react";
import type { PlanDocumentKind } from "./api";

export interface ReviewDocument {
  sessionId: string;
  generation: number;
  artifactId: string;
  content: string;
  kind?: PlanDocumentKind;
  scopeKey?: string;
}

// The sidecar validates the associated path on every read. Keep only the returned
// bytes and their digest in renderer memory; navigation memory stores no document.
export function useSessionReview(sessionId: string | null, generation: number, enabled: boolean,
  onLoaded: (document: ReviewDocument, persist: boolean) => boolean,
  onUnavailable: (sessionId: string, generation: number, persist: boolean, restoring: boolean) => boolean,
  scopeKey?: string) {
  const [document, setDocument] = useState<ReviewDocument | null>(null);
  const [loading, setLoading] = useState(false);
  const [changed, setChanged] = useState(false);
  const previousDigest = useRef<string | null>(null);
  const [error, setError] = useState(false);
  const request = useRef(0);
  const selectedKind = useRef<PlanDocumentKind | undefined>(undefined);
  const scope = useRef({ sessionId, generation, enabled, scopeKey });
  if (scope.current.sessionId !== sessionId || scope.current.generation !== generation || scope.current.enabled !== enabled || scope.current.scopeKey !== scopeKey) {
    scope.current = { sessionId, generation, enabled, scopeKey };
    ++request.current;
  }
  const onLoadedRef = useRef(onLoaded);
  onLoadedRef.current = onLoaded;
  const onUnavailableRef = useRef(onUnavailable);
  onUnavailableRef.current = onUnavailable;
  const close = useCallback(() => {
    ++request.current;
    setDocument(null);
    selectedKind.current = undefined;
    previousDigest.current = null;
    setChanged(false);
    setLoading(false);
    setError(false);
  }, []);
  useEffect(() => { close(); return () => { ++request.current; }; }, [sessionId, generation, enabled, scopeKey, close]);
  const load = useCallback(async (persist = true, restoring = false, printedPath?: string, kind?: PlanDocumentKind) => {
    if (!sessionId || !enabled) return;
    const token = ++request.current;
    const requestedKind = printedPath !== undefined ? undefined : kind ?? selectedKind.current;
    selectedKind.current = requestedKind;
    setLoading(true);
    // Never retain old document bytes across a refresh that may fail validation.
    setDocument(null);
    try {
      if (printedPath !== undefined) {
        await window.orkworks.selectTerminalPlan(sessionId, printedPath);
        if (request.current !== token) return;
      }
      const content = requestedKind === undefined ? await window.orkworks.getPlanContent(sessionId) : await window.orkworks.getPlanContent(sessionId, requestedKind);
      const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(`${requestedKind ?? "legacy"}\0${content}`));
      if (request.current !== token) return;
      const artifactId = Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2, "0")).join("");
      const next = { sessionId, generation, artifactId, content, kind: requestedKind, scopeKey };
      if (!onLoadedRef.current(next, persist)) return;
      setChanged(previousDigest.current !== null && previousDigest.current !== artifactId);
      previousDigest.current = artifactId;
      setDocument(next);
      setError(false);
    } catch (error) {
      // A rejected click stays in Terminal and reports the sidecar's explanation.
      if (printedPath !== undefined) {
        if (request.current === token) throw error;
        return;
      }
      if (request.current === token && onUnavailableRef.current(sessionId, generation, persist, restoring)) setError(true);
    } finally {
      if (request.current === token) setLoading(false);
    }
  }, [sessionId, generation, enabled, scopeKey]);
  return { document: document?.sessionId === sessionId && document.generation === generation && document.scopeKey === scopeKey && enabled ? document : null,
    loading, error, changed, load, close };
}
