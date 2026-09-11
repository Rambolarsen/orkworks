import { useEffect, useRef, useState } from "react";
import { inferenceTrustActions, type InferenceAdapterView } from "../inferenceTrust";

export default function InferenceTrustSettings() {
  const [adapters, setAdapters] = useState<InferenceAdapterView[]>([]);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const revision = useRef(0);
  useEffect(() => {
    const current = ++revision.current;
    window.orkworks.getInferenceTrust().then((views) => {
      if (current === revision.current) setAdapters(views);
    }).catch((reason: unknown) => { if (current === revision.current) setError(String(reason)); })
      .finally(() => { if (current === revision.current) setBusy(false); });
    return () => { revision.current += 1; };
  }, []);
  async function update(adapter?: InferenceAdapterView, approve = false) {
    const current = ++revision.current;
    setBusy(true); setError(null); setMessage(null);
    try {
      if (adapter) {
        const request = { harnessId: adapter.id, expectedRevision: adapter.revision };
        const accepted = approve ? await window.orkworks.approveInferenceAdapter(request)
          : (await window.orkworks.revokeInferenceAdapter(request), true);
        if (current === revision.current) setMessage(approve ? (accepted ? "Executable approved. Custom execution remains inactive." : "Approval cancelled.") : "Executable approval revoked.");
      }
      const views = await window.orkworks.getInferenceTrust();
      if (current === revision.current) setAdapters(views);
    } catch (reason) {
      if (current === revision.current) { setAdapters([]); setError(String(reason)); }
    } finally { if (current === revision.current) setBusy(false); }
  }
  return <section aria-labelledby="inference-trust-heading">
    <h4 id="inference-trust-heading">Custom inference executables</h4>
    <p className="settings-section-copy">Defined through coding-tool JSON, independently of Peon. Approval is global and does not enable execution in this version.</p>
    <p className="settings-section-copy">An approved executable receives permitted context and existing CLI credential access. It may run hooks or plugins and is not sandboxed by OrkWorks. Review the native confirmation before granting trust.</p>
    {adapters.map((adapter) => {
      const actions = inferenceTrustActions(adapter, busy);
      return <fieldset key={adapter.id}>
        <legend>{adapter.name}</legend>
        <p role="status">{adapter.state === "approved" ? "Approved — execution inactive" : adapter.state === "approval_required" ? "Approval required" : "Executable unavailable or coding tool retired"}</p>
        <dl className="recommendation-facts">
          <div><dt>Executable</dt><dd>{adapter.definition.command}</dd></div>
          <div><dt>Resolved path</dt><dd>{adapter.resolvedPath ?? "Unavailable"}</dd></div>
          <div><dt>Input / timeout</dt><dd>{adapter.definition.input} / {adapter.definition.timeoutSecs ?? 60} seconds</dd></div>
        </dl>
        <details><summary>Argument templates</summary><pre style={{ whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}>{JSON.stringify({ args: adapter.definition.args, reasoningEffortArgs: adapter.definition.reasoningEffortArgs ?? [] }, null, 2)}</pre></details>
        <button type="button" disabled={!actions.canApprove} onClick={() => void update(adapter, true)}>Review and approve executable</button>{" "}
        <button type="button" disabled={!actions.canRevoke} onClick={() => void update(adapter)}>Revoke approval</button>
      </fieldset>;
    })}
    {!busy && !error && adapters.length === 0 && <p>No custom inference definitions configured.</p>}
    {error && <p role="alert">{error}</p>}
    {message && <p role="status">{message}</p>}
    <button type="button" disabled={busy} onClick={() => void update()}>{busy ? "Loading…" : "Refresh executable approvals"}</button>
  </section>;
}
