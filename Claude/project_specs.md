# Soulbeet — Project Specs

## Overview

Soulbeet is a custom Rust/Dioxus fullstack web UI for managing a self-hosted music stack on a Raspberry Pi homeserver. It orchestrates Soulseek downloads, beets tagging, and Navidrome streaming.

**Tech stack:** Rust + Dioxus 0.7.2, SQLite, Axum, Tokio  
**Build target:** linux/arm64 (built on Mac via Docker buildx, deployed to Pi)

---

## Homeserver

- **Alias:** `homeserver` (configured in ~/.ssh/config)
- **Address:** pi@10.0.0.141
- **Hardware:** Raspberry Pi (aarch64)
- **OS:** Ubuntu 20.04.6 LTS
- **Disk:** 117G total, 40G used, 74G free
- **RAM:** 7.6G total, ~5.8G available
- **SSH:** Passwordless via ed25519 key, passphrase stored in macOS Keychain

---

## Music Stack Architecture

All services share the NFS music drive at `10.0.0.13`, mounted at `/opt/smb-downloads/Music`.

```
Soulseek (slskd) → downloads to /opt/smb-downloads/Music/downloads/  (inbox)
       ↓ (via Soulbeet UI trigger)
Beets → tags & moves files to /opt/smb-downloads/Music/Library/
       ↓
Navidrome → streams from /opt/smb-downloads/Music/Library/
```

**Flow:**
1. User searches MusicBrainz for album/track in Soulbeet UI
2. Soulbeet searches slskd and scores results by quality (FLAC > WAV > AAC > MP3)
3. Downloads queued in batches of 3, with retry/circuit breaker logic
4. WebSocket streams real-time download status to UI
5. On completion, beets runs: `beet -c <config> -d <target> import -q [-s] <source>`
6. Beets tags & moves files into `Library/$albumartist/$album/$track $title`
7. Navidrome picks up new files on next scan (every 1h)

---

## Services (Docker)

**Compose files:**
- `/opt/docker-compose.yaml` — main stack
- `/opt/soulbeet/deployment/docker-compose.yml` — soulbeet (uses `soulbeet:local` image)

| Container | Purpose | Port |
|-----------|---------|------|
| `homeassistant` | Home Assistant (behind nginx) | 8123 |
| `esphome` | ESP device firmware/config | 6052 |
| `reverse-proxy` | nginx reverse proxy | 80 |
| `portainer` | Docker management UI | 9000 |
| `duplicati` | Backup | 8200 |
| `navidrome` | Music streaming server | 4533 |
| `soulseek` (slskd) | Soulseek P2P client | 5030 |
| `soulbeet` | This app | 9765 |

**Key env vars (soulbeet deployment compose):**
- `SLSKD_URL=http://soulseek:5030` — must be on same Docker network
- `SLSKD_API_KEY` — in `deployment/.env` (gitignored)
- `SLSKD_DOWNLOAD_PATH=/app/downloads`
- `BEETS_CONFIG=/app/beets_config.yaml`
- `BEETS_ALBUM_MODE=true` — album mode for proper MusicBrainz tagging
- `NAVIDROME_DB_PATH=/navidrome/navidrome.db`

---

## Beets Configuration

**Configs:** `/opt/soulbeet/deployment/configs/`
- `beets_config.yaml` — default config; organises as `$albumartist/$album/$track $title`
- `antonino_beets.yaml`, `peri_beets.yaml` — per-user configs

**Plugins enabled:** `fetchart`, `embedart`  
**Fetchart sources:** coverart → itunes → amazon

---

## Credentials

- Soulbeet: admin/admin (default)
- slskd: slskd/slskd
- Navidrome: pez732/G0ld3n.ag3
- Soulseek account: pez732/G0ld3n.ag3

---

## Nginx Reverse Proxy

Config at `/opt/reverse-proxy/nginx.conf`. All routes use `.lan` domains (local network only).

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

---

## Port Map

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

---

## Cron Jobs

- `0 8 * * *` — `/opt/disk_space_publisher.sh` — publishes disk space daily at 8am to MQTT
- `@reboot` — `/home/pi/launch_browser.sh` — launches browser into kiosk/dashboard (Home Assistant) 60s after boot

---

## Backup

- **Tool:** Duplicati (Docker), accessible at `duplicati.lan`
- **Job:** "Pi System Backup" — backs up `/opt` daily at 4:43am
- **Destination:** `//10.0.0.13/past` → `/mnt/backup/duplicati-backup`
- Docker is configured to wait for `mnt-backup.mount` before starting (systemd drop-in at `/etc/systemd/system/docker.service.d/wait-for-mounts.conf`) — prevents mount race condition on reboot

---

## Notes

- Samba (`smbd`) shares the network drive at `10.0.0.13` (share: `past`, mounted at `/mnt/backup` and `/opt/smb-downloads`) — used by Soulseek, Soulbeet, and Navidrome
- Seeed voice card HAT is not physically connected — `seeed-voicecard` service runs but is a no-op
- `openvpn` installed but currently inactive
