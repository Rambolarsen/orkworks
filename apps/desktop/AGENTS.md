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

# Run Vite + Electron; this auto-launches the Rust sidecar
pnpm dev

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

Run these commands from `apps/desktop/`.

## Electron and renderer boundary

`electron/` (Electron main process) and `src/` (renderer) must never import from each other. They use separate TypeScript configs and separate `rootDir` settings. A cross-boundary import that creates stray compiled artifacts or requires a `rootDir` change is a design error, not a configuration problem.

IPC contract types shared across the boundary must be defined independently in both directories. This duplication is intentional: each side owns its copy. Update both copies whenever the contract changes.

Do not change `rootDir` in `tsconfig.node.json` or `tsconfig.json` to accommodate a new import. Reconsider the dependency direction instead.

## Architecture references

Read [`docs/agents/architecture.md`](../../docs/agents/architecture.md) for the Electron-main, preload, renderer, sidecar, and panel-layout boundaries. For cross-component work, also read [`crates/orkworksd/AGENTS.md`](../../crates/orkworksd/AGENTS.md).
