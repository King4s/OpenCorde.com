# OpenCorde — Future Phase Tasks

## Phase 2: Security & Bridging (Weeks 13-24)

### Weeks 13-14: End-to-End Encryption (OpenMLS)

- [ ] Integrate OpenMLS library into opencorde-crypto
- [ ] E2EE key management endpoints
- [ ] Client-side encryption/decryption (WebAssembly)
- [ ] Message encryption pipeline
- [ ] LiveKit E2EE for voice/video
- [ ] AES-256-GCM file encryption

### Weeks 15-16: Full-Text Search (Tantivy)

- [ ] Integrate Tantivy into opencorde-search
- [ ] Message indexing on creation/update
- [ ] Search endpoint with faceting
- [ ] Web UI search component

### Weeks 17-20: Bridge Services (Discord, Steam)

- [ ] Discord bridge: gateway connection
- [ ] Discord bridge: REST API client
- [ ] Discord bridge: message/user/channel mapping
- [ ] Discord bridge: ghost user management
- [ ] Steam integration: OAuth login
- [ ] Steam integration: friend list import

### Weeks 21-24: Extended Features

- [ ] Threads, replies, emoji reactions
- [ ] Message pinning, unread tracking
- [ ] Admin panel + audit log
- [ ] Production deployment guide

---

## Phase 3: Ecosystem (Weeks 25-40)

- [ ] Voice sub-channels (text-in-voice)
- [ ] Recording (LiveKit Egress)
- [ ] Mobile apps (Tauri 2.0 iOS/Android)
- [ ] Bot/plugin framework (webhooks, slash commands)
- [ ] Server discovery
- [ ] Forums, polls, scheduled events

---

**Note:** Federation via Matrix protocol is explicitly excluded per user decision.

**Last updated:** 2026-03-16
