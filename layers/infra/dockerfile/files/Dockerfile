# Multi-stage build for a JS-only, Rust-only, or polyglot workspace.
#
# This file cannot check which frontend or backend layer was selected (a
# layer never names another layer), so every stage below detects its own
# toolchain by file presence instead: `if [ -f package.json ]`, `if [ -f
# Cargo.toml ]`. Each stage always `COPY . .`s the whole build context rather
# than the manifest first, on purpose: a manifest-only COPY that has to stay
# correct whether or not that manifest exists would need an unmatched glob to
# not fail the build, and that behaviour is not something to bet a template
# on without a Docker daemon in reach to confirm it against. Losing the
# dependency-layer cache this way is a real cost, not a free simplification.

# === JavaScript dependencies and build
FROM node:22-slim AS js-build
# node:22-slim: the widest sane major-only tag. Docker has no equivalent of
# `pnpm add`/`cargo add` that resolves "latest LTS" for a base image at build
# time the way a package manager resolves a package, so the major version is
# pinned (per culm decision 5's carve-out for a config shape coupled to a
# major) and nothing narrower is.
WORKDIR /app
# `npm install -g pnpm@11` rather than `corepack enable`: corepack only
# shims the `pnpm` command and fetches the real binary lazily, on the
# container's first invocation, which is exactly the cold-start cost this
# avoids. The major version is pinned deliberately, the same carve-out as
# `node:22-slim` above: pnpm 12 (npm's unqualified "latest" the day this
# was found) broke `husky`'s own root `prepare` script inside this exact
# image in a way pnpm 11 does not, so this is not decision 5's normal
# "let the tool resolve latest" case, it is a config shape coupled to a
# major, the same reasoning that keeps the Node and Rust image tags at
# major-only rather than unpinned. Revisit once a pnpm 12.x is confirmed
# not to carry the same regression.
RUN npm install -g pnpm@11
COPY . .
RUN if [ -f package.json ]; then \
      pnpm install --frozen-lockfile || pnpm install; \
    fi

# `--if-present` no-ops rather than failing when there is no `build` script,
# which covers both a Rust-only workspace (no package.json at all) and the
# monorepo gap documented in layer.toml (a namespaced `build:<id>` that this
# stage cannot name).
RUN if [ -f package.json ]; then \
      pnpm run --if-present build; \
    fi

# === Rust build
FROM rust:1-slim AS rust-build
# rust:1-slim: Rust has shipped exactly one major version since 1.0, so this
# tag is already the widest one that exists and tracks every stable release.
WORKDIR /app
COPY . .

# The directory is created unconditionally so the runtime stage below always
# has something to copy from, even when this workspace has no Cargo.toml.
RUN mkdir -p target/release && \
    if [ -f Cargo.toml ]; then \
      cargo build --release; \
    fi

# === Runtime
FROM node:22-slim AS runtime
# One runtime image serves both toolchains. A Rust binary built above is a
# self-contained executable and needs no Rust toolchain to run, so this pays
# for Node's slim image once and costs a Rust-only workspace nothing beyond
# the binary already produced.
RUN npm install -g pnpm@11 \
    && groupadd --system app \
    && useradd --system --gid app --create-home --home-dir /app app

WORKDIR /app
COPY --from=js-build --chown=app:app /app /app
COPY --from=rust-build --chown=app:app /app/target/release /app/target/release

USER app
ENV HOST=0.0.0.0
ENV PORT=3000
EXPOSE 3000

# docker-entrypoint.sh decides at container start whether to run the compiled
# Rust binary or a JS start/preview script, for the same reason this file
# cannot decide it at build time: naming the app is naming a layer.
ENTRYPOINT ["/app/docker-entrypoint.sh"]
