export interface DiffLine {
  kind: "context" | "addition" | "deletion" | "hunk" | "metadata";
  text: string;
  oldLine: number | null;
  newLine: number | null;
}

/** Interpret only the captured patch; no filesystem or Git lookups. */
export function parseGitPatch(patch: string): DiffLine[] {
  if (!patch) return [];
  const lines = patch.split("\n");
  if (lines.at(-1) === "") lines.pop();
  let oldLine: number | null = null;
  let newLine: number | null = null;
  return lines.map(raw => {
    const text = raw.endsWith("\r") ? raw.slice(0, -1) : raw;
    const hunk = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(text);
    if (hunk) {
      oldLine = Number(hunk[1]); newLine = Number(hunk[2]);
      return {kind: "hunk", text, oldLine: null, newLine: null};
    }
    if (text.startsWith("diff --git ")) { oldLine = null; newLine = null; }
    if (oldLine === null && newLine === null && (text.startsWith("+++") || text.startsWith("---"))) {
      return {kind: "metadata", text, oldLine: null, newLine: null};
    }
    switch (text[0]) {
      case " ":
        return {kind: "context", text: text.slice(1),
          oldLine: oldLine === null ? null : oldLine++,
          newLine: newLine === null ? null : newLine++};
      case "+":
        return {kind: "addition", text: text.slice(1), oldLine: null,
          newLine: newLine === null ? null : newLine++};
      case "-":
        return {kind: "deletion", text: text.slice(1),
          oldLine: oldLine === null ? null : oldLine++, newLine: null};
      default:
        return {kind: "metadata", text, oldLine: null, newLine: null};
    }
  });
}

export interface SplitDiffRow { left: DiffLine | null; right: DiffLine | null; full?: DiffLine; leftNote?: string; rightNote?: string }

export function splitDiffRows(lines: DiffLine[]): SplitDiffRow[] {
  const result: SplitDiffRow[] = [];
  let removed: DiffLine[] = [], added: DiffLine[] = [];
  const notes = new Map<DiffLine, string>();
  const flush = () => {
    for (let i = 0; i < Math.max(removed.length, added.length); i++)
      result.push({left: removed[i] ?? null, right: added[i] ?? null,
        leftNote: notes.get(removed[i]), rightNote: notes.get(added[i])});
    removed = []; added = [];
  };
  for (const line of lines) {
    if (line.kind === "deletion") { removed.push(line); continue; }
    if (line.kind === "addition") { added.push(line); continue; }
    if (line.kind === "metadata" && line.text.startsWith("\\ No newline at end of file")) {
      const pending = added.at(-1) ?? removed.at(-1);
      if (pending) { notes.set(pending, line.text); continue; }
      const previous = result.at(-1);
      if (previous?.left?.kind === "context") {
        previous.leftNote = line.text; previous.rightNote = line.text; continue;
      }
    }
    flush();
    result.push(line.kind === "context" ? {left: line, right: line} : {left: null, right: null, full: line});
  }
  flush();
  return result;
}

export interface DiffToken { kind: "plain" | "keyword" | "string" | "comment" | "number"; text: string }

/** Basic, line-local coloring. Source is always rendered as React text. */
export function tokenizeDiffLine(text: string): DiffToken[] {
  const pattern = /\/\/.*$|#[^\n]*$|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`|\b(?:const|let|var|return|function|export|import|from|if|else|true|false|null|undefined|async|await|class|interface|type|def|fn|pub|use|impl|match|struct)\b|\b\d+(?:\.\d+)?\b/g;
  const tokens: DiffToken[] = [];
  let offset = 0;
  for (const match of text.matchAll(pattern)) {
    if (match.index > offset) tokens.push({kind: "plain", text: text.slice(offset, match.index)});
    const value = match[0];
    const kind = value.startsWith("//") || value.startsWith("#") ? "comment"
      : /^["'`]/.test(value) ? "string" : /^\d/.test(value) ? "number" : "keyword";
    tokens.push({kind, text: value}); offset = match.index + value.length;
  }
  if (offset < text.length) tokens.push({kind: "plain", text: text.slice(offset)});
  return tokens;
}
