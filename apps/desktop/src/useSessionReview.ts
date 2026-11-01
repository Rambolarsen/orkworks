import { useCallback, useEffect, useRef, useState } from "react";
import type { PlanDocumentKind } from "./api";

export interface ReviewDocument {
  sessionId: string;
  generation: number;
  artifactId: string;
  content: string;
  kind?: PlanDocumentKind;
  scopeKey?: string;
  selectionId?: string;
  branch?: string | null;
}

// The sidecar validates the associated path on every read. Keep only the returned
// bytes and their digest in renderer memory; navigation memory stores no document.
export function useSessionReview(sessionId: string | null, generation: number, enabled: boolean,
  onLoaded: (document: ReviewDocument, persist: boolean) => boolean,
  onUnavailable: (sessionId: string, generation: number, persist: boolean, restoring: boolean) => boolean,
  scopeKey?: string, locationKey?: string) {
  const [document, setDocument] = useState<ReviewDocument | null>(null);
  const [loading, setLoading] = useState(false);
  const [changed, setChanged] = useState(false);
  const previousDigest = useRef<string | null>(null);
  const [error, setError] = useState(false);
  const request = useRef(0);
  const selectedKind = useRef<PlanDocumentKind | undefined>(undefined);
  const selectedId = useRef<string | undefined>(undefined);
  const selectedBranch = useRef<string | null | undefined>(undefined);
  const explicitSelection = useRef<number | null>(null);
  const scope = useRef({ sessionId, generation, enabled, scopeKey, locationKey });
  if (scope.current.sessionId !== sessionId || scope.current.generation !== generation || scope.current.enabled !== enabled || scope.current.scopeKey !== scopeKey) {
    const sameLocation = scope.current.sessionId === sessionId && scope.current.generation === generation && scope.current.enabled === enabled && scope.current.locationKey === locationKey;
    scope.current = { sessionId, generation, enabled, scopeKey, locationKey };
    if (explicitSelection.current === null || !sameLocation) ++request.current;
  }
  const onLoadedRef = useRef(onLoaded);
  onLoadedRef.current = onLoaded;
  const onUnavailableRef = useRef(onUnavailable);
  onUnavailableRef.current = onUnavailable;
  const close = useCallback(() => {
    ++request.current;
    setDocument(null);
    selectedKind.current = undefined;
    selectedId.current = undefined;
    selectedBranch.current = undefined;
    previousDigest.current = null;
    setChanged(false);
    setLoading(false);
    setError(false);
    explicitSelection.current = null;
  }, []);
  useEffect(() => {
    if (explicitSelection.current !== request.current) close();
  }, [sessionId, generation, enabled, scopeKey, locationKey, close]);
  useEffect(() => () => { ++request.current; }, []);
  const unavailable = useCallback((kind?: PlanDocumentKind, selectionId?: string) => {
    close();
    selectedKind.current = kind;
    selectedId.current = selectionId;
    setError(true);
  }, [close]);
  const load = useCallback(async (persist = true, restoring = false, printedPath?: string, kind?: PlanDocumentKind, selectionId?: string, branch?: string | null) => {
    if (!sessionId || !enabled) return;
    const token = ++request.current;
    let requestedKind = printedPath !== undefined ? undefined : kind ?? selectedKind.current;
    let requestedId = printedPath !== undefined ? undefined : selectionId ?? (kind === undefined || kind === selectedKind.current ? selectedId.current : undefined);
    let requestedBranch = printedPath !== undefined ? undefined : branch !== undefined ? branch : kind === undefined ? selectedBranch.current : undefined;
    if (printedPath !== undefined) explicitSelection.current = token;
    selectedKind.current = requestedKind;
    selectedId.current = requestedId;
    selectedBranch.current = requestedBranch;
    setLoading(true);
    // Never retain old document bytes across a refresh that may fail validation.
    setDocument(null);
    try {
      if (printedPath !== undefined) {
        const selection = await window.orkworks.selectTerminalPlan(sessionId, printedPath);
        if (request.current !== token) return;
        requestedKind = selection?.kind;
        requestedId = selection?.selectionId;
        requestedBranch = selection?.branch;
        selectedKind.current = requestedKind;
        selectedId.current = requestedId;
        selectedBranch.current = requestedBranch;
      }
      const content = requestedKind === undefined ? await window.orkworks.getPlanContent(sessionId) : await window.orkworks.getPlanContent(sessionId, requestedKind, requestedId);
      const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(`${requestedKind ?? "legacy"}\0${content}`));
      if (request.current !== token) return;
      const artifactId = Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2, "0")).join("");
      const next = { sessionId, generation, artifactId, content, kind: requestedKind, scopeKey: scope.current.scopeKey, selectionId: requestedId, branch: requestedBranch };
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
      if (explicitSelection.current === token) explicitSelection.current = null;
      if (request.current === token) setLoading(false);
    }
  }, [sessionId, generation, enabled, scopeKey]);
  return { document: document?.sessionId === sessionId && document.generation === generation && document.scopeKey === scopeKey && enabled ? document : null,
    loading, error, changed, load, close, unavailable };
}
