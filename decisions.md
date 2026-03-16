# Architectural Decisions

**All decisions dated 2026-03-16 unless otherwise noted.**

---

## 1. PostgreSQL over MongoDB / SQLite

**Status:** ACCEPTED

**Decision:** Use PostgreSQL 16+ with sqlx for type-safe compile-time query checking.

**Rationale:**
- **Relational integrity:** Permissions and roles require complex joins (users → roles → permissions). PostgreSQL foreign keys enforce data consistency; MongoDB documents are isolated.
- **JSONB support:** PostgreSQL's JSONB column type handles flexible fields (e.g., channel metadata, user settings) while maintaining queryability. Best of both worlds.
- **Compile-time safety:** sqlx verifies SQL against live schema at build time, preventing runtime errors. No other ORM matches this.
- **Transactions:** Multi-table operations (e.g., "create server → add owner to members → create default channel") require ACID compliance. PostgreSQL guarantees this; MongoDB can lose writes in network partitions.

**Rejected alternatives:**
- **MongoDB:** Permissions and role inheritance queries are awkward (multiple $lookup stages). No compile-time query checking. Weaker consistency guarantees.
- **SQLite:** No concurrent writes. A single user uploading a file blocks all other database operations. Not viable for multi-user production.

---

## 2. Rust + Axum over Node.js

**Status:** ACCEPTED

**Decision:** Build API with Rust 1.75+ and Axum 0.8 web framework.

**Rationale:**
- **Memory safety:** Rust's type system and borrow checker eliminate entire classes of bugs (buffer overflows, use-after-free, data races). Chat servers are long-running processes where crashes = unhappy users.
- **Same async runtime as LiveKit SDK:** LiveKit's Rust SDK uses Tokio. Axum also uses Tokio. Single shared runtime means better resource efficiency and easier integration.
- **Proven precedent:** Revolt (Discord clone) built 500K+ users with Rust backend. No regrets. Used Actix-web (less modern than Axum). We're using the better framework.
- **Developer experience:** Axum's error handling is explicit. No silent failures. Middleware is composable. Easy to test.

**Rejected alternatives:**
- **Node.js:** Lower memory overhead per connection (single-threaded event loop), but Discord/Slack found that Node.js becomes a bottleneck at scale. Lack of compile-time safety means shipping bugs to users. No built-in E2EE crypto library (have to shell out to OpenSSL, which is unsafe).

---

## 3. LiveKit over Janus / Mediasoup

**Status:** ACCEPTED

**Decision:** Use LiveKit as the voice/video SFU (Selective Forwarding Unit).

**Rationale:**
- **Mature, open-source SFU:** LiveKit is production-ready with 3000+ subscribers per node. Janus is older (2013), C codebase, less maintained. Mediasoup is Node.js (bad for resource use).
- **Sub-100ms latency:** LiveKit's architecture is optimized for low latency. VP9 codec support for high-quality screen sharing.
- **Built-in E2EE:** LiveKit supports end-to-end encryption with DTLS and key exchange. Janus requires additional work.
- **Recording, analytics, webhooks:** Out of the box.
- **Self-hosted:** Can run in Docker. No vendor lock-in.

**Rejected alternatives:**
- **Janus Gateway:** Mature but aging (C, complex build). No built-in modern E2EE. Community is smaller.
- **Mediasoup:** Node.js dependency is a dealbreaker. Node event loop can't handle concurrent media streams efficiently.

---

## 4. Tauri 2.0 over Electron

**Status:** ACCEPTED

**Decision:** Build desktop client with Tauri 2.0 (native Rust backend) + SvelteKit frontend.

**Rationale:**
- **10-50x smaller binary:** Electron bundles Chromium (150MB+). Tauri reuses OS WebKit (20-50MB final binary on Windows). Huge difference for auto-updates and disk space.
- **Native Rust backend in the same process:** Crypto operations (E2EE, TOTP) run in Rust, not JavaScript. No FFI overhead. Direct access to system APIs (tray icon, file dialogs, native notifications).
- **Lower CPU/RAM:** No separate Chromium process. Example: Revolt Desktop (Electron) uses 300MB RAM at idle. Tauri targets <100MB.
- **System integration:** First-class tray support, file associations, keyboard shortcuts.

**Rejected alternatives:**
- **Electron:** Resource hog for laptops and low-end hardware. Slow startup (Chromium initialization). Large binary size makes enterprise distribution painful.

---

## 5. SvelteKit over React / Vue

**Status:** ACCEPTED

**Decision:** Build web/desktop UI with SvelteKit 2.x + Tailwind CSS 4.x.

**Rationale:**
- **Smallest bundle size:** No runtime overhead (Svelte compiles to imperative JavaScript). 20KB min+gzip for SvelteKit vs. 40KB for React 18. Matters for Tauri (bandwidth savings).
- **Readable syntax:** No JSX, no hooks mental model. Svelte's reactive bindings are intuitive for developers new to frontend.
- **Excellent ecosystem:** SvelteKit for routing, Tailwind for styling. Form libraries mature.
- **Server-Side Rendering:** SvelteKit supports SSR. Useful for future web-only deployment.

**Rejected alternatives:**
- **React:** Larger runtime, more learning curve for E2EE/crypto context. Virtual DOM overhead for desktop UI (overkill).
- **Vue:** Smaller than React, but SvelteKit's bundler integration is better. Svelte ecosystem is more cohesive.

---

## 6. Snowflake IDs over UUIDs / Auto-increment

**Status:** ACCEPTED

**Decision:** Use Discord-compatible 64-bit Snowflake IDs for all primary keys.

**Rationale:**
- **Time-ordered:** Snowflakes embed timestamp (first 42 bits). Sorted chronologically without a separate `created_at` column for partitioning. Enables efficient cursor-based pagination.
- **64-bit vs. 128-bit:** Smaller than UUIDv7 (128 bits). Saves 8 bytes per row × millions of messages = significant space savings. Easier to work with in JSON and databases.
- **Discord compatibility:** If we bridge to Discord API later, IDs are natively compatible.
- **Distributed generation:** No central sequence server. Each instance (or shard) generates unique IDs. Scales horizontally.

**Rejected alternatives:**
- **UUIDv7:** 128-bit (16 bytes). Time-ordered like Snowflake, but larger. No ecosystem advantage for chat apps.
- **Auto-increment:** Cannot be generated offline or in distributed systems. Requires a single source of truth (bottleneck). Not compatible with future horizontal scaling.

---

## 7. AGPL-3.0 License (not MIT / GPL)

**Status:** ACCEPTED

**Decision:** License OpenMesh under AGPL-3.0-or-later.

**Rationale:**
- **Network use clause:** AGPL requires that if you run OpenMesh over a network (self-hosted on a server), modifications to the source code must be made available to users. This prevents proprietary forks of the software at scale.
- **Aligns with self-hosted mission:** Users running OpenMesh on their own hardware are not affected (they can keep modifications private). But SaaS forks must open-source their changes. Keeps the ecosystem unified.
- **Not as viral as claimed:** Contrary to FUD, AGPL does not require entire stacks to be open-source. Only OpenMesh modifications are covered. Dependencies remain under their own licenses.

**Rejected alternatives:**
- **MIT:** Allows proprietary forks. A for-profit company could copy OpenMesh, add AI features, and sell it as a closed-source product without sharing improvements.
- **GPL v3:** Does not cover network use. A company could host GPL v3 software on a server and never distribute the binary.

---

## 8. Monorepo with Cargo Workspace (not Polyrepo)

**Status:** ACCEPTED

**Decision:** Single Git repository with Rust crates organized as Cargo workspace. SvelteKit frontend in `client/` subdirectory.

**Rationale:**
- **Atomic changes:** Adding a field to a User model in openmesh-core, updating the repository method, and updating the API endpoint handler can be one commit. Polyrepo requires 3 PRs with version coordination.
- **Shared types:** Rust's type system shines here. openmesh-api and openmesh-db both reference openmesh-core types directly (no code generation or manual synchronization).
- **Build optimization:** Cargo can batch compile workspace crates. Faster CI.
- **Deployment:** Single release includes all crates at the same version. No "API v1.2 is incompatible with DB v1.1" confusion.

**Rejected alternatives:**
- **Polyrepo:** Simpler permission model if teams own separate repos. But OpenMesh is a tightly integrated system. The overhead of version coordination and cross-repo testing is not worth it.

---

## 9. OpenMLS for End-to-End Encryption (Phase 2)

**Status:** ACCEPTED (Phase 2 implementation)

**Decision:** Use OpenMLS Rust library for group E2EE (defer full implementation to Phase 2).

**Rationale:**
- **IETF standard:** OpenMLS implements RFC 9420 (Messaging Layer Security). Not a custom protocol subject to amateur cryptography bugs. Actively maintained by security researchers.
- **Rust implementation:** Native Rust library, no FFI, compiles to WebAssembly for client-side decryption.
- **Scales for groups:** Supports O(log N) operations for adding/removing members. Double Ratchet (Signal protocol) is one-to-one only. Custom protocols risk key derivation bugs.
- **Future-proof:** If MLS becomes the standard in other apps (WhatsApp, Signal, etc.), OpenMesh can interop.

**Rejected alternatives:**
- **Double Ratchet:** Designed for Signal protocol (1-to-1). Awkward to adapt for group chat. Would require research to implement safely.
- **Custom E2EE:** Never. Cryptography is not a place to learn on the job. OpenMLS is maintained by experts.

---

## 10. Redis for Pub/Sub + Cache (not NATS / RabbitMQ)

**Status:** ACCEPTED

**Decision:** Use Redis 7+ for pub/sub (message broadcasting) and caching (sessions, rate limits, presence).

**Rationale:**
- **Mandatory dependency anyway:** LiveKit requires Redis for its own state management (participant presence, room state). Running a separate message broker adds complexity.
- **Simple pub/sub model:** Redis PUBLISH/SUBSCRIBE is straightforward. One WebSocket connection polls Redis, broadcasts events to connected clients. No queue durability needed (events are ephemeral).
- **Built-in data structures:** Hashes for session storage, sorted sets for rate limiting leaderboards, sets for presence tracking.
- **Performance:** Redis is in-memory and fast. Sub-millisecond latency for pub/sub.

**Rejected alternatives:**
- **NATS:** Excellent message broker, but another service to manage. Overkill for fan-out pub/sub (Redis is simpler).
- **RabbitMQ:** Overkill for our use case. Designed for task queues, not real-time event broadcast. Heavier resource footprint.

---

## 11. IRC-inspired Mesh Federation (not Matrix, not ActivityPub)

**Date:** 2026-03-16
**Status:** ACCEPTED

**Decision:** Use an IRC-inspired server-to-server mesh model for federation. Servers peer with each other directly, forming a network. Users with Ed25519 keypairs can join servers across the mesh with a single identity.

**Rationale:**
- **IRC's mesh model is battle-tested (40+ years) and conceptually simple** — servers connect directly with no central authority, no blockchain required for basic operation.
- **Users see a unified view across peered servers** — just like IRC networks, a user has one identity across all connected servers in the mesh.
- **Much simpler than Matrix's DAG-based room state resolution** — Matrix requires complex event history reconciliation across servers. IRC mesh avoids this.
- **Much simpler than ActivityPub's inbox/outbox model** — ActivityPub is designed for social media (posts, follows), not real-time chat. No native voice/video concept.
- **Fits the "OpenMesh" name and vision perfectly** — federation is a first-class feature, not an afterthought.

**Rejected alternatives:**
- **Matrix protocol:** Excluded per user decision. Complex DAG state resolution, heavy homeserver requirements, designed for social networks not real-time chat.
- **ActivityPub:** Designed for social media (posts, follows), not real-time messaging. No native voice/video concept. Inbox/outbox model does not scale for group chat.
- **Pure P2P (no servers):** Impossible for group voice/video without a Selective Forwarding Unit (SFU). Unreliable message delivery without persistence layer.

---

## 12. Ed25519 Keypair Identity (not email-primary)

**Date:** 2026-03-16
**Status:** ACCEPTED

**Decision:** User identity is primarily based on Ed25519 keypairs. The public key is the user's global identity across the mesh. Email+password login remains as an optional convenience/recovery method, not the primary identity.

**Rationale:**
- **Users cryptographically OWN their identity** — no server can revoke it. The keypair is non-custodial.
- **Same keypair used for identity AND E2EE** — Ed25519 is used for both identity signing and key package derivation in OpenMLS.
- **Portable across instances** — join any mesh server with your public key. No re-registration required.
- **Privacy-preserving** — email is optional. Users can maintain anonymity if they choose.
- **Session (Lokinet) proved this UX can work for chat** — end-user-facing chat applications have successfully shipped keypair-first identity.
- **Ed25519 is fast, compact, and widely supported** — 32-byte keys, sub-millisecond signing, native WASM support.

**Rejected alternatives:**
- **Email-primary identity:** Creates dependency on email providers. Not privacy-preserving. Not portable across servers (email is server-specific).
- **Blockchain-only identity:** Gas fees, wallet UX complexity. Creates chain dependency. May be added later as OPTIONAL name registry (e.g., ENS-like).
- **W3C Decentralized Identifiers (DIDs):** Overly complex specification. Multiple competing resolution methods. No single winner in the ecosystem.

---

## 13. Rename to OpenMesh (from CheriChat)

**Date:** 2026-03-16
**Status:** ACCEPTED

**Decision:** Rename project from CheriChat to OpenMesh. Domain: openmesh.chat.

**Rationale:**
- **Name reflects the core architecture** — mesh federation is the defining technical feature.
- **"Open" signals open-source AND open network** — the dual meaning is intentional.
- **".chat" TLD is perfect for the product** — immediately communicates the use case.
- **CheriChat was a good Discord-clone name, but the project has evolved** — now building a federated mesh platform with non-custodial identity using OpenMesh as the new name.

**Crate names and module paths:**
- Rust crates: `openmesh-core`, `openmesh-api`, `openmesh-db`, `openmesh-cli`
- Module namespace: `openmesh_core`, `openmesh_db`, etc.
- TypeScript packages: `@openmesh/client`, `@openmesh/types`
- Domain: openmesh.chat

---

## 14. Rename to OpenCorde (from OpenMesh)

**Date:** 2026-03-16
**Status:** ACCEPTED

**Decision:** Rename project from OpenMesh to OpenCorde. Domain: opencorde.com (Cloudflare).

**Rationale:**
- **"OpenCorde" = "Open" + "Corde" (Latin: heart/harmony)** — the opposite of "Discord" (disharmony). Immediately communicates "open Discord alternative" without naming it directly.
- **Provocative name** — stands out in the self-hosted communication space.
- **.com domain is cheap** — approximately 68 kr/year vs 445 kr/year for .chat. Better for international branding.
- **Renewal history** — previous domains cherichat.com and openmesh.chat also owned, ensuring no loss of investment.
- **Crate names update:**
  - Rust crates: `opencorde-core`, `opencorde-api`, `opencorde-db`, `opencorde-cli`
  - Module namespace: `opencorde_core`, `opencorde_db`, etc.
  - TypeScript packages: `@opencorde/client`, `@opencorde/types`
  - Domain: opencorde.com
  - Container names: `opencorde-postgres`, `opencorde-redis`, `opencorde-minio`, etc.
  - Database/bucket names: `opencorde`

**Rejected alternatives:**
- **Keep OpenMesh:** Strong name, but OpenCorde is more provocative and memorable.
- **Alternative TLDs:** .chat and .io are expensive; .com is cheaper and more universal.

**Rename history:**
1. Commune (internal codename)
2. CheriChat (Discord clone, 2026-01-xx)
3. OpenMesh (federation architecture, 2026-03-16)
4. OpenCorde (current, 2026-03-16)

---

## Decision Review Timeline

- **2026-03-16:** Decisions 1-14 finalized. Core MVP (decisions 1-10), federation architecture (11-12), project rename from CheriChat to OpenMesh (13), rename to OpenCorde (14).
- **Future phases:** E2EE (OpenMLS) rollout, mesh federation server-to-server peering, bridge services (Discord, Slack, Steam), search (Tantivy), optional blockchain name registry.

---

## How to Add a Decision

When a significant architectural choice arises:

1. Create a new header `## N+1. [Title]` with status (PROPOSED, ACCEPTED, REJECTED).
2. Include: **Decision**, **Rationale**, **Rejected alternatives**, **Date**.
3. If it affects the tech stack, update `project-map.yaml`.
4. Link from ReadMeFirst.md's "Key Decision Log" section.
5. Update `tasks.md` if it requires implementation.

---

**Last reviewed:** 2026-03-16
