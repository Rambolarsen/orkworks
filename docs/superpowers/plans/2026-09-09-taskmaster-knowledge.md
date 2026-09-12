# Brain-informed Taskmaster implementation

Goal: implement [the approved specification](../../../specs/taskmaster-knowledge.md)
under issue #503. Architecture: Electron verifies reference bundles; Rust owns
analysis and recommendation state; renderer uses narrow settings/status APIs.

## Global constraints

- Independent Taskmaster model; no Peon selection changes or fallback.
- Eight daily evaluations, one at a time, one-hour workspace interval by default.
- Durable reservation; bounded context; no background repository commands.
- Signature-verified updates every six hours; compatible cached fallback.
- Evidence provenance, unchanged-evidence dismissal, explicit Fix with AI only.
- pnpm for Node work. Root and both scoped AGENTS.md apply.

## Tasks

1. Record accepted spec and ADR before code; create implementation issues.
2. Implement allowlisted signed publisher and verified Electron updater with
   tamper, privacy, compatibility, cache, and offline tests.
3. Implement independent provider inference, context restrictions, durable
   budgets, grounded recommendations and settings/status routes in Rust.
4. Integrate settings persistence, preload, restoration, background updater,
   renderer controls and provenance. Add boundary and regression tests.
5. Verify relevant suites, review changed code, open logical-unit PRs, and
   report any deployment prerequisites explicitly.

## Routing and ownership

Root owns final integration, specs, publisher/updater, and desktop changes.
One read-only explorer maps provider/settings interfaces while root writes
specs. A later Rust implementation worker owns only crates/orkworksd changes
after the interface is fixed. Root's updater does not consume worker results;
desktop runtime integration waits for the agreed Rust contract. No parallel
writers per file; maximum root plus three helpers. Final correctness and
coverage reviewers use separate contexts and distinct questions. One review
round; root applies evidence-backed corrections within the authorized task.

## Progress

- Initial checkout: 155b42f; clean main. Isolated brain-recommendations worktree.
- Issue #503 created. Accepted spec and ADR 0053 written before implementation.
- No application or brain publishing changes have been deployed.
- Publisher, signed updater, packaged starter/key, settings UI/IPC, independent
  Ollama inference, bounded context, relevance retrieval, durable budgets/cache,
  proactive evidence, and explicit handoff are implemented in the owned worktrees.
- One independent correctness review and one completeness review found privacy,
  lease-lifetime, stale-write, relevance, and desktop handoff gaps. Root applied
  corrections and added regression tests; no native `/code-review` command was
  available, so the repository's named pre-merge gate is not claimed complete.
- Earlier cloud inference was blocked: the safety gate required direct user approval for
  sending bounded Taskmaster repository/session evidence to Claude CLI. The
  attempted closed-profile implementation was removed, not bypassed. UI marks
  non-Ollama providers unavailable.
- Publishing is not activated. The private signing key must be installed as the
  brain repository's `ORKWORKS_KNOWLEDGE_SIGNING_KEY` secret, then the first Pages
  publication initialized. Do not merge its workflow before configuring the key.
- Implementation commits, logical-unit PRs, deployment, and live UI/provider smoke
  tests remain pending. Existing local spec commit: `e08840a`. Issue #503 remains
  open. Both primary checkouts remain clean; foreign worktrees were untouched.

## Close-out uncertainty check

The surfacing-blind-spots check investigated the highest-risk assumptions:
provider authority (blocked explicitly), public knowledge activation (not yet
configured), and cross-component handoff (desktop override fixed). These are
within the existing issue #503, not new unrelated audit work. No duplicate issue
was filed. A full Rust test run exposed a pre-existing one-second process-start
test failure (PID file absent); the unchanged test passed in isolation, and a
bounded-concurrency full rerun checks the contention hypothesis.

Final verification: full Rust run with `--test-threads=4` passed 1,073 unit
tests plus 3 script tests; the unchanged timeout test also passed in isolation.
The desktop Node suite, both TypeScript checks, sidecar build, rustfmt check,
six publisher tests, brain validation (179 pages, zero errors/warnings), OKF pin
check, VitePress build, diff check, doc check, and worktree currency check passed.
No live cloud inference, GUI smoke, signing-secret installation, or deployment
was performed. The first unrestricted Rust run's single timeout-fixture failure
is retained here rather than hidden by the successful bounded rerun.

## Provider-agnostic correction (in progress)

### CLI-login continuation routing

The user requires existing CLI logins; separate API credentials are rejected.
Root owns implementation and investigates Codex. Two read-only investigations
independently establish (1) Claude/Copilot and (2) OpenCode/Aider invocation,
authentication preservation, tool/config isolation, and output parsing. They
consume no other worker results. Cap: root plus two investigators, one round.
Root reconciles evidence into the shared adapter implementation. A later fresh
correctness verifier checks enforced authority; a separate completeness verifier
checks provider/auth/output coverage. Neither writes files. No commits, PRs,
publishing, real inference, or changes to user authentication are authorized by
these research jobs.

The user subsequently approved provider-agnostic Taskmaster selection, explicitly
including Codex rather than Claude alone. The earlier Claude-specific approval
question is therefore no longer the design question. Provider selection must
remain independent of Peon and must not confer coding-tool authority.

The dispatcher, scheduler, settings status, and UI now consume a shared
`inferenceOnly` provider capability instead of maintaining Taskmaster-specific
provider-name allowlists. This is plumbing, not complete provider coverage:
only the existing Ollama HTTP transport is currently verified and enabled.
Unsupported transports do not run, fall back, or mutate Peon state.

Read-only local CLI help and upstream Codex tool-registration source were
inspected; no real inference was run. Disabling Codex's shell feature does not
remove its model-dependent `apply_patch` registration. An empty temporary working
directory alone also does not enforce the permitted-context boundary. A verified
prompt-only CLI profile has not been established for every shared provider.
The partial command-runner experiment was removed; no unverified profile is
enabled. The originally desired all-provider dispatch regression remains unmet,
not reclassified as completed by capability tests.

Authentication decision (resolved by the user): reuse existing CLI logins.
Separate API credentials are not an option. Continue investigating CLI-specific
inference-only modes without silently relaxing the accepted context/authority contract.

Correction verification: the capability regression was observed failing against
the old name-based UI rule, then passing with metadata-based selection. A full
Rust run passed 1,078 tests across both suites. A subsequent required aggregate
run stopped at the pre-existing descendant-timeout fixture (its PID file was
absent); that test passed in isolation. The 37 focused desktop tests, renderer
type-check, diff check, documentation currency, and worktree currency passed.
The aggregate run is not claimed clean. Issue #503 records the unresolved
transport decision; no duplicate audit issue was created.

### CLI-login investigation and review outcome

Drafted fixed Claude/Codex argv, bounded process transport, strict output decoders,
private temporary context, and selective nonsecret Codex login preference handling.
No credential files were read/copied and no real inference was performed.

The single independent review round found blocking authority gaps: Codex retains
system/cloud/managed configuration despite `--ignore-user-config`, and empty MCP
tables merge rather than remove servers. Claude safe mode still honors managed
policies, including managed hooks. Copilot, OpenCode, and Aider investigations
also did not establish strict isolation while preserving all existing logins.
Consequently **all CLI drafts remain disabled in shared capability reporting and
dispatch**. Only Ollama is enabled; all-provider support is incomplete. The drafts
are not production-ready and must not be enabled merely because argument tests
pass. Review also identified missing installed-version gating and real-child
environment/failure-path coverage; these remain required before activation.

Custom path/wrapper recognition is tightened to canonical ID/command pairs, but
this is not a substitute for verified transport identity/effective policy.
The capability regression first failed with the unsafe advertised coverage;
it now requires every unverified CLI to return unsupported without invocation.

The required aggregate run failed: the first Codex integration-status assertion
poisoned a shared test mutex, and two process timing fixtures also failed. The
three initial failures passed individually; this does not establish the cause or
make the aggregate green. Focused desktop tests passed (37). No merge/PR handoff
or complete implementation claim is made. The unresolved decision is whether
the existing strict context/authority contract can change to permit managed CLI
policy effects; that requires the owner's explicit direction and a spec update.

### Approved managed-policy continuation — 2026-09-10

The owner approved honoring managed CLI policies and requested implementation.
ADR 0054 and the spec now distinguish OrkWorks collection/request authority from
required CLI-managed hooks, context, and routing. No new authentication choice
is needed. This continuation is limited to the existing Codex/Claude profiles;
Copilot/OpenCode/Aider remain explicitly unsupported, not silently routed.

- [x] Enable canonical profiles; require a bounded version preflight before the
  context-bearing invocation. Reject old/unknown versions and preserve Peon.
- [x] Test real child cwd/environment/stdin transport with local fixtures, plus
  malformed output, authentication errors, timeouts, and incompatible versions.
- [x] Update Settings disclosure and documentation; run targeted and aggregate
  verification, retaining any failures in the handoff.

Root is sole writer. Implementation remains sequential in the owned worktree.
One review round follows: at most two fresh read-only verifiers, one checking
policy/auth correctness and one checking missing test/dispatch coverage. Their
inputs are the resulting adapter/spec files, not each other's reports. Root owns
reconciliation. No commit, push, PR, or deployment without the human gate.

Harness adapter note: IDs codex/claude-code; fixed `codex exec` / `claude -p`
profiles in providers/inference.rs. No resume/latest fallback, interactive session
ID, voice, capacity, or label-reset behavior changes. Existing harness contracts
remain untouched. Version/help probes are read-only; real inference is not part
of automated tests. Tests live beside the provider dispatcher and profile.

Review resolution: the correctness pass found no confirmed defect. The coverage
pass identified missing wire-capability coverage (added) and a potential Claude
MCP conflict. Official [managed MCP documentation](https://code.claude.com/docs/en/managed-mcp)
confirms that a strict MCP override refuses startup with a managed MCP file.
Removed the strict/empty MCP overrides; the real-child fixture first failed on
that documented conflict and then passed. Claude safe mode retains the CLI's own
managed policy settings/hooks semantics; it does not promise every managed
customization or server is activated. No policy source is changed.

The proposed fake test of an entire managed-policy engine was not adopted:
mirroring upstream behavior would not prove the real CLI honors policy. Local
tests pin our argv/environment/output boundary; actual managed deployments and
live inference remain unverified. The default HOME-based Codex path and UI prose
do not gain separate integration smoke tests in this continuation. These are
validation limits within issue #503, not new unrelated implementation issues.

Verification before review corrections: the required aggregate helper passed
with RUST_TEST_THREADS=4, including Rust build/tests, desktop type-check and 684
tests, desktop/docs builds, formatting, diff check, and documentation currency.
The worktree check warned about another session's merged branch in the primary
checkout; it was not touched. Earlier intermittent failures remain recorded above.
No live provider call, credential change, commit, push, PR, or publication occurred.

Final post-review verification: the complete repository helper passed again
(RUST_TEST_THREADS=4): 1,084 Rust unit tests plus 3 script tests, one intentionally
ignored installed-CLI configuration probe, 684 desktop tests, both application
builds/type-checks, docs build, formatting/diff/documentation/worktree checks.
The CLI policy continuation is implemented and verified locally. Issue #503
remains open for broader provider coverage, live smoke testing, publication
activation, and commit/PR integration of the larger brain feature. The native
named `/code-review` pre-merge command is unavailable in this harness; these two
focused subagent reviews are recorded but do not claim that separate gate.

### Remaining CLI adapters — 2026-09-10 continuation

The owner requested work on Copilot, OpenCode, and Aider under the same approved
login-reuse/managed-policy contract. Root owns Copilot investigation and all
implementation. Two independent read-only investigations cover OpenCode and
Aider respectively (neither consumes another investigator's results). Each must
establish exact argv, auth preservation, optional-integration controls, output
decoding, and version requirements from local help or primary sources. No live
inference, installs, credentials/config dumps, or writes by investigators.
Cap: root plus two investigators, followed by at most two fresh read-only review
contexts (correctness versus missing coverage), one review round. Root is sole
writer and owns reconciliation. Commit/push/PR/publication still need approval.
Installed probes: Copilot 1.0.81, OpenCode 1.18.29; Aider is not installed.
Missing Aider installation is a validation limitation, not authorization to
install it or invent its behavior.

#### Adapter investigation outcome

No additional provider is enabled by this investigation. The remaining gap is
an auth-preserving, invocation-local configuration boundary, not API credentials.
The existing Codex/Claude implementation and Peon configuration are unchanged.

- **Copilot:** stdin and JSONL output are supported. A private
  `.github/copilot/settings.json` with `disableAllHooks: true` can disable
  ordinary hooks while mandatory policy hooks remain active
  ([upstream hook reference](https://docs.github.com/en/copilot/reference/hooks-reference)).
  This does not disable optional user/plugin extensions: inspected 1.0.80
  `app.js` loads user/plugin/session extension sources in prompt mode, defaults
  extension mode to `load_and_augment`, and gates only project sources with
  `GITHUB_COPILOT_PROMPT_MODE_EXTENSIONS`. No invocation-local disable-all
  extension switch was verified. `COPILOT_HOME` isolates these sources but also
  relocates login state; copying that state is not an approved workaround
  ([configuration reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference)).
  A further compatibility pitfall: the same installed launcher returned 1.0.81
  normally and 1.0.80 with `--no-auto-update`. Any future version probe must use
  the same loader controls as inference. The npm package's 1.0.28 `app.js` is
  not evidence for the effective cached runtime.
- **OpenCode 1.18.29:** `run --pure --format json --model provider/model`
  consumes non-TTY stdin. `--pure` suppresses external plugins, not MCP or
  built-in plugins. Redirecting `XDG_CONFIG_HOME` retains the separate auth data
  store but loses custom provider configuration. Preserving config leaves
  merged optional MCP/instruction settings; an empty MCP object does not clear
  them. Config supplied through `OPENCODE_CONFIG`, `OPENCODE_CONFIG_DIR`, and
  `OPENCODE_CONFIG_CONTENT` can mix provider credentials with integrations.
  Do not silently discard those sources and claim generic login reuse.
  `OPENCODE_PERMISSION` is unsuitable for the policy contract because it is
  applied after managed config. See the pinned
  [config loader](https://github.com/anomalyco/opencode/blob/v1.18.29/packages/opencode/src/config/config.ts),
  [plugin loader](https://github.com/anomalyco/opencode/blob/v1.18.29/packages/opencode/src/plugin/index.ts),
  and [headless command](https://github.com/anomalyco/opencode/blob/v1.18.29/packages/opencode/src/cli/cmd/run.ts).
- **Aider:** ask/dry-run/message-file mode is a candidate, not a verified
  adapter. Normal configuration can preload files or request startup lint/test
  commands; replacing it with an empty config can lose credentials stored
  there. Plain stdout also lacks a structured success/tool-action envelope.
  No supported minimum version was established and no binary is installed.
  See [scripting](https://aider.chat/docs/scripting.html),
  [configuration precedence](https://aider.chat/docs/config/aider_conf.html),
  and [startup source](https://github.com/Aider-AI/aider/blob/main/aider/main.py).

These findings do not establish that safe adapters are impossible. They rule
out enabling the initially proposed flag-only profiles under the approved
contract. Next design work must establish per-provider separation of existing
auth/provider settings from optional executable integrations without copying
credentials, changing the user's normal settings, or overriding managed policy.
Treat accepting optional user hooks/plugins as a new owner decision, not an
extension of the already approved administrator-policy exception. No real model
call, installation, credential read/copy, or user-config mutation was performed.
