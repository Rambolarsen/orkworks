import { gitLocationLabel } from "../gitLocationLabel";
import { useEffect, useRef, useState } from "react";
import type { SessionInfo } from "../api";
import type { GitChangeBases, GitChangesCapture, GitComparison, GitFilePatch } from "../gitChangesTypes";
import { RefreshCw, GitBranch, Search } from "lucide-react";
import GitDiff from "./GitDiff";

const message = (e: unknown) => e instanceof Error ? e.message : "Changes unavailable; retry.";
const short = (oid: string | null) => oid?.slice(0, 8) ?? "empty tree";

export default function GitChangesPanel({ session }: { session: SessionInfo | null }) {
  // The shell keys this component by workspace generation and selected-session ID.
  const [comparison, setComparison] = useState<GitComparison>("uncommitted");
  const [bases, setBases] = useState<GitChangeBases | null>(null);
  const [baseError, setBaseError] = useState<string | null>(null);
  const [baseRef, setBaseRef] = useState("");
  const [refresh, setRefresh] = useState(0);
  const [filter, setFilter] = useState("");
  const [diffMode, setDiffMode] = useState<"unified" | "split">("unified");
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
  const visibleFiles = capture?.files.filter(f => `${f.oldPath ?? ""} ${f.path}`.toLowerCase().includes(filter.toLowerCase())) ?? [];
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
  const name = (path: string) => path.split("/").at(-1) || path;
  const folder = (path: string) => path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "Repository root";
  const totals = (added: number, removed: number) => <span className="git-changes-totals"><span className="git-added">+{added}</span> <span className="git-removed">−{removed}</span></span>;
  return <section className="git-changes" aria-label="Git changes">
    <header className="git-changes-header"><div><h2 ref={heading} data-shell-page-heading tabIndex={-1}>Changes</h2>
      <p className="git-changes-branch"><GitBranch size={14} aria-hidden="true" />{capture?.branch ?? session?.branch ?? "No branch"}</p></div>
    </header>
    <div className="git-changes-toolbar">
    <label>Comparison<select data-git-comparison value={comparison} onChange={e => choose(e.target.value as GitComparison)}>
      <option value="uncommitted">Uncommitted changes</option><option value="branch">Branch changes</option>
    </select></label>
    {comparison === "branch" &&
      <label>Base branch<select value={baseRef} onChange={e => { setBaseRef(e.target.value); setSelected(null); }}>
        <option value="">Choose a base branch</option>{bases?.refs.map(ref => <option key={ref} value={ref}>{ref.replace(/^refs\/(heads|remotes)\//, "")}</option>)}
      </select></label>}
      <div className="git-changes-refresh"><span>{capture ? `Updated ${new Date(capture.capturedAt).toLocaleTimeString()}` : "No capture yet"}</span>
        <button type="button" data-git-refresh disabled={!session || current?.loading} onClick={() => setRefresh(n => n + 1)}><RefreshCw size={14} aria-hidden="true" />Refresh</button></div>
    </div>
    {comparison === "branch" && <div className="git-changes-local-refs">Local refs; no fetch performed.{bases?.limited && " Ref list limited to 500 entries."}</div>}
    <details className="git-changes-details"><summary>Comparison details</summary>
      <dl><dt>Worktree</dt><dd className="git-changes-subject">{capture?.worktree ?? session?.worktreeBinding?.root ?? session?.repoRoot ?? session?.cwd}</dd>
      <dt>Source</dt><dd>{gitLocationLabel(capture ? capture.gitContextSource : session?.gitContextSource)}</dd>
      <dt>Baseline</dt><dd className="git-changes-baseline">{comparison === "uncommitted" ? "HEAD → working tree · staged, unstaged, new files" : `merge-base(${baseRef.replace(/^refs\/(heads|remotes)\//, "") || "choose base"}, HEAD) → HEAD · committed only`}</dd>
      {capture && <><dt>Commits</dt><dd>{short(capture.baselineOid)} → {comparison === "branch" ? short(capture.headOid) : "working tree"}{comparison === "branch" && ` · base tip ${short(capture.baseOid)}`}</dd></>}
      <dt>Scope</dt><dd>Worktree and branch changes may include other sessions’ work. Worktree reads describe a capture interval.</dd></dl>
    </details>
    <div className="git-changes-status">
      {!session && <p role="status">Select a session to inspect its changes.</p>}
      {comparison === "branch" && <>
      {bases?.unavailableReason && <p role="status">{bases.unavailableReason}</p>}
      {baseError && <p role="alert">{baseError}</p>}
      </>}
    {current?.loading && <p role="status">Loading changes…</p>}
    {current?.error && <p role="alert">{current.error}{capture && ` Showing capture from ${new Date(capture.capturedAt).toLocaleTimeString()}.`}</p>}
    {capture?.notice && <p role="status">{capture.notice}</p>}
    {capture?.partial && !capture.notice && <p role="status">Partial comparison; some content and totals are unavailable.</p>}
    </div>
    {capture && <div className="git-changes-browser">
      <nav className="git-changes-file-list" aria-label="Changed files"><div className="git-changes-file-list-header">
      <h3 ref={fileHeading} tabIndex={-1}>Changed files <span>{capture.files.length}{capture.partial ? " shown" : ""}</span></h3>
      {capture.partial ? <span className="git-changes-count">Totals unavailable</span> : totals(capture.files.reduce((n,f) => n+(f.additions??0),0),capture.files.reduce((n,f) => n+(f.deletions??0),0))}</div>
      <label className="git-changes-filter"><Search size={14} aria-hidden="true" /><span className="sr-only">Filter changed files</span><input type="search" placeholder="Filter files…" value={filter} onChange={e => setFilter(e.target.value)} /></label>
      {!capture.files.length && <p>{capture.partial ? "Comparison incomplete; refresh or inspect in your coding tool." : comparison === "branch" ? "No committed branch changes since the merge base." : "No net uncommitted changes against HEAD."}</p>}
      {!!capture.files.length && !visibleFiles.length && <p role="status">No files match “{filter}”.</p>}
      <ul className="git-changes-files">{visibleFiles.map(f => <li key={f.id}><button type="button" aria-pressed={f.id === file?.id}
        aria-label={`${f.status}: ${f.oldPath ? `${f.oldPath} → ` : ""}${f.path}${f.additions !== null ? `, +${f.additions} −${f.deletions}` : ""}`} onClick={() => setSelected(f.path)}>
        <span className="git-changes-file-status" title={f.status} aria-hidden="true">{f.status.slice(0,1)}</span>
        <span className="git-changes-file-name"><strong>{name(f.path)}</strong><small>{f.oldPath ? `${f.oldPath} → ${f.path}` : folder(f.path)}</small></span>
        {f.additions !== null && totals(f.additions,f.deletions ?? 0)}
      </button></li>)}</ul>
      </nav>
      <section className="git-changes-file" aria-label="Selected file diff">{file ? <>
        <header className="git-changes-file-header"><div><h3>{name(file.path)}</h3><p>{file.oldPath ? `${file.oldPath} → ${file.path}` : file.path}</p></div>
          {file.additions !== null && totals(file.additions,file.deletions ?? 0)}
          <div className="git-changes-mode" role="group" aria-label="Diff layout">{(["unified","split"] as const).map(mode => <button key={mode} type="button" data-diff-mode={mode} aria-pressed={diffMode === mode} onClick={() => setDiffMode(mode)}>{mode === "unified" ? "Unified" : "Split"}</button>)}</div>
        </header>
        <div className="git-changes-file-content">
        {!text && <p role="status">Loading patch…</p>}{text?.error && <p role="alert">{text.error}</p>}
        {text?.result?.notice && <p>{text.result.notice}</p>}
        {text?.result?.patch && <GitDiff patch={text.result.patch} path={file.path} mode={diffMode} />}
        {text?.result && !text.result.patch && !text.result.notice && <p>No text patch for this change.</p>}
        </div>
      </> : <p>Select a changed file to inspect its diff.</p>}</section>
    </div>}
  </section>;
}
