# /deploy/

Purpose: Deployment configuration and setup scripts.

Pattern: One subdirectory per service or tool.

| Directory | Purpose | Contents |
|-----------|---------|----------|
| livekit/ | LiveKit voice server config | docker-compose, configuration.yaml |
| caddy/ | Reverse proxy config | Caddyfile, SSL cert management |
| scripts/ | Automation scripts | setup.sh, backup.sh, migrate.sh, health-check.sh |
