#!/usr/bin/env bash
# Put soulscale.store live on this server (NYC1) behind Caddy:
#   /privacy/ and /tos/  -> static pages from /srv/soulscale.store
#   everything else      -> the game server on 127.0.0.1:3001 once it's running,
#                           or a "coming soon" holding page until then.
#
#   sudo bash install.sh            # install or update (safe to re-run)
#   sudo MODE=holding bash install.sh   # force the holding page even if the game is up
#   sudo MODE=game bash install.sh      # force proxying to the game even if it's down
#
# It never edits another site's config. It backs up what it changes, checks the whole Caddy
# config with `caddy validate` before reloading, and puts everything back if that check fails.
set -euo pipefail

DOMAIN="${DOMAIN:-soulscale.store}"
SRV_DIR="${SRV_DIR:-/srv/soulscale.store}"
CADDYFILE="${CADDYFILE:-/etc/caddy/Caddyfile}"
CADDY_DIR="${CADDY_DIR:-$(dirname "$CADDYFILE")}"
SITE_FILE="${SITE_FILE:-$CADDY_DIR/sites/$DOMAIN.caddy}"
UPSTREAM="${UPSTREAM:-127.0.0.1:3001}"
MODE="${MODE:-auto}"
RELOAD="${RELOAD:-systemctl reload caddy}"
CHECK="${CHECK:-1}"

here="$(cd "$(dirname "$0")" && pwd)"
stamp="$(date +%Y%m%d-%H%M%S)"

say()  { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
ok()   { printf '\033[1;32m ok\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m !!\033[0m %s\n' "$*"; }
die()  { printf '\033[1;31mERR\033[0m %s\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------- preflight
[ -d "$here/www" ] || die "can't find $here/www (run this from the unpacked soulscale-site folder)"
if [ "$(id -u)" -ne 0 ] && [ "$SRV_DIR" = /srv/soulscale.store ]; then
  die "run it with sudo: sudo bash $0"
fi
command -v caddy >/dev/null || die "caddy isn't installed on this machine"
[ -f "$CADDYFILE" ] || die "no Caddy config at $CADDYFILE (set CADDYFILE=... if it lives elsewhere)"

# Refuse to fight an existing hand-written soulscale.store block. Ignore our own site file,
# our backups, comments, and the import line a previous run of this script added.
others="$(grep -rnF "$DOMAIN" "$CADDY_DIR" 2>/dev/null |
  grep -vF "$SITE_FILE:" |
  grep -v '\.bak\.' |
  grep -vE "^[^:]+:[0-9]+:[[:space:]]*(#|import[[:space:]]+${SITE_FILE}[[:space:]]*$)" || true)"
if [ -n "$others" ]; then
  die "$DOMAIN is already configured elsewhere:
$others
     Remove or rename that block first, then re-run this script."
fi

# ---------------------------------------------------------------- 1. static files
say "Copying pages to $SRV_DIR"
mkdir -p "$(dirname "$SRV_DIR")"
new="$SRV_DIR.new.$$"
rm -rf "$new"
cp -R "$here/www" "$new"
chmod -R u=rwX,go=rX "$new"
[ "$(id -u)" -eq 0 ] && chown -R root:root "$new"
if [ -d "$SRV_DIR" ]; then
  mv "$SRV_DIR" "$SRV_DIR.old.$$"
  mv "$new" "$SRV_DIR"
  rm -rf "$SRV_DIR.old.$$"
else
  mv "$new" "$SRV_DIR"
fi
ok "$(find "$SRV_DIR" -type f | wc -l) files in place"

# ---------------------------------------------------------------- 2. game or holding page
if [ "$MODE" = auto ]; then
  if [ "$(curl -fsS --max-time 3 "http://$UPSTREAM/api/health" 2>/dev/null || true)" = ok ]; then
    MODE=game
  else
    MODE=holding
  fi
fi
case "$MODE" in
  game)    ok "game server answers on $UPSTREAM: / will go to the game" ;;
  holding) warn "no game server on $UPSTREAM yet: / will show the holding page (re-run this after installing the game)" ;;
  *)       die "MODE must be auto, game or holding" ;;
esac

if [ "$MODE" = game ]; then
  rest="	# Everything else is the game (page, /api, /ws WebSocket).
	handle {
		reverse_proxy $UPSTREAM
	}"
else
  rest="	# The game isn't installed yet: show the holding page instead.
	handle {
		root * $SRV_DIR
		import soulscale_static_headers
		file_server
	}"
fi

www=""
if getent hosts "www.$DOMAIN" >/dev/null 2>&1; then
  www="
www.$DOMAIN {
	redir https://$DOMAIN{uri} 308
}"
fi

# ---------------------------------------------------------------- 3. Caddy site file
say "Writing $SITE_FILE"
mkdir -p "$(dirname "$SITE_FILE")"
tmp="$(mktemp)"
cat >"$tmp" <<EOF
# $DOMAIN: Soulscale: Purgatory (Discord Activity) and its legal pages.
# Written by soulscale-site/install.sh on $stamp ($MODE mode). Re-run that script instead of editing.

(soulscale_static_headers) {
	header {
		Content-Security-Policy "default-src 'none'; style-src 'self'; font-src 'self'; img-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
		X-Content-Type-Options nosniff
		Referrer-Policy no-referrer
		Cache-Control "public, max-age=3600"
	}
	encode zstd gzip
}

$DOMAIN {
	# Static pages from $SRV_DIR
	@site path /privacy /privacy/* /tos /tos/* /legal/*
	handle @site {
		root * $SRV_DIR
		import soulscale_static_headers
		file_server
	}

	@terms path /terms /terms/
	redir @terms /tos/ 308
	@privacy_alias path /privacy-policy /privacy-policy/
	redir @privacy_alias /privacy/ 308

$rest
}
$www
EOF

backup_caddyfile="$CADDYFILE.bak.$stamp"
backup_site=""
cp -p "$CADDYFILE" "$backup_caddyfile"
if [ -f "$SITE_FILE" ]; then
  backup_site="$SITE_FILE.bak.$stamp"
  cp -p "$SITE_FILE" "$backup_site"
fi
install -m 0644 "$tmp" "$SITE_FILE"
rm -f "$tmp"

restore() {
  warn "putting the previous Caddy config back"
  cp -p "$backup_caddyfile" "$CADDYFILE"
  if [ -n "$backup_site" ]; then cp -p "$backup_site" "$SITE_FILE"; else rm -f "$SITE_FILE"; fi
}

# Is our file already pulled in (e.g. by an `import sites/*` line)? If not, add an import.
if caddy adapt --config "$CADDYFILE" --adapter caddyfile 2>/dev/null | grep -qF "\"$DOMAIN\""; then
  ok "your Caddyfile already imports $SITE_FILE"
else
  printf '\n# Soulscale (added by soulscale-site/install.sh %s)\nimport %s\n' "$stamp" "$SITE_FILE" >>"$CADDYFILE"
  ok "added 'import $SITE_FILE' to $CADDYFILE"
fi

say "Checking the whole Caddy config"
if ! caddy validate --config "$CADDYFILE" --adapter caddyfile; then
  restore
  die "caddy validate failed; nothing was reloaded. Your old config is back in place."
fi
ok "config is valid"

say "Reloading Caddy"
if ! $RELOAD; then
  restore
  $RELOAD || true
  die "reload failed; previous config restored"
fi
ok "Caddy reloaded"

# ---------------------------------------------------------------- 4. is it live?
[ "$CHECK" = 1 ] || { ok "done (live check skipped)"; exit 0; }

say "Checking DNS for $DOMAIN"
dns_ips="$(getent ahostsv4 "$DOMAIN" 2>/dev/null | awk '{print $1}' | sort -u | tr '\n' ' ')"
my_ips="$(hostname -I 2>/dev/null || true)"
if [ -z "$dns_ips" ]; then
  warn "$DOMAIN doesn't resolve yet. Add an A record pointing at this server ($my_ips) and re-run."
  exit 0
fi
match=0
for ip in $dns_ips; do case " $my_ips " in *" $ip "*) match=1 ;; esac; done
if [ $match = 1 ]; then
  ok "$DOMAIN -> $dns_ips (this server)"
else
  warn "$DOMAIN -> $dns_ips, which isn't this server ($my_ips)."
  warn "Fine if that's Cloudflare's proxy; otherwise fix the A record, or Caddy can't get a certificate."
fi

say "Fetching https://$DOMAIN/privacy/ and /tos/ (first run waits for the certificate)"
for _ in $(seq 1 12); do
  if curl -fsS --max-time 10 "https://$DOMAIN/privacy/" 2>/dev/null | grep -q "Privacy Policy" &&
     curl -fsS --max-time 10 "https://$DOMAIN/tos/" 2>/dev/null | grep -q "Terms of Service"; then
    ok "https://$DOMAIN/privacy/ and https://$DOMAIN/tos/ are live"
    echo
    echo "Paste these into the Discord Developer Portal > your app > General Information:"
    echo "  Terms of Service URL: https://$DOMAIN/tos/"
    echo "  Privacy Policy URL:   https://$DOMAIN/privacy/"
    exit 0
  fi
  sleep 5
done
warn "not answering over HTTPS yet. Look at: journalctl -u caddy --since '5 min ago'"
warn "Usual causes: DNS not pointing here yet, or ports 80/443 closed in the firewall (ufw allow 80,443/tcp)."
exit 1
