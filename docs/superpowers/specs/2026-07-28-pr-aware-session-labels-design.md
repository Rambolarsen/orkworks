# PR-aware session labels

## Goal

Make Peon's one-shot session topic name the user’s concrete task, preserving a pull request number when it is present, rather than a description of how the coding tool was instructed.

## Scope

This is limited to the `InputLabel` Peon inference path. It does not add label editing, change label lifecycle rules from ADR 0029, or alter turn-by-turn summaries.

## Design

The Peon prompt will distinguish its normal terminal-output summary from a topic generated for `[User input]`. For input labels, it must name the requested task or artifact in a concise present-tense phrase, preserving an explicit PR number. It must not describe the control interaction or the coding tool, such as “instructing system”, “continuing task execution”, or “user asked”.

The sidecar will independently reject inferred labels that are blank or consist of those generic control phrases. A rejected inference leaves the immediately-seeded typed-input label in place. This is a defensive fallback for an LLM response that does not follow the prompt; it does not attempt to synthesize a replacement label.

Examples:

- `keep watching PR #249` → `Monitoring PR #249`
- `review PR #249 feedback` → `Reviewing PR #249 feedback`
- `continue current task execution` → retain `continue current task execution` rather than replacing it with `Instructing system to continue current task execution`

## Testing

Unit tests will pin the prompt’s PR-aware label instruction and the deterministic rejection of generic meta-instruction labels. The existing runtime test will prove that a rejected inference preserves the fallback label in memory and persisted metadata.

## Error handling

No inference result, blank output, or a rejected meta-instruction label is non-fatal. The session keeps its synchronous fallback label, matching ADR 0029’s one-shot topic behavior.
