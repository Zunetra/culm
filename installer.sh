#!/bin/sh
# culm installer (handbook 0004's shell channel).
#
#   curl -fsSL https://github.com/Zunetra/culm/releases/latest/download/installer.sh | sh
#
# Downloads the prebuilt binary for this machine from the GitHub release and
# verifies it against the release's SHA256SUMS before installing anything.
# A checksum that does not match stops the install; there is no flag to skip
# the check.
#
# CULM_VERSION      a tag such as v0.1.0 (default: the latest release)
# CULM_INSTALL_DIR  where the binary goes (default: $HOME/.local/bin)

set -eu

REPO="Zunetra/culm"
VERSION="${CULM_VERSION:-latest}"
INSTALL_DIR="${CULM_INSTALL_DIR:-$HOME/.local/bin}"

fail() {
    echo "culm installer: $*" >&2
    exit 1
}

case "$(uname -s)" in
    Linux) os="unknown-linux-gnu" ;;
    Darwin) os="apple-darwin" ;;
    *) fail "no prebuilt binary for $(uname -s); try 'cargo install culm' or 'npx culm'" ;;
esac

case "$(uname -m)" in
    x86_64 | amd64) arch="x86_64" ;;
    arm64 | aarch64) arch="aarch64" ;;
    *) fail "no prebuilt binary for $(uname -m); try 'cargo install culm'" ;;
esac

artifact="culm-${arch}-${os}"

if [ "$VERSION" = "latest" ]; then
    base="https://github.com/${REPO}/releases/latest/download"
else
    base="https://github.com/${REPO}/releases/download/${VERSION}"
fi

if command -v curl >/dev/null 2>&1; then
    fetch() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
    fetch() { wget -q "$1" -O "$2"; }
else
    fail "needs curl or wget"
fi

if command -v sha256sum >/dev/null 2>&1; then
    digest() { sha256sum "$1" | cut -d ' ' -f 1; }
elif command -v shasum >/dev/null 2>&1; then
    digest() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
else
    fail "needs sha256sum or shasum to verify the download"
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "Downloading ${artifact} (${VERSION})"
fetch "${base}/${artifact}" "${work}/${artifact}" || fail "cannot download ${base}/${artifact}"
fetch "${base}/SHA256SUMS" "${work}/SHA256SUMS" || fail "cannot download ${base}/SHA256SUMS"

expected="$(grep " ${artifact}\$" "${work}/SHA256SUMS" | cut -d ' ' -f 1)"
[ -n "$expected" ] || fail "SHA256SUMS has no entry for ${artifact}"
actual="$(digest "${work}/${artifact}")"
[ "$expected" = "$actual" ] || fail "checksum mismatch for ${artifact}: expected ${expected}, got ${actual}"

mkdir -p "$INSTALL_DIR"
install -m 755 "${work}/${artifact}" "${INSTALL_DIR}/culm" 2>/dev/null || {
    cp "${work}/${artifact}" "${INSTALL_DIR}/culm"
    chmod 755 "${INSTALL_DIR}/culm"
}

echo "Installed culm to ${INSTALL_DIR}/culm"
case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *) echo "Add ${INSTALL_DIR} to your PATH to run it as 'culm'." ;;
esac
