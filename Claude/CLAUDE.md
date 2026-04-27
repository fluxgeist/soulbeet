# Claude Code - soulbeet

> For project architecture, services, and config reference see [project_specs.md](project_specs.md)
> For the upstream merge plan and progress tracking see [merge-upstream.md](merge-upstream.md)

## Session Management

**Context warning:** When context usage reaches ~40%, proactively tell the user with this message:
> ⚠️ Context is around 40% used. Consider running `/compact` to summarise the session before continuing, or wrapping up the current task and starting a fresh session.

Check the context indicator in the Claude Code status bar — it shows a percentage. Flag it at 40%, remind again at 60%.

## Quick Reference

**Deploy:**
```bash
docker buildx build --platform linux/arm64 --output type=docker,name=soulbeet:local,dest=/tmp/soulbeet.tar .
cat /tmp/soulbeet.tar | ssh homeserver 'docker load'
ssh homeserver 'cd /opt/soulbeet/deployment && docker compose down && docker compose up -d'
```

**Beets shell:**
```bash
docker exec deployment-soulbeet-1 beet -c /app/beets_config.yaml <command>
```

**DB paths:**
- Beets: `/opt/soulbeet/deployment/data/musiclibrary.db` (container: `/data/musiclibrary.db`)
- Navidrome: `/opt/navidrome/navidrome.db`

---

## Deploy Workflow

Build on Mac (faster than Pi), transfer via tar:

```bash
# 1. Build linux/arm64 image to tar
docker buildx build --platform linux/arm64 --output type=docker,name=soulbeet:local,dest=/tmp/soulbeet.tar .

# 2. Transfer and load on homeserver
cat /tmp/soulbeet.tar | ssh homeserver 'docker load'

# 3. Restart (down + up to force recreation)
ssh homeserver 'cd /opt/soulbeet/deployment && docker compose down && docker compose up -d'
```

> ⚠️ **Do NOT use `docker save soulbeet:local | ssh homeserver 'docker load'`**  
> `buildx --output dest=` writes to a tar file only — it does NOT load into the local Docker daemon.  
> `docker save` reads the stale local image and silently overwrites the new build on the server.

---

## Rules & Gotchas

### Docker / Dioxus
- **dioxus-cli must be pinned:** `cargo install dioxus-cli --version 0.7.2 --locked` — unversioned install causes version mismatch and stale WASM
- **ENTRYPOINT is `/app/server/web`** — dioxus 0.7.2 names the binary after the Cargo package (`web`), not `server`
- **Dockerfile artifact path:** `COPY --from=builder /app/target/dx/web/release/web /app/server`

### Beets
- **Never pass `-l` to beet import** — overrides `library:` in config and writes to wrong DB; let beet use `library: /data/musiclibrary.db` from config directly
- **`beet ls -f "$path"` returns empty strings** — paths stored relative in DB (e.g. `Artist/Album/track.flac`); do NOT use `beet ls` for path resolution; query `musiclibrary.db` via SQLite and prepend `BEETS_LIBRARY_DIR`
- **`beet remove` has the same path issue** — delete from SQLite directly instead
- **fetchart beetcamp plugin is incompatible with beets 2.9.0** — causes UnknownPairError; do not use; current sources: coverart → itunes → amazon
- **fetchart `sources` config must be a YAML list**, not an inline string
- **`state.pickle`** (beets incremental taghistory) is cleared on album delete so albums can be redownloaded cleanly
- **Rogue DB file** at `/app/library/.beets_library.db` — leftover from a prior bug, safe to delete

### AIFF files
Navidrome serves AIFF as `format=raw` — browsers can't play it (only Safari). Convert to FLAC:
```bash
for f in /path/to/album/*.aiff; do
  docker run --rm -v /path/to/album:/album mwader/static-ffmpeg:8.0.1 \
    -i "/album/$(basename "$f")" "/album/$(basename "$f" .aiff).flac" -y 2>/dev/null
done
rm /path/to/album/*.aiff
# Update beets DB:   UPDATE items SET path=REPLACE(path, '.aiff', '.flac') WHERE ...
# Remove from Navidrome: DELETE FROM media_file WHERE path LIKE '%.aiff'
```

### Manual artwork embedding
For albums missing from fetchart sources — download cover as `cover.jpg`, copy to album folder, then:
```bash
docker exec deployment-soulbeet-1 python3 -c "
from mutagen.flac import FLAC, Picture; import os
img = open('/app/library/Artist/Album/cover.jpg', 'rb').read()
for n in os.listdir('/app/library/Artist/Album'):
    if not n.endswith('.flac'): continue
    f = FLAC(os.path.join('/app/library/Artist/Album', n))
    f.clear_pictures()
    p = Picture(); p.type = 3; p.mime = 'image/jpeg'; p.data = img
    f.add_picture(p); f.save()
"
```
Albums imported externally (not via beets) won't be found by `beet embedart` — use mutagen directly.

Albums with manually embedded art:
- Joanne Robertson — Blurrr (from Bandcamp)
- Joanne Robertson & Dean Blunt — Backstage Raver, Wahalla (from Discogs)

### DB maintenance
To remove orphaned beets entries: run `/tmp/remove_orphans.py` inside the soulbeet container.  
Navidrome orphans: check `album` table for entries whose `media_file` paths no longer exist on disk.

---

## Pending

- **The Body — All The Waters Of The Earth Turn To Blood:** incomplete download (1 track), needs redownload
- **Skee Mask — Pool:** missing artwork (not on CoverArt/iTunes/Amazon/Bandcamp)
- **ESPHome:** invalid timezone `Europe/Germany` → should be `Europe/Berlin`
- **HomeKit bridge** (port 21063): broken — HA shows QR code but pairing fails

---

## GitHub

Fork: https://github.com/fluxgeist/soulbeet.git (branch: `feat/library-tab`)  
Upstream: https://github.com/terry90/soulbeet.git
