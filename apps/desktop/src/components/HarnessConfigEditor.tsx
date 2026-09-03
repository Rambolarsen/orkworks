import { useState } from "react";
import {
  HarnessApiError,
  HARNESS_ACTIVE_DELETE_FORBIDDEN_CODE,
  deleteHarness,
  removeHarnessProfile,
  saveHarnessConfiguration,
  type HarnessMutationResponse,
} from "../api";
import {
  parseHarnessDraft,
  type HarnessConfigEntry,
  type HarnessDraftParseResult,
  type HarnessEditorMetadata,
  type HarnessEditorMode,
} from "../harnessTypes";
import Button from "./Button";

interface HarnessConfigEditorProps {
  mode: HarnessEditorMode;
  draftText: string;
  metadata: HarnessEditorMetadata;
  onCancel: () => void;
  onSaved: (result: HarnessMutationResponse) => void | Promise<void>;
  onDeleted?: (kind: "delete" | "reset") => void | Promise<void>;
  onRevisionConflict?: () => void | Promise<void>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === "object" && !Array.isArray(value);
}

function effectivePreview(
  mode: HarnessEditorMode,
  parsed: HarnessDraftParseResult,
  entry: HarnessConfigEntry | undefined,
): unknown {
  if (!parsed.value) return entry ?? null;
  if (mode !== "override" || !entry) return parsed.value;
  const preview: Record<string, unknown> = { ...entry };
  for (const [key, value] of Object.entries(parsed.value)) {
    if (isRecord(preview[key]) && isRecord(value)) {
      preview[key] = { ...(preview[key] as Record<string, unknown>), ...value };
    } else {
      preview[key] = value;
    }
  }
  return preview;
}

function titleFor(mode: HarnessEditorMode, entry: HarnessConfigEntry | undefined): string {
  if (mode === "create") return "New custom coding tool";
  return entry?.name ?? "Coding tool configuration";
}

function modeLabel(mode: HarnessEditorMode, entry: HarnessConfigEntry | undefined): string {
  if (mode === "create" || entry?.origin === "custom") return "Custom";
  return entry?.origin === "override" ? "Built-in Override" : "Built-in";
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
}

export default function HarnessConfigEditor({
  mode,
  draftText,
  metadata,
  onCancel,
  onSaved,
  onDeleted,
  onRevisionConflict,
}: HarnessConfigEditorProps) {
  const [draft, setDraft] = useState(draftText);
  const [busy, setBusy] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);
  const parsed = parseHarnessDraft(draft, mode);
  const entry = metadata.entry;
  const profile = entry?.compatibility.profile ?? null;

  async function save() {
    if (parsed.diagnostics.length > 0 || !parsed.value || busy) return;
    if (mode === "custom" && entry && parsed.value.id !== entry.id) {
      setServerError("The harness ID cannot change after creation.");
      return;
    }
    setBusy(true);
    setServerError(null);
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      const result = await saveHarnessConfiguration(baseUrl, {
        mode,
        harnessId: entry?.id,
        definition: parsed.value,
        expectedRevision: metadata.documentRevision,
        ...(metadata.duplicateSourceId ? { duplicateSourceId: metadata.duplicateSourceId } : {}),
      });
      await onSaved(result);
    } catch (error) {
      if (error instanceof HarnessApiError && error.status === 409) {
        setServerError("This configuration changed elsewhere. Your draft is still here; refresh the latest version, compare, then save again.");
        await onRevisionConflict?.();
      } else {
        setServerError(errorMessage(error, "Couldn't save coding tool configuration."));
      }
    } finally {
      setBusy(false);
    }
  }

  async function removeProfile() {
    if (!entry || entry.origin !== "custom" || !profile || busy) return;
    setBusy(true);
    setServerError(null);
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      await onSaved(await removeHarnessProfile(baseUrl, entry.id, metadata.documentRevision));
    } catch (error) {
      if (error instanceof HarnessApiError && error.status === 409) await onRevisionConflict?.();
      setServerError(errorMessage(error, "Couldn't remove the compatibility profile."));
    } finally {
      setBusy(false);
    }
  }

  async function deleteOrReset() {
    if (!entry || busy) return;
    const action = entry.origin === "override" ? "reset this built-in override" : "delete this custom coding tool";
    if (!window.confirm(`Are you sure you want to ${action}?`)) return;
    setBusy(true);
    setServerError(null);
    try {
      const baseUrl = await window.orkworks.getBackendUrl();
      await deleteHarness(baseUrl, entry.id, metadata.documentRevision);
      await onDeleted?.(entry.origin === "override" ? "reset" : "delete");
    } catch (error) {
      if (error instanceof HarnessApiError && error.status === 409) await onRevisionConflict?.();
      setServerError(error instanceof HarnessApiError && error.code === HARNESS_ACTIVE_DELETE_FORBIDDEN_CODE
        ? "Disable this coding tool and save the active coding tools before deleting it."
        : errorMessage(error, "Couldn't update coding tool configuration."));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="harness-config-editor">
      <div className="harness-config-editor-header">
        <div>
          <button type="button" className="harness-config-editor-back" onClick={onCancel} disabled={busy}>
            ← Back to coding tools
          </button>
          <h3>{titleFor(mode, entry)}</h3>
        </div>
        <span className="harness-origin-badge">{modeLabel(mode, entry)}</span>
      </div>
      <p className="settings-section-copy">
        Edit the complete harness definition. Changes are validated before they can be saved.
      </p>

      {mode === "create" && (
        <div className="harness-config-editor-notice">
          This creates an independent custom coding tool. Saving does not enable it, install a hook, or assign a compatibility profile.
        </div>
      )}
      {mode === "override" && (
        <div className="harness-config-editor-notice">
          Only these fields are customized. Unspecified fields continue using the built-in defaults. Future built-in improvements will apply automatically. Reset removes the override.
        </div>
      )}
      {mode === "custom" && (
        <div className="harness-config-editor-notice">
          This is an independent copy. Future changes to the source harness will not modify it. Its definition and provider settings remain independently editable. Saving does not change workspace enablement; disable it and save the active coding tools before deleting it.
        </div>
      )}

      {profile && (
        <div className="harness-config-editor-readonly" aria-label="Compatibility profile">
          <strong>Compatibility profile: {profile}</strong> <span>(read-only)</span>
          <div>This profile is code-owned. The command and harness settings remain independently editable.</div>
          <div>{profile === "copilot"
            ? "This tool uses the Copilot integration; the shared hook remains installed while any active compatible tool uses it."
            : "Derived integration bindings remain code-owned. Derived session signals remain code-owned."}</div>
        </div>
      )}

      <div className="harness-config-editor-grid">
        <label className="harness-config-editor-pane">
          <span className="harness-config-editor-pane-title">
            {mode === "override" ? "Override JSON" : "Configuration JSON"}
            {parsed.diagnostics.length === 0 ? <em>valid</em> : <em className="harness-config-editor-invalid">invalid</em>}
          </span>
          <textarea
            value={draft}
            onChange={(event) => {
              setDraft(event.target.value);
              setServerError(null);
            }}
            spellCheck={false}
            aria-label="Configuration JSON"
            disabled={busy}
          />
          {parsed.diagnostics.length > 0 && (
            <div className="harness-config-editor-diagnostics" role="alert">
              {parsed.diagnostics.map((diagnostic, index) => (
                <div key={`${diagnostic.code}-${diagnostic.path ?? index}`}>
                  {diagnostic.path ? `${diagnostic.path}: ` : ""}{diagnostic.message}
                  {diagnostic.line ? ` (line ${diagnostic.line}, column ${diagnostic.column ?? 1})` : ""}
                </div>
              ))}
            </div>
          )}
        </label>
        <div className="harness-config-editor-pane">
          <span className="harness-config-editor-pane-title">Effective configuration <em>read-only</em></span>
          <pre>{JSON.stringify(effectivePreview(mode, parsed, entry), null, 2)}</pre>
        </div>
      </div>

      {serverError && <div className="harness-config-editor-error" role="alert">{serverError}</div>}
      <div className="harness-config-editor-actions">
        <Button variant="secondary" onClick={onCancel} disabled={busy}>Cancel</Button>
        {mode === "custom" && profile && (
          <Button variant="secondary" onClick={() => void removeProfile()} disabled={busy}>
            Remove compatibility profile
          </Button>
        )}
        {entry && (entry.origin === "custom" || entry.origin === "override") && (
          <Button variant="secondary" onClick={() => void deleteOrReset()} disabled={busy}>
            {entry.origin === "override" ? "Reset built-in" : "Delete coding tool"}
          </Button>
        )}
        <Button variant="primary" onClick={() => void save()} disabled={busy || parsed.diagnostics.length > 0}>
          {busy ? "Saving…" : "Save configuration"}
        </Button>
      </div>
    </div>
  );
}
