# /client/src/lib/livekit/

Purpose: LiveKit SDK integration for voice, video, and screen sharing.

Pattern: Separated by concern — room lifecycle, track management, controls.

| File | Purpose |
|------|---------|
| room.ts | LiveKit room connection, participant management |
| tracks.ts | Audio/video track subscription, display, unsubscribe |
| controls.ts | Mute/unmute, camera toggle, screen share start/stop |
