# Claude Code - fluxgeist

## Homeserver

- **Alias:** `homeserver` (configured in ~/.ssh/config)
- **Address:** pi@10.0.0.141
- **Hardware:** Raspberry Pi (aarch64)
- **OS:** Ubuntu 20.04.6 LTS
- **Disk:** 117G total, 40G used, 74G free
- **RAM:** 7.6G total, ~5.8G available
- **SSH:** Passwordless via ed25519 key, passphrase stored in macOS Keychain

Run commands on homeserver directly:
```bash
ssh homeserver '<command>'
```

### Services (systemd)

**Networking / Remote Access**
- `ssh` — SSH server
- `avahi-daemon` — mDNS/Bonjour (local network discovery)
- `openvpn` — OpenVPN (installed, currently exited/inactive)

**File Sharing**
- `smbd` + `nmbd` — Samba (Windows-compatible file sharing)
- `rpcbind` — RPC portmapper (likely NFS-related)

**Media / Audio**
- `shairport-sync` — AirPlay audio receiver
- `cups` + `cups-browsed` — printing service
- `seeed-voicecard` — Seeed voice card HAT (microphone)

**IoT / Home Automation**
- `mosquitto` — MQTT broker

**Containers**
- `docker` + `containerd` — Docker runtime (containers surveyed separately)

### Services (Docker)

**Running**
- `homeassistant` — Home Assistant (behind nginx proxy)
- `esphome` — ESP device firmware/config management (home automation)
- `reverse-proxy` (nginx) — reverse proxy, port 80
- `portainer` — Docker management UI, port 9000
- `duplicati` — backup software, port 8200

**Running (music stack)**
- `navidrome` — music streaming server (pointed at network drive)
- `soulseek` (slskd) — Soulseek P2P client (downloads to network drive)
- `soulbeet` — custom Rust/Dioxus web UI for managing Soulseek + beets; image built on Mac (Apple Silicon, linux/arm64), transferred to Pi via `buildx --dest tar` + `cat tar | ssh docker load`

### Music Stack Architecture
All services share the NFS music drive at `10.0.0.13`, mounted at `/opt/smb-downloads/Music`.

```
Soulseek (slskd) → downloads to /opt/smb-downloads/Music/downloads/  (inbox)
       ↓ (via Soulbeet UI trigger)
Beets → tags & moves files to /opt/smb-downloads/Music/Library/
       ↓
Navidrome → streams from /opt/smb-downloads/Music/Library/
```

**Compose files:**
- `/opt/docker-compose.yaml` — main stack (soulseek, navidrome, nginx, HA, ESPHome, duplicati, portainer)
- `/opt/soulbeet/deployment/docker-compose.yml` — soulbeet separately (uses `soulbeet:local` image built from `/opt/soulbeet/`)

**Beets configs:** `/opt/soulbeet/deployment/configs/`
- `beets_config.yaml` — default config, moves files into `Library/` organized as `$albumartist/$album/$track $title`
- `antonino_beets.yaml`, `peri_beets.yaml` — per-user configs

**Soulbeet tech stack:** Rust + Dioxus (fullstack web framework), SQLite, Axum, Tokio. Built on Mac (Apple Silicon, linux/arm64 via Docker buildx), deployed to Pi.

**How the flow works:**
1. User searches MusicBrainz for album/track in Soulbeet UI
2. Soulbeet searches slskd (Soulseek) and scores results by quality (FLAC > WAV > AAC > MP3)
3. Downloads queued in batches of 3, with retry/circuit breaker logic
4. WebSocket streams real-time download status to UI
5. On completion, beets runs: `beet -c <config> -d <target> import -q [-s] <source>`
   (do NOT pass `-l` — that would override `library:` in the config and write to a wrong DB)
6. Beets tags & moves files into `Library/$albumartist/$album/$track $title`
7. Navidrome picks up new files on next scan (every 1h)

**Key env vars (deployment compose):**
- `SLSKD_URL=http://soulseek:5030` — must be on same Docker network
- `SLSKD_API_KEY` — in `deployment/.env` (gitignored), matches slskd.yml
- `SLSKD_DOWNLOAD_PATH=/app/downloads` — where soulbeet looks for downloaded files
- `BEETS_CONFIG=/app/beets_config.yaml`
- `BEETS_ALBUM_MODE=true` — album mode for proper MusicBrainz tagging
- `NAVIDROME_DB_PATH=/navidrome/navidrome.db` — for targeted Navidrome DB cleanup on delete

**Mac build workflow (faster than building on Pi):**
1. Edit code in `~/Desktop/Developer/soulbeet` (branch: `feat/library-tab`)
2. `docker buildx build --platform linux/arm64 --output type=docker,name=soulbeet:local,dest=/tmp/soulbeet.tar .`
3. `cat /tmp/soulbeet.tar | ssh homeserver 'docker load'`
   ⚠️  Do NOT use `docker save soulbeet:local | ssh homeserver 'docker load'` — `buildx --dest` writes to a tar file only, it does NOT load into the local Docker daemon. `docker save` would read the old local image and overwrite the new one on the homeserver.
4. `ssh homeserver 'cd /opt/soulbeet/deployment && docker compose down && docker compose up -d'`
   Note: use `down` + `up` (not just `up -d`) to force container recreation with new image

**Fixes and features (2026-04-19):**
- Fixed volume path mismatch: Soulseek downloads to `Music/downloads/`, beets moves to `Music/Library/`
- Fixed broken entrypoint in deployment compose
- Enabled `BEETS_ALBUM_MODE=true` for proper album-level MusicBrainz tagging
- Added `fetchart` + `embedart` beets plugins for automatic artwork
- Fixed fetchart `sources` config format (must be a YAML list, not inline string)
- fetchart sources: coverart → itunes → amazon (beetcamp/bandcamp plugin tried but incompatible with beets 2.9.0 — it auto-registers as a fetchart source and causes UnknownPairError, do not use)
- `deployment/` folder tracked in git (minus secrets/data)
- Added Library tab to Soulbeet: browse albums, delete (removes files + beets DB entry + Navidrome DB rows)
- Navidrome delete now uses targeted SQLite delete instead of full DB rebuild
- Beet remove now uses path-based query (more reliable than artist+album string)
- Delete also clears beets incremental taghistory (`state.pickle`) so albums can be redownloaded cleanly
- Imported existing music library into beets DB (377 tracks)
- Mac build setup: Apple Silicon builds linux/arm64 natively via Docker buildx
- Library tab sorting is case-insensitive (lowercase artist names sort correctly)
- Library tab has sort options: by Artist (grouped), by Album (flat), by Date Added (flat)
- dioxus-cli pinned to `0.7.2 --locked` in Dockerfile (unversioned install caused version mismatch, wasm-opt SIGABRT, and stale WASM binary)
- Dockerfile ENTRYPOINT is `/app/server/web` (dioxus 0.7.2 names the binary after the package `web`, not `server`)

**Manual artwork embedding (for albums missing from fetchart sources):**
- Download cover image, `scp` to homeserver, copy to album folder as `cover.jpg`
- Embed via: `docker exec deployment-soulbeet-1 python3 -c "from mutagen.flac import FLAC, Picture; import os; img=open('/app/library/Artist/Album/cover.jpg','rb').read(); [setattr(f:=FLAC(os.path.join('/app/library/Artist/Album',n)), 'x', [f.clear_pictures(), f.add_picture(p:=Picture()), setattr(p,'type',3), setattr(p,'mime','image/jpeg'), setattr(p,'data',img), f.save()]) for n in os.listdir('/app/library/Artist/Album') if n.endswith('.flac')]"`
- Joanne Robertson — Blurrr: art embedded from Bandcamp
- Joanne Robertson & Dean Blunt — Backstage Raver, Wahalla: art embedded from Discogs
- Note: these albums are NOT in the beets DB (imported externally) — beets embedart won't find them, use mutagen directly

**Beets import DB bug (fixed 2026-04-26):**
- `beets/mod.rs` was passing `-l /app/library/.beets_library.db` to every beet import, overriding the `library: /data/musiclibrary.db` config setting
- Beet would tag/move files correctly but write DB entries to the wrong file — albums were invisible in the library tab
- Fix: removed the `-l` flag so beet uses the config's `library:` path directly
- Data migration: ran a Python script to copy 189 missing tracks (13 albums) from `.beets_library.db` into `/data/musiclibrary.db`
- The rogue `.beets_library.db` file still exists at `/app/library/.beets_library.db` — it can be safely deleted

**Important beets 2.x gotcha:**
- `beet ls -f "$path"` returns empty strings — paths are stored relative in the DB (e.g. `Artist/Album/track.flac`) without the library dir prefix
- Do NOT use `beet ls` for path resolution — query `musiclibrary.db` directly via SQLite and prepend `BEETS_LIBRARY_DIR`
- `beet remove` has the same issue — delete from DB directly instead
- get_library and delete_library_album both use rusqlite now (commit 620b31f)

**Confirmed working end-to-end (2026-04-20):**
- Delete album from Library tab → files removed, beets DB cleared, Navidrome updated instantly
- Redownload deleted album → beets imports cleanly (taghistory cleared on delete)
- Tested with: Aphex Twin, Keith Fullerton Whitman - Lisbon

**AIFF files won't play in Navidrome:**
- Navidrome serves AIFF as `format=raw` with no transcoding — browsers can't play it (only Safari)
- Fix: convert to FLAC using `mwader/static-ffmpeg` Docker image:
  ```bash
  for f in /path/to/album/*.aiff; do
    docker run --rm -v /path/to/album:/album mwader/static-ffmpeg:8.0.1 \
      -i "/album/$(basename "$f")" "/album/$(basename "$f" .aiff).flac" -y 2>/dev/null
  done
  rm /path/to/album/*.aiff
  ```
- Also update beets DB paths: `UPDATE items SET path=REPLACE(path, '.aiff', '.flac') WHERE ...`
- Then delete ghost AIFF entries from Navidrome: `DELETE FROM media_file WHERE path LIKE '%.aiff'`
- Skee Mask — Pool was converted this way (2026-04-20), still missing artwork

**Pending:**
- The Body — All The Waters Of The Earth Turn To Blood: download was incomplete (only 1 track), cleaned up, needs redownload
- Skee Mask — Pool: missing artwork (not on CoverArt/iTunes/Amazon/Bandcamp)

**Current issues:**
1. ESPHome has invalid timezone `Europe/Germany` — should be `Europe/Berlin`

**GitHub:** fork at https://github.com/fluxgeist/soulbeet.git (branch: `feat/library-tab`), upstream at https://github.com/terry90/soulbeet.git

**Credentials:**
- Soulbeet: admin/admin (default), slskd: slskd/slskd
- Navidrome: pez732/G0ld3n.ag3
- Soulseek account: pez732/G0ld3n.ag3

**Beets library DB:** `/opt/soulbeet/deployment/data/musiclibrary.db` (inside container: `/data/musiclibrary.db`)
To manually interact: `docker exec deployment-soulbeet-1 beet -c /app/beets_config.yaml <command>`
To remove orphaned entries (files deleted but still in DB): run `/tmp/remove_orphans.py` script inside the soulbeet container

**Navidrome DB:** `/opt/navidrome/navidrome.db` — targeted deletes can be done via Python inside the navidrome container
To find orphans: check `album` table for entries whose `media_file` paths no longer exist on disk

### Nginx Reverse Proxy Routes
Config at `/opt/reverse-proxy/nginx.conf`, served via Docker on port 80. All routes use `.lan` domains (local network only).

| Domain | Service | Port |
|--------|---------|------|
| `homeassistant.lan`, `hass.lan` | Home Assistant | 8123 |
| `esphome.lan` | ESPHome | 6052 |
| `portainer.lan` | Portainer | 9000 |
| `duplicati.lan` | Duplicati | 8200 |
| `navidrome.lan`, `music.lan` | Navidrome | 4533 |
| `soulseek.lan` | Soulseek (slskd) | 5030 |
| `soulbeet.lan` | Soulbeet | 9765 |
| `_` (default) | index.html service listing | — |

### Port Map
| Port | Service |
|------|---------|
| 22 | SSH |
| 80 | nginx reverse proxy |
| 111 | rpcbind |
| 139, 445 | Samba |
| 631 | CUPS (localhost only) |
| 1883 | Mosquitto MQTT |
| 5000 | Shairport-sync (AirPlay control) |
| 6052 | ESPHome web UI |
| 8123 | Home Assistant (direct) |
| 8200 | Duplicati |
| 9000 | Portainer |
| 21063 | Home Assistant HomeKit bridge |

### Cron Jobs
- `0 8 * * *` — `/opt/disk_space_publisher.sh` — publishes disk space daily at 8am (likely to MQTT)
- `@reboot` — `/home/pi/launch_browser.sh` — launches browser into kiosk/dashboard (Home Assistant) 60s after boot via `slim` display manager on an attached screen

### Notable Installed Packages
- `nodejs` + `yarn` — JS runtime and package manager
- `python3.8` + `python3.8-dev` — Python 3.8
- `build-essential`, `cmake`, `clang`, `gcc` — C/C++ build tools (likely residue from compiling drivers/shairport-sync from source)
- `qtcreator` — Qt IDE (accidental/forgotten install, not actively used)
- `qemu-user-static` — cross-arch emulation (building Docker images for non-ARM)
- `tightvncserver` + `vino` — VNC servers
- `remmina` — remote desktop client (RDP/VNC)
- `xubuntu-core` / `xfce4-*` — XFCE desktop environment
- `midori` / `epiphany-browser` — lightweight browsers (one used for kiosk)
- `i2c-tools` — I2C bus tools for Seeed voice card HAT
- `alsa-*` — ALSA audio layer (used by Seeed HAT)
- `duplicity` — CLI backup (separate from Duplicati Docker container)
- `transmission-gtk` — BitTorrent client
- `tmux` + `screen` — terminal multiplexers
- `rsync` — file sync

### Notes
- Samba (`smbd`) shares the network drive at `10.0.0.13` (share: `past`, mounted at `/mnt/backup` and `/opt/smb-downloads`) that Soulseek, Soulbeet, and Navidrome all use
- The music stack (Soulseek + Soulbeet + Navidrome) is running as of 2026-04-19
- HomeKit bridge (port 21063) is broken — HA keeps showing the QR code but pairing doesn't work; needs investigation
- Seeed voice card HAT (GPIO/I2C) is not physically connected — service runs but is a no-op; was used for high quality audio jack output via shairport-sync → ALSA → HAT

### Backup
- **Tool:** Duplicati (Docker), accessible at `duplicati.lan`
- **Job:** "Pi System Backup" — backs up `/opt` daily at 4:43am
- **Destination:** `//10.0.0.13/past` mounted at `/mnt/backup`, writing to `/mnt/backup/duplicati-backup`
- **Status:** Working as of 2026-04-19 (43 backups completed)
- **Fix applied:** Added systemd drop-in `/etc/systemd/system/docker.service.d/wait-for-mounts.conf` so Docker waits for `mnt-backup.mount` before starting — prevents the mount race condition on reboot that caused backups to fail
