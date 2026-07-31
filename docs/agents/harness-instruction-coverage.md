# Harness Instruction Coverage

`AGENTS.md` remains the complete repository fallback until every configured APM
target has evidence that it receives path-scoped instructions. A link to the
root file establishes discovery only; it does not establish scoped delivery.

| Harness | Current configured entry point | Nested/path-scoped delivery | Required manual probe before promotion |
| --- | --- | --- | --- |
| Claude | `CLAUDE.md`, which imports root `AGENTS.md` | **unverified** — this repository has no retained probe evidence for native scoped delivery. | Give Claude a path-local task whose answer depends on a unique local instruction, then retain the transcript or PR evidence showing it received and followed that instruction. |
| Codex | Root `AGENTS.md` | **unverified** — this repository has no retained probe evidence for native scoped delivery. | Give Codex a path-local task whose answer depends on a unique local instruction, then retain the transcript or PR evidence showing it received and followed that instruction. |
| Copilot | `.github/copilot-instructions.md`, which points to root `AGENTS.md` | **unverified** — this repository has no retained probe evidence for native scoped delivery. | Give Copilot a path-local task whose answer depends on a unique local instruction, then retain the transcript or PR evidence showing it received and followed that instruction. |
| OpenCode | `opencode.json` project configuration | **unverified** — this repository has no retained probe evidence for native scoped delivery. | Give OpenCode a path-local task whose answer depends on a unique local instruction, then retain the transcript or PR evidence showing it received and followed that instruction. |

Do not move a rule out of root `AGENTS.md` until all four rows have validated
delivery evidence. Cross-boundary constraints remain root-owned even after a
future scoped-instruction promotion.
