#!/usr/bin/env bash
# Build a ready-to-install Linux bundle of Soulscale.
#
#   ./tools/package-linux.sh            -> soulscale-linux.tar.gz in the repo root
#
# Built on Ubuntu 24.04 the server needs glibc 2.34+ to run (Ubuntu 22.04+, Debian 12+);
# the bundle's install.sh checks that. Building needs Rust 1.88+ and Node 22+ with npm.
#
# Install on the server: copy the tarball over, then
#   tar xzf soulscale-linux.tar.gz && sudo bash soulscale/install.sh
# Caddy and the legal pages: site/README.md (the site installer wires up soulscale.store).
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

for tool in cargo npm; do
  command -v "$tool" >/dev/null || { echo "$tool not found" >&2; exit 1; }
done

echo "== client"
(cd client && { [ -d node_modules ] || npm ci; } && npm run build)

echo "== server"
cargo build --release --locked -p soulscale-server -p soulscale-sim

echo "== bundle"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
out="$stage/soulscale"
mkdir -p "$out/client" "$out/deploy" "$out/data"
cp target/release/soulscale-server target/release/soulscale-sim "$out/"
cp -r client/dist "$out/client/dist"
cp .env.example "$out/.env.example"
cp deploy/soulscale.service deploy/soulscale-tmpfiles.conf "$out/deploy/"
cp README.md THIRD_PARTY.md "$out/"
install -m 0755 tools/install-server.sh "$out/install.sh"
tar czf soulscale-linux.tar.gz -C "$stage" soulscale
echo "wrote $root/soulscale-linux.tar.gz ($(du -h soulscale-linux.tar.gz | cut -f1))"
