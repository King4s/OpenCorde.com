// Run from repository root: node tests/client/dm-store.mjs
// Real TypeScript store + Svelte runtime; only API/gateway boundaries are mocked.
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const require = createRequire(resolve(root, "client/package.json"));
const { createServer } = await import(
  pathToFileURL(
    resolve(dirname(require.resolve("vite/package.json")), "dist/node/index.js"),
  ).href
);
const storage = new Map([
  ["opencorde_token", "synthetic-session-a"],
  ["opencorde_server", "https://example.invalid"],
]);
const originalStorage = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
Object.defineProperty(globalThis, "localStorage", {
  configurable: true,
  value: { getItem: (key) => storage.get(key) ?? null },
});
const handlers = new Set();
let getResponse = async () => [];
let postResponse = async () => { throw new Error("Unexpected API POST"); };
globalThis.__dmStoreTest = {
  api: {
    get: (...args) => getResponse(...args),
    post: (...args) => postResponse(...args),
  },
  gateway: {
    on(event, handler) {
      assert.equal(event, "DmMessageCreate");
      handlers.add(handler);
    },
    off(event, handler) {
      assert.equal(event, "DmMessageCreate");
      handlers.delete(handler);
    },
  },
};
const server = await createServer({
  configFile: false,
  root: resolve(root, "client"),
  server: { middlewareMode: true, hmr: false, watch: null },
  plugins: [{
    name: "dm-store-boundaries",
    enforce: "pre",
    resolveId(id) {
      if (id === "$lib/api/client" || id === "$lib/api/websocket") return `\0${id}`;
    },
    load(id) {
      if (id === "\0$lib/api/client") return "export default globalThis.__dmStoreTest.api;";
      if (id === "\0$lib/api/websocket") return "export const gateway = globalThis.__dmStoreTest.gateway;";
    },
  }],
});
const read = (store) => {
  let value;
  store.subscribe((next) => { value = next; })();
  return value;
};
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
const message = (id, dm_id = "a") => ({
  id, dm_id, author_id: "recipient", content: id, created_at: "2026-01-01T00:00:00Z",
});
const emit = (msg) => { for (const handler of [...handlers]) handler({ message: msg }); };
let passed = 0;
try {
  const dm = await server.ssrLoadModule("/src/lib/stores/dms.ts");
  async function scenario(name, run) {
    storage.set("opencorde_token", "synthetic-session-a");
    dm.resetDmState?.();
    dm.dmChannels.set([{ id: "a" }, { id: "b" }]);
    dm.activeDmMessages.set([]);
    dm.dmUnreadCounts.set(new Map());
    getResponse = async () => [];
    postResponse = async () => { throw new Error("Unexpected API POST"); };
    const cleanup = dm.initDmListener();
    try {
      await run();
      console.log(`PASS ${name}`);
      passed++;
    } finally {
      cleanup?.();
    }
  }
  for (const first of ["api", "gateway"]) {
    await scenario(`${first}-first delivery appends one message`, async () => {
      await dm.fetchDmMessages("a");
      dm.dmChannels.set([{ id: "b" }, { id: "a" }]);
      const pending = deferred();
      postResponse = () => pending.promise;
      const msg = message(`send-${first}`);
      const sending = dm.sendDmMessage("a", msg.content);
      if (first === "gateway") emit(msg);
      pending.resolve(msg);
      await sending;
      emit(msg);
      assert.deepEqual(read(dm.activeDmMessages).map((m) => m.id), [msg.id]);
      assert.equal(read(dm.dmUnreadCounts).size, 0);
      assert.equal(read(dm.dmChannels)[0].id, "a");
    });
  }
  await scenario("inactive delivery increments unread once and bubbles channel", async () => {
    await dm.fetchDmMessages("a");
    emit(message("inactive", "b"));
    emit(message("inactive", "b"));
    assert.equal(read(dm.dmUnreadCounts).get("b"), 1);
    assert.equal(read(dm.dmChannels)[0].id, "b");
    assert.equal(read(dm.activeDmMessages).length, 0);
  });
  await scenario("listener owners share one handler and release independently", async () => {
    const releaseOne = dm.initDmListener();
    const releaseTwo = dm.initDmListener();
    assert.equal(handlers.size, 1);
    releaseOne();
    releaseOne();
    emit(message("owned", "b"));
    assert.equal(read(dm.dmUnreadCounts).get("b"), 1);
    releaseTwo();
    assert.equal(handlers.size, 1); // scenario still owns its listener
  });
  assert.equal(handlers.size, 0, "last owner detaches the gateway listener");
  await scenario("leaving a conversation clears active state and restores unread", async () => {
    await dm.fetchDmMessages("a");
    dm.clearActiveDm();
    emit(message("after-leave"));
    assert.equal(read(dm.dmUnreadCounts).get("a"), 1);
    assert.equal(read(dm.activeDmMessages).length, 0);
  });
  await scenario("late fetch cannot overwrite a newer conversation or loading state", async () => {
    const a = deferred(), b = deferred();
    getResponse = (path) => path.includes("/a/") ? a.promise : b.promise;
    const first = dm.fetchDmMessages("a");
    const second = dm.fetchDmMessages("b");
    a.resolve([message("old-a")]);
    await first;
    assert.equal(read(dm.dmLoading), true);
    assert.equal(read(dm.activeDmMessages).length, 0);
    b.resolve([message("new-b", "b")]);
    await second;
    assert.deepEqual(read(dm.activeDmMessages).map((m) => m.id), ["new-b"]);
    assert.equal(read(dm.dmLoading), false);
  });
  await scenario("late send cannot append into a different conversation", async () => {
    await dm.fetchDmMessages("a");
    const pending = deferred();
    postResponse = () => pending.promise;
    const sending = dm.sendDmMessage("a", "late");
    await dm.fetchDmMessages("b");
    pending.resolve(message("late"));
    await sending;
    emit(message("late"));
    assert.equal(read(dm.activeDmMessages).length, 0);
    assert.equal(read(dm.dmUnreadCounts).size, 0);
  });
  await scenario("gateway-first inactive send removes only its own unread echo", async () => {
    await dm.fetchDmMessages("a");
    const pending = deferred();
    postResponse = () => pending.promise;
    const sending = dm.sendDmMessage("a", "own");
    await dm.fetchDmMessages("b");
    emit(message("someone-else"));
    emit(message("own"));
    emit(message("own"));
    assert.equal(read(dm.dmUnreadCounts).get("a"), 2);
    pending.resolve(message("own"));
    await sending;
    assert.equal(read(dm.dmUnreadCounts).get("a"), 1);
    assert.equal(read(dm.activeDmMessages).length, 0);
  });
  await scenario("fetch preserves live arrival and pagination deduplicates overlap", async () => {
    const pending = deferred();
    getResponse = () => pending.promise;
    const fetching = dm.fetchDmMessages("a");
    emit(message("live"));
    pending.resolve([message("history"), message("live")]);
    await fetching;
    getResponse = async () => [message("older"), message("history")];
    await dm.fetchDmMessages("a", "history");
    assert.deepEqual(read(dm.activeDmMessages).map((m) => m.id), ["older", "history", "live"]);
  });
  await scenario("failed fetch releases loading and propagates error", async () => {
    getResponse = async () => { throw new Error("synthetic fetch failure"); };
    await assert.rejects(dm.fetchDmMessages("a"), /synthetic fetch failure/);
    assert.equal(read(dm.dmLoading), false);
  });
  await scenario("logout rejects late data and old-session gateway delivery", async () => {
    const pending = deferred();
    getResponse = () => pending.promise;
    const fetching = dm.fetchDmMessages("a");
    storage.delete("opencorde_token");
    emit(message("after-logout"));
    pending.resolve([message("old-session")]);
    await fetching;
    assert.equal(read(dm.activeDmMessages).length, 0);
    assert.equal(read(dm.dmChannels).length, 0);
    assert.equal(read(dm.dmUnreadCounts).size, 0);
    assert.equal(read(dm.dmLoading), false);
  });
  await scenario("new session ignores pending send and channel-list response", async () => {
    const pendingSend = deferred(), pendingChannels = deferred();
    postResponse = () => pendingSend.promise;
    getResponse = () => pendingChannels.promise;
    const sending = dm.sendDmMessage("a", "old");
    const fetching = dm.fetchDmChannels();
    storage.set("opencorde_token", "synthetic-session-b");
    emit(message("new-session", "b"));
    pendingSend.resolve(message("old-send"));
    pendingChannels.resolve([{ id: "old-channel" }]);
    await Promise.all([sending, fetching]);
    assert.equal(read(dm.activeDmMessages).length, 0);
    assert.equal(read(dm.dmChannels).length, 0);
    assert.equal(read(dm.dmUnreadCounts).get("b"), 1);
  });
  console.log(`DM store: ${passed} scenarios passed`);
} finally {
  await server.close();
  delete globalThis.__dmStoreTest;
  if (originalStorage) Object.defineProperty(globalThis, "localStorage", originalStorage);
  else delete globalThis.localStorage;
}
