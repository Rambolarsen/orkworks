# Copilot Native Voice Capability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Declare GitHub Copilot CLI's native voice capability and surface it in the active session details without adding audio handling.

**Architecture:** Keep voice metadata in the existing Rust harness definition and resolved capability registry. Pass the existing harness registry into `SessionDetailPanel`, where a small pure presentation helper determines whether the active session's harness advertises native voice.

**Tech Stack:** Rust/serde harness registry, React/TypeScript, Node test runner, Cargo test.

## Global Constraints

- Native harness voice is pass-through only; OrkWorks must not capture, proxy, record, store, or forward audio.
- The PTY remains text-only and no microphone or OS-permission API is added.
- Use `pnpm` for Node package tasks.
- Keep Electron-main and renderer imports separated.

---

### Task 1: Declare Copilot voice capability in the built-in registry

**Files:**
- Modify: `crates/orkworksd/resources/harnesses-v2.json:10`
- Modify: `crates/orkworksd/src/harness/registry.rs` voice capability tests near `false_voice_flags_do_not_advertise_voice_support`

**Interfaces:**
- Consumes: embedded `HarnessDefinition` JSON parsed by `BuiltinDocument::parse`.
- Produces: resolved `copilot` definition with `VoiceCapability` and `CapabilityName::Voice`.

- [ ] **Step 1: Write the failing test**

Add a test that parses embedded builtins, resolves the default registry, and asserts Copilot has the exact four voice flags and the `Voice` capability name.

```rust
#[test]
fn copilot_declares_native_voice_capability() {
    let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
    let registry = resolve_document(&builtins, &HarnessUserDocument::default()).unwrap();
    let copilot = registry.get("copilot").expect("Copilot builtin");
    let voice = copilot
        .definition
        .voice
        .as_ref()
        .expect("Copilot voice capability");

    assert!(voice.native_voice);
    assert!(voice.requires_microphone_permission);
    assert!(!voice.orkworks_dictation);
    assert!(!voice.orkworks_voice_commands);
    assert!(copilot.effective_capabilities.contains(&CapabilityName::Voice));
}
```

- [ ] **Step 2: Run the focused test to verify it fails**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml copilot_declares_native_voice_capability -- --exact`

Expected: FAIL because the current Copilot definition has `voice: null`.

- [ ] **Step 3: Write the minimal implementation**

Replace Copilot's `"voice": null` in `harnesses-v2.json` with:

```json
"voice": {
  "nativeVoice": true,
  "requiresMicrophonePermission": true,
  "orkworksDictation": false,
  "orkworksVoiceCommands": false
}
```

- [ ] **Step 4: Run the focused test to verify it passes**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml copilot_declares_native_voice_capability -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/resources/harnesses-v2.json crates/orkworksd/src/harness/registry.rs
git commit -m "feat(copilot): declare native voice capability"
```

### Task 2: Surface native voice in session details

**Files:**
- Modify: `apps/desktop/src/harnessTypes.ts`
- Modify: `apps/desktop/src/components/DockviewApp.tsx`
- Modify: `apps/desktop/src/components/SessionDetailPanel.tsx`
- Create: `apps/desktop/src/nativeVoicePresentation.ts`
- Create: `apps/desktop/tests/nativeVoicePresentation.test.ts`

**Interfaces:**
- Consumes: `HarnessConfig[]` from the existing `DockviewContext` and `SessionInfo.harness`.
- Produces: `nativeVoicePresentation(sessionHarness, harnesses)` returning either `null` or `{ label: string; detail: string }`.

- [ ] **Step 1: Write the failing test**

Create a pure helper test with the resolved capability shape:

```typescript
import test from "node:test";
import assert from "node:assert/strict";
import { nativeVoicePresentation } from "../src/nativeVoicePresentation.ts";

test("nativeVoicePresentation reports native voice and unverified microphone state", () => {
  assert.deepEqual(
    nativeVoicePresentation("copilot", [
      { id: "copilot", name: "GitHub Copilot CLI", voice: {
        nativeVoice: true,
        requiresMicrophonePermission: true,
        orkworksDictation: false,
        orkworksVoiceCommands: false,
      } },
    ]),
    { label: "Voice: native supported", detail: "Microphone permission not verified" },
  );
});

test("nativeVoicePresentation ignores harnesses without native voice", () => {
  assert.equal(nativeVoicePresentation("codex", [
    { id: "codex", name: "Codex", voice: null },
  ]), null);
});
```

Extend `HarnessConfig` with the typed optional `voice` shape while retaining `null` for unsupported harnesses.

- [ ] **Step 2: Run the focused test to verify it fails**

Run: `node --experimental-strip-types --test tests/nativeVoicePresentation.test.ts` from `apps/desktop/`.

Expected: FAIL because the helper does not exist.

- [ ] **Step 3: Write the minimal implementation**

Implement the helper by resolving either the session's harness id or display name, then returning the native-supported/unverified strings only when `voice.nativeVoice` is true. Do not inspect the microphone or call any platform API.

Pass `ctx.harnesses` from `DetailPanel` to `SessionDetailPanel`, call the helper, and render its result as a compact `detail-voice` block in the session facts area.

- [ ] **Step 4: Run the focused test to verify it passes**

Run: `node --experimental-strip-types --test tests/nativeVoicePresentation.test.ts` from `apps/desktop/`.

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src/harnessTypes.ts apps/desktop/src/nativeVoicePresentation.ts apps/desktop/src/components/DockviewApp.tsx apps/desktop/src/components/SessionDetailPanel.tsx apps/desktop/tests/nativeVoicePresentation.test.ts
git commit -m "feat(desktop): show native harness voice status"
```

### Task 3: Verify the complete change and documentation currency

**Files:**
- No additional source files expected.

- [ ] **Step 1: Run Rust validation**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`

Expected: all Rust tests pass.

- [ ] **Step 2: Run desktop validation**

Run from `apps/desktop/`: `npx tsc --noEmit` and `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`.

Expected: type-check and all desktop tests pass.

- [ ] **Step 3: Run repository checks**

Run from the repository root: `git diff --check`, `bash scripts/doc-check.sh`, and `bash .claude/hooks/worktree-check.sh`.

Expected: no whitespace errors; doc check has no unresolved flags; worktree check reports the feature branch and any unrelated existing worktrees without modifying them.

- [ ] **Step 4: Commit any required documentation adjustment**

If `scripts/doc-check.sh` identifies a required authoritative-doc update caused by the capability change, update only that doc and commit it with `docs: document Copilot native voice capability`; otherwise make no extra documentation change.
