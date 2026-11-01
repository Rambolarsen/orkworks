import { useMemo, useState } from "react";
import { parseGitPatch, splitDiffRows, tokenizeDiffLine, type DiffLine } from "../gitDiff";

export default function GitDiff({ patch, path, mode }: {patch: string; path: string; mode: "unified" | "split"}) {
  const [visibleRows, setVisibleRows] = useState(1000);
  const lines = useMemo(() => parseGitPatch(patch), [patch]);
  const split = useMemo(() => mode === "split" ? splitDiffRows(lines) : [], [lines, mode]);
  const code = (line: DiffLine) => <code>{tokenizeDiffLine(line.text).map((token, i) =>
    <span key={i} className={`git-token-${token.kind}`}>{token.text}</span>)}</code>;
  const number = (value: number | null) => <span className="git-diff-number" aria-hidden="true">{value}</span>;
  const sign = (line: DiffLine) => <span className="git-diff-sign" aria-hidden="true">{line.kind === "addition" ? "+" : line.kind === "deletion" ? "−" : " "}</span>;
  const annotation = (line: DiffLine, key: number) => <div key={key} className={`git-diff-annotation git-diff-${line.kind}`}><code>{line.text}</code></div>;
  const cell = (line: DiffLine | null, side: "left" | "right", note?: string) => <div className={`git-diff-cell git-diff-${line?.kind ?? "empty"}`}>
    {number(line ? side === "left" ? line.oldLine : line.newLine : null)}
    {line && <><span className="sr-only">{line.kind === "addition" ? "Added: " : line.kind === "deletion" ? "Removed: " : ""}</span>{sign(line)}{code(line)}</>}
    {note && <span className="git-diff-note">{note}</span>}
  </div>;
  const rowCount = mode === "unified" ? lines.length : split.length;
  if (!patch) return null;
  return <><div className={`git-changes-patch git-diff--${mode}`} tabIndex={0} role="region" aria-label={`${mode === "split" ? "Split" : "Unified"} diff for ${path}`}>
    {mode === "split" && <div className="git-diff-sides"><span>Before</span><span>After</span></div>}
    {mode === "unified" ? lines.slice(0, visibleRows).map((line, i) => line.kind === "metadata" || line.kind === "hunk" ? annotation(line, i) :
      <div key={i} className={`git-diff-line git-diff-${line.kind}`}>
        {number(line.oldLine)}{number(line.newLine)}{sign(line)}
        <span className="sr-only">{line.kind === "addition" ? "Added: " : line.kind === "deletion" ? "Removed: " : ""}</span>{code(line)}
      </div>) : split.slice(0, visibleRows).map((row, i) => row.full ? annotation(row.full, i) :
      <div className="git-diff-pair" key={i}>{cell(row.left, "left", row.leftNote)}{cell(row.right, "right", row.rightNote)}</div>)}
  </div>{rowCount > visibleRows && <div className="git-diff-more">
    <p role="status">Showing {visibleRows.toLocaleString("en-US")} of {rowCount.toLocaleString("en-US")} diff rows.</p>
    <button type="button" data-diff-more onClick={() => setVisibleRows(n => n + 1000)}>Show next {Math.min(1000, rowCount - visibleRows).toLocaleString("en-US")}</button>
  </div>}</>;
}
