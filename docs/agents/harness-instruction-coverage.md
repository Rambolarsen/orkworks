# Harness Instruction Coverage

Root `AGENTS.md` is the repository entry point and routing contract. The two
canonical scoped files are `apps/desktop/AGENTS.md` and
`crates/orkworksd/AGENTS.md`; work that spans both subsystems must read both.
The delivery mechanisms below were observed in the retained 2026-07-31 probe
log.

| Harness | Current configured entry point | Scoped-instruction mechanism | Exact scoped file(s) | Retained successful probe evidence |
| --- | --- | --- | --- | --- |
| Claude | `CLAUDE.md`, which imports root `AGENTS.md` | Path-scoped `.claude/rules/*.md` rules activate when Claude reads a matching file and import the canonical instructions. | `.claude/rules/desktop-agent-instructions.md` → `apps/desktop/AGENTS.md`; `.claude/rules/rust-agent-instructions.md` → `crates/orkworksd/AGENTS.md` | 2026-07-31 production probe log below: returned both scoped answers after reading matching files. |
| Codex | Root `AGENTS.md` | Nested `AGENTS.md` discovery. | `apps/desktop/AGENTS.md`; `crates/orkworksd/AGENTS.md` | 2026-07-31 production probe log below: returned both scoped answers without tools. |
| Copilot | `.github/copilot-instructions.md`, which points to root `AGENTS.md` | Nested `AGENTS.md` discovery. | `apps/desktop/AGENTS.md`; `crates/orkworksd/AGENTS.md` | 2026-07-31 production probe log below: returned both scoped answers without tools. |
| OpenCode | Root `AGENTS.md` | Root-router fallback: the root explicitly requires reading the relevant canonical file on demand. Do not rely on nested `AGENTS.md` discovery or use `opencode.json`'s session-wide `instructions` list for this split. | `apps/desktop/AGENTS.md`; `crates/orkworksd/AGENTS.md` | 2026-07-31 production probe log below: read both canonical files through the root router. |

## Production probe log

2026-07-31 — Read-only probes against the canonical production files, not test
fixtures. Desktop prompts requested the scoped TypeScript command; Rust prompts
requested the scoped owner of `SessionMetadata`.

| Harness | Desktop result | Rust result |
| --- | --- | --- |
| Claude | After reading `apps/desktop/package.json`, returned `npx tsc --noEmit`. | After reading `crates/orkworksd/src/metadata.rs`, returned `metadata.rs`. |
| Codex 0.146.0 | From `apps/desktop/`, returned `npx tsc --noEmit` without tools. | From `crates/orkworksd/`, returned `metadata.rs` without tools. |
| Copilot CLI 1.0.75 | From `apps/desktop/`, returned `npx tsc --noEmit` without tools. | From `crates/orkworksd/`, returned `crates/orkworksd/src/metadata.rs` without tools. |
| OpenCode | From root, read `apps/desktop/AGENTS.md` through the router and returned `npx tsc --noEmit`. | From root, read `crates/orkworksd/AGENTS.md` through the router and returned `metadata.rs`. |

## Fixture probe log

2026-07-31 — An isolated temporary Git repository used root `AGENTS.md` with
`ROOT-TOKEN` and `nested/AGENTS.md` with `NESTED-TOKEN`; each harness was asked
to return the instruction token without tools.

| Harness | Outcome | Consequence |
| --- | --- | --- |
| Codex 0.146.0 | Returned `NESTED-TOKEN` from `nested/`. | Native nested `AGENTS.md` delivery is observed. |
| OpenCode | Returned `ROOT-TOKEN` from `nested/`. | Do not rely on nested `AGENTS.md` auto-discovery; use the root router. |
| Claude | A `.claude/rules/` file with `paths: ["nested/**"]` returned `CLAUDE-RULE-TOKEN` after Claude read `nested/probe.txt`. | Claude-native path-scoped rules are observed. |
| Copilot CLI 1.0.75 | Returned `NESTED-TOKEN` from `nested/` after GitHub device login. | Native nested `AGENTS.md` delivery is observed. |

2026-07-31 — OpenCode's documented `opencode.json` `instructions` list was
also tested in isolation and loaded an explicit `nested/AGENTS.md` file. This
is session-wide inclusion, not path-conditional loading, so it is not used for
the scoped split.
