# soulscale.store: Terms, Privacy and the front page

**The picture:** you copy this `soulscale-site` folder from your PC to NYC1 and run one script there.
The script puts the pages in `/srv/soulscale.store/`, adds a `soulscale.store` site to Caddy, checks
Caddy's config, reloads it, and tells you when the pages are live over HTTPS:

| URL | What it is | Served from |
|---|---|---|
| https://soulscale.store/tos/ | Terms of Service | `/srv/soulscale.store/tos/index.html` |
| https://soulscale.store/privacy/ | Privacy Policy | `/srv/soulscale.store/privacy/index.html` |
| https://soulscale.store/ | "Coming soon" page until the game server runs on NYC1, then the game | `/srv/soulscale.store/index.html`, later the game on 127.0.0.1:3001 |

`/terms` redirects to `/tos/`, and `/privacy-policy` redirects to `/privacy/`.

## Before you start (once)

1. **DNS:** at your domain registrar, add an **A record** for `soulscale.store` pointing at NYC1's
   public IPv4 address. (An A record is the DNS entry that says "this name lives at this IP address".)
   Optional: the same for `www.soulscale.store`. If it exists, the script redirects it to
   `soulscale.store`.
2. **Firewall:** ports 80 and 443 open on NYC1. They already are if Caddy serves other sites there.
3. **Mailbox:** make sure `support@soulscale.store` actually receives mail. Both pages promise
   replies there.

## Put it live (Windows PowerShell)

1. Unzip `soulscale-site.zip` into your Downloads folder. You get a folder called `soulscale-site`.
2. In PowerShell: `cd $HOME\Downloads`
3. Copy the folder to NYC1: `scp -r soulscale-site nyc1:~/`
4. Log in to NYC1: `ssh nyc1`
5. Run the installer: `sudo bash ~/soulscale-site/install.sh`
6. The last lines should say `https://soulscale.store/privacy/ and https://soulscale.store/tos/ are live`
   and print both URLs. The first run can take up to a minute while Caddy gets the certificate.
7. Discord Developer Portal → your app → **General Information** → paste
   `https://soulscale.store/tos/` into **Terms of Service URL** and `https://soulscale.store/privacy/`
   into **Privacy Policy URL** → **Save Changes**.

## Later

- **Changing the wording:** edit the files in `www/` (the repo copy lives in `site/www/`), then
  repeat steps 3 to 5. Change the "Last updated" date at the top of the page you edited.
- **After installing the game server on NYC1:** run step 5 again. The script sees the game answering
  on 127.0.0.1:3001 and switches `/` from the holding page to the game.
- **Undo:** delete `/etc/caddy/sites/soulscale.store.caddy`, remove the `import` line the script added
  at the end of `/etc/caddy/Caddyfile` (it's labelled "Soulscale"), then `sudo systemctl reload caddy`.
  Every run also leaves a timestamped backup next to each file it changed (`*.bak.<date-time>`).

## What the script won't do

- Touch any other site. If `soulscale.store` already appears in some other Caddy config, it stops and
  tells you where.
- Reload a broken config. If `caddy validate` fails, it puts your previous files back and stops.

Options, if you ever need them: `MODE=holding` or `MODE=game` forces the front page choice;
`UPSTREAM=127.0.0.1:3005` if the game runs on another port; `CADDYFILE=/path/Caddyfile` if your
Caddy config isn't in `/etc/caddy`.
