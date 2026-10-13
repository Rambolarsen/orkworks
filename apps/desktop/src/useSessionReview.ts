import { useCallback, useEffect, useRef, useState } from "react";

export interface ReviewDocument {
  sessionId: string;
  generation: number;
  artifactId: string;
  content: string;
}

// The sidecar validates the associated path on every read. Keep only the returned
// bytes and their digest in renderer memory; navigation memory stores no document.
export function useSessionReview(sessionId: string | null, generation: number, enabled: boolean,
  onLoaded: (document: ReviewDocument, persist: boolean) => void) {
  const [document, setDocument] = useState<ReviewDocument | null>(null);
  const [loading, setLoading] = useState(false);
  const [changed, setChanged] = useState(false);
  const previousDigest = useRef<string | null>(null);
  const [error, setError] = useState(false);
  const request = useRef(0);
  const scope = useRef({ sessionId, generation, enabled });
  if (scope.current.sessionId !== sessionId || scope.current.generation !== generation || scope.current.enabled !== enabled) {
    scope.current = { sessionId, generation, enabled };
    ++request.current;
  }
  const onLoadedRef = useRef(onLoaded);
  onLoadedRef.current = onLoaded;
  const close = useCallback(() => {
    ++request.current;
    setDocument(null);
    previousDigest.current = null;
    setChanged(false);
    setLoading(false);
    setError(false);
  }, []);
  useEffect(() => { close(); return () => { ++request.current; }; }, [sessionId, generation, enabled, close]);
  const load = useCallback(async (persist = true) => {
    if (!sessionId || !enabled) return;
    const token = ++request.current;
    setLoading(true);
    setError(false);
    // Never retain old document bytes across a refresh that may fail validation.
    setDocument(null);
    try {
      const content = await window.orkworks.getPlanContent(sessionId);
      const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(content));
      if (request.current !== token) return;
      const artifactId = Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2, "0")).join("");
      setChanged(previousDigest.current !== null && previousDigest.current !== artifactId);
      previousDigest.current = artifactId;
      const next = { sessionId, generation, artifactId, content };
      setDocument(next);
      onLoadedRef.current(next, persist);
    } catch {
      if (request.current === token) setError(true);
    } finally {
      if (request.current === token) setLoading(false);
    }
  }, [sessionId, generation, enabled]);
  return { document: document?.sessionId === sessionId && document.generation === generation && enabled ? document : null,
    loading, error, changed, load, close };
}
