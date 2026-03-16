# /client/src/routes/

Purpose: SvelteKit file-based routes.

Pattern: kebab-case directories. Each route has +page.svelte. Dynamic segments in brackets [paramName].

| Route | Purpose |
|-------|---------|
| login/ | Login form (+page.svelte) |
| register/ | Registration form (+page.svelte) |
| servers/[serverId]/ | Server view (member list, settings) |
| servers/[serverId]/channels/[channelId]/ | Channel with messages and voice |
| settings/ | User profile and app settings |
| +layout.svelte | Root layout, nav initialization |
