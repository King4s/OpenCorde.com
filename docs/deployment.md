# Deployment

This project is shipped as a static SvelteKit build for the web client, with a separate Rust API and supporting services.

## Web client routing

The client build emits:
- `index.html` for the prerendered landing page at `/`
- `200.html` for client-side routes such as `/login`, `/register`, `/servers`, and `/invite/[code]`

Your reverse proxy or static host must serve the SPA fallback for non-root routes. For Caddy, the intended pattern is:

```caddyfile
handle /api/* {
    reverse_proxy localhost:8080
}

handle /ws {
    reverse_proxy localhost:8080
}

handle /files/* {
    reverse_proxy localhost:9010
}

@root path /
handle @root {
    root * /srv/opencorde/client/build
    file_server
}

handle {
    root * /srv/opencorde/client/build
    try_files {path} {path}/ /200.html
    file_server
}
```

## Deployment checklist

- Build the client with `pnpm build` in `client/`
- Deploy the entire `client/build/` directory
- Ensure the host serves `200.html` for deep links
- Keep API, WebSocket, and file routes proxied separately
- Verify `/login`, `/register`, `/servers`, and `/invite/testcode` directly in the deployed environment after release

## Common failure mode

If deep links show the marketing page instead of the app shell, the host is almost certainly serving `index.html` for every path or routing all requests through the wrong entrypoint. Fix the host fallback first before changing application code.
