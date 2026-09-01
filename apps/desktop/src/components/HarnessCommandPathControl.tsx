import { useEffect, useState } from "react";
import { saveHarnessConfiguration, stripDerivedHarnessFields } from "../api";
import type { HarnessConfigEntry } from "../harnessTypes";

// Mirrors the sole direct-reference condition in the backend probe
// (crates/orkworksd/src/harness/detect.rs::probe_installed_tool): POSIX
// absolute (`/...`), Windows drive-letter (`C:\...` / `C:/...`), or UNC
// (`\\server\...`).
function looksAbsolute(command: string): boolean {
  return command.startsWith("/") || /^[a-zA-Z]:[\\/]/.test(command) || command.startsWith("\\\\");
}

interface HarnessCommandPathControlProps {
  harnessId: string;
  harnessName: string;
  harness: HarnessConfigEntry | undefined;
  documentRevision?: string | null;
  disabled?: boolean;
  onChanged?: (harnessId: string) => void;
}

export default function HarnessCommandPathControl({
  harnessId,
  harnessName,
  harness,
  documentRevision,
  disabled = false,
  onChanged,
}: HarnessCommandPathControlProps) {
  // Narrow once up front; the early `return null` for non-command-template
  // harnesses lives below the hooks so hook order stays stable even if the
  // `harness` prop's launch kind were to change between renders.
  const launch = harness?.launch.kind === "command-template" ? harness.launch : undefined;
  const launchCommand = launch?.command;
  const hasCustomPath = launchCommand !== undefined && looksAbsolute(launchCommand);
  const isCustom = harness?.origin === "custom";
  const [customPathDraft, setCustomPathDraft] = useState<string>(() => (hasCustomPath ? launchCommand ?? "" : ""));
  // Locally owned rather than derived from `hasCustomPath` on every render:
  // the `harness` prop only refreshes when Settings is reopened, so a
  // save/clear updates this immediately instead of leaving the Clear
  // button stuck showing pre-save state until the modal is closed/reopened.
  const [customPathActive, setCustomPathActive] = useState<boolean>(() => hasCustomPath);
  const [customPathBusy, setCustomPathBusy] = useState(false);
  const [customPathError, setCustomPathError] = useState<string | null>(null);
  const trimmedCustomPath = customPathDraft.trim();

  useEffect(() => {
    if (hasCustomPath) {
      setCustomPathDraft(launchCommand);
      setCustomPathActive(true);
      return;
    }
    setCustomPathDraft("");
    setCustomPathActive(false);
  }, [hasCustomPath, launchCommand]);

  async function saveCustomPathHandler() {
    setCustomPathBusy(true);
    setCustomPathError(null);
    try {
      const result = isCustom && harness && launch
        ? await (async () => {
          const baseUrl = await window.orkworks.getBackendUrl();
          const definition = stripDerivedHarnessFields({
            ...harness,
            launch: { ...launch, command: customPathDraft.trim() },
          });
          return { ok: true as const, harness: (await saveHarnessConfiguration(baseUrl, {
            mode: "custom",
            harnessId,
            definition,
            expectedRevision: documentRevision ?? harness.documentRevision ?? null,
          })).harness };
        })()
        : await window.orkworks.setHarnessCommandOverride(harnessId, customPathDraft.trim());
      if (!result.ok) {
        setCustomPathError(result.error);
        return;
      }
      setCustomPathDraft(result.harness.launch.kind === "command-template" ? result.harness.launch.command : trimmedCustomPath);
      setCustomPathActive(true);
      onChanged?.(harnessId);
    } catch (error) {
      setCustomPathError(error instanceof Error ? error.message : "Couldn't set the custom path.");
    } finally {
      setCustomPathBusy(false);
    }
  }

  async function clearCustomPathHandler() {
    if (isCustom) return;
    setCustomPathBusy(true);
    setCustomPathError(null);
    try {
      const result = await window.orkworks.clearHarnessCommandOverride(harnessId);
      if (!result.ok) {
        setCustomPathError(result.error);
        return;
      }
      setCustomPathActive(false);
      setCustomPathDraft("");
      onChanged?.(harnessId);
    } catch (error) {
      setCustomPathError(error instanceof Error ? error.message : "Couldn't clear the custom path.");
    } finally {
      setCustomPathBusy(false);
    }
  }

  if (!launch) return null;

  return (
    <div className="settings-config-item-actions settings-config-custom-path">
      <label className="settings-config-custom-path-field">
        <span>Custom path</span>
        <input
          className="settings-config-custom-path-input"
          type="text"
          value={customPathDraft}
          onChange={(event) => setCustomPathDraft(event.target.value)}
          placeholder="/path/to/binary"
          disabled={disabled || customPathBusy}
        />
      </label>
      <p className="settings-section-copy">
        This also becomes the command OrkWorks launches {harnessName} sessions with —
        make sure it points at the real binary.
      </p>
      <div className="settings-config-custom-path-actions">
        <button
          type="button"
          onClick={saveCustomPathHandler}
          disabled={disabled || customPathBusy || !looksAbsolute(customPathDraft.trim())}
        >
          {customPathBusy ? "Saving…" : "Save"}
        </button>
        {customPathActive && !isCustom && (
          <button type="button" onClick={clearCustomPathHandler} disabled={disabled || customPathBusy}>
            Clear
          </button>
        )}
      </div>
      {customPathError && (
        <span className="settings-config-status">{customPathError}</span>
      )}
    </div>
  );
}
