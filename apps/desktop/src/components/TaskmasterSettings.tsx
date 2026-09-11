import { useEffect, useState } from "react";
import InferenceTrustSettings from "./InferenceTrustSettings";
import { editTaskmasterScope, supportsTaskmasterAnalysis, type AnalysisContext, type TaskmasterSettings as Settings, type TaskmasterSettingsStatus } from "../taskmasterSettings";

export default function TaskmasterSettings() {
  const [status, setStatus] = useState<TaskmasterSettingsStatus | null>(null);
  const [draft, setDraft] = useState<Settings | null>(null);
  const [scope, setScope] = useState<"global" | "workspace">("global");
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let disposed = false;
    window.orkworks.getTaskmasterSettings().then((value) => {
      if (!disposed) { setStatus(value); setDraft(value.settings); }
    }).catch((reason: unknown) => { if (!disposed) setError(String(reason)); });
    return () => { disposed = true; };
  }, []);
  const workspace = scope === "workspace" ? status?.workspacePath ?? null : null;
  const effective = draft && workspace ? { ...draft, ...draft.workspaceOverrides[workspace] } : draft;
  const provider = effective?.selection?.provider ?? "";
  const providers = status?.providers ?? [];
  const selectedProvider = providers.find((item) => item.id === provider);
  // Static suggestions only: discovery is executable code outside inference trust.
  const models = selectedProvider?.models ?? [];
  function edit(patch: Partial<Settings>) {
    setDraft((value) => value && editTaskmasterScope(value, workspace, patch));
    setSaved(false);
  }
  async function save() {
    if (!draft || !status) return;
    setBusy(true); setError(null); setSaved(false);
    try {
      const current = await window.orkworks.getTaskmasterSettings();
      if (current.workspacePath !== status.workspacePath) throw new Error("Workspace changed. Reopen these settings before saving.");
      const selections = [draft.selection, ...Object.values(draft.workspaceOverrides).map((override) => override.selection)];
      for (const selection of selections) {
        if (selection?.reasoningEffort !== undefined && current.providers.find((item) => item.id === selection.provider)?.supportsReasoningEffort === false) {
          throw new Error("The selected provider does not support reasoning effort. Select Provider default before saving.");
        }
      }
      const cleanPaths = (paths: string[]) => paths.map((value) => value.trim()).filter(Boolean);
      const normalized = { ...draft, excludedPaths: cleanPaths(draft.excludedPaths), workspaceOverrides: Object.fromEntries(
        Object.entries(draft.workspaceOverrides).map(([key, value]) => [key, { ...value, ...(value.excludedPaths ? { excludedPaths: cleanPaths(value.excludedPaths) } : {}) }]),
      ) };
      const result = await window.orkworks.saveTaskmasterSettings(normalized);
      setStatus(result); setDraft(result.settings); setSaved(true);
    } catch (reason) { setError(String(reason)); }
    finally { setBusy(false); }
  }
  if (!draft || !effective || !status) return <div className="settings-section" role="status">{error ?? "Loading recommendation settings…"}</div>;
  return <div className="settings-section taskmaster-settings">
    <h3>Recommendations</h3>
    <p className="settings-section-copy">Taskmaster uses its own model to suggest improvements. Changes still require your approval.</p>
    <p className="settings-section-copy">Coding-tool providers reuse your existing CLI login. Administrator-managed policies remain active and may run required hooks, add context, or control routing. Taskmaster requests recommendations, not coding actions.</p>
    <label>Settings scope
      <select value={scope} onChange={(event) => { setScope(event.target.value as "global" | "workspace"); setSaved(false); }}>
        <option value="global">All workspaces</option>
        <option value="workspace" disabled={!status.workspacePath}>This workspace</option>
      </select>
    </label>
    {workspace && <button type="button" disabled={busy} onClick={() => { setDraft(editTaskmasterScope(draft, workspace, null)); setSaved(false); }}>Use global defaults for this workspace</button>}
    <label className="taskmaster-checkbox"><input type="checkbox" checked={effective.enabled} onChange={(event) => edit({ enabled: event.target.checked })} /> Discover improvements in the background</label>
    <label>Taskmaster model provider
      <select value={provider} onChange={(event) => edit({ selection: event.target.value ? { provider: event.target.value, model: "" } : null })}>
        <option value="">Not configured — deterministic recommendations only</option>
        {provider && !selectedProvider && <option value={provider} disabled>{provider} — unavailable (saved selection)</option>}
        {providers.map((item) => <option key={item.id} value={item.id} disabled={!supportsTaskmasterAnalysis(item)}>{item.label}{item.state !== "ready" ? ` — ${item.state.replaceAll("_", " ")}` : ""}</option>)}
      </select>
    </label>
    {provider && <>
      {selectedProvider?.state === "execution_inactive" && <p role="status">Adapter approved. Custom background execution is not active in this build.</p>}
      {selectedProvider?.state === "approval_required" && <p role="status">Approve this adapter below before it can become eligible for background execution.</p>}
      <label>Model
        <input list="taskmaster-model-options" value={effective.selection?.model ?? ""} onChange={(event) => edit({ selection: { ...effective.selection!, model: event.target.value } })} placeholder="Model ID" />
        <datalist id="taskmaster-model-options">{models.map((model) => <option key={model} value={model} />)}</datalist>
      </label>
      <p className="settings-section-copy">Enter the model ID used by your provider. Suggestions are static; opening these settings does not run model discovery.</p>
      <label>Reasoning effort (optional)
        <select value={effective.selection?.reasoningEffort ?? ""} onChange={(event) => edit({ selection: { ...effective.selection!, reasoningEffort: event.target.value || undefined } })}>
          <option value="">Provider default</option><option value="low" disabled={!selectedProvider?.supportsReasoningEffort}>Low</option><option value="medium" disabled={!selectedProvider?.supportsReasoningEffort}>Medium</option><option value="high" disabled={!selectedProvider?.supportsReasoningEffort}>High</option>
        </select>
      </label>
      {provider === "ollama" && <label>Ollama URL
        <input value={effective.selection?.ollamaBaseUrl ?? "http://127.0.0.1:11434"} onChange={(event) => edit({ selection: { ...effective.selection!, ollamaBaseUrl: event.target.value } })} />
      </label>}
    </>}
    <label>Analysis context
      <select value={effective.contextLevel} onChange={(event) => edit({ contextLevel: event.target.value as AnalysisContext })}>
        <option value="session_observations">Session observations</option>
        <option value="workflow_context">Workflow context</option>
        <option value="source_code">Relevant source code</option>
      </select>
    </label>
    <p className="settings-section-copy">Allowed context may be sent to the selected model provider. Workflow context includes selected instructions, documentation, manifests, and CI configuration. Source code also permits selected source files. Ignored files, credential files, and paths outside the workspace are excluded. Additional terminal replay is not read.</p>
    <label>Excluded paths (one relative path per line)
      <textarea rows={3} value={effective.excludedPaths.join("\n")} onChange={(event) => edit({ excludedPaths: event.target.value.split("\n") })} />
    </label>
    <label>Minimum interval per workspace (minutes)
      <input type="number" min={1} max={1440} value={effective.minIntervalMinutes} onChange={(event) => edit({ minIntervalMinutes: Number(event.target.value) })} />
    </label>
    <label>Daily AI evaluations across the app
      <input type="number" min={1} max={64} disabled={scope === "workspace"} value={draft.dailyEvaluationLimit} onChange={(event) => edit({ dailyEvaluationLimit: Number(event.target.value) })} />
    </label>
    <p className="settings-section-copy">Calls, including failed calls, count toward this limit. Cost depends on the selected model. The daily limit resets at midnight UTC.</p>
    <label className="taskmaster-checkbox"><input type="checkbox" disabled={scope === "workspace"} checked={draft.automaticKnowledgeUpdates} onChange={(event) => edit({ automaticKnowledgeUpdates: event.target.checked })} /> Update shared knowledge automatically</label>
    <dl className="recommendation-facts">
      <div><dt>Analysis</dt><dd>{status.analysisStatus.replaceAll("_", " ")}</dd></div>
      <div><dt>Remaining today</dt><dd>{status.remainingEvaluations}</dd></div>
      <div><dt>Knowledge version</dt><dd>{status.knowledgeUpdate.version ?? status.knowledgeVersion ?? "Not loaded"}</dd></div>
      <div><dt>Last successful knowledge check</dt><dd>{status.knowledgeUpdate.lastSuccessfulUpdate ?? "Not yet checked"}</dd></div>
    </dl>
    {(status.lastError || status.knowledgeUpdate.lastError) && <p role="status">{status.lastError || status.knowledgeUpdate.lastError}</p>}
    {error && <p role="alert">{error}</p>}
    {saved && <p role="status">Recommendation settings saved.</p>}
    <button type="button" disabled={busy} onClick={() => void save()}>{busy ? "Saving…" : "Save recommendation settings"}</button>
    <InferenceTrustSettings />
  </div>;
}
