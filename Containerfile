# OrkWorks toolchain dev container — Tier 1 of issue #80.
#
# Podman-first (OCI); builds and runs identically under `docker compose`.
# This is a TOOLCHAIN-ONLY image: it copies no source. The repo is bind-mounted
# at /workspace by compose.yaml, so editing source never triggers a rebuild.
#
# Matches release.yml: Node 22, pnpm 11 (corepack), Rust stable.

FROM node:22-bookworm-slim

# System build deps for the Rust sidecar: libgit2-sys builds a vendored libgit2
# via cmake + a C toolchain; git2's default `https` feature links openssl-sys,
# which needs libssl-dev + pkg-config on Linux (ubuntu CI ships these already,
# bookworm-slim does not); git/curl/ca-certificates back rustup and crate fetches.
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential \
      cmake \
      pkg-config \
      libssl-dev \
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
        --default-toolchain stable --component clippy rustfmt

# pnpm 11 via corepack (bundled with Node 22), matching the packageManager pin
# in apps/desktop/package.json.
RUN corepack enable && corepack prepare pnpm@11.9.0 --activate

WORKDIR /workspace

# Keep the service alive for `compose exec`; `compose run --rm` overrides this.
CMD ["sleep", "infinity"]
