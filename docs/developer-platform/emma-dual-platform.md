# Emma Dual-Platform Bot — Architecture Specification

> Version 1.0 — 2026-06-09

## Target

Emma skal fungere på BÅDE Discord (nuværende Red-DiscordBot) og OpenCorde (ny adapter) med samme kerne-logik.

## Arkitektur

```
EmmaCore (Python — commands, AI, knowledge, profiles, music decisions)
├── DiscordAdapter (Red/discord.py events → core → Discord responses)
└── OpenCordeAdapter (opencorde-bot-client → core → OpenCorde API responses)
```

## EmmaCore Interface

```python
class EmmaPlatformAdapter(Protocol):
    async def send_message(self, channel_id: str, content: str, embeds: list[dict] = None) -> str: ...
    async def reply_interaction(self, interaction_id: str, token: str, response: dict) -> None: ...
    async def register_commands(self, commands: list[dict]) -> None: ...
    async def get_member(self, server_id: str, user_id: str) -> dict: ...
    async def add_reaction(self, channel_id: str, message_id: str, emoji: str) -> None: ...
```

## OpenCorde Bot Client

Python library der bruger OpenCorde's bot API:
- Bot token auth → gateway connect → identify med intents
- Heartbeat/ack → receive dispatch events
- Register application commands via REST
- Respond to interactions (ephemeral/deferred/followup)
- Send messages med embeds
- Voice integration (senere)

## Feature Matrix

| Feature | Discord (Red) | OpenCorde MVP | OpenCorde Blocked By |
|---------|--------------|---------------|---------------------|
| Chat | message event | MESSAGE_CREATE | Gateway events |
| Slash commands | app_commands | application_commands | Commands API |
| Welcome | member_join | MemberJoin event | Gateway |
| Profiles | user_notes | user profiles | User API exists |
| Knowledge | AI Gateway | Same AI Gateway | — |
| Music | Lavalink | — | Voice bot API (senere) |
| Embeds | Discord embeds | MessageEmbed | Embeds model færdig |

## Deployment

Emma kører fortsat på Thor (10.8.0.1) som Docker container.
OpenCorde adapter tilføjes som separat Python package i Emma's container.
Begge adapters kører i samme proces — Emma lytter på begge gateways samtidigt.

## Næste skridt

1. Byg opencorde-bot-client Python package
2. Implementer EmmaCore interface
3. Registrer Emma som OpenCorde application med bot token
4. Implementer commands: /emma ping, /emma status, /emma test
5. Live proof: Emma svarer på OpenCorde
