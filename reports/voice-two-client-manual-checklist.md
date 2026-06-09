# Two-client voice/video manual checklist — Issue #7

Generated: 2026-05-14T20:14:58+02:00
Scope: OpenCorde two-client voice workflow parity proof for joining a voice channel, proving audio stream presence, muting/unmuting, and leaving.

## Why this is manual first

A Playwright-only proof is not reliable enough for the current voice surface. Chromium can be launched with fake media devices and permission grants, but the current OpenCorde UI does not expose stable test IDs or a WebRTC/LiveKit stats hook that lets the harness prove a real remote audio stream from DOM state alone. It can prove buttons were clicked and API calls occurred; it cannot, without extra instrumentation, prove that remote inbound RTP audio packets/bytes increased for the other client.

Until the client exposes test-friendly RTC state, use this manual browser checklist with `chrome://webrtc-internals` exports as the evidence source. Future automation should add stable `data-testid` selectors plus an opt-in test hook that surfaces LiveKit participant connection state and audio inbound/outbound stats.

## Preconditions

- Base URL: `https://opencorde.com` unless testing a local deployment.
- Two test accounts are available and are members of the same server. Use local/operator env vars for credentials; do not write passwords into reports.
  - Client A: `OC_MEMBER_EMAIL` / `OC_MEMBER_PASSWORD`
  - Client B: `OC_SECOND_EMAIL` / `OC_SECOND_PASSWORD`
- The target server has at least one voice channel visible to both users with `VIEW_CHANNEL`, `CONNECT`, and `SPEAK` permissions.
- Use two isolated browser profiles/windows, or two browsers, so each client has an independent login session.
- Grant microphone permission for both clients.
- For deterministic audio-stream proof, use either two physical microphones, a known test tone played near one microphone at a time, or OS-level virtual audio devices.
- Open `chrome://webrtc-internals` in each browser before joining voice. Keep both tabs open for the whole run and export dumps at the end.

Suggested evidence bundle:

- `reports/parity-screenshots/voice-two-client/01-a-joined.png`
- `reports/parity-screenshots/voice-two-client/02-b-joined.png`
- `reports/parity-screenshots/voice-two-client/03-a-muted-b-sees-muted.png`
- `reports/parity-screenshots/voice-two-client/04-a-unmuted-speaking.png`
- `reports/parity-screenshots/voice-two-client/05-a-left-b-remains.png`
- `reports/raw/voice-two-client-webrtc-a.json` or `.txt` exported from `chrome://webrtc-internals`
- `reports/raw/voice-two-client-webrtc-b.json` or `.txt` exported from `chrome://webrtc-internals`
- A short execution note recording pass/fail, base URL, browser versions, channel name/id, and sanitized user labels only (for example, `client_a`, `client_b`).

## Checklist

### 1. Setup and login

- [ ] Open Client A in a fresh profile/window.
- [ ] Open Client B in a separate fresh profile/window.
- [ ] Log Client A in as the first test member.
- [ ] Log Client B in as the second test member.
- [ ] Navigate both clients to the same server.
- [ ] Confirm both clients can see the same target voice channel in the channel sidebar.
- [ ] Open `chrome://webrtc-internals` in both browser sessions before starting the join flow.

Expected result:

- Both clients are authenticated independently and no secret values are visible in screenshots or exported reports.
- The same voice channel is visible to both clients.

### 2. Client A joins voice

- [ ] In Client A, click the target voice channel.
- [ ] Accept the browser microphone permission prompt if shown.
- [ ] Wait for the `Voice Connected` panel to appear.
- [ ] Confirm Client A's participant list contains Client A.
- [ ] Confirm the microphone button is in the unmuted state (`🎤`, title/label `Mute`), unless the browser has no microphone and the UI intentionally falls back to muted.
- [ ] Capture screenshot `01-a-joined.png`.

Expected result:

- Client A stays connected to the voice channel.
- Client A's local participant appears in the voice participant list.
- `chrome://webrtc-internals` for Client A shows an outbound audio sender once the microphone is enabled.

### 3. Client B joins the same voice channel

- [ ] In Client B, click the same voice channel.
- [ ] Accept the browser microphone permission prompt if shown.
- [ ] Wait for the `Voice Connected` panel to appear in Client B.
- [ ] Confirm Client B's participant list contains Client A and Client B.
- [ ] Confirm Client A's participant list updates to contain Client A and Client B without a reload.
- [ ] Capture screenshots `02-b-joined.png` and, if useful, a matching Client A screenshot.

Expected result:

- Both clients remain connected.
- Each client sees both participants.
- No full-page reload is needed for participant list convergence.

### 4. Prove audio stream A → B

- [ ] Make Client A produce a clear sound: speak, clap, or play a short test tone into Client A's selected microphone.
- [ ] Keep Client B unmuted/undeafened.
- [ ] In Client B, confirm the remote participant for Client A shows activity (green speaking indicator / active speaker styling) while sound is present.
- [ ] In Client B's `chrome://webrtc-internals`, find the inbound RTP audio stats for the LiveKit peer connection and confirm `packetsReceived`, `bytesReceived`, or `audioLevel` changes/increases while Client A produces sound.
- [ ] In Client A's `chrome://webrtc-internals`, confirm outbound RTP audio `packetsSent` or `bytesSent` increases during the same window.

Expected result:

- Client B receives a remote audio stream from Client A.
- UI activity and WebRTC stats agree: active speaker UI changes while inbound audio stats increase.

### 5. Prove audio stream B → A

- [ ] Repeat the same flow in the opposite direction: make Client B produce a clear sound.
- [ ] Confirm Client A shows Client B speaking in the participant list.
- [ ] Confirm Client A's inbound RTP audio stats increase while Client B produces sound.
- [ ] Confirm Client B's outbound RTP audio stats increase.
- [ ] Capture screenshot `04-a-unmuted-speaking.png` or equivalent evidence showing the active/speaking state.

Expected result:

- Client A receives a remote audio stream from Client B.
- UI activity and WebRTC stats agree for the reverse direction.

### 6. Mute and unmute Client A

- [ ] In Client A, click the microphone button.
- [ ] Confirm Client A button changes to muted (`🔇`, title/label `Unmute`).
- [ ] Confirm Client A's participant row shows muted state locally.
- [ ] Confirm Client B sees Client A as muted or stops seeing Client A active speaker changes.
- [ ] Produce sound near Client A's microphone and confirm Client B's inbound audio packets/bytes/audioLevel stop increasing meaningfully for Client A's track while muted.
- [ ] Capture screenshot `03-a-muted-b-sees-muted.png`.
- [ ] In Client A, click the microphone button again.
- [ ] Confirm the button returns to unmuted (`🎤`, title/label `Mute`).
- [ ] Produce sound again and confirm Client B sees activity and inbound audio stats increasing.

Expected result:

- Mute disables Client A audio publishing/remote audio activity.
- Unmute restores Client A audio publishing/remote audio activity without rejoining.

### 7. Mute and unmute Client B

- [ ] Repeat the mute/unmute test from Client B.
- [ ] Confirm Client A sees the expected mute/activity behavior for Client B.
- [ ] Confirm stats stop increasing meaningfully while muted and resume after unmute.

Expected result:

- Both directions have symmetric mute/unmute behavior.

### 8. Leave flow

- [ ] In Client A, click the disconnect (`✕`, title/label `Disconnect`) button.
- [ ] Confirm Client A's `Voice Connected` panel disappears.
- [ ] Confirm Client B's participant list removes Client A without a reload.
- [ ] Confirm Client B remains connected to the voice channel.
- [ ] Capture screenshot `05-a-left-b-remains.png`.
- [ ] In Client B, click disconnect.
- [ ] Confirm Client B's `Voice Connected` panel disappears.

Expected result:

- Leaving clears local voice state for the leaving client.
- Remaining clients receive a participant update without reload.
- The last client can leave cleanly and no ghost participants remain.

### 9. Evidence export and pass/fail note

- [ ] Export `chrome://webrtc-internals` dumps from Client A and Client B.
- [ ] Save screenshots under `reports/parity-screenshots/voice-two-client/`.
- [ ] Save sanitized WebRTC dumps under `reports/raw/`.
- [ ] Write a short sanitized execution note with:
  - Base URL
  - Browser and OS versions
  - Target server/channel label or id
  - Pass/fail for each checklist section
  - Any console errors, failed network requests, or permission prompts
  - No passwords, JWTs, cookies, Authorization headers, LiveKit access tokens, or private user data

## Current report status

This checklist is an execution protocol, not proof that the workflow passes. `reports/discord-parity.json` should continue to mark the two-client voice workflow as unproven until this checklist is executed and the evidence bundle is attached.
