import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import vm from "node:vm";
import ts from "typescript";
import * as stores from "svelte/store";

// Real frontend stores/composer scripts. Only native IPC and HTTP are replaced;
// real native crypto, filesystem crashes and IPC are covered by Rust tests.
async function client({
  encrypted = true,
  native = true,
  hasGroup = true,
  cryptoError = false,
  incompleteRelay = false,
  members = [{ user_id: "owner" }, { user_id: "member" }],
} = {}) {
  const requests = [],
    nativeCalls = [];
  const handlers = new Map();
  const channels = stores.writable([
    { id: "channel", e2ee_enabled: encrypted },
  ]);
  const state = {
    session_id: "session",
    device_id: "device",
    key_package_hex: "abcd",
    channels: hasGroup
      ? [
          {
            channel_id: "channel",
            sequence: "2",
            epoch: "1",
            active: true,
            welcome_ack: null,
          },
        ]
      : [],
    pending: null,
    retry_channel: null,
  };
  let failedDelivery = false,
    nextFailure = false;
  const api = Object.fromEntries(
    ["get", "post", "put", "patch"].map((method) => [
      method,
      async (path, body) => {
        requests.push({
          method,
          path,
          body: body === undefined ? undefined : structuredClone(body),
        });
        if (path.endsWith("/members")) return members;
        if (path.includes("/key-packages/one")) return { key_package: "q80" };
        if (path.endsWith("/e2ee/v2/welcome"))
          throw { status: 404, message: "No invitation" };
        if (method === "get" && path.includes("/e2ee/v2/events?"))
          return {
            events: [],
            has_more: false,
            sequence: incompleteRelay ? "999" : state.channels[0]?.sequence,
            epoch: state.channels[0]?.epoch,
          };
        if (method === "post" && path.endsWith("/e2ee/v2/events")) {
          if (nextFailure) {
            nextFailure = false;
            failedDelivery = true;
            throw new Error("Network interrupted");
          }
          return {
            operation_id: body.operation_id,
            kind: body.kind,
            ciphertext: body.ciphertext ?? "",
            sequence: String(Number(body.expected_sequence) + 1),
            epoch: String(Number(body.epoch) + Number(body.kind === "commit")),
            sender_id: "owner",
            added: (body.added ?? []).map((member) => member.user_id),
            removed: [],
            message_id: body.kind === "application" ? "message" : null,
          };
        }
        if (path.includes("/messages?")) return [];
        if (path === "/threads/thread")
          return { id: "thread", channel_id: "channel" };
        if (path === "/threads/thread/messages") return [];
        if (path === "/posts/post")
          return { post: { id: "post", channel_id: "channel" }, replies: [] };
        return { id: "message", channel_id: "channel", ...body };
      },
    ]),
  );
  api.setToken = () => {};
  const invoke = async (command, args) => {
    nativeCalls.push({ command, args });
    if (command === "crypto_lock" || command.includes("_token")) return null;
    if (command === "crypto_init") return structuredClone(state);
    if (command === "crypto_status") return structuredClone(state);
    if (command === "crypto_key_package") {
      if (args.confirmed_hex) state.key_package_hex = null;
      return structuredClone(state);
    }
    if (command === "crypto_prepare_message" && cryptoError)
      throw new Error("Native encryption failed");
    if (command.startsWith("crypto_prepare_")) {
      const kind =
        command === "crypto_prepare_init"
          ? "init"
          : command === "crypto_prepare_members"
            ? "commit"
            : "application";
      const channel = state.channels.find(
        (item) => item.channel_id === args.channel_id,
      );
      state.pending = {
        channel_id: args.channel_id,
        operation: {
          operation_id: "persisted-operation",
          expected_sequence: channel?.sequence ?? "0",
          epoch: channel?.epoch ?? "0",
          kind,
          ciphertext: kind === "init" ? "" : "qrs",
          added: (args.added ?? []).map((member) => ({
            user_id: member.user_id,
            welcome: "q80",
          })),
        },
      };
      return structuredClone(state.pending);
    }
    if (command === "crypto_accept_event") {
      const channel = state.pending.channel_id;
      state.channels = state.channels.filter(
        (item) => item.channel_id !== channel,
      );
      state.channels.push({
        channel_id: channel,
        sequence: args.event.sequence,
        epoch: args.event.epoch,
        active: true,
        welcome_ack: null,
      });
      state.pending = null;
      return null;
    }
    if (command === "crypto_cached_message") return "decrypted cached message";
    if (command === "crypto_apply_events") return null;
    throw new Error("Unexpected native command " + command);
  };
  const modules = {
    "svelte/store": stores,
    "$lib/api/client": { default: api },
    "$lib/api/websocket": {
      gateway: {
        on(name, handler) {
          handlers.set(name, handler);
        },
        connect() {},
      },
    },
    "$lib/stores/toasts.svelte": { toastError() {} },
    "$app/environment": { browser: true },
    "@tauri-apps/api/core": { invoke },
    "./channels": { channels },
    "$lib/stores/channels": { channels },
    "$lib/stores/auth": { currentUser: stores.writable({ id: "owner" }) },
    "$lib/actions/edgeResize": { edgeResize() {} },
  };
  function load(name, component = false, extra = "") {
    const path = component
      ? "../src/lib/components/chat/" + name + ".svelte"
      : "../src/lib/stores/" + name + ".ts";
    let source = readFileSync(new URL(path, import.meta.url), "utf8");
    if (component)
      source = source.match(/<script lang="ts">([\s\S]*?)<\/script>/)[1];
    const compiled = ts.transpileModule(source + extra, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
      },
    }).outputText;
    const exports = {};
    vm.runInNewContext(compiled, {
      exports,
      require(name) {
        if (name.startsWith("./") && name.endsWith(".svelte")) return {};
        assert.ok(name in modules, "Unexpected import " + name);
        return modules[name];
      },
      window: {
        ...(native ? { __TAURI_INTERNALS__: {} } : {}),
        location: { origin: "https://api.test" },
        confirm() {
          return true;
        },
      },
      localStorage: {
        getItem() {
          return null;
        },
        setItem() {},
        removeItem() {},
      },
      console,
      atob,
      btoa,
      URLSearchParams,
      DataView,
      $state: (value) => value,
      $effect() {},
      $props: () => ({
        channelId: "channel",
        spaceId: "server",
        onSend: (...args) => messageStore.sendMessage("channel", ...args),
      }),
    });
    return exports;
  }
  const e2ee = load("e2ee");
  modules["./e2ee"] = e2ee;
  modules["$lib/stores/e2ee"] = e2ee;
  const messageStore = load("messages");
  const slashCommandsStore = load("slashCommands.svelte").slashCommandsStore;
  modules["$lib/stores/slashCommands.svelte"] = { slashCommandsStore };
  modules["$lib/stores/typing"] = { sendTyping() {} };
  modules["svelte"] = { onMount() {} };
  if (native) await e2ee.openE2EE("9007199254740993");
  requests.length = 0;
  return {
    messages: messageStore,
    e2ee,
    requests,
    nativeCalls,
    channels,
    load,
    slashCommandsStore,
    handlers,
    state,
    failNextDelivery() {
      nextFailure = true;
    },
    writes() {
      return requests.filter((request) => request.method !== "get");
    },
    get failedDelivery() {
      return failedDelivery;
    },
  };
}
for (const scenario of [
  { name: "browser without native crypto", native: false },
  { name: "missing local group", hasGroup: false },
  { name: "native encryption failure", cryptoError: true },
]) {
  test("encrypted send fails closed for " + scenario.name, async () => {
    const app = await client(scenario);
    await assert.rejects(app.messages.sendMessage("channel", "secret"));
    assert.equal(app.writes().length, 0);
  });
}
test("unknown metadata blocks plaintext while ordinary channels continue", async () => {
  const app = await client({ encrypted: false, native: false });
  await app.messages.sendMessage("channel", "ordinary");
  assert.equal(app.writes()[0].body.content, "ordinary");
  app.requests.length = 0;
  app.channels.set([]);
  await assert.rejects(app.messages.sendMessage("channel", "secret"));
  assert.equal(app.writes().length, 0);
});
test("encrypted sends publish only the durable public operation", async () => {
  const app = await client();
  await app.messages.sendMessage("channel", "secret");
  const request = app.writes()[0];
  assert.match(request.path, /e2ee\/v2\/events$/);
  assert.equal(request.body.ciphertext, "qrs");
  assert.equal(request.body.content, undefined);
  assert.ok(!JSON.stringify(request.body).includes("secret"));
  assert.equal(app.state.pending, null);
});
test("uncertain delivery retries the exact persisted envelope before continuing", async () => {
  const app = await client();
  app.failNextDelivery();
  await assert.rejects(
    app.messages.sendMessage("channel", "secret"),
    /Network interrupted/,
  );
  const first = structuredClone(app.writes()[0].body);
  assert.ok(app.state.pending);
  await app.e2ee.joinE2EEGroup("channel");
  assert.deepEqual(app.writes()[1].body, first);
  assert.equal(app.state.pending, null);
});
test("native enrollment preserves Snowflake string and public encoding", async () => {
  const app = await client();
  assert.equal(
    app.nativeCalls.find((call) => call.command === "crypto_init").args.user_id,
    "9007199254740993",
  );
  assert.equal(
    app.nativeCalls.find(
      (call) =>
        call.command === "crypto_key_package" && call.args.confirmed_hex,
    ).args.confirmed_hex,
    "abcd",
  );
});
test("logout invalidates handles and blocks further encrypted transport", async () => {
  const app = await client();
  await app.e2ee.lockE2EE();
  await assert.rejects(app.messages.sendMessage("channel", "secret"));
  assert.equal(app.writes().length, 0);
});
test("encrypted edit, scheduling, commands, threads and forum operations are blocked", async () => {
  const app = await client();
  app.messages.messages.set([{ id: "message", channel_id: "channel" }]);
  await assert.rejects(app.messages.editMessage("message", "secret"));
  await assert.rejects(
    app.slashCommandsStore.dispatchCommand("channel", "/command", ["secret"]),
  );
  const threads = app.load("threads.svelte").threadStore;
  await assert.rejects(threads.createThread("channel", "message", "secret"));
  const forum = app.load("forum.svelte").forumStore;
  await assert.rejects(forum.createPost("channel", "title", "secret"));
  const composer = app.load(
    "MessageInput",
    true,
    `
    exports.schedule = async () => {
      content = "secret"; showSchedulePicker = true; scheduleTime = "2030-01-01T12:00";
      await handleSubmit({ preventDefault() {} }); return { content, uploadError };
    };
  `,
  );
  assert.match(
    (await composer.schedule()).uploadError,
    /unavailable in encrypted channels/,
  );
  assert.equal(app.writes().length, 0);
});
test("composer retains failed draft and prevents concurrent duplicate sends", async () => {
  const failed = await client({ cryptoError: true });
  const composer = failed.load(
    "MessageInput",
    true,
    `
    exports.send = async () => { content = "secret"; await handleSubmit({ preventDefault() {} }); return { content, uploadError }; };
  `,
  );
  const result = await composer.send();
  assert.equal(result.content, "secret");
  assert.match(result.uploadError, /Native encryption failed/);
  const app = await client();
  const active = app.load(
    "MessageInput",
    true,
    `
    exports.sendTwice = async () => { content = "secret";
      await Promise.all([handleSubmit({ preventDefault() {} }), handleSubmit({ preventDefault() {} })]); };
  `,
  );
  await active.sendTwice();
  assert.equal(app.writes().length, 1);
});
test("browser never consumes a Welcome or uploads an unencrypted attachment", async () => {
  const app = await client({ native: false });
  await assert.rejects(app.e2ee.joinE2EEGroup("channel"));
  const composer = app.load(
    "MessageInput",
    true,
    `
    exports.upload = async () => { await handleFilesSelected({ target: { files: [{ name: "secret.txt" }], value: "" } }); return uploadError; };
  `,
  );
  assert.match(await composer.upload(), /desktop app/);
  assert.equal(app.requests.length, 0);
});
test("channel setup uses actual user_id, skips creator and batches members", async () => {
  const app = await client({ hasGroup: false });
  const settings = app.load(
    "../modals/ChannelSettingsModal",
    true,
    "\nexports.enable = handleE2EEToggle;",
  );
  await settings.enable();
  const lookups = app.requests.filter((request) =>
    request.path.includes("/key-packages/one"),
  );
  assert.equal(lookups.length, 1);
  assert.equal(
    lookups[0].path,
    "/users/member/key-packages/one?channel_id=channel",
  );
  const added = app.nativeCalls.find(
    (call) => call.command === "crypto_prepare_members",
  );
  assert.equal(added.args.added[0].user_id, "member");
  assert.equal(added.args.added[0].key_package_hex, "abcd");
  for (const request of app.writes())
    assert.ok(!("group_state" in (request.body ?? {})));
});
test("zero-peer setup persists a solo group", async () => {
  const app = await client({
    hasGroup: false,
    members: [{ user_id: "owner" }],
  });
  const settings = app.load(
    "../modals/ChannelSettingsModal",
    true,
    "\nexports.enable = handleE2EEToggle;",
  );
  await settings.enable();
  assert.equal(
    app.nativeCalls.filter((call) => call.command === "crypto_prepare_members")
      .length,
    0,
  );
  assert.equal(app.e2ee.getGroupState("channel"), "channel");
});
test("reconnect and relay notices drive normal channel catchup", async () => {
  const app = await client();
  await app.messages.fetchMessages("channel");
  app.messages.initMessageListener();
  app.handlers.get("E2eeSync")({ channel_id: "channel" });
  await new Promise(setImmediate);
  assert.ok(
    app.requests.some((request) => request.path.includes("/e2ee/v2/events?")),
  );
  app.requests.length = 0;
  app.handlers.get("Ready")({});
  await new Promise(setImmediate);
  assert.ok(
    app.requests.some((request) => request.path.includes("/messages?")),
  );
});

test("empty relay page cannot hide a newer encrypted stream head", async () => {
  const app = await client({ incompleteRelay: true });
  await assert.rejects(
    app.e2ee.joinE2EEGroup("channel"),
    /synchronization is incomplete/,
  );
});

test("background encrypted channels catch up on relay notices and reconnect", async () => {
  const app = await client();
  app.messages.initMessageListener();
  app.handlers.get("E2eeSync")({ channel_id: "channel" });
  await new Promise(setImmediate);
  assert.ok(
    app.requests.some((request) => request.path.includes("/e2ee/v2/events?")),
  );
  app.requests.length = 0;
  app.handlers.get("Ready")({});
  await new Promise(setImmediate);
  assert.ok(
    app.requests.some((request) => request.path.includes("/e2ee/v2/events?")),
  );
});
