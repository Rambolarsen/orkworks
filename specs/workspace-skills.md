# Workspace and personal skills

- Status: owner-approved design, 2026-10-28; runtime implementation in progress.
- Date: 2026-10-28
- Tracking: [#1118](https://github.com/Rambolarsen/orkworks/issues/1118)

## Intent and scope

Let users see the skills available in their workspace and personal coding-tool
setup, understand where each comes from, and add skills through APM without
manually managing generated files. The user requested both scopes and a clear
distinction between them. Personal means the current OS user's existing skills,
not a new OrkWorks-only library.

The proposed first release includes read-only discovery and explicit APM adds
for both scopes. Searchable marketplaces, updates/removal, skill authoring,
automatic syncing between scopes/worktrees, and automatic session injection are
follow-ups. No package operation commits or pushes changes. Installation is not
proof that an agent loaded, invoked, or followed a skill; the separate
[skill-evidence proposal](../docs/superpowers/specs/2026-10-04-skill-usage-evidence-design.md)
and #740 capability gates remain unchanged.

## User experience

Skills appears directly below Recommendations in the left sidebar and opens in
the main view at every width, including through the collapsed rail and compact
Workspace menu. It replaces the former Skills section in Settings and contains
one row per exact skill name across scopes, a search field, coding-tool and
scope filters, Refresh, and Add skill. Each row expands to its sources, each
clearly labeled Workspace or Personal. Back, its toggle, or Escape within the page
returns to the previous central content; selecting a session opens its Terminal.
Skills is transient and is not restored after restart.
Personal discovery works without an open workspace; Workspace then explains
that opening a workspace is required.
Plugin registration names appear on source entries and remain searchable.
A name groups all matching source entries; it does not establish that their
instructions are identical, enabled or effective. Read skill opens instructions
from the selected source ID, without choosing a winning source.

Workspace names the exact open directory, including when it is a linked
worktree. It does not substitute the Git common directory or another session's
reported worktree. Personal explains that these skills may be available across
projects, subject to each coding tool's own discovery rules.

Each name row shows its source count. Each source shows its description, scope,
source directory, coding-tool discovery coverage, and management provenance: APM,
coding-tool plugin, or local file. Show package source and locked version only when recorded evidence exists.
Missing provenance is Unknown, never an inferred package URL. A detail view can
show SKILL.md as text and its source; skill instructions are untrusted content,
not instructions to OrkWorks. Discovery never runs skill scripts.

Skills installed without APM remain visible and read-only. Invalid metadata and
unreadable roots produce specific diagnostics; partial discovery is not an empty
successful inventory. APM being missing disables Add but not local discovery.

The same name in two scopes shares one name row with separate scoped sources.
Counts describe distinct exact names and a separate matching source-entry total,
not a coding tool’s loaded or enabled skill total. Search and coding-tool/scope
filters apply to source entries before grouping; counts and expanded lists show
only matching sources. Codex discovery includes its explicit system
skill directory and bounded cached plugin packages with portable or compatibility
manifests. Cached versions remain separate sources when they resolve to different
directories; no active/latest version is inferred from cache presence.

Aliases of the same directory within a scope can share one row with multiple
source/discovery labels. Never merge source identities across Workspace and
Personal merely because names or contents match; only their presentation groups
by name. Show precedence only for a verified coding-tool/version
rule; otherwise say that precedence is unknown. No universal workspace-wins
rule. Discovery means Found, not Loaded or In use.

## Discovery boundaries

Use a small coding-tool discovery registry, initially covering the existing
Claude Code, Codex, GitHub Copilot CLI and OpenCode integrations. Before claiming
coverage, verify each tool's project/user skill roots, supported configuration
roots, plugin skill sources, ancestor inheritance, and precedence against primary
documentation and bounded fixtures. Unsupported roots/configuration forms are
reported as uncovered, not silently guessed. Custom coding tools have unknown
coverage until an adapter exists. An APM target is not discovery evidence.

Workspace sources cover skills rooted in the selected workspace or its
verified inherited project roots; inherited rows name the actual ancestor.
Personal sources cover verified user roots and discoverable user-installed plugin
skills. A workspace plugin belongs to Workspace. Environment/configuration
redirects use the desktop's actual context; a session with different overrides
is not claimed to have the same inventory. Do not scan the whole home directory,
all repositories, or every session directory. Report discovered source roots and
coverage gaps explicitly.

Preserve the logical discovery scope and show resolved paths for symlinked
skills. Bound traversal, reject cycles, and never follow links during writes on
the strength of inventory alone. No arbitrary filesystem read/execute API is
exposed to the renderer. Implementation chooses finite file/depth/byte limits
and tests limit diagnostics. Only bounded skill metadata is read on inventory;
load bounded content when opening a detail view.

## Adding a skill

1. Choose Workspace or Personal. Default to Workspace when one is open; otherwise
   Personal. Show the exact destination before proceeding. A source picker
   accepts a GitHub package reference with optional ref, or a selected local
   package directory. Personal local sources must be absolute. Reject embedded
   credentials and inputs interpreted as CLI options.
2. Select supported coding-tool targets explicitly. Preserve existing manifest
   target policy; do not silently add/remove targets or apply unrelated pending
   dependencies. Offer individual skills when the package exposes a verifiable
   selection. Do not claim that selecting a skill excludes all other primitive
   types from a package.
3. Run a bounded read-only APM preview and show its declared scope, source, skill
   selection, targets, affected configuration categories/known paths, and gaps.
   APM dry-run is not a complete exact file diff. If resolution or mutation
   coverage is incomplete, explain why installation is unavailable and offer
   the equivalent reviewed terminal command to use outside OrkWorks. Do not
   silently install the full package as a fallback.
4. For a qualified operation, ask for confirmation of the concrete reviewed
   scope and changes. Revalidate destination, executable/version, source/ref,
   manifest/lock/config inputs and selected targets before execution; changes
   invalidate the preview. Workspace changes invalidate pending workspace adds.
   A personal add keeps its explicit personal destination across navigation.
5. Run APM with fixed argv and no shell. Workspace installs use the exact selected
   directory as cwd; Personal uses APM user scope, not a home directory masquerading
   as a workspace. Refresh inventory and show actual success, failure or partial
   changes. Explain when the coding tool needs a new session/reload; never restart
   or resume a running session automatically.

APM owns its manifests, locks, deployment and recorded ownership. OrkWorks does
not hand-edit generated skill trees, adopt foreign files, force conflicts, bypass
policy/security checks, grant lifecycle trust, or install/upgrade APM itself.
No automatic compile, MCP installation, executable deployment, hooks or plugin
activation is authorized by selecting a skill. Packages or existing manifests
requiring those additional mutations are blocked in the initial UI and explained
with the terminal fallback. A trusted lifecycle script is still executable work;
never assume its absence because no new trust prompt was shown.

The implementation must qualify an APM version/operation profile with disposable
fixtures, including personal destinations and pre-existing manifests. CLI help
alone is not qualification. If APM cannot safely provide the proposed reviewed
operation for a package/profile, that operation remains unavailable. Preview
incompleteness must not turn into a false safety or reproducibility guarantee.

## Architecture and recovery

Proposed boundary: Electron main owns local discovery and APM process execution,
through narrow typed preload operations. This avoids giving the workspace
sidecar a general user-home package-management API. Renderer provides validated
requests and displays results; main derives the canonical current workspace and
user roots. Preserve context isolation and the electron/src import boundary.
Record this choice in an ADR before implementation after owner review.

Main serializes installs per destination, rejects duplicate submissions, fences
stale previews/results, bounds process time/output, and retains no inherited
OrkWorks report/API credentials in the APM child environment. Standard Git/APM
credential mechanisms remain user-owned; output is redacted before display.
No raw command execution or arbitrary cwd/flag endpoint is exposed.

APM installs are not assumed atomic. On failure, cancellation or app shutdown,
report that partial changes may remain; refresh affected inventories and direct
the user to inspect local changes. No automatic rollback, deletion, trust change
or retry. Serialize OrkWorks-owned operations without claiming to lock external
APM editors/processes; invalidate on observed input drift and surface conflicts.

Installing in one worktree does not install in another. Personal installations
can affect future sessions in multiple workspaces, so confirmation names that
scope. Existing skill inventories do not change permissions, roles, lifecycle,
Taskmaster recommendations or running-session assignment evidence.

## Delivery and acceptance

After owner approval, write one concise implementation plan and the ADR. Deliver
inventory for both scopes first, then only qualified APM add profiles. Builder
chooses module names, UI layout details and bounded traversal limits within this
contract; no new database or parallel dependency manifest is required.
The combined feature is complete only after at least one representative
skill-only package can be added in each scope for at least one supported coding
tool. Inventory may land separately, but does not satisfy the Add requirement.
If no add profile qualifies, report the concrete APM gap for owner review rather
than shipping an always-disabled Add action as completed functionality.

Required checks for implementation:

- Workspace-only, personal-only, inherited, plugin, duplicate and symlinked
  skills retain correct scope/provenance; malformed/denied/limited roots have
  diagnostics and do not collapse into empty success.
- Missing APM still permits browsing; unsupported discovery/precedence remains
  unknown; installed is never relabeled loaded or used.
- Add targets the selected workspace/worktree or explicit personal scope and
  never another session's path. Existing unmanaged files are preserved.
- Incomplete previews, input drift, mixed-content packages, lifecycle scripts,
  conflicting files, unavailable targets, duplicate clicks and partial failures
  cannot become silent broader installs or success messages.
- Process fixtures verify argv, destination, timeout/output bounds and credential
  stripping. Disposable real-APM fixtures qualify supported operation profiles;
  tests must not mutate a developer's real personal skills.
- Desktop type checks, relevant tests, visual/accessibility verification and
  repository-required verification pass before runtime delivery is claimed.

## Evidence and current limitations

Inspected the repository's apm.yml, APM guide, Settings modal and preload boundary.
The installed APM reports version 0.33.0 (18c4c43); its install help advertises
--global, --target, --skill, --only apm and --dry-run. No installation or native
coding-tool discovery probe was run. Runtime support remains unverified.

Primary references (mutable upstream documentation; recheck against the qualified
version during implementation):

- [APM install reference](https://microsoft.github.io/apm/reference/cli/install/)
  documents user scope, target selection, skill subsets and incomplete previews.
- [APM package installation](https://microsoft.github.io/apm/consumer/install-packages/)
  documents source forms and separate instruction compilation.

Owner approved this two-scope inventory and qualified APM-add design with “go
ahead” on 2026-10-28, including explicit fallback when a package cannot be
reviewed safely. This approves the architecture and dependent implementation,
not a claim that any installation profile is already qualified.

## Design review

A separate read-only correctness review found no substantive conflict with the
MVP or preload boundaries. A separate builder/completeness review identified the
need for a minimum usable Add profile; the delivery criteria above now require
a representative package in both scopes. These are design reviews, not runtime
qualification or owner approval.
