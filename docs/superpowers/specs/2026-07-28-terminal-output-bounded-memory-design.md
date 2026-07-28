# Bounded Terminal-Output Memory Design

## Goal

Prevent terminal-history reads and trims from allocating a collection for every line in a persisted terminal file.

## Scope

This implements GitHub issue #192 only. It does not migrate dormant oversized terminal files, change renderer polling, or address the renderer-memory investigation in #247.

## Design

Replace the full-file `Vec<&str>` collection in `MetadataStore::read_terminal_output` and `MetadataStore::trim_terminal_output` with one shared bounded-tail reader.

The helper reads the file sequentially through a buffered reader and retains only the newest requested number of lines in a `VecDeque<String>`. After the scan, it applies the existing byte budget by dropping oldest retained lines until the retained tail fits. The read path returns that tail; the trim path rewrites it only when content was discarded.

The helper preserves the current persistence contract: output is replayed in original order, byte and line budgets apply to whole lines, and an oversized dormant file is not rewritten merely because it is read. Existing append-triggered trimming remains the only normal mutation path.

## Error Handling

Unreadable terminal files continue to yield no replay output and no trim mutation. A failed line read follows the existing best-effort policy: the public read path returns no replay; the trim path leaves the original file untouched.

## Testing

Add metadata-store tests using a file with many short lines and a small requested tail. The tests verify that reads return the correct newest lines and trimming writes only the bounded tail while retaining byte-budget behavior. Existing terminal-output limit tests remain unchanged and must pass.

## Constraints

- Keep the 1,000-line and 1 MiB public persistence limits unchanged.
- Do not add dependencies.
- Do not alter the Electron/renderer boundary or terminal replay API.
