import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
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
