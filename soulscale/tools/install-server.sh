#!/usr/bin/env bash
# Install or update the Soulscale game server from this bundle (Ubuntu 22.04+ / Debian 12+).
#
#   sudo bash install.sh                         # first install or update; asks for Discord keys once
#   sudo BIND=127.0.0.1:3005 bash install.sh     # if port 3001 is taken by something else
#
# Safe to re-run. Updates keep your .env (settings and keys) and data/ (saved matches).
# Stops at the first problem instead of carrying on regardless.
set -euo pipefail

PREFIX="${PREFIX:-/opt/soulscale}"
SVC="${SVC:-soulscale}"
SVC_USER="${SVC_USER:-soulscale}"
SYSTEMD_DIR="${SYSTEMD_DIR:-/etc/systemd/system}"
TMPFILES_DIR="${TMPFILES_DIR:-/etc/tmpfiles.d}"
SYSTEMCTL="${SYSTEMCTL:-systemctl}"
BIND="${BIND:-}"

here="$(cd "$(dirname "$0")" && pwd)"
say()  { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m ok\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m !!\033[0m %s\n' "$*"; }
die()  { printf '\033[1;31mERR\033[0m %s\n' "$*" >&2; exit 1; }

# ------------------------------------------------------------------ preflight
[ -x "$here/soulscale-server" ] || die "run this from the unpacked bundle folder (soulscale-server not found next to it)"
[ "$(id -u)" -eq 0 ] || die "run it with sudo: sudo bash $0"
glibc="$(ldd --version 2>/dev/null | head -1 | grep -oE '[0-9]+\.[0-9]+$' || echo 0.0)"
if [ "$(printf '%s\n2.34\n' "$glibc" | sort -V | head -1)" != 2.34 ]; then
  die "this server has glibc $glibc; Soulscale needs 2.34+ (Ubuntu 22.04 or newer, Debian 12 or newer)"
fi
ok "$(hostname): glibc $glibc"

# Port: from BIND=, else the existing .env, else the default.
if [ -z "$BIND" ] && [ -f "$PREFIX/.env" ]; then
  BIND="$(grep -E '^SOULSCALE_BIND=' "$PREFIX/.env" | tail -1 | cut -d= -f2- || true)"
fi
BIND="${BIND:-127.0.0.1:3001}"
host="${BIND%:*}"; port="${BIND##*:}"
ours_running=0
if $SYSTEMCTL is-active --quiet "$SVC" 2>/dev/null; then ours_running=1; fi
if [ $ours_running = 0 ] && (exec 3<>"/dev/tcp/$host/$port") 2>/dev/null; then
  die "something else is already listening on $BIND.
     Pick a free port and run again, e.g.:  sudo BIND=127.0.0.1:3005 bash $0
     (then use UPSTREAM=127.0.0.1:3005 with the site installer too)"
fi
ok "port $BIND is ours to use"

# ------------------------------------------------------------------ user and files
if id "$SVC_USER" >/dev/null 2>&1; then
  ok "user $SVC_USER already exists"
else
  useradd --system --home "$PREFIX" --shell /usr/sbin/nologin "$SVC_USER"
  ok "created system user $SVC_USER"
fi

if [ $ours_running = 1 ]; then
  say "Stopping the running server for the update"
  $SYSTEMCTL stop "$SVC"
fi

say "Copying the server to $PREFIX"
mkdir -p "$PREFIX/client" "$PREFIX/deploy" "$PREFIX/data"
install -m 0755 "$here/soulscale-server" "$PREFIX/soulscale-server"
if [ -f "$here/soulscale-sim" ]; then install -m 0755 "$here/soulscale-sim" "$PREFIX/soulscale-sim"; fi
rm -rf "$PREFIX/client/dist.new"
cp -R "$here/client/dist" "$PREFIX/client/dist.new"
rm -rf "$PREFIX/client/dist"
mv "$PREFIX/client/dist.new" "$PREFIX/client/dist"
cp "$here/.env.example" "$PREFIX/.env.example"
cp "$here/deploy/"* "$PREFIX/deploy/"
for f in README.md THIRD_PARTY.md; do
  if [ -f "$here/$f" ]; then cp "$here/$f" "$PREFIX/"; fi
done
# Everything root-owned and world-readable except data/ (saved matches hold player names).
find "$PREFIX" -path "$PREFIX/data" -prune -o -exec chown root:root {} + -exec chmod u=rwX,go=rX {} +
chown -R "$SVC_USER:$SVC_USER" "$PREFIX/data"
chmod 0750 "$PREFIX/data"
ok "binary, client and deploy files in place (data/ and .env untouched)"

# ------------------------------------------------------------------ .env
set_env() { # set_env KEY VALUE  (replace the KEY= line in $PREFIX/.env)
  K="$1" V="$2" awk 'BEGIN{k=ENVIRON["K"]; v=ENVIRON["V"]; done=0}
    index($0, k"=")==1 {print k"="v; done=1; next} {print}
    END{if(!done) print k"="v}' "$PREFIX/.env" >"$PREFIX/.env.tmp"
  mv "$PREFIX/.env.tmp" "$PREFIX/.env"
}
if [ -f "$PREFIX/.env" ]; then
  ok "keeping your existing $PREFIX/.env"
else
  say "Creating $PREFIX/.env"
  cp "$PREFIX/.env.example" "$PREFIX/.env"
  set_env SOULSCALE_DEV_LOGIN off
  set_env SOULSCALE_DATA_DIR data
  set_env SOULSCALE_STATIC_DIR client/dist
  if [ -t 0 ]; then
    echo "Discord keys: Developer Portal > your app > OAuth2. Press Enter to skip any you don't have yet."
    read -r -p "  Client ID: " cid
    read -r -s -p "  Client Secret (hidden): " csec; echo
    read -r -s -p "  Bot token, optional (hidden): " btok; echo
    if [ -n "$cid" ]; then set_env DISCORD_CLIENT_ID "$cid"; fi
    if [ -n "$csec" ]; then set_env DISCORD_CLIENT_SECRET "$csec"; fi
    if [ -n "$btok" ]; then set_env DISCORD_BOT_TOKEN "$btok"; fi
  fi
fi
set_env SOULSCALE_BIND "$BIND"
chown "root:$SVC_USER" "$PREFIX/.env"
chmod 0640 "$PREFIX/.env"
if ! grep -qE '^DISCORD_CLIENT_SECRET=.+' "$PREFIX/.env"; then
  warn "no Discord keys yet: the server runs, but nobody can sign in from Discord until you add them:"
  warn "  sudo nano $PREFIX/.env   then   sudo systemctl restart $SVC"
fi

# ------------------------------------------------------------------ systemd + clean-up rule
say "Installing the service"
sed "s#/opt/soulscale#$PREFIX#g; s#^User=.*#User=$SVC_USER#; s#^Group=.*#Group=$SVC_USER#" \
  "$here/deploy/soulscale.service" >"$SYSTEMD_DIR/$SVC.service"
sed "s#/opt/soulscale#$PREFIX#g" "$here/deploy/soulscale-tmpfiles.conf" >"$TMPFILES_DIR/$SVC.conf"
$SYSTEMCTL daemon-reload
$SYSTEMCTL enable --quiet "$SVC"
$SYSTEMCTL restart "$SVC"
ok "service $SVC enabled and (re)started; saved matches auto-delete after 12 months"

say "Waiting for http://$BIND/api/health"
for _ in $(seq 1 20); do
  if [ "$(curl -fsS --max-time 2 "http://$BIND/api/health" 2>/dev/null || true)" = ok ]; then
    ok "Soulscale is running on $BIND"
    site="$(getent passwd "${SUDO_USER:-root}" | cut -d: -f6)/soulscale-site/install.sh"
    echo
    if [ -f "$site" ]; then
      echo "Last step: switch soulscale.store's front page from 'coming soon' to the game:"
      echo "  sudo UPSTREAM=$BIND bash $site"
    else
      echo "Last step: run the soulscale-site installer again (with UPSTREAM=$BIND) so soulscale.store points at the game."
    fi
    exit 0
  fi
  sleep 1
done
warn "it didn't answer. The server's own log says:"
journalctl -u "$SVC" -n 30 --no-pager 2>/dev/null || true
exit 1
