import type { IBufferLine, ILinkHandler, ILinkProvider, Terminal } from "@xterm/xterm";
import { pushToast } from "./feedback.ts";

export function terminalLinkHandler(openExternal: (url: string) => Promise<void>): ILinkHandler {
  return {
    activate(_event, url) {
      void openExternal(url).catch((error) => console.error("[terminal] couldn't open external link", error));
    },
  };
}

const PLAN_PATH = /(?:(?<![A-Za-z0-9_.~\/\\-])~\/(?:[^\r\n/]+\/)*(?:docs\/[ \t]*superpowers\/[ \t]*(?:plans|specs)|specs)\/[^\r\n]*?\.md|(?<![A-Za-z0-9_.~\/\\-])~\\(?:[^\r\n\\]+\\)*(?:docs\\[ \t]*superpowers\\[ \t]*(?:plans|specs)|specs)\\[^\r\n]*?\.md|(?<![A-Za-z0-9_.~-])\/(?:[^\r\n/]+\/)*(?:docs\/[ \t]*superpowers\/[ \t]*(?:plans|specs)|specs)\/[^\r\n]*?\.md|[A-Za-z]:\\(?:[^\r\n\\]+\\)*(?:docs\\[ \t]*superpowers\\[ \t]*(?:plans|specs)|specs)\\[^\r\n]*?\.md|(?<![\\/])(?:docs\/[ \t]*superpowers\/[ \t]*(?:plans|specs)|specs)\/[^\r\n]*?\.md|(?<![\\/])(?:docs\\[ \t]*superpowers\\[ \t]*(?:plans|specs)|specs)\\[^\r\n]*?\.md)\b/g;

function normalizePlanPath(path: string): string {
  return path.replace(/\/[ \t]+/g, "/").replace(/\\[ \t]+/g, "\\");
}

export function terminalPlanPaths(line: string): string[] {
  return [...line.matchAll(PLAN_PATH)].map((match) => normalizePlanPath(match[0]));
}

function columnForTextOffset(line: IBufferLine, offset: number): number {
  let textOffset = 0;
  for (let column = 0; column < line.length; column += 1) {
    const cell = line.getCell(column);
    if (!cell || cell.getWidth() === 0) continue;
    const text = cell.getChars() || " ";
    if (offset === textOffset) return column + 1;
    if (offset <= textOffset + text.length) return column + 1 + cell.getWidth();
    textOffset += text.length;
  }
  return line.length + 1;
}

function endColumnForTextOffset(line: IBufferLine, offset: number): number {
  const targetOffset = offset - 1;
  let textOffset = 0;
  let lastEndColumn = 1;
  for (let column = 0; column < line.length; column += 1) {
    const cell = line.getCell(column);
    if (!cell || cell.getWidth() === 0) continue;
    const text = cell.getChars() || " ";
    if (targetOffset < textOffset + text.length) return column + cell.getWidth();
    textOffset += text.length;
    lastEndColumn = column + cell.getWidth();
  }
  return lastEndColumn;
}

// Bounds how many wrapped continuation rows a single logical line can span.
// Legacy `.terminal` replay files predate the 1,000-line/1 MiB retention cap
// (see AGENTS.md) and can still hold tens of megabytes of unbroken output;
// without a cap, an un-newlined blob turns this scan into an unbounded walk
// of the whole buffer on every call. No real plan-path match needs anywhere
// near this many wrapped rows.
const MAX_LOGICAL_LINE_ROWS = 200;

function isHardAbsolutePlanContinuation(prefix: string, nextText: string): boolean {
  const absolutePlanPrefix = /(?:^|\s)\/(?:[^\r\n/]+\/)*docs\/(?:superpowers\/)?$/.test(prefix);
  const planRootContinuation = /^(?:superpowers\/)?(?:plans|specs)\//.test(nextText.trimStart());
  return absolutePlanPrefix && planRootContinuation;
}

// Harnesses rewrap their own output with hard newlines plus an indentation
// (Claude Code positions continuation rows a couple of columns in), so a
// long plan path can be split across rows that are not marked `isWrapped`.
// A row is an indent-continuation candidate when it begins with that indent.
function isIndentedRow(text: string): boolean {
  return /^[ \t]{2,}\S/.test(text);
}

// Characters that can appear inside a path token. A run of these at the end
// of the accumulated text is a path fragment that may continue on the next
// indented row.
const PATH_TAIL_RUN = /[A-Za-z0-9_.~/\\:-]+$/;

// The accumulated text's trailing path-chars run is an "open" path fragment:
// a path that may still continue on the next indented row. A trailing run
// ending in `.md` is a complete match ending exactly at the prefix end —
// joining further would only concatenate unrelated content into a bogus
// longer match, so that shape is closed.
function openPlanPathTail(prefix: string): boolean {
  const run = PATH_TAIL_RUN.exec(prefix)?.[0] ?? "";
  return run.length > 0 && !/\.md$/.test(run);
}

// A genuine harness rewrap fills the row it breaks (the token overflowed the
// width), while an ordinary indented line break leaves a short row. Requiring
// a full row keeps natural indented line breaks (separate bullets, wrapped
// prose) from being joined into one logical line and corrupted into a bogus
// cross-boundary match.
function extendsFullWidthRow(rawLength: number, cols: number): boolean {
  return rawLength >= cols;
}

function logicalLine(terminal: Terminal, y: number): Array<{ y: number; line: IBufferLine; text: string; skip: number }> {
  const textOf = (line: IBufferLine): string => line.translateToString(true);
  let start = y;
  let backSteps = 0;
  while (start > 1 && backSteps < MAX_LOGICAL_LINE_ROWS) {
    const previous = terminal.buffer.active.getLine(start - 2);
    const current = terminal.buffer.active.getLine(start - 1);
    if (!previous || !current) break;
    const previousText = textOf(previous);
    const currentText = textOf(current);
    if (!current.isWrapped && !isHardAbsolutePlanContinuation(previousText, currentText)
      && !(isIndentedRow(currentText) && extendsFullWidthRow(previousText.length, terminal.cols)
        && openPlanPathTail(previousText))) break;
    start -= 1;
    backSteps += 1;
  }
  const lines = [];
  let joinedIndented = false;
  for (let current = start; current - start < MAX_LOGICAL_LINE_ROWS; current += 1) {
    const line = terminal.buffer.active.getLine(current - 1);
    if (!line) break;
    const raw = textOf(line);
    let skip = 0;
    if (lines.length > 0 && joinedIndented) {
      // A row reached across a hard break by the indent rule is a harness
      // continuation row: strip the injected indent so the joined text
      // reconstructs the printed path exactly. Soft wrapped rows and the
      // `/docs/` hard-break shape keep their full text; leading whitespace
      // there is real content (e.g. a space inside a wrapped path).
      skip = isIndentedRow(raw) ? raw.length - raw.trimStart().length : 0;
    }
    lines.push({ y: current, line, text: raw.slice(skip), skip });
    const next = terminal.buffer.active.getLine(current);
    if (!next) break;
    const nextText = textOf(next);
    if (next.isWrapped) {
      joinedIndented = false;
      continue;
    }

    // Some harness output includes a real newline while printing a long
    // absolute path. Treat the narrow `/docs/` -> `superpowers/...` shape as
    // one path so the later relative `specs/...` fragment is not selected on
    // its own. Ordinary hard line breaks remain separate.
    const prefix = lines.map((part) => part.text).join("");
    if (isHardAbsolutePlanContinuation(prefix, nextText)) {
      joinedIndented = false;
      continue;
    }
    // An indented row continues an open path fragment — but only across a
    // genuine wrap (the row being extended is full-width): join it with the
    // indent stripped so the reconstructed text stays a valid path.
    const lastRow = lines.at(-1);
    const lastRowFull = lastRow !== undefined
      && extendsFullWidthRow(lastRow.text.length + lastRow.skip, terminal.cols);
    if (isIndentedRow(nextText) && lastRowFull && openPlanPathTail(prefix)) {
      joinedIndented = true;
      continue;
    }
    break;
  }
  return lines;
}

export function createTerminalPlanLinkProvider(
  terminal: Terminal,
  onPlanPath: (path: string) => Promise<void>,
): ILinkProvider {
  return {
    provideLinks(y, callback) {
      const lines = logicalLine(terminal, y);
      const text = lines.map((part) => part.text).join("");
      const linksForLine = [...text.matchAll(PLAN_PATH)].flatMap((match) => {
        const rawPath = match[0];
        const path = normalizePlanPath(rawPath);
        const startOffset = match.index ?? 0;
        const endOffset = startOffset + rawPath.length;
        let consumed = 0;
        let start: { x: number; y: number } | undefined;
        let end: { x: number; y: number } | undefined;
        for (const part of lines) {
          const partStart = consumed;
          const partEnd = consumed + part.text.length;
          consumed = partEnd;
          const segmentStart = Math.max(startOffset, partStart);
          const segmentEnd = Math.min(endOffset, partEnd);
          if (segmentStart >= segmentEnd) continue;
          // Offsets are relative to the row's stripped text; add back the
          // indent cells removed from a hard-continuation row so the column
          // math maps into the real buffer row.
          start ??= { x: columnForTextOffset(part.line, part.skip + segmentStart - partStart), y: part.y };
          end = { x: endColumnForTextOffset(part.line, part.skip + segmentEnd - partStart), y: part.y };
        }
        if (!start || !end || y < start.y || y > end.y) return [];
        return [{
          text: path,
          range: { start, end },
          activate: () => {
            void onPlanPath(path).catch((error) => {
              console.error("[terminal] couldn't select plan", error);
              pushToast("error", error instanceof Error ? error.message : "Couldn't open this plan.");
            });
          },
        }];
      });
      callback(linksForLine.length ? linksForLine : undefined);
    },
  };
}
