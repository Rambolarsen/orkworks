# PR CI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a pull-request CI workflow that runs desktop checks for desktop changes, Rust tests for Rust changes, and a lightweight green no-op check for docs-only or unrelated PRs.

**Architecture:** Keep release packaging isolated in the existing tag-driven workflow and add a separate PR validation workflow under `.github/workflows/`. Use a routing job to classify changed paths, then gate `desktop`, `rust`, and `noop` jobs off those outputs so the PR only pays for relevant validation.

**Tech Stack:** GitHub Actions, `dorny/paths-filter`, pnpm, Node 22, TypeScript, Node test runner, Rust stable, Cargo

---

### Task 1: Add the PR validation workflow skeleton and routing job

**Files:**
- Create: `.github/workflows/pr-ci.yml`

- [ ] **Step 1: Create the workflow file with PR trigger and permissions**

Create `.github/workflows/pr-ci.yml` with this initial header:

```yaml
name: PR CI

on:
  pull_request:
    branches:
      - main

permissions:
  contents: read
```

- [ ] **Step 2: Add the `changes` job using path filters**

Extend `.github/workflows/pr-ci.yml` with this routing job:

```yaml
jobs:
  changes:
    name: Detect changed surfaces
    runs-on: ubuntu-latest
    outputs:
      desktop_changed: ${{ steps.filter.outputs.desktop }}
      rust_changed: ${{ steps.filter.outputs.rust }}
      relevant_code_changed: ${{ steps.filter.outputs.desktop == 'true' || steps.filter.outputs.rust == 'true' }}
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - id: filter
        uses: dorny/paths-filter@v3
        with:
          filters: |
            desktop:
              - 'apps/desktop/**'
            rust:
              - 'crates/orkworksd/**'

      - name: Print routing decision
        run: |
          echo "desktop_changed=${{ steps.filter.outputs.desktop }}"
          echo "rust_changed=${{ steps.filter.outputs.rust }}"
```

- [ ] **Step 3: Validate the workflow YAML shape locally**

Run:

```bash
python3 - <<'PY'
import yaml, pathlib
path = pathlib.Path(".github/workflows/pr-ci.yml")
data = yaml.safe_load(path.read_text())
assert data["name"] == "PR CI"
assert data["on"]["pull_request"]["branches"] == ["main"]
assert "changes" in data["jobs"]
print("workflow header OK")
PY
```

Expected: `workflow header OK`

- [ ] **Step 4: Commit the routing workflow skeleton**

```bash
git add .github/workflows/pr-ci.yml
git commit -m "ci: add PR workflow routing job"
```

### Task 2: Add the desktop validation job

**Files:**
- Modify: `.github/workflows/pr-ci.yml`

- [ ] **Step 1: Add the desktop job gated on desktop changes**

Append this job to `.github/workflows/pr-ci.yml`:

```yaml
  desktop:
    name: Desktop
    needs: changes
    if: needs.changes.outputs.desktop_changed == 'true'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: pnpm
          cache-dependency-path: apps/desktop/pnpm-lock.yaml

      - uses: pnpm/action-setup@v4
        with:
          version: "11"

      - name: Install desktop dependencies
        working-directory: apps/desktop
        run: pnpm install --frozen-lockfile

      - name: Type-check desktop
        working-directory: apps/desktop
        run: npx tsc --noEmit

      - name: Run desktop tests
        working-directory: apps/desktop
        run: node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs

      - name: Build desktop
        working-directory: apps/desktop
        run: pnpm build
```

- [ ] **Step 2: Re-read the workflow file and confirm the desktop gate**

Run:

```bash
python3 - <<'PY'
import yaml, pathlib
data = yaml.safe_load(pathlib.Path(".github/workflows/pr-ci.yml").read_text())
desktop = data["jobs"]["desktop"]
assert desktop["needs"] == "changes"
assert desktop["if"] == "needs.changes.outputs.desktop_changed == 'true'"
print("desktop gate OK")
PY
```

Expected: `desktop gate OK`

- [ ] **Step 3: Commit the desktop CI job**

```bash
git add .github/workflows/pr-ci.yml
git commit -m "ci: add desktop PR validation job"
```

### Task 3: Add the Rust validation and no-op jobs

**Files:**
- Modify: `.github/workflows/pr-ci.yml`

- [ ] **Step 1: Add the Rust job gated on Rust changes**

Append this job:

```yaml
  rust:
    name: Rust
    needs: changes
    if: needs.changes.outputs.rust_changed == 'true'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: crates/orkworksd

      - name: Run Rust tests
        run: cargo test --manifest-path crates/orkworksd/Cargo.toml
```

- [ ] **Step 2: Add the green no-op job for non-code PRs**

Append this job:

```yaml
  noop:
    name: No relevant code changes
    needs: changes
    if: needs.changes.outputs.relevant_code_changed != 'true'
    runs-on: ubuntu-latest
    steps:
      - name: Explain skip
        run: echo "No desktop or Rust changes detected; skipping code validation."
```

- [ ] **Step 3: Validate the Rust and no-op gates locally**

Run:

```bash
python3 - <<'PY'
import yaml, pathlib
data = yaml.safe_load(pathlib.Path(".github/workflows/pr-ci.yml").read_text())
assert data["jobs"]["rust"]["if"] == "needs.changes.outputs.rust_changed == 'true'"
assert data["jobs"]["noop"]["if"] == "needs.changes.outputs.relevant_code_changed != 'true'"
print("rust/noop gates OK")
PY
```

Expected: `rust/noop gates OK`

- [ ] **Step 4: Commit the Rust and no-op jobs**

```bash
git add .github/workflows/pr-ci.yml
git commit -m "ci: add Rust and noop PR validation jobs"
```

### Task 4: Document the new PR CI behavior

**Files:**
- Modify: `README.md`
- Modify: `AGENTS.md`

- [ ] **Step 1: Update `README.md` to mention the new PR CI workflow**

In the build/release section, add a short paragraph after the release workflow description:

```md
Normal pull requests use `.github/workflows/pr-ci.yml`. That workflow routes by changed surface:

- `apps/desktop/**` runs desktop type-check, tests, and build
- `crates/orkworksd/**` runs Rust tests
- PRs that touch neither surface get a lightweight passing no-op check for status clarity
```

- [ ] **Step 2: Update `AGENTS.md` to mention PR CI alongside the release workflow**

Add a short note near the package manager / workflow guidance:

```md
GitHub Actions now has two distinct workflow classes:

- `.github/workflows/release.yml` for tag-driven release packaging only
- `.github/workflows/pr-ci.yml` for pull-request validation on `main`

PR CI is path-routed: desktop changes run desktop validation, Rust changes run Rust tests, and non-code PRs receive a lightweight passing no-op check.
```

- [ ] **Step 3: Commit the docs updates**

```bash
git add README.md AGENTS.md
git commit -m "docs: describe PR CI workflow"
```

### Task 5: Verify the workflow end to end and run repo completion checks

**Files:**
- Verify: `.github/workflows/pr-ci.yml`
- Verify: `README.md`
- Verify: `AGENTS.md`

- [ ] **Step 1: Re-run local workflow structure validation**

Run:

```bash
python3 - <<'PY'
import yaml, pathlib
data = yaml.safe_load(pathlib.Path(".github/workflows/pr-ci.yml").read_text())
assert set(data["jobs"]) == {"changes", "desktop", "rust", "noop"}
print("job set OK")
PY
```

Expected: `job set OK`

- [ ] **Step 2: Run the desktop verification commands the workflow will execute**

Run:

```bash
cd apps/desktop && pnpm install --frozen-lockfile
cd apps/desktop && npx tsc --noEmit
cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
cd apps/desktop && pnpm build
```

Expected: all four commands succeed

- [ ] **Step 3: Run the Rust verification command the workflow will execute**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: tests pass

- [ ] **Step 4: Run the doc currency hook**

Run:

```bash
bash .claude/hooks/doc-check.sh
```

Expected: exit 0 with no required follow-up docs flagged

- [ ] **Step 5: Commit any final workflow/doc fixes from verification**

```bash
git add .github/workflows/pr-ci.yml README.md AGENTS.md
git commit -m "ci: finalize PR validation workflow"
```
