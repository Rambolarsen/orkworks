interface Fence {
  marker: "`" | "~";
  length: number;
}

function lineEnd(source: string, start: number): number {
  const end = source.indexOf("\n", start);
  return end === -1 ? source.length : end + 1;
}

function fenceAt(source: string, start: number): Fence | null {
  const match = source.slice(start).match(/^ {0,3}(`{3,}|~{3,})/);
  return match ? { marker: match[1][0] as Fence["marker"], length: match[1].length } : null;
}

function closesFenceAt(source: string, start: number, fence: Fence): boolean {
  const end = lineEnd(source, start);
  const line = source.slice(start, end).replace(/\r?\n$/, "");
  const match = line.match(/^ {0,3}(`{3,}|~{3,})[ \t]*$/);
  return Boolean(
    match &&
      match[1][0] === fence.marker &&
      match[1].length >= fence.length,
  );
}

function multilineDestination(source: string, start: number): { value: string; end: number } | null {
  let index = start;
  let parentheses = 0;
  let angleDestination = false;
  let changed = false;
  let value = "";
  let title: { delimiter: "\"" | "'" | "("; depth: number } | null = null;

  while (index < source.length) {
    const character = source[index];

    if (character === "\\" && index + 1 < source.length) {
      value += source.slice(index, index + 2);
      index += 2;
      continue;
    }

    if (character === "\r" || character === "\n") {
      const nextLine = character === "\r" && source[index + 1] === "\n" ? index + 2 : index + 1;
      let next = nextLine;
      while (next < source.length && (source[next] === " " || source[next] === "\t")) next += 1;
      if (next < source.length && (source[next] === "\r" || source[next] === "\n")) return null;
      // A line break before or inside a link title is whitespace; a break
      // inside a URL is a wrapped destination and should disappear with its
      // indentation.
      value += title || (next < source.length && parentheses === 0 && /["'(]/.test(source[next])) ? " " : "";
      changed = true;
      index = next;
      continue;
    }

    if (title) {
      if (title.delimiter === "(") {
        if (character === "(") title.depth += 1;
        if (character === ")") {
          title.depth -= 1;
          if (title.depth === 0) title = null;
        }
      } else if (character === title.delimiter) {
        title = null;
      }
      value += character;
      index += 1;
      continue;
    }

    if (
      parentheses === 0 &&
      !angleDestination &&
      /["'(]/.test(character) &&
      /[ \t]$/.test(value)
    ) {
      title = { delimiter: character as "\"" | "'" | "(", depth: character === "(" ? 1 : 0 };
    }

    if (character === "<" && parentheses === 0) angleDestination = true;
    if (character === ">" && angleDestination) angleDestination = false;
    if (character === "(" && !angleDestination) parentheses += 1;
    if (character === ")" && !angleDestination) {
      if (parentheses === 0) return changed ? { value: `${value})`, end: index + 1 } : null;
      parentheses -= 1;
    }

    value += character;
    index += 1;
  }

  return null;
}

/** Join physical line breaks inside inline Markdown link destinations. */
export function normalizeMarkdownLinkDestinations(source: string): string {
  let result = "";
  let index = 0;
  let fence: Fence | null = null;
  let inlineCodeLength: number | null = null;
  let openLabels = 0;

  while (index < source.length) {
    if (index === 0 || source[index - 1] === "\n") {
      const marker = fenceAt(source, index);
      if (fence) {
        const end = lineEnd(source, index);
        const closing = marker && closesFenceAt(source, index, fence);
        result += source.slice(index, end);
        index = end;
        if (closing) fence = null;
        continue;
      }
      if (marker) {
        fence = marker;
        const end = lineEnd(source, index);
        result += source.slice(index, end);
        index = end;
        continue;
      }
      if (source.slice(index).startsWith("    ")) {
        const end = lineEnd(source, index);
        result += source.slice(index, end);
        index = end;
        continue;
      }
    }

    if (inlineCodeLength !== null) {
      if (source[index] === "`") {
        let run = 0;
        while (source[index + run] === "`") run += 1;
        if (run === inlineCodeLength) inlineCodeLength = null;
        result += source.slice(index, index + run);
        index += run;
      } else {
        result += source[index];
        index += 1;
      }
      continue;
    }

    if (source[index] === "`") {
      let run = 0;
      while (source[index + run] === "`") run += 1;
      const closing = source.indexOf("`".repeat(run), index + run);
      result += source.slice(index, index + run);
      index += run;
      if (closing !== -1) inlineCodeLength = run;
      continue;
    }

    if (source[index] === "\\" && index + 1 < source.length) {
      result += source.slice(index, index + 2);
      index += 2;
      continue;
    }

    if (source[index] === "[") {
      openLabels += 1;
    } else if (source[index] === "]") {
      const hasLinkLabel = openLabels > 0;
      if (hasLinkLabel) openLabels -= 1;
      if (source[index + 1] === "(" && hasLinkLabel) {
        const link = multilineDestination(source, index + 2);
        if (link) {
          result += `](${link.value}`;
          index = link.end;
          continue;
        }
      }
    }

    result += source[index];
    index += 1;
  }

  return result;
}
