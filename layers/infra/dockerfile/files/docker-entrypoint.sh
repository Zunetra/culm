#!/bin/sh
# Runs whichever process this workspace produced.
#
# The Dockerfile cannot name a specific frontend or backend layer, so this
# detects the same way at container start: a compiled Rust binary wins if
# one exists, otherwise a JS `start` script, otherwise a JS `preview`
# script. A workspace with more than one Rust binary (a monorepo with
# several backend layers) runs whichever one sorts first, which is a known
# limit of one container running one process; split into per-app services
# once that gap is worth closing.
set -e
cd /app

rust_bin=$(find target/release -maxdepth 1 -type f -executable 2>/dev/null | sort | head -n1)

if [ -n "$rust_bin" ]; then
  exec "./$rust_bin"
fi

if [ -f package.json ]; then
  has_script() {
    node -e "const s=(require('/app/package.json').scripts||{}); process.exit(s['$1']?0:1)"
  }

  if has_script start; then
    exec pnpm run start
  fi

  if has_script preview; then
    exec pnpm run preview
  fi
fi

echo "docker-entrypoint: found no Rust binary and no start/preview script in package.json" >&2
exit 1
