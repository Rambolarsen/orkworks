import { useEffect, useState } from "react";
import type { HarnessConfig, IntegrationStatusResult } from "../harnessTypes";
import { isAttentionSignal, shouldShowInstalledConfirmation } from "../harnessIntegrationPresentation";

// Mirrors the sole direct-reference condition in the backend probe
// (crates/orkworksd/src/harness/detect.rs::probe_installed_tool): POSIX
// absolute (`/...`), Windows drive-letter (`C:\...` / `C:/...`), or UNC
// (`\\server\...`).
function looksAbsolute(command: string): boolean {
  return command.startsWith("/") || /^[a-zA-Z]:[\\/]/.test(command) || command.startsWith("\\\\");
}

interface HarnessIntegrationSectionProps {
  harnessId: string;
  harnessName: string;
  harness: HarnessConfig | undefined;
  refreshGeneration?: number;
  onDetectionChanged?: (harnessId: string) => void;
}

export default function HarnessIntegrationSection({
  harnessId,
  harnessName,
  harness,
  refreshGeneration = 0,
  onDetectionChanged,
}: HarnessIntegrationSectionProps) {
  const launchCommand = harness?.launch.kind === "command-template" ? harness.launch.command : null;
  const hasCustomPath = launchCommand !== null && looksAbsolute(launchCommand);
  const [integration, setIntegration] = useState<IntegrationStatusResult | null>(null);
  const [integrationBusy, setIntegrationBusy] = useState(false);
  const [customPathDraft, setCustomPathDraft] = useState<string>(() =>
    hasCustomPath && launchCommand ? launchCommand : "",
  );
  // Locally owned rather than derived from `hasCustomPath` on every render:
  // the `harness` prop only refreshes when Settings is reopened, so a
  // save/clear updates this immediately instead of leaving the Clear
  // button (and the block's visibility once detection succeeds) stuck
  // showing pre-save state until the modal is closed and reopened.
  const [customPathActive, setCustomPathActive] = useState<boolean>(() => hasCustomPath);
  const [customPathBusy, setCustomPathBusy] = useState(false);
  const [customPathError, setCustomPathError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setIntegration(null);
    window.orkworks.getHarnessIntegrationStatus(harnessId).then((result) => {
      if (!cancelled) setIntegration(result);
    });
    return () => {
      cancelled = true;
    };
  }, [harnessId, refreshGeneration]);

  async function installIntegrationHandler() {
    setIntegrationBusy(true);
    try {
      const result = await window.orkworks.installHarnessIntegration(harnessId);
      setIntegration(result);
      if (result.ok) onDetectionChanged?.(harnessId);
    } finally {
      setIntegrationBusy(false);
    }
  }

  async function uninstallIntegrationHandler() {
    setIntegrationBusy(true);
    try {
      const result = await window.orkworks.uninstallHarnessIntegration(harnessId);
      setIntegration(result);
      if (result.ok) onDetectionChanged?.(harnessId);
    } finally {
      setIntegrationBusy(false);
    }
  }

  async function saveCustomPathHandler() {
    setCustomPathBusy(true);
    setCustomPathError(null);
    try {
      const result = await window.orkworks.setHarnessCommandOverride(harnessId, customPathDraft.trim());
      if (!result.ok) {
        setCustomPathError(result.error);
        return;
      }
      setCustomPathActive(true);
      onDetectionChanged?.(harnessId);
      setIntegration(await window.orkworks.getHarnessIntegrationStatus(harnessId));
    } catch (error) {
      setCustomPathError(error instanceof Error ? error.message : "Couldn't set the custom path.");
    } finally {
      setCustomPathBusy(false);
    }
  }

  async function clearCustomPathHandler() {
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
      onDetectionChanged?.(harnessId);
      setIntegration(await window.orkworks.getHarnessIntegrationStatus(harnessId));
    } catch (error) {
      setCustomPathError(error instanceof Error ? error.message : "Couldn't clear the custom path.");
    } finally {
      setCustomPathBusy(false);
    }
  }

  return (
    <div className="settings-config-item-actions">
      {integration === null && (
        <span className="settings-config-status">checking {harnessName} integration…</span>
      )}
      {integration && !integration.ok && (
        <span className="settings-config-status">{integration.error}</span>
      )}
      {integration?.ok && integration.status.registration === "installed" && (
        <>
          {shouldShowInstalledConfirmation(integration.status.diagnostics) &&
            (integration.status.activation === "needs_trust" ? (
              <span className="settings-config-status">
                Installed — approve the hook inside {harnessName} (run /hooks) to activate it
              </span>
            ) : (
              <span className="settings-config-status settings-config-status--ok">
                {isAttentionSignal(harnessId)
                  ? "✓ Attention hooks installed"
                  : integration.status.activation === "active"
                    ? "✓ Session capture hook active"
                    : "✓ Session capture hook installed"}
              </span>
            ))}
          <button type="button" onClick={uninstallIntegrationHandler} disabled={integrationBusy}>
            {integrationBusy ? "Removing…" : "Uninstall"}
          </button>
        </>
      )}
      {integration?.ok &&
        (integration.status.registration === "absent" ||
          integration.status.registration === "drifted") && (
          <>
            {integration.status.confirmation && (
              <p className="settings-section-copy">
                Installing will add {isAttentionSignal(harnessId) ? "attention hooks" : "a session capture hook"} to{" "}
                {integration.status.confirmation.relativePaths.join(", ")} in this
                workspace ({integration.status.confirmation.coverageSummary}).
                {integration.status.confirmation.executableCodeWarning && (
                  <> OrkWorks reports when {harnessName} waits for input and begins a tool action.</>
                )}
              </p>
            )}
            <button type="button" onClick={installIntegrationHandler} disabled={integrationBusy}>
              {integrationBusy
                ? "Installing…"
                : integration.status.registration === "drifted"
                  ? "Reinstall"
                  : isAttentionSignal(harnessId)
                    ? "Install attention hook"
                    : "Install session capture hook"}
            </button>
          </>
        )}
      {integration?.ok && integration.status.registration === "unsupported" && (
        <span className="settings-config-status">
          Attention hook isn't supported for this coding tool.
        </span>
      )}
      {integration?.ok && integration.status.diagnostics.length > 0 && (
        <span className="settings-config-status">
          {integration.status.diagnostics[0].message}
        </span>
      )}
      {integration?.ok &&
        (integration.status.diagnostics.some((d) => d.code === "tool_not_detected") ||
          customPathActive) && (
          <div className="settings-config-custom-path">
            <label>
              Custom path
              <input
                type="text"
                value={customPathDraft}
                onChange={(e) => setCustomPathDraft(e.target.value)}
                placeholder="/path/to/binary"
                disabled={customPathBusy}
              />
            </label>
            <p className="settings-section-copy">
              This also becomes the command OrkWorks launches {harnessName} sessions with —
              make sure it points at the real binary.
            </p>
            <button
              type="button"
              onClick={saveCustomPathHandler}
              disabled={customPathBusy || !looksAbsolute(customPathDraft.trim())}
            >
              {customPathBusy ? "Saving…" : "Save"}
            </button>
            {customPathActive && (
              <button type="button" onClick={clearCustomPathHandler} disabled={customPathBusy}>
                Clear
              </button>
            )}
            {customPathError && (
              <span className="settings-config-status">{customPathError}</span>
            )}
          </div>
        )}
    </div>
  );
}
