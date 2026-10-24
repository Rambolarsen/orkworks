import { useEffect, useRef, useState } from "react";
import type { SessionInfo } from "../api";
import type { GitChangeBases, GitChangesCapture, GitComparison, GitFilePatch } from "../gitChangesTypes";

const message = (e: unknown) => e instanceof Error ? e.message : "Changes unavailable; retry.";
const short = (oid: string | null) => oid?.slice(0, 8) ?? "empty tree";

export default function GitChangesPanel({ session, onBack }: { session: SessionInfo | null; onBack: () => void }) {
  // The shell keys this component by workspace generation and selected-session ID.
  const [comparison, setComparison] = useState<GitComparison>("uncommitted");
  const [bases, setBases] = useState<GitChangeBases | null>(null);
  const [baseError, setBaseError] = useState<string | null>(null);
  const [baseRef, setBaseRef] = useState("");
  const [refresh, setRefresh] = useState(0);
  const [state, setState] = useState<{ key: string; capture: GitChangesCapture | null; error: string | null; loading: boolean } | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [patch, setPatch] = useState<{ key: string; result?: GitFilePatch; error?: string } | null>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const fileHeading = useRef<HTMLHeadingElement>(null);
  const comparisonBase = comparison === "branch" ? baseRef : "";
  const key = `${comparison}:${comparisonBase}`;
  const current = state?.key === key ? state : null;
  const capture = current?.capture;
  const file = capture?.files.find(f => f.path === selected) ?? capture?.files[0];
  const patchKey = capture && file ? `${capture.snapshotId}:${file.id}` : "";

  useEffect(() => { heading.current?.focus({ preventScroll: true }); }, []);
  useEffect(() => {
    if (!session) return;
    let active = true;
    window.orkworks.readGitChanges({ sessionId: session.id, kind: "bases" }).then(result => {
      if (!active) return;
      const b = result as GitChangeBases;
      setBases(b); setBaseRef(old => old || b.defaultRef || ""); setBaseError(null);
    }).catch(e => { if (active) setBaseError(message(e)); });
    return () => { active = false; };
  }, [session?.id, refresh]);

  useEffect(() => {
    if (!session || (comparison === "branch" && !comparisonBase)) return;
    let active = true;
    setState(old => ({ key, capture: old?.key === key ? old.capture : null, error: null, loading: true }));
    window.orkworks.readGitChanges({ sessionId: session.id, kind: "capture", comparison,
      ...(comparison === "branch" ? { baseRef: comparisonBase } : {}) }).then(result => {
      if (!active) return;
      const c = result as GitChangesCapture;
      if (c.sessionId !== session.id || c.comparison !== comparison || (comparison === "branch" && c.baseRef !== comparisonBase)) throw new Error("Inspection source changed; refresh.");
      setState({ key, capture: c, error: null, loading: false });
      setSelected(old => {
        if (old && c.files.some(f => f.path === old)) return old;
        if (old) fileHeading.current?.focus({ preventScroll: true });
        return c.files[0]?.path ?? null;
      });
    }).catch(e => {
      if (!active) return;
      const error = message(e);
      setState(old => ({ key, capture: /source changed|Session unavailable|No available|Worktree unavailable/i.test(error) ? null : old?.key === key ? old.capture : null, error, loading: false }));
    });
    return () => { active = false; };
  }, [session?.id, comparison, comparisonBase, refresh, key]);

  useEffect(() => {
    if (!session || !capture || !file) return;
    let active = true;
    setPatch(null);
    if (file.notice) { setPatch({ key: patchKey, result: { snapshotId: capture.snapshotId, fileId: file.id, patch: "", notice: file.notice } }); return; }
    window.orkworks.readGitChanges({ sessionId: session.id, kind: "file", snapshotId: capture.snapshotId, fileId: file.id }).then(result => {
      if (!active) return;
      const p = result as GitFilePatch;
      if (p.snapshotId !== capture.snapshotId || p.fileId !== file.id) throw new Error("Inspection expired; refresh.");
      setPatch({ key: patchKey, result: p });
    }).catch(e => { if (active) setPatch({ key: patchKey, error: message(e) }); });
    return () => { active = false; };
  }, [session?.id, patchKey]);

  const choose = (value: GitComparison) => { setComparison(value); setSelected(null); };
  const text = patch?.key === patchKey ? patch : null;
  return <section className="git-changes" aria-label="Git changes">
    <button type="button" data-git-back onClick={onBack}>Back to Details</button>
    <h3 ref={heading} tabIndex={-1}>Changes · {session?.label ?? "No selected session"}</h3>
    <p className="git-changes-subject">{capture?.worktree ?? session?.repoRoot ?? session?.cwd}<br />{capture?.branch ?? session?.branch}</p>
    <label>Comparison<select data-git-comparison value={comparison} onChange={e => choose(e.target.value as GitComparison)}>
      <option value="uncommitted">Uncommitted changes</option><option value="branch">Branch changes</option>
    </select></label>
    {comparison === "branch" && <>
      <label>Base branch<select value={baseRef} onChange={e => { setBaseRef(e.target.value); setSelected(null); }}>
        <option value="">Choose a base branch</option>{bases?.refs.map(ref => <option key={ref} value={ref}>{ref.replace(/^refs\/(heads|remotes)\//, "")}</option>)}
      </select></label>
      <p>Local refs; no fetch performed.{bases?.limited && " Ref list limited to 500 entries."}</p>
      {bases?.unavailableReason && <p role="status">{bases.unavailableReason}</p>}
      {baseError && <p role="alert">{baseError}</p>}
    </>}
    <p className="git-changes-baseline">{comparison === "uncommitted" ? "HEAD → working tree · staged, unstaged, new files" : `merge-base(${baseRef.replace(/^refs\/(heads|remotes)\//, "") || "choose base"}, HEAD) → HEAD · committed only`}</p>
    {capture && <p>{short(capture.baselineOid)} → {comparison === "branch" ? short(capture.headOid) : "working tree"}{comparison === "branch" && ` · base tip ${short(capture.baseOid)}`}</p>}
    <div className="git-changes-refresh"><span>{capture ? `Updated ${new Date(capture.capturedAt).toLocaleTimeString()}` : "No capture yet"}</span>
      <button type="button" data-git-refresh disabled={!session || current?.loading} onClick={() => setRefresh(n => n + 1)}>Refresh</button></div>
    <p className="git-changes-caveat">Worktree and branch changes may include other sessions’ work. Worktree reads describe a capture interval.</p>
    {current?.loading && <p role="status">Loading changes…</p>}
    {current?.error && <p role="alert">{current.error}{capture && ` Showing capture from ${new Date(capture.capturedAt).toLocaleTimeString()}.`}</p>}
    {capture?.notice && <p role="status">{capture.notice}</p>}
    {capture && <>
      <h4 ref={fileHeading} tabIndex={-1}>Changed files · {capture.files.length}{capture.partial ? " shown; totals unavailable" : ` · +${capture.files.reduce((n,f) => n+(f.additions??0),0)} −${capture.files.reduce((n,f) => n+(f.deletions??0),0)}`}</h4>
      {!capture.files.length && <p>{capture.partial ? "Comparison incomplete; refresh or inspect in your coding tool." : comparison === "branch" ? "No committed branch changes since the merge base." : "No net uncommitted changes against HEAD."}</p>}
      <ul className="git-changes-files">{capture.files.map(f => <li key={f.id}><button type="button" aria-pressed={f.id === file?.id} onClick={() => setSelected(f.path)}>
        <span>{f.status}</span> {f.oldPath ? `${f.oldPath} → ${f.path}` : f.path}{f.additions !== null && ` · +${f.additions} −${f.deletions}`}
      </button></li>)}</ul>
      {file && <div className="git-changes-file"><h4>{file.path}</h4>
        {!text && <p role="status">Loading patch…</p>}{text?.error && <p role="alert">{text.error}</p>}
        {text?.result?.notice && <p>{text.result.notice}</p>}
        {text?.result?.patch && <pre className="git-changes-patch" tabIndex={0} aria-label={`Unified diff for ${file.path}`}>{text.result.patch}</pre>}
      </div>}
    </>}
  </section>;
}
