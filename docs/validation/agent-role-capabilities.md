# Coding-tool role capabilities

Date: 2026-10-04
Installed CLI: `GitHub Copilot CLI 1.0.90` (`/opt/homebrew/bin/copilot`; version output captured during this investigation). The official release tag is [v1.0.90](https://github.com/github/copilot-cli/releases/tag/v1.0.90), dated 2026-09-30. The release tag is a version reference; the current docs links below are authoritative vendor docs but are not version-pinned. The later [native receipt qualification](copilot-native-receipt-qualification.md) inspects version-matched embedded runtime schemas/source; it supplies static evidence, not exercised role controls.

For an immutable official-documentation source pointer, GitHub's CLI reference history identifies the source-sync commit [`b87a8cfb5931ae735ff4dc3d936ccbc2a4c0b1e7`](https://github.com/github/docs/commit/b87a8cfb5931ae735ff4dc3d936ccbc2a4c0b1e7) for the CLI reference page. This is a pinned documentation snapshot, not a claim that its date/version matches the 1.0.90 binary; where behavior differs, the installed help is the v1.0.90 evidence and current official docs are explicitly labeled unpinned.

## Decision

**Candidate mechanisms exist; end-to-end feasibility remains unverified. All six OrkWorks role/version combinations (orchestrator, research, implementation, review, verification, remediation) are no-go / unavailable today because exact content delivery and effective tool restrictions have not been verified.** The installed CLI help and GitHub documentation expose (a) a native custom-agent prompt with tool allowlists, (b) session-wide allow/deny and tool-visibility filters, and (c) optional OS-level shell sandbox controls. These are credible mechanisms for a future adapter; this is not a claim that Copilot CLI cannot enforce the profiles. No exact launch profile has been exercised against a disposable child session, and several important details are not bound to v1.0.90 or can be bypassed through startup settings, tool composition, MCP, or subagents. Keep the six profiles unavailable until the disposable probe procedure below establishes the effective combination. Do not substitute prompt-only roles.

This matches the proposal's [coding-tool permission release bar](../superpowers/specs/2026-10-04-agent-hierarchy-and-configuration-learning-design.md#coding-tool-permissions): only offer a role/tool pairing after version-specific evidence, reject unknown combinations, and do not label edit-tool removal read-only while arbitrary shell remains. The baseline's [approved-plan boundary](../superpowers/specs/2026-09-26-taskmaster-orchestrated-child-sessions-design.md#approved-plan) expressly excludes OS confinement, and its [purpose](../superpowers/specs/2026-09-26-taskmaster-orchestrated-child-sessions-design.md#purpose) preserves ordinary child sessions. Section links refer to the proposed documents in this same revision rather than stale mutable line numbers.

Evidence levels used below:

- **V1.0.90**: installed executable version/help or the immutable v1.0.90 release tag.
- **Official-docs**: GitHub primary documentation, current as browsed 2026-10-04, but not pinned to 1.0.90 unless stated.
- **Local**: repository source with an absolute path and line number.
- **UNVERIFIED**: no allowed evidence establishes the behavior; treat as unavailable.

## Automated parent continuation: additional evidence requirement

The user chose to keep the orchestrator running after research. The proposed
[preparation lifecycle](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md)
requires a verified coordination event/tool channel that delivers reports,
approvals, blockers and capacity changes to the same waiting parent agent.
A live PTY, hook registration or native resume recipe alone does not establish
that the model loop continues. This capability is **UNVERIFIED** for the current
Copilot candidate; no live event-continuation probe has been run. A separately
authorized adapter probe must verify the declared wait/return mechanism,
duplicate/gap handling, parent runtime identity, revocation and absence of
terminal typing or replacement-session fallback. The no-go status remains.

## What the installed CLI establishes

The exact installed CLI reports `GitHub Copilot CLI 1.0.90.` Its help exposes `--agent`, `--available-tools`, `--excluded-tools`, `--deny-tool`, `--allow-tool`, `--disable-mcp-server`, `--disable-builtin-mcps`, `--add-dir`, `--plugin-dir`, `--additional-mcp-config`, `--allow-url`/`--deny-url`, `--allow-all-paths`, `--allow-all`, `--no-custom-instructions`, `--no-auto-update`, `--fleet`, and `--experimental`. Help says `--available-tools` limits what is available to the model, `--deny-tool` denies use without prompting, and `--agent` selects a custom agent. These are recorded observations from read-only `copilot --version`, `copilot --help`, and `copilot --experimental --help` in this investigation; the same option definitions appear in the [official CLI reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#command-line-options) (Official-docs). The release tag does not authenticate local executable bytes or publish a role-control contract; local hashes below bind this investigation.

`copilot help permissions` on the installed version describes `--available-tools` as hiding all other tools from the model and `--deny-tool` as a permission denial that is not promptable; deny rules take precedence over grants. Its documented permission patterns include shell, file write (excluding shell effects), URL, and named MCP servers. Its help does not list a separate read(path) permission pattern. It describes path verification as CWD plus descendants and temp-dir by default, with `--allow-all-paths` disabling that check. Installed help also exposes `--disallow-temp-dir` to prevent automatic system-temp access. Relative write rules match trailing components; absolute write rules scope one location. Shell patterns match a command stem/subcommand and do not establish exact argv/effect confinement. Reproduce the local observations with the commands below; related current docs are [tool permissions](https://docs.github.com/en/copilot/how-tos/copilot-cli/use-copilot-cli/allowing-tools) and [CLI tool permission patterns](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#tool-permission-patterns). (V1.0.90 + Official-docs.)

The v1.0.90 `copilot help sandbox` describes shell command sandboxing as experimental, disabled by default, and managed through `/sandbox enable` or the sandbox settings. In that help, disabled sandbox means shell commands run with the user's access; enabled sandbox can restrict files/network, but a default-allowed bypass can rerun a blocked command outside the sandbox. The exact `copilot --help` and `copilot --experimental --help` output did **not** list a `--sandbox` option. The retained help also describes managed sandbox floors and `allowBypass: false`; that is documented support, not an exercised policy. Current docs mention `--sandbox` and administrator-only `failIfUnavailable`; launch-flag support and actual managed fail-closed behavior remain **UNVERIFIED for 1.0.90**. See installed `copilot help sandbox` and the current [CLI sandbox reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#command-sandboxing-experimental). Do not assume the current docs page applies unchanged to this exact release.

## Supported / limited / unsupported matrix

| Capability | Finding for Copilot CLI 1.0.90 | Evidence and consequence |
| --- | --- | --- |
| Deliver a role instruction prompt | **Documented native custom-agent route; delivery unverified.** `.agent.md` profiles contain role-specific instructions; `--agent <id>` is present in local help. | Official docs define project profiles in `.github/agents/`, user profiles in `~/.copilot/agents/`, and say a custom agent specifies expertise, tools, and instructions ([creating agents](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/create-custom-agents-for-cli)). Version-specific actual startup injection is **UNVERIFIED** until the disposable probe. A prompt in the initial assignment is another delivery channel, but is not permission enforcement. |
| Deliver startup/repository instructions | **Discovery confirmed; actual inheritance/delivery unverified.** The installed `copilot instruction list` detected `.github/copilot-instructions.md`, root `AGENTS.md`, `CLAUDE.md`, and two path-scoped Claude rules in this checkout. | The local result is a discovery list, not an immutable content receipt. Current docs say CLI combines user/repository/agent instructions; `--no-custom-instructions` disables them; `COPILOT_CUSTOM_INSTRUCTIONS_DIRS` adds more ([instructions reference](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions)). Custom selected main agents receive repo instructions; custom agents spawned as subagents do not by default unless `include-custom-instructions: true`; built-in `explore`, `task`, and `code-review` subagents omit them ([subagent inheritance table](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#repository-custom-instructions-for-subagents)). |
| Preserve mandatory repo rules for every child | **Limited.** A selected top-level custom agent inherits repository instructions by default only if startup has not disabled custom instructions. Native subagent inheritance is not universal. | Current docs explicitly distinguish session agent, built-in subagents, and custom subagents; a custom subagent needs `include-custom-instructions: true`, and `--no-custom-instructions` overrides that flag (same inheritance table above). OrkWorks requires root `AGENTS.md` and scoped nested files to be available to Copilot ([root instruction-scoping contract](https://github.com/Rambolarsen/orkworks/blob/16e4eac313ad112ba2958ceebd07ab9b16d7a3e3/AGENTS.md#L170-L178)). Avoid `task`/built-in subagents or test each path; do not infer inheritance from hooks. |
| Make a selected skill available | **Documented discovery; not delivery.** Local v1.0.90 `copilot skill list --json` returned only two enabled built-in skills; no project skill appeared in this checkout. | Official docs list recognized project paths (`.github/skills`, `.agents/skills`, `.claude/skills`), personal/plugin/custom paths, and `--add-dir`; `copilot skill list` reports discovered skills ([CLI skill locations and management](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#skill-locations)). This repo's APM target list includes `copilot` and dependencies, but that setup is not evidence the generated skills are present or loaded in this checkout ([APM targets and pinned dependencies](https://github.com/Rambolarsen/orkworks/blob/16e4eac313ad112ba2958ceebd07ab9b16d7a3e3/apm.yml#L8-L28); `docs/agents/apm.md:11`). |
| Force a named skill's content into a task | **Limited.** Official docs say explicitly mention `/skill-name` in the prompt; when Copilot chooses/uses the skill, `SKILL.md` is injected. The CLI has a `skill` tool and can list/enable/disable skills. There is no `--skills` startup option in v1.0.90 help and no custom-agent `skills` frontmatter field in the CLI custom-agent reference. | [Adding agent skills](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills#using-agent-skills) says the model decides based on prompt/description; an explicit slash prompt tells it to use a skill, and only when chosen is its content injected. Thus OrkWorks can request invocation and prove discovery, but cannot yet assert deterministic loaded-content delivery per assignment. Current SDK docs have a `skills` field but concern the SDK and do not establish CLI 1.0.90 behavior; do not transfer that interface to CLI. |
| Restrict model-visible tools | **Documented control; role-combination support still unverified.** Session-wide `--available-tools` is an allowlist; built-in names include `bash`, shell session read/write/list/stop tools, file tools (`view`, `edit`, `create`, `apply_patch`), delegation (`task`, `write_agent`), `skill`, `web_fetch`, and `grep`/`glob`. | [Official CLI tool list](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#tool-availability-values). A custom agent's `tools` property defaults to `[*]`; the docs permit narrowing it to specific tools ([agent frontmatter fields](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#custom-agent-frontmatter-fields)). The actual interaction between that field, startup global `--available-tools`, skills' `allowed-tools`, and MCP tools is **UNVERIFIED for 1.0.90**. Use both session-level and agent-level restrictions and verify effective tools before offering support. |
| Role-specific read-only vs write capabilities | **Potentially enforceable at tool layer, but no certified profile yet.** Excluding all shell tools and write/edit/create/patch plus delegation and MCP tools is necessary for review/research read-only behavior. | Official docs separate model tool availability from permission approval and say omitted tools cannot be used even if otherwise allowed ([allow/deny docs](https://docs.github.com/en/copilot/how-tos/copilot-cli/use-copilot-cli/allowing-tools#restricting-the-choice-of-tools-available-to-the-ai-model)). Tool filtering is not a same-user security boundary; child OS processes, shell redirection, credentials, network and paths need their own treatment if shell is allowed. |
| Bound shell commands, file paths, and network | **Limited; sandbox/version support is unresolved.** Tool permission patterns can deny shell or narrow commands, writes can be path-scoped, and URL permissions are separate. Local sandbox can provide OS-level file/network restrictions but is experimental, default-off, and bypassable under default settings in v1.0.90 help. | Use current docs only as design leads: [tool permission patterns](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#tool-permission-patterns) and [sandbox configuration](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference#sandbox-settings). Exact availability of `--sandbox` and fail-closed config in 1.0.90 is **UNVERIFIED**. No verified verification/implementation/remediation profile may grant bash yet. |
| Restrict MCP/connector access | **Potentially controllable, but actual environment must be enumerated.** CLI has `--disable-builtin-mcps`, per-name `--disable-mcp-server`, and named-server permission patterns. Custom tools may include MCP. | Local help also accepts `--additional-mcp-config`, `--plugin-dir`, `--add-dir`, and user settings may define servers/plugins. Exact list/config may vary by account, project trust, and `COPILOT_ALLOW_ALL=true`. Current [MCP config reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference#mcp-server-configuration) states local `stdio` and remote HTTP/SSE servers exist; remote MCP servers are never sandboxed (current sandbox docs). Treat unknown server/config as unsupported. |
| Restrict delegation / prevent grandchildren | **Control surface exists; unverified effective composition.** Hide `task`, `write_agent`, shell and `--fleet`; a custom profile can omit task tools. | CLI help exposes `--fleet`; official list identifies `task` as the subagent tool. Custom agents can be selected for the primary session or invoked as subagents; those are different routes with different inheritance behavior. Current [custom-agent docs](https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/create-custom-agents-for-cli#using-a-custom-agent) explicitly say the model may choose whether to delegate. The OrkWorks proposal keeps [parent-only delegation](../superpowers/specs/2026-10-04-agent-hierarchy-and-configuration-learning-design.md#agreed-product-decisions); verify the task tool is absent and no alternate route remains. |
| Stop startup configuration from widening a role | **Unverified / high-risk until wrapper policy is defined and probed.** A safe candidate must avoid `--allow-all`, `--yolo`, `--allow-all-tools`, `--allow-all-paths`, `--allow-all-urls`, `--allow-tool` broad grants, `--add-dir`, `--plugin-dir`, `--additional-mcp-config`, `--enable-mcp-server`, and `--fleet`; ensure `COPILOT_ALLOW_ALL` is unset/false; disable unexpected MCP servers; pin updates off. | v1.0.90 local help lists these controls. Current docs say `COPILOT_ALLOW_ALL=true` additionally trusts the worktree and loads its skills, plugins, MCPs, and hooks; other truthy forms auto-approve. `--no-auto-update` and `--no-custom-instructions` are present locally. Existing OrkWorks integration/hooks do not establish a role launch wrapper or permission boundary. |

## Concrete bypass surfaces to include in an adapter audit

| Surface | Bypass / failure to account for | Evidence level |
| --- | --- | --- |
| Main prompt / custom agent | Instructions govern behavior, not access. A different selected profile or an inferred profile may change prompt/tools. `--agent` is a selection mechanism, not proof of effective final tools. | Official-docs; composition in 1.0.90 **UNVERIFIED**. |
| Repository instructions | `--no-custom-instructions` disables; environment adds instruction directories. Instructions can conflict; docs define no general precedence. Main-session inheritance differs from subagent inheritance. | Official-docs; installed `instruction list` discovery recorded above. |
| Skill | Discovered/enabled is not selected; selected/invoked is not durable proof of adherence. `allowed-tools` on a skill can auto-approve tools, so skill content must not widen permission profile. | Official-docs; exact startup injection **UNVERIFIED**. |
| Shell | If `bash`/`powershell` is available, it can invoke arbitrary user-level programs, redirection, subprocesses and network clients; denying editor tools alone cannot make a role read-only. The experimental shell sandbox can be disabled or bypassed unless policy/config prevents it. | v1.0.90 help + current docs; no 1.0.90 fail-closed managed behavior proven. |
| File/path | File tools, shell reads/writes, symlinks, temp directory, `--allow-all-paths`, added roots, and toolchain/config path grants may differ. | Permission help and current sandbox docs; end-to-end allow/deny **UNVERIFIED**. |
| Network | `web_fetch`, `url` permission, shell networking, local-network/loopback, and remote MCP each need separate checks. A URL deny list does not prove shell/network isolation. | Official-docs; effective v1.0.90 network policy **UNVERIFIED**. |
| MCP/connectors | Built-in GitHub MCP, user/repo/plugin MCP, explicit additional config, local stdio vs remote services, inherited auth/credentials. Server names and available tools must be enumerated. | Local CLI help + current MCP/sandbox docs. |
| Subagent/fleet | `task`, `write_agent`, `--fleet`, built-in agents, user/project/added-root agent priority and tool sets; subagents may omit AGENTS. | Official-docs; actual constrained dispatch **UNVERIFIED**. |
| Startup/update/config | `COPILOT_ALLOW_ALL`, saved permission approvals, settings, config/home directory, trusted roots, enabled plugins, automatic update, model/provider environment, `--no-custom-instructions`. | v1.0.90 local help and current docs. Inspect only safe, non-secret effective config during future disposable probe. |

## Local OrkWorks evidence boundary

The local APM config targets Copilot and pins plugin/skill dependencies ([APM targets and pinned dependencies](https://github.com/Rambolarsen/orkworks/blob/16e4eac313ad112ba2958ceebd07ab9b16d7a3e3/apm.yml#L8-L28)); the APM guide says install populates `.agents/skills/` and Copilot hooks (`docs/agents/apm.md:11`). Yet the installed `copilot skill list --json` in this checkout returned only the two built-in skills, and the earlier investigated checkout had no `.agents/` directory. That historical observation establishes **discovery result in that checkout only**, not absence from every runtime installation.

The repo's Copilot integration contract is explicitly about CLI v1.0.90 hooks, session ID/resume, and attention signals, with the row marked feature-probed but limited because its hook schema is not version-bound (`docs/agents/harness-integration-contracts.md:30`). It is **not evidence that OrkWorks supplies role agent prompts, skills, or permission restrictions**. Root AGENTS says Copilot loads nested AGENTS files ([root instruction-scoping contract](https://github.com/Rambolarsen/orkworks/blob/16e4eac313ad112ba2958ceebd07ab9b16d7a3e3/AGENTS.md#L170-L178)); the CLI `instruction list` confirmed files discovered in this checkout. Neither proves that each planned role receives the right content or that tools are restricted.

## Candidate controls and exact CLI flags

These are candidate inputs, not a verified supported recipe:

1. Use the installed/pinned binary and `--no-auto-update`; pass `--agent <role-id>` for the role prompt. Do not pass `--no-custom-instructions` when repository rules are mandatory.
2. Set `--available-tools` to the smallest explicitly required set and `--deny-tool` for prohibited classes/actions. At minimum, read-only roles must exclude all shell tool names (`bash`, `powershell`, `list_*`, `read_*`, `write_*`, `stop_*`), all edit tools (`edit`, `create`, `apply_patch`), `task`, `write_agent`, and any MCP server tools; only include `view`, `grep`, `glob`, and optionally a reviewed `web_fetch` if the version probe validates those names. The official docs say `--available-tools` removes all other tools from model visibility.
3. For implementation/verification/remediation, shell must not be granted until version-specific coding-tool controls establish the requested profile. Optional vendor shell sandboxing is a lead to investigate, not an OrkWorks OS-confinement requirement. Command/path/URL permission settings must be evaluated for their actual effect coverage. Do not use permissive flags/env, add roots, plugins, or extra MCP config unless the approved role requires them and the exact resulting tool set is pinned.
4. Keep the base worktree and instruction files stable; capture hashes of OrkWorks-supplied shared instruction, role template, assignment, and requested skill files. `--agent` does not provide a documented CLI skill allowlist. A slash skill request may ask Copilot to invoke a discovered skill, but content digest and loaded receipt need a separate verified evidence mechanism.

The exact v1.0.90 candidate flags confirmed by local help are: `--agent`, `--available-tools`, `--excluded-tools`, `--deny-tool`, `--disable-builtin-mcps`, repeated `--disable-mcp-server`, `--deny-url`, `--allow-url`, `--no-auto-update`, `--no-custom-instructions`, and `--experimental`. `--sandbox` did not appear in normal or experimental local help. Current docs reference `--sandbox`; treat it as unavailable in 1.0.90 until a safe help/config probe proves otherwise.

## Reproducible future disposable probe (not run here)

Run only after a task explicitly permits model probes. Keep it separate from production sessions and credentials; do not use a real project checkout.

1. Pin the exact Copilot CLI package/release to `v1.0.90` (or the newly approved version), capture binary SHA-256, `copilot --version`, normal help, experimental help, `copilot help permissions`, and `copilot help sandbox`. Disable auto-update and retain the complete command output as evidence.
2. Use a separate explicitly authorized probe session with temporary `COPILOT_HOME` and a synthetic Git repo with harmless sentinel files: one in the working tree, one outside it, one writable path, one read-only path, plus mock external/local-network endpoints. No production source, personal MCP config, GitHub data, or host secrets. Any inference authentication for a separately authorized probe follows the ordinary coding tool; it is excluded from the evidence packet. Make a minimal custom agent for each role with explicit `tools`; keep profile names/digests.
3. Inspect non-secret instruction discovery (`copilot instruction list`) and skill discovery (`copilot skill list --json`). Confirm only approved files are discovered; compare prompt/skill content digests to the proposed configuration. Confirm approved skill content is supplied through the declared native-agent/startup-context mechanism with matching byte identities. Native invocation observation is separately optional: discovery or `/skill-name` text never establishes successful use, and absent recognized invocation evidence remains unknown.
4. Start each role with a clean explicit argument vector; no resume, `--continue`, `--fleet`, `--allow-all*`, `--yolo`, broad grants, `COPILOT_ALLOW_ALL`, added roots, plugin dirs, or extra MCP config. Test separately with a restrictive `--available-tools` allowlist plus deny rules. Include mock MCP server(s) and built-ins in the matrix; prove no unlisted tool can be invoked and no settings, skill frontmatter, saved approval, or subagent broadens access.
5. Exercise attempts (and require denial) for each role across: built-in edit/write/patch; shell subprocess, redirection, `python`/`curl`, background/detached child; outside-root and temp filesystem reads/writes, symlinks; external URL and localhost/private network; direct MCP tool invocation and local stdio / remote MCP; `task`/`write_agent`, built-in and custom subagent; clearing/disabling instructions and selecting a conflicting user/repo agent. Keep the test prompts and mock outputs; do not use production content.
6. For any profile that needs shell, exercise the exact approved commands, arguments and applicable coding-tool permission controls. Confirm denied commands/effects remain denied, including alternate interpreters, redirections, outside paths and network calls. Vendor shell sandbox controls may be evaluated if available but are not a mandatory OrkWorks feature. If the requested command/effect restrictions cannot be established, mark that profile unverified or unsupported and keep it unavailable. This contract does not certify OS containment, credential isolation, descendant process ownership or prevention of direct same-user sidecar calls.
7. Verify the top-level `--agent` route and any eventual OrkWorks launch/resume route. Native subagent/fleet execution is denied under the parent-only design; attempt those routes as negative cases, not supported child launch paths. Confirm mandatory instructions and final tools after startup settings/profile/skill configuration are combined.
8. Save a bounded evidence packet with CLI version/hash, OS, exact argument vector (redact secrets), effective non-secret settings, discovered instruction/skill paths and hashes, tool/MCP inventory, every allowed/denied attempt and result, and caveats. A fresh verifier should independently inspect it. Any untested or ambiguous surface stays **UNVERIFIED** and blocks this harness/role profile.

No model, permission, hook, settings, or sandbox probe was run in this assignment. This report is an evidence gate, not launch authorization.

## Role/profile eligibility matrix

| Tool/version/platform | Role/profile | Candidate instruction delivery | Skill delivery | Permission result | Usage coverage | Eligibility |
| --- | --- | --- | --- | --- | --- | --- |
| Copilot 1.0.90 / macOS arm64 | Orchestrator: coordination-only | Top-level custom agent or startup context, unverified | Unverified | Exact plan-only connector/action scope unverified; no codebase/shell/delegation | Unknown native skill observer | No-go: unverified |
| Copilot 1.0.90 / macOS arm64 | Research: declared reads and optional external sources | Same | Unverified | No-shell/write/delegation candidates exist; declared read/URL/MCP scope unverified | Unknown | No-go: unverified |
| Copilot 1.0.90 / macOS arm64 | Implementation: declared edits/commands | Same | Unverified | File write patterns exclude shell effects; exact command/effect scope unverified | Unknown | No-go: unverified |
| Copilot 1.0.90 / macOS arm64 | Review: declared artifact reads | Same | Unverified | Read-only tool filtering plausible; exact read scope and composition unverified | Unknown | No-go: unverified |
| Copilot 1.0.90 / macOS arm64 | Verification: checks/generated output | Same | Unverified | Check-command/output/temp effects unverified | Unknown | No-go: unverified |
| Copilot 1.0.90 / macOS arm64 | Remediation: finding-scoped edits/checks | Same | Unverified | Same implementation surfaces, narrower assignment unverified | Unknown | No-go: unverified |

Other coding tools are **not investigated for this role contract**. Existing
Codex/Claude/OpenCode integration records remain integration evidence only, not
substitutes for a verified role profile. No cross-tool fallback is selected.
No-go here means absence of demonstrated launch eligibility, not impossibility.

## Local executable identity and safe reproduction

Read-only inspection on 2026-10-04 found a useful identity mismatch: the npm
loader manifest says `@github/copilot 1.0.28`, while both the loader invocation
and resolved native executable report `GitHub Copilot CLI 1.0.90.` A manifest
version alone would incorrectly pin the running tool. Host: Darwin 25.6.0 arm64.

| Observed file | SHA-256 |
| --- | --- |
| `/opt/homebrew/lib/node_modules/@github/copilot/npm-loader.js` | `8ba289084c305e25ea9ae09da860c31614c7eb32f6e7ec099aa71f74f2d4f130` |
| Loader `package.json` | `aa9b3831c5bf16cec0b9f1a235c01f9952036462e081f0b22d288bced9a769da` |
| Native `node_modules/@github/copilot-darwin-arm64/copilot` | `3ae21a3f00fcc216faaa1f062ee98451c7807066ee26f0fe1c85c6ed315e5458` |

The loader resolves the platform package and invokes it; these hashes identify
locally inspected files, not a vendor signature or complete install attestation.
No reason for the manifest/version mismatch is inferred. No package update was
performed. A role adapter must resolve and pin its actual executable chain.

```bash
rtk proxy copilot --version
rtk proxy copilot --help
rtk proxy copilot --experimental --help
rtk proxy copilot help permissions
rtk proxy copilot help sandbox
rtk proxy copilot instruction list
rtk proxy copilot skill list --json
```

These commands inspect version/help/discovery only; they do not execute model
probes or prove denial/content delivery. Record safe output, exact cwd/platform,
and resolved executable hashes on each future investigation. Never print
credential-bearing settings or environment values.

## Handoff and remaining gate

Configuration #741 consumes this register's exact-version/evidence identity and
no-go decision. Preparation #742 cannot launch research until a profile is
verified. Usage #743 may distinguish supplied startup content from native skill
invocation: selected/discovered stays distinct from confirmed-loaded, agent
claims remain reported, and absent observation remains unknown.

Review this register and the proposed role contract before any capability
probe execution plan or runtime adapter is approved. A separate authorized
probe session can produce bounded fixtures and a supported slice or retain
no-go findings. The native OS boundary issue #617 is not a prerequisite added
by this evidence draft. No runtime implementation plan can claim an eligible
role until the necessary tool/version/profile evidence and reviewed handoff
are complete. #610 scope acceptance is recorded in PR #747; it supplies no
capability evidence. #740 remains open for probe evidence and reviewed handoff.

## Issue #740 follow-up: retained evidence and scoped handoff

The refreshed version and executable-chain hashes match the earlier observation.
[Retained manifest](fixtures/copilot-1.0.90/manifest.json) binds exact commands,
cwd, platform, exit codes, stdout bytes and SHA-256 values. It is an inspection
packet, **not** a verified `CapabilityEvidenceSnapshot` or permission fixture.
Normal and experimental help were byte-identical. Node color warnings on stderr
were excluded; no stderr claim is used. No inference/model process was started.

| Retained evidence | Established fact | Still unknown |
| --- | --- | --- |
| [Version](fixtures/copilot-1.0.90/version.txt), [CLI help](fixtures/copilot-1.0.90/help.txt) | Version 1.0.90; the documented parser/help controls are present | Actual effective startup/tool/model composition |
| [Permission help](fixtures/copilot-1.0.90/permissions.txt) | Tool visibility and approval are separate; write rules exclude shell effects; default paths include cwd descendants and temp | Exact read-scope, argv/effect and denial coverage |
| [Sandbox help](fixtures/copilot-1.0.90/sandbox.txt) | Local help describes managed floors and inherited shell environments with a fixed blocklist | Effective immutable policy and #741's empty-start command environment |
| [Skill discovery](fixtures/copilot-1.0.90/skill-discovery.json) | The synthetic project sentinel is discovered/enabled in `.agents/skills`; two built-in skills also remain discovered with temporary `COPILOT_HOME` | Selected content delivery, skill invocation, tool widening or adherence |
| [Root discovery](fixtures/copilot-1.0.90/instruction-discovery.json), [scoped discovery](fixtures/copilot-1.0.90/scoped-instruction-discovery.json) | Root AGENTS is discovered; scoped cwd additionally discovers `scope/AGENTS.md` | Actual inheritance/content delivery when running a role or changing cwd |

The manifest retains the harmless input files as well. Discovery changed only
cwd and `COPILOT_HOME`; ambient sources were not exhaustively inventoried. The
temporary settings directory does not establish configuration or OS isolation.
The fixture qualifies discovery in this exact synthetic layout, not production
role delivery. This also prevents the earlier missing-generated-skills
observation from being read as evidence that `.agents/skills` is unsupported.

### Contract review findings

1. **Read scope needs its own proof.** #741 requires declared file/subtree/root
   reads and rejection of symlink escape. The installed permission help lists no
   separate read-path pattern. A no-shell role may still read undeclared files
   through `view`, `grep` or `glob`. Tool filtering alone is insufficient. The
   first proposed probe explicitly requests the whole synthetic worktree root;
   it cannot certify a narrower artifact-only profile.
2. **Command environment is a distinct eligibility gate.** #741's
   [command environment policy](../superpowers/specs/2026-10-04-agent-role-configuration-design.md#command-environment-policy)
   requires empty-start allowlisted bindings before every task command. Retained
   sandbox help describes inheritance apart from a fixed blocklist; local CLI
   help's `--secret-env-vars` is another removal list. Neither proves the
   required contract. Command-enabled roles remain unverified without exact
   executable/argv/effects and environment evidence. No OS-confinement or broker
   requirement is added to overcome the gap.
3. **Content delivery and usage stay independent.** Synthetic discovery and a
   model repeating a marker cannot confirm selected skill bytes were supplied.
   A version-bound content transport/receipt must identify mandatory rules,
   selected skills and referenced resources. Supplied startup content can
   qualify delivery without asserting native invocation; missing native usage
   coverage stays unknown. Existing session hooks are not that receipt.
4. **Continuation is independently unverified.** Exact native resume or an open
   PTY does not prove a report/approval/capacity event resumes the same model
   loop. The proposed parent also needs a coordination-only tool profile;
   a no-shell review/read profile cannot qualify it.

These findings constrain the evidence methodology; they do not amend #741 or
#742, relax requested permissions, or establish impossibility. The current
`HarnessDefinition` and integration/launch/resume seams do not implement these
role/profile or delivery contracts. Other coding-tool integrations provide
session signals and owned reporter installation, not substitutes for role
capability evidence.

### Acceptance coverage and consumer handoff

| #740 criterion | Current disposition |
| --- | --- |
| Exact versions, primary references, bounded fixtures | Retained local help/version/discovery; runtime controls remain unverified |
| Role/native/startup rules and selected skill delivery | Native OTel capture candidate inspected in the [transport research](copilot-role-observation-transport.md); native schema is now version-bound in the [qualification report](copilot-native-receipt-qualification.md); runtime sample, complete content receipt and safe capture remain open |
| Six roles' tools/commands/files/network/connectors/bypasses | Audited surfaces and role matrix retained; no complete profile fixture |
| Unsupported eligibility, no silent widening/fallback | All six profiles unavailable; absence of evidence is not impossibility |
| Reported/observed/unknown usage coverage | Version-bound native invocation/delivery/ref schemas retained; CLI JSON stdout excludes the receipts, native emission/coverage remains unverified and discovery is not usage |
| Initial slice or explicit no-go | Current no-go retained; smallest proposed experiment is a synthetic local-only review profile |
| Same-parent continuation | Requires its own schema/event/runtime/generation fixture; no live probe |
| Reviewed specification/research execution handoff | Concrete [probe plan](../superpowers/plans/2026-10-04-copilot-role-capability-probes.md) prepared for written review and separate probe-session authorization |

The probe plan maps all ten mandatory #741 evidence surfaces, optional native
usage, exact resume and mandatory parent continuation. It stops at absent
measurement, unexpected access, unknown startup composition or ambiguous
results. First-batch success would qualify only the exact reviewed synthetic
profile, never all six roles or a production runtime launch.

#741 consumes exact content/profile/settings identities and the unchanged
no-go; #742 consumes the separate continuation/resume gate; #743 consumes
source/coverage distinctions for discovery, delivery and usage. #740 stays
open until the remaining written review, necessary capability evidence and
reviewed handoff are resolved. No runtime implementation or live capability
probe is authorized by this document.


## Observation transport follow-up — 2026-10-05

The [transport research](copilot-role-observation-transport.md) identifies a
concrete native candidate: installed Copilot 1.0.90 monitoring help advertises
local OTel JSON-lines capture of system instructions and tool definitions.
The [new inspection packet](fixtures/copilot-1.0.90-transport/manifest.json)
retains update-disabled version/help outputs and labelled companion schema/source
excerpts. The native executable/loader hashes match the earlier packet, while
both companion npm manifests report 1.0.28. Their `system.message`,
`skill.invoked`, file-exporter shape and global built-in `tools.list` catalogue
cannot certify the running native release or a session's effective inventory.

No runtime stream or content receipt was captured. Exact schema/sample,
capture safety before storage, rule/skill/resource byte identity, session/turn
correlation and complete model-facing tool coverage remain qualification gates.
In particular, coupled response capture and an invocation-only tool snapshot
need investigation; post-capture redaction or one observed tool is insufficient.
The probe plan now links these concrete checks without marking transport or
role qualification complete. All six profiles remain unverified/no-go, and
#740 remains open for the necessary evidence and reviewed consumer handoff.

## Native source qualification follow-up — 2026-10-05

The [native qualification report](copilot-native-receipt-qualification.md) and
[static inspection packet](fixtures/copilot-1.0.90-native/manifest.json) resolve
the source/version mismatch: the official native 1.0.90 executable matches the
installed bytes, and its embedded runtime contains version-matched schemas.
Six selected cached runtime files, including the native addon, match the
embedded archive. The earlier 1.0.28 companion packet remains a distinct lead.
The loader can report its version before importing the cached runtime, so
future launch identity must bind the actual loaded distribution as well.

Native schemas define exact skill-delivery receipts and same-session content
references, plus an experimental initialized-session tool metadata query.
These are static source facts. The ordinary `--output-format json` writer
explicitly omits the necessary system/skill receipts and is no-go for delivery
measurement. Tool metadata has optional input schemas and no mandatory model
request/generation binding; it is not yet a complete effective inventory.
OTel capture safety and any alternative observer remain unverified. No live
probe ran, no SDK integration is proposed, all six roles remain unavailable,
and #740 remains open.
