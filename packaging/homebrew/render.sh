#!/bin/sh
# Renders the Homebrew formula for one release from its SHA256SUMS:
#   packaging/homebrew/render.sh <version-without-v> <SHA256SUMS> > culm.rb
set -eu
version="$1"
sums="$2"
here="$(dirname "$0")"

sha() {
    value="$(grep " culm-$1\$" "$sums" | cut -d ' ' -f 1)"
    [ -n "$value" ] || { echo "no checksum for culm-$1" >&2; exit 1; }
    echo "$value"
}

sed \
    -e "s/@VERSION@/${version}/g" \
    -e "s/@SHA_AARCH64_APPLE_DARWIN@/$(sha aarch64-apple-darwin)/" \
    -e "s/@SHA_X86_64_APPLE_DARWIN@/$(sha x86_64-apple-darwin)/" \
    -e "s/@SHA_AARCH64_UNKNOWN_LINUX_GNU@/$(sha aarch64-unknown-linux-gnu)/" \
    -e "s/@SHA_X86_64_UNKNOWN_LINUX_GNU@/$(sha x86_64-unknown-linux-gnu)/" \
    "$here/culm.rb.in" | grep -v '^# ' | sed '1{/^$/d}'
