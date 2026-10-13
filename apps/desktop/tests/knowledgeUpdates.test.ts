import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fork } from "node:child_process";
import { once } from "node:events";
import fsExt from "fs-ext";
import fsPromises from "node:fs/promises";
import { syncBuiltinESMExports } from "node:module";
import { KnowledgeUpdates, synchronizeKnowledge } from "../electron/knowledgeUpdates.ts";
import { verifyKnowledgeActivation } from "../electron/knowledgeProof.ts";
import { signer } from "./knowledgeTestFixtures.ts";
async function setup(t: import("node:test").TestContext) {
  const directory = await mkdtemp(join(tmpdir(), "knowledge-update-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const key = signer();
  const starter = key.proof(1).verified;
  const next = key.proof(2);
  const starterPath = join(directory, "starter.json");
  await writeFile(starterPath, JSON.stringify(starter.activation));
  const calls: string[] = [];
  const fetcher = async (url: string) => { calls.push(url); return new Response(url.endsWith("manifest.json") ? next.manifest : next.bundle); };
  const options = { directory: join(directory, "cache"), starterPath, publicKey: key.publicKey,
    feedUrl: "https://example.org/knowledge/manifest.json", fetcher, now: () => 1_800_000_000_000 };
  return { options, calls, key, starter, next };
}
test("synchronization sends exact activation proof and recovers from older local snapshot", async t => {
  const f = await setup(t);
  const activated: string[] = [];
  await synchronizeKnowledge(new KnowledgeUpdates(f.options), async activation => {
    const value = verifyKnowledgeActivation(Buffer.from(JSON.stringify(activation)), f.key.publicKey);
    if (value.bundle.sequence === 1) throw new Error("knowledge bundle sequence is older than the active bundle");
    assert.deepEqual(activation, f.next.verified.activation);
    activated.push(value.bundle.version);
  }, () => true);
  assert.deepEqual(activated, ["2"]);
  assert.equal(f.calls.length, 2);
});
test("synchronization propagates other activation failures", async t => {
  const f = await setup(t);
  await assert.rejects(synchronizeKnowledge(new KnowledgeUpdates(f.options), async () => { throw new Error("unauthorized"); }, () => true), /unauthorized/);
  assert.deepEqual(f.calls, []);
});
test("synchronization stops at every stale workspace boundary", async t => {
  const f = await setup(t);
  for (const stage of ["load", "activate", "check"]) {
    let current = true;
    let checks = 0;
    let activations = 0;
    await synchronizeKnowledge({ load: async () => { if (stage === "load") current = false; return f.starter; },
      check: async () => { checks++; if (stage === "check") current = false; return f.next.verified; } },
    async () => { activations++; if (stage === "activate") current = false; }, () => current);
    assert.equal(activations, stage === "load" ? 0 : 1, stage);
    assert.equal(checks, stage === "check" ? 1 : 0, stage);
  }
});
test("verified update survives restart and scheduled checks respect interval", async t => {
  const f = await setup(t);
  const updates = new KnowledgeUpdates(f.options);
  assert.equal((await updates.load())?.bundle.version, "1");
  assert.equal((await updates.check())?.bundle.version, "2");
  assert.equal((await new KnowledgeUpdates(f.options).load())?.bundle.version, "2");
  await updates.check();
  assert.equal(f.calls.length, 2);
  assert.equal(updates.status().lastError, null);
});
test("tampered signed response and offline refresh preserve verified knowledge", async t => {
  const f = await setup(t);
  const bad = new KnowledgeUpdates({ ...f.options, fetcher: async () => new Response(f.next.manifest.toString().replace('"signature":"', '"signature":"AAAA')) });
  assert.equal((await bad.check())?.bundle.version, "1");
  assert.match(bad.status().lastError ?? "", /signature/i);
  await new KnowledgeUpdates({ ...f.options, now: () => f.options.now() + 7 * 3_600_000 }).check();
  const offline = new KnowledgeUpdates({ ...f.options, now: () => f.options.now() + 14 * 3_600_000, fetcher: async () => { throw new Error("offline"); } });
  assert.equal((await offline.check())?.bundle.version, "2");
  assert.match(offline.status().lastError ?? "", /offline/);
});
test("selector filters unsupported policy and format before ranking", async t => {
  const f = await setup(t);
  const manifest = f.key.signed({ formatVersion: 1, bundles: [f.next.entry,
    { ...f.next.entry, formatVersion: 2, sequence: 99, version: "future", path: "bundles/future.json" },
    { ...f.next.entry, privacyPolicyVersion: 2, sequence: 100, version: "policy", path: "bundles/policy.json" }] });
  const updates = new KnowledgeUpdates({ ...f.options, fetcher: async url => new Response(url.endsWith("manifest.json") ? manifest : f.next.bundle) });
  assert.equal((await updates.check())?.bundle.version, "2");
});
test("equal sequence different signed envelope reports equivocation before download shortcut", async t => {
  const f = await setup(t);
  const other = f.key.proof(1, "1", "Different reviewed guidance.");
  const updates = new KnowledgeUpdates({ ...f.options, fetcher: async () => new Response(other.manifest) });
  assert.equal((await updates.check())?.identity.bundleSha256, f.starter.identity.bundleSha256);
  assert.match(updates.status().lastError ?? "", /equivocation/i);
});
test("same bundle in a new manifest remains idempotent", async t => {
  const f = await setup(t);
  const one = f.key.proof(1);
  const newManifest = f.key.signed({ formatVersion: 1, bundles: [one.entry,
    { ...one.entry, formatVersion: 2, version: "future", path: "bundles/future.json" }] });
  const calls: string[] = [];
  const updates = new KnowledgeUpdates({ ...f.options, fetcher: async url => { calls.push(url); return new Response(newManifest); } });
  assert.deepEqual((await updates.check())?.identity, f.starter.identity);
  assert.equal(updates.status().lastError, null);
  assert.equal(calls.length, 1);
});
test("disabled updates load proof without network and disabling cancels pending request", async t => {
  const f = await setup(t);
  const disabled = new KnowledgeUpdates(f.options);
  disabled.setEnabled(false);
  assert.equal((await disabled.check())?.bundle.version, "1");
  assert.equal(f.calls.length, 0);
  let started!: () => void;
  const ready = new Promise<void>(resolve => { started = resolve; });
  let signal: AbortSignal | undefined;
  const updates = new KnowledgeUpdates({ ...f.options, fetcher: async (_url, init) => {
    signal = init?.signal as AbortSignal; started();
    return new Promise<Response>((_resolve, reject) => { signal!.addEventListener("abort", () => reject(new Error("cancelled")), { once: true }); });
  } });
  const pending = updates.check();
  await ready; updates.setEnabled(false);
  assert.equal((await pending)?.bundle.version, "1");
  assert.equal(signal?.aborted, true);
});
test("no eligible starter still permits independently verified feed recovery", async t => {
  const f = await setup(t);
  await writeFile(f.options.starterPath, JSON.stringify({ sequence: 9999 }));
  const updates = new KnowledgeUpdates(f.options);
  assert.equal(await updates.load(), null);
  assert.equal(updates.status().version, null);
  assert.equal((await updates.check())?.bundle.version, "2");
});

test("invalid packaged key fails knowledge closed without preventing application initialization", async t => {
  const f = await setup(t);
  const updates = new KnowledgeUpdates({ ...f.options, publicKey: "" });
  assert.equal(await updates.load(), null);
  assert.equal(await updates.check(), null);
  assert.match(updates.status().lastError ?? "", /key/i);
  assert.deepEqual(f.calls, []);
});
test("synchronization activates the actual newer durable winner after a download race", async t => {
  const f = await setup(t);
  const { KnowledgeCache } = await import("../electron/knowledgeCache.ts");
  const winner = f.key.proof(3).verified;
  const updates = new KnowledgeUpdates({ ...f.options, fetcher: async url => {
    if (url.endsWith("manifest.json")) return new Response(f.next.manifest);
    await new KnowledgeCache(f.options).commit(winner);
    return new Response(f.next.bundle);
  } });
  const activated: number[] = [];
  await synchronizeKnowledge(updates, async activation => {
    const proof = verifyKnowledgeActivation(Buffer.from(JSON.stringify(activation)), f.key.publicKey);
    activated.push(proof.identity.sequence);
  }, () => true);
  assert.deepEqual(activated, [1, 3]);
});

test("disabling a downloaded update waiting on the real cache lock cancels publication even after reenable", async t => {
  const f = await setup(t);
  const { KnowledgeCache } = await import("../electron/knowledgeCache.ts");
  const holder = fork(join(import.meta.dirname, "knowledgeCacheWorker.ts"), [], { execArgv: ["--experimental-strip-types"], stdio: ["ignore", "ignore", "ignore", "ipc"] });
  t.after(() => { if (holder.exitCode === null) holder.kill(); });
  const ready = once(holder, "message"); holder.send({ action: "hold", options: f.options }); await ready;
  let waiting!: () => void;
  const blocked = new Promise<void>(resolve => { waiting = resolve; });
  const originalFlock = fsExt.flockSync;
  fsExt.flockSync = (fd, flags) => {
    try { return originalFlock(fd, flags); }
    catch (error) { waiting(); throw error; }
  };
  t.after(() => { fsExt.flockSync = originalFlock; });
  const updates = new KnowledgeUpdates(f.options);
  const activated: number[] = [];
  const pending = synchronizeKnowledge(updates, async activation => {
    activated.push(verifyKnowledgeActivation(Buffer.from(JSON.stringify(activation)), f.key.publicKey).identity.sequence);
  }, () => true);
  await blocked;
  updates.setEnabled(false);
  updates.setEnabled(true);
  const exited = once(holder, "exit"); holder.send({ release: true }); await exited;
  await pending;
  assert.equal((await new KnowledgeCache(f.options).load())?.identity.sequence, 1);
  assert.deepEqual(activated, [1]);
  assert.equal(updates.status().version, "1");
  assert.equal(updates.status().lastSuccessfulUpdate, null);
  assert.match(updates.status().lastError ?? "", /abort|cancel|disabled/i);
});
for (const outcome of ["unchanged", "offline"] as const) {
  test(`a concurrent durable winner reaches synchronization after an ${outcome} feed check`, async t => {
    const f = await setup(t);
    const { KnowledgeCache } = await import("../electron/knowledgeCache.ts");
    let release!: () => void;
    const barrier = new Promise<void>(resolve => { release = resolve; });
    let started!: () => void;
    const ready = new Promise<void>(resolve => { started = resolve; });
    const updates = new KnowledgeUpdates({ ...f.options, fetcher: async () => {
      started(); await barrier;
      if (outcome === "offline") throw new Error("offline");
      return new Response(f.key.proof(1).manifest);
    } });
    const activated: number[] = [];
    const pending = synchronizeKnowledge(updates, async activation => {
      activated.push(verifyKnowledgeActivation(Buffer.from(JSON.stringify(activation)), f.key.publicKey).identity.sequence);
    }, () => true);
    await ready;
    await new KnowledgeCache(f.options).commit(f.key.proof(3).verified);
    release(); await pending;
    assert.deepEqual(activated, [1, 3]);
    assert.equal(updates.status().version, "3");
    if (outcome === "offline") {
      assert.match(updates.status().lastError ?? "", /offline/);
      assert.equal(updates.status().lastSuccessfulUpdate, null);
    } else assert.equal(updates.status().lastError, null);
  });
}
test("reconciliation rejects equal-sequence different durable identity", async t => {
  const f = await setup(t);
  const { KnowledgeCache } = await import("../electron/knowledgeCache.ts");
  const alternate = f.key.proof(1, "alternate").verified;
  const cache = new KnowledgeCache(f.options);
  const updates = new KnowledgeUpdates({ ...f.options, fetcher: async () => {
    await (await import("node:fs/promises")).mkdir(cache.directory, { recursive: true });
    await writeFile(join(cache.directory, "active.json"), JSON.stringify(alternate.activation));
    return new Response(f.key.proof(1).manifest);
  } });
  assert.equal((await updates.check())?.identity.bundleSha256, f.starter.identity.bundleSha256);
  assert.match(updates.status().lastError ?? "", /equivocation/i);
  assert.equal(updates.status().lastSuccessfulUpdate, null);
});

test("failed feed reconciliation retains verified memory when all durable candidates disappear", async t => {
  const f = await setup(t);
  const updates = new KnowledgeUpdates({ ...f.options, fetcher: async () => {
    await rm(f.options.starterPath);
    throw new Error("offline after starter removal");
  } });
  assert.equal((await updates.check())?.identity.sequence, 1);
  assert.equal(updates.status().version, "1");
  assert.match(updates.status().lastError ?? "", /offline after starter removal/);
});
for (const cutoff of ["before", "after"] as const) {
  test(`cancellation ${cutoff} native replacement dispatch honors the atomic publication cutoff`, async t => {
    if (process.platform === "win32") return t.skip("POSIX replacement interception; Windows uses File.Replace");
    const f = await setup(t);
    const { KnowledgeCache } = await import("../electron/knowledgeCache.ts");
    const cache = new KnowledgeCache(f.options);
    const activePath = join(cache.directory, "active.json");
    const originalStat = fsPromises.stat;
    const originalRename = fsPromises.rename;
    const updates = new KnowledgeUpdates(f.options);
    if (cutoff === "before") {
      fsPromises.stat = (async (...args: Parameters<typeof fsPromises.stat>) => {
        try { return await originalStat(...args); }
        finally { if (String(args[0]) === activePath) updates.setEnabled(false); }
      }) as typeof fsPromises.stat;
    } else {
      fsPromises.rename = async (source, target) => {
        await originalRename(source, target);
        if (String(target) === activePath) updates.setEnabled(false);
      };
    }
    syncBuiltinESMExports();
    t.after(() => { fsPromises.stat = originalStat; fsPromises.rename = originalRename; syncBuiltinESMExports(); });
    assert.equal((await updates.check())?.identity.sequence, cutoff === "before" ? 1 : 2);
    assert.equal((await cache.load())?.identity.sequence, cutoff === "before" ? 1 : 2);
    assert.equal(updates.status().lastSuccessfulUpdate, null);
    assert.match(updates.status().lastError ?? "", /cancel/i);
  });
}
