# Harness Instruction Coverage

`AGENTS.md` remains the complete repository fallback until every configured APM
target has evidence that it receives path-scoped instructions. A link to the
root file establishes discovery only; it does not establish scoped delivery.

Before a rule may leave root `AGENTS.md`, every configured target must have all
three promotion artifacts below: its native scoped-instruction mechanism, the
exact local file that mechanism loads, and retained successful probe evidence.
The current baseline deliberately records none of those unverified facts.

| Harness | Current configured entry point | Native scoped-instruction mechanism | Exact scoped file | Retained successful probe evidence |
| --- | --- | --- | --- | --- |
| Claude | `CLAUDE.md`, which imports root `AGENTS.md` | **unverified — to be established before promotion** | **unverified — to be established before promotion** | **unverified** — retain a transcript or PR showing a path-local task was answered by following a unique local instruction. |
| Codex | Root `AGENTS.md` | **unverified — to be established before promotion** | **unverified — to be established before promotion** | **unverified** — retain a transcript or PR showing a path-local task was answered by following a unique local instruction. |
| Copilot | `.github/copilot-instructions.md`, which points to root `AGENTS.md` | **unverified — to be established before promotion** | **unverified — to be established before promotion** | **unverified** — retain a transcript or PR showing a path-local task was answered by following a unique local instruction. |
| OpenCode | Root `AGENTS.md`; `opencode.json` has no `instructions` field | **unverified — establish by evidence; do not assume nested `AGENTS.md` auto-discovery** | **unverified — to be established before promotion** | **unverified** — retain a transcript or PR showing a path-local task was answered by following a unique local instruction. |

Do not move a rule out of root `AGENTS.md` until all four rows have validated
all three promotion artifacts. Cross-boundary constraints remain root-owned
even after a future scoped-instruction promotion.
