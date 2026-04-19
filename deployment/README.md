# Soulbeet Deployment Configuration

This directory contains your personal deployment configuration for Soulbeet on your homelab server.

## Structure

```
deployment/
├── docker-compose.yml       # Your Docker Compose configuration
├── configs/
│   ├── beets_config.yaml    # Default beets configuration
│   ├── antonino_beets.yaml  # Antonino's beets configuration
│   └── peri_beets.yaml      # Peri's beets configuration
└── data/
    ├── soulbeet.db          # Soulbeet application database
    └── musiclibrary.db      # Beets music library database
```

## Important Files

### docker-compose.yml
Your active Docker Compose configuration. This file contains:
- Container settings
- Volume mounts to NFS (`/opt/smb-downloads/Music`)
- Environment variables
- Network configuration
- **CONTAINS SECRETS**: SLSKD_API_KEY

### configs/beets_config.yaml
The active beets configuration used by the container. Customize this to change:
- Import behavior (autotag, timid mode, etc.)
- Match thresholds
- Path formatting for organized music
- Plugin settings

### data/
Contains your databases - **BACKUP THIS REGULARLY**:
- `soulbeet.db` - User accounts and folder configurations
- `musiclibrary.db` - Beets music library metadata

## Running the Container

From the `/opt/soulbeet` directory:

```bash
# Start the container
docker-compose -f deployment/docker-compose.yml up -d

# View logs
docker-compose -f deployment/docker-compose.yml logs -f

# Stop the container
docker-compose -f deployment/docker-compose.yml down

# Restart the container
docker-compose -f deployment/docker-compose.yml restart
```

## Updating Soulbeet

1. Stop the container:
   ```bash
   docker-compose -f deployment/docker-compose.yml down
   ```

2. Pull latest source code:
   ```bash
   git pull
   ```

3. Rebuild the image:
   ```bash
   docker build -t soulbeet:local .
   ```

4. Start the container:
   ```bash
   docker-compose -f deployment/docker-compose.yml up -d
   ```

## Volume Mounts

- **Downloads**: `/opt/smb-downloads/Music` → `/app/downloads` (shared with slskd)
- **Music Library**: `/opt/smb-downloads/Music/Library` → `/music` (organized output)
- **Data**: `./deployment/data` → `/data` (databases)
- **Config**: `./deployment/configs/beets_config.yaml` → `/app/beets_config.yaml` (beets config)

## Network

Connected to external network `opt_default` to communicate with the slskd container.

## Backup Strategy

Regularly backup:
1. `deployment/docker-compose.yml` - Your configuration
2. `deployment/configs/` - All beets configurations
3. `deployment/data/` - Both database files

## Troubleshooting

### Check container logs
```bash
docker logs soulbeet-soulbeet-1
```

### Access web interface
http://localhost:9765

### Test beets import manually
```bash
docker exec soulbeet-soulbeet-1 beet import -A "/app/downloads/your-album"
```

### Check slskd connectivity
Ensure slskd is running: `docker ps | grep soulseek`

---

Last updated: 2026-01-30
