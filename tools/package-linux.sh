#!/usr/bin/env bash
# Build a ready-to-install Linux bundle of Soulscale.
#
#   ./tools/package-linux.sh            -> soulscale-linux.tar.gz in the repo root
#
# Run it on the server itself (or a machine with the same Linux release, e.g. matching
# Ubuntu in WSL), because the binary links against that system's C library.
# Needs: Rust 1.88+ (https://rustup.rs), Node 22+ and npm.
#
# Install on the server (as root):
#   tar xzf soulscale-linux.tar.gz -C /opt          # creates /opt/soulscale
#   useradd --system --home /opt/soulscale --shell /usr/sbin/nologin soulscale
#   cp /opt/soulscale/.env.example /opt/soulscale/.env   # then fill it in
#   mkdir -p /opt/soulscale/data && chown -R soulscale:soulscale /opt/soulscale/data
#   cp /opt/soulscale/deploy/soulscale.service /etc/systemd/system/
#   cp /opt/soulscale/deploy/soulscale-tmpfiles.conf /etc/tmpfiles.d/soulscale.conf
#   systemctl daemon-reload && systemctl enable --now soulscale
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
tar czf soulscale-linux.tar.gz -C "$stage" soulscale
echo "wrote $root/soulscale-linux.tar.gz ($(du -h soulscale-linux.tar.gz | cut -f1))"
