# Upstream Merge Tracking

Merging `terry90/soulbeet` master into `feat/library-tab`.
Fork point: `ee74975`
Upstream tip at time of planning: `53038ba`

**Branch strategy:** Create `feat/upstream-merge` from `feat/library-tab`, do all work there, PR back to `feat/library-tab` when done.

---

## Status Overview

| Phase | Status |
|---|---|
| 1. Setup | ⬜ Not started |
| 2. Resolve conflicts | ⬜ Not started |
| 3. Dioxus 0.7.4 upgrade | ⬜ Not started |
| 4. DB migrations | ⬜ Not started |
| 5. New env vars | ⬜ Not started |
| 6. Build & test | ⬜ Not started |
| 7. Deploy & verify | ⬜ Not started |
| 8. Docs update | ⬜ Not started |

---

## Phase 1 — Setup

- [ ] Create branch: `git checkout -b feat/upstream-merge`
- [ ] Add upstream remote: `git remote add upstream https://github.com/terry90/soulbeet.git`
- [ ] Fetch upstream: `git fetch upstream`
- [ ] Run merge: `git merge upstream/master --no-commit --no-ff`
- [ ] Confirm the 7 expected conflict files and no surprises

---

## Phase 2 — Resolve Conflicts

### Hard conflicts (need careful manual merge)

- [ ] **`lib/soulbeet/src/beets/mod.rs`** ⚠️ Hardest
  - Keep our change: remove `-l /app/library/.beets_library.db` flag from beet import command
  - Keep upstream change: improved timeout/kill logic — spawns child, waits with timeout, explicitly kills on timeout via `child.kill().await`; adds `read_child_stdout` / `read_child_stderr` helpers
  - These overlap in the same function body — apply both changes to the rewritten function

- [ ] **`Dockerfile`** ⚠️ Hard
  - Take from upstream: Node 22 via NodeSource, dioxus-cli 0.7.4, clean `npm install` approach
  - Take from fork: explicit `npm install @tailwindcss/oxide-linux-arm64-gnu` arm64 binding (upstream's Node 22 may already handle this — verify)
  - Take from fork: `pillow` in pip deps
  - Entrypoint: use upstream's `/app/server/server` (dioxus 0.7.4 renames binary back to `server`)
  - Drop: our `--version 0.7.2 --locked` pin (replaced by 0.7.4)

- [ ] **`web/src/main.rs`** ⚠️ Medium
  - Keep our additions: `LibraryPage` route, Library navbar link + SVG icon, `use ui::LibraryPage`
  - Keep upstream additions: `DashboardPage` route, `SettingsProvider` wrapper in `App`, `AutoDownloadSignal`, `SearchPrefill`, `start_channel_cleanup_task()` on startup, cfg-gated WebSocket imports
  - Watch for: upstream restructured the `App` component tree — our navbar RSX additions need to fit inside the new tree

### Easy conflicts (additive only)

- [ ] **`api/Cargo.toml`** — Add our `rusqlite` dep + upstream's new deps (`serde_json`, `aes-gcm`, `sha2`, `base64`, `futures`); reconcile version number to `0.5.2`
- [ ] **`api/src/server_fns/mod.rs`** — Add our `pub mod library` + upstream's `pub mod discovery`, `pub mod navidrome`, `pub mod settings`; keep upstream's `cleanup_empty_ancestors` function
- [ ] **`ui/src/components/mod.rs`** — Add our `pub mod library` + upstream's new component modules
- [ ] **`web/src/views/mod.rs`** — Add our `mod library` + upstream's `mod dashboard`

---

## Phase 3 — Dioxus 0.7.4 Upgrade

The upstream bumped dioxus from 0.7.2 → 0.7.4. This affects the build pipeline.

- [ ] Verify `Cargo.toml` (root + workspace members) reflect 0.7.4 dioxus deps
- [ ] Run `cargo update` to regenerate `Cargo.lock` for new versions
- [ ] Confirm dioxus-cli pin in Dockerfile is `--version 0.7.4 --locked`
- [ ] Confirm ENTRYPOINT in Dockerfile is `/app/server/server` (0.7.4 renames binary back from `web` to `server`)
- [ ] Check `dx bundle` output path has changed accordingly: `target/dx/web/release/server/`
- [ ] Update Dockerfile `COPY` line if artifact path changed

---

## Phase 4 — DB Migrations

Upstream added 13 new SQL migrations covering multi-user, discovery playlists, recommendation engine, ListenBrainz, per-profile settings.

- [ ] List all new migration files: `ls api/src/migrations/` and compare to what's in production DB
- [ ] Check if the migration runner applies them automatically on startup (likely yes — confirm in code)
- [ ] Identify any migrations that touch existing tables (downloads, items) vs purely new tables
- [ ] Note: do NOT run migrations against production DB until build is verified — migrations may be irreversible
- [ ] Plan: test migrations against a copy of the production DB first

---

## Phase 5 — New Env Vars

Upstream's discovery feature requires new configuration.

- [ ] Identify all new env vars added by upstream (check `api/src/config.rs` and compose examples)
  - `NAVIDROME_MUSIC_PATH` — path prefix mapping for cross-container path resolution (likely `/app/library`)
  - Possibly: ListenBrainz username (per-user in settings, not env var — verify)
- [ ] Add new vars to `deployment/docker-compose.yml` with sensible defaults
- [ ] Document new vars in `project_specs.md`

---

## Phase 6 — Build & Test

- [ ] Build Docker image locally:
  ```bash
  docker buildx build --platform linux/arm64 --output type=docker,name=soulbeet:local,dest=/tmp/soulbeet.tar .
  ```
- [ ] Check build log for: dioxus version mismatch warnings, wasm-opt errors, missing deps
- [ ] Verify WASM binary is freshly built (check content hash in build output)
- [ ] Smoke test: run container locally (x86 or with QEMU) and hit the UI if possible
- [ ] Check that existing functionality compiles: library tab, beets import, delete album

---

## Phase 7 — Deploy & Verify

- [ ] Transfer image to homeserver:
  ```bash
  cat /tmp/soulbeet.tar | ssh homeserver 'docker load'
  ssh homeserver 'cd /opt/soulbeet/deployment && docker compose down && docker compose up -d'
  ```
- [ ] Check container logs on startup — watch for migration output and any panics
- [ ] Verify existing features still work:
  - [ ] Library tab loads and displays albums
  - [ ] Sort options work (Artist / Album / Date Added)
  - [ ] Delete album works (removes files + beets DB + Navidrome)
  - [ ] Search and download a track end-to-end
  - [ ] Beets import runs after download completes
- [ ] Verify new upstream features:
  - [ ] Cancel download button visible
  - [ ] One-tap download UX (cover art, inline icons)
  - [ ] Discovery tab appears (may need Navidrome + ListenBrainz config to fully work)

---

## Phase 8 — Docs Update

- [ ] Update `Claude/CLAUDE.md`:
  - Change dioxus-cli version note from 0.7.2 → 0.7.4
  - Update ENTRYPOINT note (`/app/server/server` not `/app/server/web`)
- [ ] Update `Claude/project_specs.md`:
  - Add new env vars (`NAVIDROME_MUSIC_PATH`, etc.)
  - Add discovery feature to architecture overview
  - Update version references
- [ ] Commit and push to `feat/upstream-merge`
- [ ] Open PR from `feat/upstream-merge` → `feat/library-tab`

---

## Notes & Gotchas

- **Arm64 Tailwind binding:** Upstream fixed this via Node 22 — verify their approach works before re-adding our explicit `@tailwindcss/oxide-linux-arm64-gnu` install. If upstream's Node 22 approach works, don't add it again (avoid double-install).
- **Discovery requires Navidrome integration:** The discovery engine needs `NAVIDROME_URL`, `NAVIDROME_USER`, `NAVIDROME_PASSWORD` (already in compose) plus `NAVIDROME_MUSIC_PATH` (new). Without it the Discovery tab will show an error/status banner — that's expected.
- **ListenBrainz username:** Set per-user in Settings UI, not an env var. Users need to add their LB username in the app to get recommendations.
- **13 new migrations:** These will run automatically on first startup. Back up production DB before deploying.
- **Binary rename:** dioxus 0.7.4 renames the binary back to `server` (was `web` in 0.7.2). This is the opposite of the bug we already fixed — don't get confused.
