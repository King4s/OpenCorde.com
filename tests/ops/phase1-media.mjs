/**
 * Real two-client LiveKit media acceptance; invoke only against a disposable QA room.
 *
 * Required environment:
 *   PHASE1_LIVEKIT_URL=wss://<application>/livekit
 *   PHASE1_MEDIA_TOKEN_A, PHASE1_MEDIA_TOKEN_B: short-lived tokens for distinct
 *     identities in the same otherwise empty room, with publish/subscribe grants.
 *   PHASE1_TURN_HOST=<TURN_DOMAIN> (required for relay or both modes).
 * Optional:
 *   PHASE1_MEDIA_MODE=both|baseline|relay (default both)
 *   PHASE1_TURN_TLS_PORT=443 (override for isolated port-remapped QA only)
 *   PHASE1_MEDIA_INSECURE_TLS=1 (isolated self-signed QA only)
 *   PHASE1_CHROMIUM_EXECUTABLE=<browser path>
 *   PHASE1_LIVEKIT_BUNDLE=<installed livekit-client.umd.js path>
 *
 * Run: node tests/ops/phase1-media.mjs
 * Requires root Playwright dependency/browser and the locked client dependency
 * installation. Never downloads dependencies, creates tokens or starts containers.
 * Emits a credential-free JSON result on stdout; nonzero exit means failure.
 * No browser console, network URL, trace, exception text or JWT is logged.
 * A remapped port or insecure certificate NEVER establishes public-443 acceptance.
 * This exercises LiveKit transport, not the application's call UI or token API.
 * API contract: https://github.com/livekit/client-sdk-js/blob/v2.18.0/src/options.ts
 */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { readFile } from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';

let browser;
let stage = 'configuration';
let deadline;
const report = { schema_version: 1, status: 'failed', checks: {},
  turn_tls_443: 'not established', external_network_reachability: 'not established by this helper',
  clients: 2, media: 'synthetic audio and video' };

// Use fixed error stages: browser/SDK errors may include URL access tokens.
function required(name) {
  assert.ok(process.env[name]?.trim(), 'Required environment missing');
  return process.env[name].trim();
}

function configure() {
  const url = new URL(required('PHASE1_LIVEKIT_URL'));
  assert.equal(url.protocol, 'wss:');
  assert.ok(!url.username && !url.password && !url.search && !url.hash);
  const tokens = [required('PHASE1_MEDIA_TOKEN_A'), required('PHASE1_MEDIA_TOKEN_B')];
  assert.notEqual(tokens[0], tokens[1]);
  // Reject accidental same-identity or different-room inputs before sending them.
  const claims = tokens.map((token) => JSON.parse(Buffer.from(token.split('.')[1], 'base64url')));
  assert.ok(claims.every((claim) => claim.sub && claim.video?.roomJoin && claim.video?.room));
  assert.notEqual(claims[0].sub, claims[1].sub);
  assert.equal(claims[0].video.room, claims[1].video.room);
  const mode = process.env.PHASE1_MEDIA_MODE || 'both';
  assert.ok(['both', 'baseline', 'relay'].includes(mode));
  const port = Number(process.env.PHASE1_TURN_TLS_PORT || 443);
  assert.ok(Number.isInteger(port) && port >= 1 && port <= 65535);
  const turnHost = mode === 'baseline' ? '' : required('PHASE1_TURN_HOST').toLowerCase();
  assert.ok(!turnHost || /^[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?$/.test(turnHost));
  const insecure = process.env.PHASE1_MEDIA_INSECURE_TLS === '1';
  report.tls_validation = insecure ? 'disabled for isolated QA' : 'browser trust store';
  report.turn_tls_port = port;
  report.port_remapped = port !== 443;
  return { url: url.toString(), tokens, mode, port, turnHost, insecure };
}

// Executed before loading the SDK. Observe native PCs rather than SDK internals.
// In relay mode keep only the server-advertised TLS endpoint and its ephemeral
// credentials. A missing advertised endpoint fails instead of trying direct ICE.
function installProbe({ relay, turnHost, port }) {
  const NativePC = window.RTCPeerConnection;
  const pcs = [];
  window.__mediaProbe = { pcs, relayConfigurations: 0, subscriptionErrors: 0,
    progress: 'initializing', peerDiagnostics: [] };
  function restrict(configuration = {}) {
    if (!relay) return configuration;
    const iceServers = (configuration.iceServers || []).flatMap((server) => {
      const urls = (Array.isArray(server.urls) ? server.urls : [server.urls]).flatMap((value) => {
        const match = /^turns:([^:/?]+)(?::(\d+))?(?:\?transport=tcp)?$/i.exec(value || '');
        if (!match || match[1].toLowerCase() !== turnHost || Number(match[2] || 5349) !== 443) return [];
        return [`turns:${turnHost}:${port}?transport=tcp`];
      });
      return urls.length ? [{ ...server, urls }] : [];
    });
    if (!iceServers.length) throw new Error('Expected advertised TURN/TLS endpoint absent');
    window.__mediaProbe.relayConfigurations++;
    return { ...configuration, iceTransportPolicy: 'relay', iceServers };
  }
  window.RTCPeerConnection = class extends NativePC {
    constructor(configuration, constraints) {
      super(restrict(configuration), constraints);
      pcs.push(this);
      const diagnostic = { states: [], candidateTypes: {}, iceErrors: [] };
      if (window.__mediaProbe.peerDiagnostics.length < 16) {
        window.__mediaProbe.peerDiagnostics.push(diagnostic);
      }
      const state = () => {
        if (diagnostic.states.length < 24) diagnostic.states.push({
          connection: this.connectionState, ice: this.iceConnectionState,
          gathering: this.iceGatheringState });
      };
      for (const name of ['connectionstatechange', 'iceconnectionstatechange', 'icegatheringstatechange']) {
        this.addEventListener(name, state);
      }
      this.addEventListener('icecandidate', ({ candidate }) => {
        if (candidate && ['host', 'srflx', 'prflx', 'relay'].includes(candidate.type)) {
          diagnostic.candidateTypes[candidate.type] = (diagnostic.candidateTypes[candidate.type] || 0) + 1;
        }
      });
      this.addEventListener('icecandidateerror', (event) => {
        if (diagnostic.iceErrors.length >= 12) return;
        // Do not retain candidate addresses, arbitrary URLs or server error text.
        const text = String(event.errorText || '').toLowerCase();
        diagnostic.iceErrors.push({ code: Number(event.errorCode) || 0,
          category: text.includes('permission') ? 'permission' : text.includes('auth') ? 'authentication'
            : text.includes('host') ? 'host-resolution' : text.includes('cert') ? 'certificate'
              : text.includes('reach') ? 'unreachable' : 'other' });
      });
    }
    setConfiguration(configuration) { return super.setConfiguration(restrict(configuration)); }
  };
}

async function join(page, config, token, relay) {
  await page.evaluate(async ({ url, token, relay }) => {
    const sdk = window.LivekitClient;
    sdk.setLogLevel('silent');
    const probe = window.__mediaProbe;
    const room = new sdk.Room({ adaptiveStream: false, dynacast: false,
      publishDefaults: { simulcast: false, videoCodec: 'vp8', dtx: false } });
    probe.room = room;
    room.on(sdk.RoomEvent.TrackSubscriptionFailed, () => probe.subscriptionErrors++);
    room.on(sdk.RoomEvent.TrackSubscribed, (track) => {
      const element = track.attach();
      element.muted = true;
      element.autoplay = true;
      document.body.append(element);
      void element.play().catch(() => {});
    });
    probe.progress = 'connect';
    await room.connect(url, token, { autoSubscribe: true, maxRetries: 0,
      peerConnectionTimeout: 20000, websocketTimeout: 15000,
      rtcConfig: { iceTransportPolicy: relay ? 'relay' : 'all' } });
    probe.progress = 'capture';
    probe.stream = await navigator.mediaDevices.getUserMedia({ audio: true,
      video: { width: 320, height: 240, frameRate: 15 } });
    for (const track of probe.stream.getTracks()) {
      probe.progress = `publish-${track.kind}`;
      await room.localParticipant.publishTrack(track, {
        source: track.kind === 'video' ? sdk.Track.Source.Camera : sdk.Track.Source.Microphone,
      });
    }
    probe.progress = 'published';
  }, { url: config.url, token, relay });
}

async function failureDiagnostics(page, rejection) {
  // Classify locally; never serialize the Playwright/SDK exception message.
  const text = String(rejection?.message || '').toLowerCase();
  const category = text.includes('advertised turn') ? 'advertised-turn-endpoint-missing'
    : text.includes('peerconnection') || text.includes('peer connection') || text.includes('pc connection') ? 'peer-connection'
      : text.includes('signal') ? 'signaling' : text.includes('publish') ? 'publication'
        : text.includes('timeout') || text.includes('timed out') ? 'timeout'
          : text.includes('connect') ? 'connection' : 'other';
  try {
    return { category, ...await page.evaluate(() => ({
      progress: window.__mediaProbe.progress,
      relayConfigurations: window.__mediaProbe.relayConfigurations,
      subscriptionErrors: window.__mediaProbe.subscriptionErrors,
      peers: window.__mediaProbe.peerDiagnostics,
    })) };
  } catch { return { category, diagnostics: 'page unavailable' }; }
}

async function snapshot(page) {
  return page.evaluate(async () => {
    const probe = window.__mediaProbe;
    const streams = [];
    const transports = new Map();
    for (const [index, pc] of probe.pcs.entries()) {
      if (pc.connectionState === 'closed') continue;
      const stats = await pc.getStats();
      for (const stat of stats.values()) {
        if (!['inbound-rtp', 'outbound-rtp'].includes(stat.type) || stat.isRemote) continue;
        const kind = stat.kind || stat.mediaType;
        if (!['audio', 'video'].includes(kind)) continue;
        const bytes = stat.bytesReceived ?? stat.bytesSent ?? 0;
        if (!bytes) continue;
        const transport = stats.get(stat.transportId);
        const pair = stats.get(transport?.selectedCandidatePairId);
        const candidate = stats.get(pair?.localCandidateId);
        const key = `${index}:${stat.transportId}`;
        transports.set(key, { key, state: pair?.state || 'missing',
          candidate_type: candidate?.candidateType || 'missing',
          relay_protocol: candidate?.relayProtocol || 'missing',
          // Whitelist only the TURN URI shape, never arbitrary URLs/credentials.
          turn_url: /^turns:[a-z0-9.-]+:\d+\?transport=tcp$/i.test(candidate?.url || '')
            ? candidate.url : '', ice_policy: pc.getConfiguration().iceTransportPolicy });
        streams.push({ key: `${index}:${stat.id}`, kind, direction: stat.type,
          bytes, frames: stat.framesDecoded ?? stat.framesEncoded ?? 0 });
      }
    }
    return { connected: probe.room?.state === 'connected',
      remoteParticipants: probe.room?.remoteParticipants.size,
      localTracks: probe.room?.localParticipant.trackPublications.size,
      subscriptionErrors: probe.subscriptionErrors,
      relayConfigurations: probe.relayConfigurations,
      streams, transports: [...transports.values()] };
  });
}

function hasMedia(sample) {
  return sample.connected && sample.remoteParticipants === 1 && sample.localTracks === 2 &&
    !sample.subscriptionErrors && ['inbound-rtp', 'outbound-rtp'].every((direction) =>
      ['audio', 'video'].every((kind) => sample.streams.some((stream) =>
        stream.direction === direction && stream.kind === kind && stream.bytes > 0 &&
        (kind !== 'video' || stream.frames > 0))));
}

function verifyProgress(before, after, config, relay) {
  assert.ok(hasMedia(before) && hasMedia(after), 'Media disappeared');
  const deltas = after.streams.map((stream) => {
    const previous = before.streams.find((entry) => entry.key === stream.key);
    return { kind: stream.kind, direction: stream.direction,
      bytes: previous ? stream.bytes - previous.bytes : 0,
      frames: previous ? stream.frames - previous.frames : 0 };
  });
  for (const direction of ['inbound-rtp', 'outbound-rtp']) {
    for (const kind of ['audio', 'video']) {
      assert.ok(deltas.some((stream) => stream.direction === direction && stream.kind === kind &&
        stream.bytes > 0 && (kind !== 'video' || stream.frames > 0)), 'Media counters stalled');
    }
  }
  assert.ok(after.transports.length > 0);
  for (const transport of after.transports) {
    assert.equal(transport.state, 'succeeded');
    if (relay) {
      assert.ok(after.relayConfigurations > 0);
      assert.equal(transport.ice_policy, 'relay');
      assert.equal(transport.candidate_type, 'relay');
      assert.equal(transport.turn_url, `turns:${config.turnHost}:${config.port}?transport=tcp`);
      // Chromium may label TLS-over-TCP as either tls or tcp; turns: plus the
      // TLS-only ICE configuration disambiguates it from plaintext TURN/TCP.
      assert.ok(['tls', 'tcp'].includes(transport.relay_protocol));
    }
  }
  return { media_deltas: deltas, selected_transports: after.transports };
}

async function scenario(config, bundle, relay) {
  const name = relay ? 'turn_tls_relay' : 'baseline_media';
  const contexts = [];
  const pages = [];
  const started = Date.now();
  stage = `${name}: browser setup`;
  report.checks[name] = { status: 'failed' };
  try {
    for (let index = 0; index < 2; index++) {
      const context = await browser.newContext({ ignoreHTTPSErrors: config.insecure,
        permissions: ['camera', 'microphone'], serviceWorkers: 'block' });
      contexts.push(context);
      // This origin is fulfilled locally, so no unrelated site receives tokens.
      await context.route('https://phase1-media.invalid/**', (route) => route.fulfill({
        contentType: 'text/html', body: '<!doctype html><title>Isolated media QA</title>' }));
      await context.addInitScript(installProbe, { relay, turnHost: config.turnHost, port: config.port });
      const page = await context.newPage();
      await page.goto('https://phase1-media.invalid/');
      await page.addScriptTag({ content: bundle });
      pages.push(page);
    }
    stage = `${name}: join and publish`;
    const joined = await Promise.allSettled(pages.map((page, index) =>
      join(page, config, config.tokens[index], relay)));
    if (joined.some((result) => result.status === 'rejected')) {
      report.checks[name].diagnostics = await Promise.all(pages.map((page, index) =>
        failureDiagnostics(page, joined[index].reason)));
    }
    assert.ok(joined.every((result) => result.status === 'fulfilled'));
    stage = `${name}: await bidirectional media`;
    let before;
    const until = Date.now() + 30000;
    do {
      before = await Promise.all(pages.map(snapshot));
      if (before.every(hasMedia)) break;
      await delay(500);
    } while (Date.now() < until);
    assert.ok(before.every(hasMedia));
    stage = `${name}: verify RTP and selected ICE transport`;
    await delay(2500);
    const after = await Promise.all(pages.map(snapshot));
    const clients = after.map((sample, index) => verifyProgress(before[index], sample, config, relay));
    report.checks[name] = { status: 'passed', seconds: (Date.now() - started) / 1000, clients };
    if (relay && config.port === 443 && !config.insecure) {
      report.turn_tls_443 = 'authenticated relay media passed with browser TLS validation';
    }
  } finally {
    await Promise.allSettled(contexts.map((context) => context.close()));
  }
}

try {
  const config = configure();
  stage = 'load installed dependencies';
  const requireClient = createRequire(new URL('../../client/package.json', import.meta.url));
  const bundle = await readFile(process.env.PHASE1_LIVEKIT_BUNDLE || requireClient.resolve('livekit-client'), 'utf8');
  const { chromium } = await import('playwright');
  stage = 'launch isolated Chromium';
  browser = await chromium.launch({ headless: true,
    executablePath: process.env.PHASE1_CHROMIUM_EXECUTABLE || undefined,
    args: ['--use-fake-device-for-media-stream', '--use-fake-ui-for-media-stream',
      '--autoplay-policy=no-user-gesture-required', '--disable-background-timer-throttling',
      '--disable-renderer-backgrounding', ...(config.insecure ? ['--ignore-certificate-errors'] : [])] });
  await Promise.race([
    (async () => {
      for (const relay of config.mode === 'both' ? [false, true] : [config.mode === 'relay']) {
        await scenario(config, bundle, relay);
      }
    })(),
    new Promise((_, reject) => { deadline = setTimeout(() => reject(new Error('Deadline')), 180000); }),
  ]);
  report.status = 'passed';
} catch {
  report.failure_stage = stage;
  process.exitCode = 1;
} finally {
  clearTimeout(deadline);
  await browser?.close().catch(() => {});
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
}
