# OrkWorks toolchain dev container — Tier 1 of issue #80.
#
# Podman-first (OCI); builds and runs identically under `docker compose`.
# The dev target copies no source; compose.yaml bind-mounts /workspace.
# The explicit headless-replay target snapshots only its allowlisted fixture.
#
# Matches release.yml: Node 22, pnpm 12 (corepack), Rust stable.

FROM node:22-bookworm-slim AS dev

# System build deps for the Rust sidecar: libgit2-sys builds a vendored libgit2
# via cmake + a C toolchain; git2's default `https` feature links openssl-sys,
# which needs libssl-dev + pkg-config on Linux (ubuntu CI ships these already,
# bookworm-slim does not); git/curl/ca-certificates back rustup and crate fetches.
# python3 is a runtime dep of the harness reporter scripts (hook payloads);
# macOS hosts and ubuntu CI runners ship it, bookworm-slim does not.
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential \
      cmake \
      pkg-config \
      libssl-dev \
      python3 \
      git \
      curl \
      ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Rust via rustup. The channel and components ultimately come from
# rust-toolchain.toml at runtime; pre-install stable here so the first `cargo`
# invocation inside the container does not pay a toolchain download.
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --no-modify-path --profile minimal \
        --default-toolchain stable --component clippy --component rustfmt

# /etc/profile (sourced by login shells, e.g. the documented `bash -lc` runs)
# resets PATH and would drop cargo/rustup; re-export it for login shells.
RUN printf 'export PATH="/usr/local/cargo/bin:$PATH"\n' > /etc/profile.d/cargo.sh

# pnpm 12 via corepack (bundled with Node 22), matching the packageManager pin
# in apps/desktop/package.json.
RUN corepack enable && corepack prepare pnpm@12.18.0 --activate

WORKDIR /workspace

# Keep the service alive for `compose exec`; `compose run --rm` overrides this.
CMD ["sleep", "infinity"]

# Explicit trusted-source fixture snapshot; no host mounts or runtime downloads.
FROM dev AS headless-replay
RUN apt-get update && apt-get install -y --no-install-recommends \
      xvfb xauth libgtk-3-0 libnss3 libasound2 libgbm1 fonts-dejavu-core \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /opt/replay
COPY apps/desktop/package.json apps/desktop/pnpm-lock.yaml apps/desktop/pnpm-workspace.yaml ./
COPY apps/desktop/patches ./patches
COPY apps/desktop/scripts/setup-electron.mjs ./scripts/setup-electron.mjs
RUN pnpm install --frozen-lockfile && pnpm exec install-electron \
    && ldd node_modules/electron/dist/electron > /tmp/electron-ldd \
    && ( ! grep -q 'not found' /tmp/electron-ldd || { cat /tmp/electron-ldd; exit 1; } )
COPY apps/desktop/tsconfig.json ./tsconfig.json
COPY apps/desktop/src ./src
COPY apps/desktop/tests/applicationShell.test.mjs ./tests/applicationShell.test.mjs
COPY apps/desktop/tests/fixtures/shellSubprocess.mjs ./tests/fixtures/shellSubprocess.mjs
RUN chmod -R go-w /opt/replay && mkdir -p /artifacts/replay \
    && chown 1000:1000 /artifacts/replay
ENV HOME=/tmp/orkworks-home \
    TMPDIR=/tmp \
    ORKWORKS_REPLAY_ARTIFACT_DIR=/artifacts/replay
USER 1000:1000
CMD ["node", "--experimental-strip-types", "--test", "--test-name-pattern=fixed shell: app-review-replay$", "tests/applicationShell.test.mjs"]

# Optional local analysis stage; the normal compose service targets dev.
FROM dev AS sonar
RUN mkdir -p /opt/sonar-scanner && pnpm --dir /opt/sonar-scanner add @sonar/scan@5.0.1
ENV PATH=/opt/sonar-scanner/node_modules/.bin:$PATH
CMD ["sonar-scanner-npm"]
