import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import vm from "node:vm";
import ts from "typescript";
import * as stores from "svelte/store";

// Exercise the real stores, replacing only API/native/browser boundaries.
function client({
  encrypted = true,
  native = true,
  cryptoError = false,
  members = [{ user_id: "owner" }, { user_id: "member" }],
} = {}) {
  const requests = [];
  const nativeCalls = [];
  const channels = stores.writable([
    { id: "channel", e2ee_enabled: encrypted },
  ]);
  const api = Object.fromEntries(
    ["get", "post", "put", "patch"].map((method) => [
      method,
      async (path, body) => {
        requests.push({ method, path, body });
        if (path.endsWith("/members")) return members;
        if (path.includes("/key-packages/one")) return { key_package: "q80" };
        if (path.endsWith("/welcome")) return { welcome_message: "AA" };
        if (path === "/threads/thread")
          return { id: "thread", channel_id: "channel" };
        if (path === "/threads/thread/messages") return [];
        if (path === "/posts/post")
          return { post: { id: "post", channel_id: "channel" }, replies: [] };
        if (path === "/users/@me/scheduled-messages")
          return [{ id: "scheduled", channel_id: "channel" }];
        return { id: "message", channel_id: "channel", ...body };
      },
    ]),
  );
  api.setToken = () => {};
  const invoke = async (command, args) => {
    nativeCalls.push({ command, args });
    if (command === "crypto_init") return "abcd";
    if (cryptoError) throw new Error("Native encryption failed");
    if (
      command === "crypto_create_group" ||
      command === "crypto_process_welcome"
    )
      return "PRIVATE_GROUP_STATE";
    return {
      group_state_hex: "PRIVATE_GROUP_STATE",
      ciphertext_hex: "aabb",
      welcome_hex: "abcd",
      commit_hex: "cdef",
    };
  };
  const modules = {
    "svelte/store": stores,
    "$lib/api/client": { default: api },
    "$lib/api/websocket": { gateway: { on() {} } },
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
    source += extra;
    const compiled = ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
      },
    }).outputText;
    const exports = {};
    vm.runInNewContext(compiled, {
      exports,
      require: (name) => {
        if (name.startsWith("./") && name.endsWith(".svelte")) return {};
        assert.ok(name in modules, "Unexpected import " + name);
        return modules[name];
      },
      localStorage: {
        getItem() {
          return null;
        },
        setItem() {},
        removeItem() {},
      },
      window: native ? { __TAURI_INTERNALS__: {} } : {},
      console,
      atob,
      btoa,
      URLSearchParams,
      $effect() {},
      $state: (value) => value,
      $props: () => ({
        channelId: "channel",
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
  return {
    messages: messageStore,
    e2ee,
    requests,
    nativeCalls,
    channels,
    load,
    slashCommandsStore,
  };
}

for (const scenario of [
  { name: "browser without native crypto", native: false, state: true },
  { name: "missing group state", state: false },
  { name: "failed native encryption", cryptoError: true, state: true },
]) {
  test("encrypted sends reject " + scenario.name + " before HTTP", async () => {
    const app = client(scenario);
    if (scenario.state) app.e2ee.setGroupState("channel", "private");
    await assert.rejects(app.messages.sendMessage("channel", "secret"));
    assert.equal(app.requests.length, 0);
  });
}

test("unknown channel metadata cannot permit a plaintext send", async () => {
  const app = client({ encrypted: false });
  app.channels.set([]);
  await assert.rejects(app.messages.sendMessage("channel", "secret"));
  assert.equal(app.requests.length, 0);
});

test("successful native encryption sends ciphertext only", async () => {
  const app = client();
  app.e2ee.setGroupState("channel", "private");
  await app.messages.sendMessage("channel", "secret");
  assert.equal(app.requests[0].body.content, "enc:aabb");
});

test("ordinary channels still send plaintext", async () => {
  const app = client({ encrypted: false, native: false });
  await app.messages.sendMessage("channel", "ordinary");
  assert.equal(app.requests[0].body.content, "ordinary");
});

test("encrypted edits reject before HTTP", async () => {
  const app = client();
  app.messages.messages.set([{ id: "message", channel_id: "channel" }]);
  await assert.rejects(app.messages.editMessage("message", "secret"));
  assert.equal(app.requests.length, 0);
});

test("group lifecycle never uploads private group state", async () => {
  const app = client();
  await app.e2ee.initE2EEGroup("channel", new Map([["member", "aa"]]));
  await app.e2ee.joinE2EEGroup("channel");
  await app.e2ee.addE2EEMember("channel", "member2", "bb");
  assert.equal(app.e2ee.getGroupState("channel"), "PRIVATE_GROUP_STATE");
  assert.equal(app.requests.filter((r) => r.method === "put").length, 0);
  for (const request of app.requests.filter((r) => r.path.endsWith("/init"))) {
    assert.equal(request.body.group_state, "");
    assert.ok(request.body.member_welcomes.length);
    assert.ok(!JSON.stringify(request.body).includes("PRIVATE_GROUP_STATE"));
  }
});

test("encrypted slash commands, threads and forums reject before HTTP", async () => {
  const app = client();
  await assert.rejects(
    app.slashCommandsStore.dispatchCommand("channel", "/command", ["secret"]),
  );
  const threads = app.load("threads.svelte").threadStore;
  await assert.rejects(threads.createThread("channel", "message", "secret"));
  await threads.openThread("thread");
  app.requests.length = 0;
  await assert.rejects(threads.sendMessage("thread", "secret"));
  const forum = app.load("forum.svelte").forumStore;
  await assert.rejects(forum.createPost("channel", "title", "secret"));
  await forum.fetchPost("post");
  app.requests.length = 0;
  await assert.rejects(forum.createReply("post", "secret"));
  assert.equal(app.requests.length, 0);
});

test("encrypted scheduling in the composer rejects before HTTP and retains draft", async () => {
  const app = client();
  const composer = app.load(
    "MessageInput",
    true,
    `
    exports.schedule = async () => {
      content = "secret";
      showSchedulePicker = true;
      scheduleTime = "2030-01-01T12:00";
      await handleSubmit({ preventDefault() {} });
      return { content, uploadError };
    };
  `,
  );
  const result = await composer.schedule();
  assert.equal(result.content, "secret");
  assert.match(result.uploadError, /unavailable in encrypted channels/);
  assert.equal(app.requests.length, 0);
});

test("encrypted scheduled edits reject before HTTP", async () => {
  const app = client();
  const panel = app.load(
    "ScheduledMessagesPanel",
    true,
    `
    exports.edit = async () => {
      messages = [{ id: "scheduled", channel_id: "channel" }];
      editContent = "secret";
      await saveEdit("scheduled");
      return error;
    };
  `,
  );
  assert.match(await panel.edit(), /unavailable in encrypted channels/);
  assert.equal(app.requests.length, 0);
});

test("unsupported attachment uploads reject before HTTP", async () => {
  const app = client({ native: false });
  const composer = app.load(
    "MessageInput",
    true,
    `
    exports.upload = async () => {
      await handleFilesSelected({ target: { files: [{ name: "secret.txt" }], value: "" } });
      return uploadError;
    };
  `,
  );
  assert.match(await composer.upload(), /desktop app/);
  assert.equal(app.requests.length, 0);
});

test("failed normal send keeps composer draft and displays the error", async () => {
  const app = client({ cryptoError: true });
  app.e2ee.setGroupState("channel", "private");
  const composer = app.load(
    "MessageInput",
    true,
    `
    exports.send = async () => {
      content = "secret";
      await handleSubmit({ preventDefault() {} });
      return { content, uploadError };
    };
  `,
  );
  const result = await composer.send();
  assert.equal(result.content, "secret");
  assert.match(result.uploadError, /Native encryption failed/);
  assert.equal(app.requests.length, 0);
});

test("unsupported browser never consumes a Welcome", async () => {
  const app = client({ native: false });
  await assert.rejects(app.e2ee.joinE2EEGroup("channel"));
  assert.equal(app.requests.length, 0);
});

test("opening an already joined channel does not consume another Welcome", async () => {
  const app = client();
  app.e2ee.setGroupState("channel", "existing");
  await app.e2ee.joinE2EEGroup("channel");
  assert.equal(app.requests.length, 0);
  assert.equal(app.e2ee.getGroupState("channel"), "existing");
});

test("native key packages use base64url for upload and hex for native add-member", async () => {
  const app = client();
  const auth = app.load("auth", false, "\nexports.upload = uploadKeyPackage;");
  await auth.upload("9007199254740993");
  assert.equal(
    app.nativeCalls.find((call) => call.command === "crypto_init").args.user_id,
    "9007199254740993",
  );
  assert.equal(
    app.requests.find((r) => r.path === "/users/me/key-packages").body
      .key_package,
    "q80",
  );
  app.requests.length = 0;
  const settings = app.load(
    "../modals/ChannelSettingsModal",
    true,
    "\nexports.enable = handleE2EEToggle;",
  );
  await settings.enable();
  assert.equal(
    app.nativeCalls.find((call) => call.command === "crypto_add_member").args
      .member_key_package_hex,
    "abcd",
  );
});

test("composer prevents duplicate concurrent sends while retaining the pending draft", async () => {
  const app = client();
  app.e2ee.setGroupState("channel", "private");
  const composer = app.load(
    "MessageInput",
    true,
    `
    exports.sendTwice = async () => {
      content = "secret";
      await Promise.all([
        handleSubmit({ preventDefault() {} }),
        handleSubmit({ preventDefault() {} }),
      ]);
    };
  `,
  );
  await composer.sendTwice();
  assert.equal(app.requests.length, 1);
});

test("channel setup uses member user_id and excludes the creator", async () => {
  const app = client();
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
  const init = app.requests.find((request) =>
    request.path.endsWith("/e2ee/init"),
  );
  assert.equal(init.body.member_welcomes.length, 1);
  assert.equal(init.body.member_welcomes[0].user_id, "member");
});

test("channel setup initializes a local solo group with no peers", async () => {
  const app = client({ members: [{ user_id: "owner" }] });
  const settings = app.load(
    "../modals/ChannelSettingsModal",
    true,
    "\nexports.enable = handleE2EEToggle;",
  );
  await settings.enable();
  assert.equal(
    app.requests.filter((request) => request.path.includes("/key-packages/one"))
      .length,
    0,
  );
  assert.equal(app.e2ee.getGroupState("channel"), "PRIVATE_GROUP_STATE");
  const init = app.requests.find((request) =>
    request.path.endsWith("/e2ee/init"),
  );
  assert.equal(init.body.member_welcomes.length, 0);
  assert.equal(init.body.group_state, "");
});
