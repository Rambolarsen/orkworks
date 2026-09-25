# Desktop App Instructions

Read the root [`AGENTS.md`](../../AGENTS.md) first. These instructions apply before changing anything under `apps/desktop/`.

## Package manager and validation

The root [`AGENTS.md`](../../AGENTS.md) owns the repository-wide pnpm-only rule.

```bash
# Install pnpm if missing
npm install -g corepack   # Node 25+ no longer bundles corepack
corepack enable
corepack prepare pnpm@11.9.0 --activate

# Install dependencies
pnpm install

# Run Vite + Electron; this builds and auto-launches the Rust sidecar
pnpm dev

# Dev server port: defaults to 5173 and fails loudly when it is taken
# (strictPort). When another checkout already holds 5173 — for example while
# UI-verifying a change from a sibling worktree — run this checkout on its own
# port; the resolved URL is passed to Electron either way. Invalid values fall
# back to 5173:
ORKWORKS_DEV_PORT=5273 pnpm dev

# Build Electron and package a host-architecture release artifact
pnpm build
pnpm package:release

# Build the Rust sidecar through the desktop package
pnpm build:rust

# Type-check and test
npx tsc --noEmit
node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
node --experimental-strip-types --test tests/api.test.ts
```

Run these commands from `apps/desktop/`. Linux desktop tests require Xvfb
(`xvfb-run`) when no display is available; the Electron layout test starts its
own virtual display. GitHub-hosted Ubuntu runners include Xvfb.

## Electron and renderer boundary

`electron/` (Electron main process) and `src/` (renderer) must never import from each other. They use separate TypeScript configs and separate `rootDir` settings. A cross-boundary import that creates stray compiled artifacts or requires a `rootDir` change is a design error, not a configuration problem.

IPC contract types shared across the boundary must be defined independently in both directories. This duplication is intentional: each side owns its copy. Update both copies whenever the contract changes.

Do not change `rootDir` in `tsconfig.node.json` or `tsconfig.json` to accommodate a new import. Reconsider the dependency direction instead.

## Settings coding-tool toggle confirmation

Flipping an integration-capable coding tool's Settings toggle off→on immediately persists the merged active-tool selection and reconciles only that tool's integration group (`enableHarnessIntegrationImmediate`); a coding tool without an integration binding stays draft until Save. When that reconcile plans an install or repair, it shows the native OS confirmation dialog — listing the mutation it is about to perform — at toggle time, not at the later modal-wide Save click; when no mutation is planned (integration already healthy, unsupported, or its status lookup failed) no dialog appears. This is intentional, not a regression; do not change or document it as deferred to Save. Flipping on→off remains draft-only; its cleanup mutation still waits for Save.

## Architecture references

Read [`docs/agents/architecture.md`](../../docs/agents/architecture.md) for the Electron-main, preload, renderer, sidecar, and panel-layout boundaries. For cross-component work, also read [`crates/orkworksd/AGENTS.md`](../../crates/orkworksd/AGENTS.md).
