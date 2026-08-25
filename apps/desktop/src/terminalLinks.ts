import type { IBufferLine, ILinkHandler, ILinkProvider, Terminal } from "@xterm/xterm";
import { pushToast } from "./feedback.ts";

export function terminalLinkHandler(openExternal: (url: string) => Promise<void>): ILinkHandler {
  return {
    activate(_event, url) {
      void openExternal(url).catch((error) => console.error("[terminal] couldn't open external link", error));
    },
  };
}

const PLAN_PATH = /(?:\/(?:[^\r\n/]+\/)*(?:docs\/[ \t]*(?:superpowers\/[ \t]*)?(?:plans|specs)|specs)\/[^\r\n]*?\.md|[A-Za-z]:\\(?:[^\r\n\\]+\\)*(?:docs\\[ \t]*(?:superpowers\\[ \t]*)?(?:plans|specs)|specs)\\[^\r\n]*?\.md|(?:docs\/[ \t]*(?:superpowers\/[ \t]*)?(?:plans|specs)|specs)\/[^\r\n]*?\.md|(?:docs\\[ \t]*(?:superpowers\\[ \t]*)?(?:plans|specs)|specs)\\[^\r\n]*?\.md)\b/g;

export function terminalPlanPaths(line: string): string[] {
  return [...line.matchAll(PLAN_PATH)].map((match) => match[0].replace(/\/[ \t]+/g, "/"));
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

function partAtOffset(
  lines: Array<{ y: number; line: IBufferLine; text: string }>,
  offset: number,
  endBoundary = false,
): { part: { y: number; line: IBufferLine; text: string }; offset: number } | undefined {
  let consumed = 0;
  for (const part of lines) {
    if (offset < consumed + part.text.length || (endBoundary && offset === consumed + part.text.length)) {
      return { part, offset: offset - consumed };
    }
    consumed += part.text.length;
  }
  return undefined;
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

function logicalLine(terminal: Terminal, y: number): Array<{ y: number; line: IBufferLine; text: string }> {
  let start = y;
  let backSteps = 0;
  while (start > 1 && backSteps < MAX_LOGICAL_LINE_ROWS) {
    const previous = terminal.buffer.active.getLine(start - 2);
    const current = terminal.buffer.active.getLine(start - 1);
    if (!previous || !current || (!current.isWrapped && !isHardAbsolutePlanContinuation(
      previous.translateToString(true),
      current.translateToString(true),
    ))) break;
    start -= 1;
    backSteps += 1;
  }
  const lines = [];
  for (let current = start; current - start < MAX_LOGICAL_LINE_ROWS; current += 1) {
    const line = terminal.buffer.active.getLine(current - 1);
    if (!line) break;
    lines.push({ y: current, line, text: line.translateToString(true) });
    const next = terminal.buffer.active.getLine(current);
    if (!next) break;
    if (next.isWrapped) continue;

    // Some harness output includes a real newline while printing a long
    // absolute path. Treat the narrow `/docs/` -> `superpowers/...` shape as
    // one path so the later relative `specs/...` fragment is not selected on
    // its own. Ordinary hard line breaks remain separate.
    const prefix = lines.map((part) => part.text).join("");
    const nextText = next.translateToString(true);
    if (!isHardAbsolutePlanContinuation(prefix, nextText)) break;
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
      const links = [...text.matchAll(PLAN_PATH)].map((match) => {
        const rawPath = match[0];
        const path = rawPath.replace(/\/[ \t]+/g, "/");
        const startOffset = match.index ?? 0;
        const endOffset = startOffset + rawPath.length;
        const start = partAtOffset(lines, startOffset);
        const end = partAtOffset(lines, endOffset, true);
        if (!start || !end) return undefined;
        return {
          text: path,
          range: {
            start: { x: columnForTextOffset(start.part.line, start.offset), y: start.part.y },
            end: { x: columnForTextOffset(end.part.line, end.offset), y: end.part.y },
          },
          activate: () => {
            void onPlanPath(path).catch((error) => {
              console.error("[terminal] couldn't select plan", error);
              pushToast("error", error instanceof Error ? error.message : "Couldn't open this plan.");
            });
          },
        };
      }).filter((link): link is NonNullable<typeof link> => link !== undefined);
      callback(links.length ? links : undefined);
    },
  };
}
