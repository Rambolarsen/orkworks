import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtemp, writeFile, readFile, rm, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { fork } from "node:child_process";
import { once } from "node:events";
import { tmpdir } from "node:os";
import { KnowledgeCache } from "../electron/knowledgeCache.ts";
import { signer } from "./knowledgeTestFixtures.ts";

async function setup(t: import("node:test").TestContext) {
  const directory = await mkdtemp(join(tmpdir(), "knowledge-cache-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const key = signer();
  const starter = key.proof(1).verified;
  const starterPath = join(directory, "starter.json");
  await writeFile(starterPath, JSON.stringify(starter.activation));
  const options = { directory, starterPath, publicKey: key.publicKey };
  const cache = new KnowledgeCache(options);
  return { directory, key, starter, starterPath, options, cache, namespace: join(directory, "keys", starter.identity.publicKeySha256) };
}
test("signed offline starter ignores unproven legacy sequence poisoning", async t => {
  const f = await setup(t);
  await writeFile(join(f.directory, "active.json"), JSON.stringify({ sequence: Number.MAX_SAFE_INTEGER }));
  assert.equal((await f.cache.load())?.identity.sequence, 1);
  assert.equal(JSON.parse(await readFile(join(f.directory, "active.json"), "utf8")).sequence, Number.MAX_SAFE_INTEGER);
});
test("standalone signed cache is ineligible and corrupt active falls back independently", async t => {
  const f = await setup(t);
  await mkdir(f.namespace, { recursive: true });
  await writeFile(join(f.namespace, "active.json"), f.key.proof(99).bundle);
  assert.equal((await f.cache.load())?.identity.sequence, 1);
  await f.cache.commit(f.key.proof(2).verified);
  await f.cache.commit(f.key.proof(3).verified);
  await writeFile(join(f.namespace, "active.json"), "tampered");
  assert.equal((await f.cache.load())?.identity.sequence, 2);
});
test("slower writer adopts newer durable proof and equivocation rejects", async t => {
  const f = await setup(t);
  const slow = new KnowledgeCache(f.options);
  await slow.load();
  await f.cache.commit(f.key.proof(3).verified);
  assert.equal((await slow.commit(f.key.proof(2).verified)).identity.sequence, 3);
  await assert.rejects(slow.commit(f.key.proof(3, "different").verified), /equivocation/i);
  assert.equal((await f.cache.load())?.identity.sequence, 3);
});
test("key namespaces preserve each other's active previous status and lock", async t => {
  const f = await setup(t);
  await f.cache.commit(f.key.proof(2).verified);
  const original = await readFile(join(f.namespace, "active.json"));
  const key = signer();
  const starterPath = join(f.directory, "other-starter.json");
  await writeFile(starterPath, JSON.stringify(key.proof(1).verified.activation));
  const other = new KnowledgeCache({ directory: f.directory, starterPath, publicKey: key.publicKey });
  await other.commit(key.proof(3).verified);
  assert.deepEqual(await readFile(join(f.namespace, "active.json")), original);
  assert.equal((await f.cache.load())?.identity.sequence, 2);
  assert.equal((await other.load())?.identity.sequence, 3);
});
test("unsigned starter cannot establish eligibility and oversized cache falls back", async t => {
  const f = await setup(t);
  await mkdir(f.namespace, { recursive: true });
  await writeFile(join(f.namespace, "active.json"), Buffer.alloc(6 * 1024 * 1024 + 1, 32));
  assert.equal((await f.cache.load())?.identity.sequence, 1);
  await writeFile(f.starterPath, JSON.stringify({ sequence: 999 }));
  assert.equal(await f.cache.load(), null);
});

function worker(t: import("node:test").TestContext) {
  const child = fork(join(import.meta.dirname, "knowledgeCacheWorker.ts"), [], { execArgv: ["--experimental-strip-types"], stdio: ["ignore", "ignore", "pipe", "ipc"] });
  t.after(() => { if (child.exitCode === null) child.kill(); });
  child.stderr?.resume();
  const message = () => once(child, "message").then(([value]) => value as { ready?: boolean; sequence?: number; error?: string });
  return { child, message };
}
test("two processes serialize writers and slower one adopts the durable newer sequence", async t => {
  const f = await setup(t);
  const candidatePath = join(f.directory, "candidate.json");
  await writeFile(candidatePath, JSON.stringify(f.key.proof(2).verified.activation));
  const slow = worker(t);
  const ready = slow.message();
  slow.child.send({ action: "slow", options: f.options, candidatePath });
  assert.equal((await ready).ready, true);
  await f.cache.commit(f.key.proof(3).verified);
  const done = slow.message(); slow.child.send({ continue: true });
  assert.equal((await done).sequence, 3);
  assert.equal((await f.cache.load())?.identity.sequence, 3);
});
test("retained lock contention is bounded and preserves the signed starter", async t => {
  const f = await setup(t);
  const holder = worker(t);
  const ready = holder.message(); holder.child.send({ action: "hold", options: f.options });
  await ready;
  const started = Date.now();
  await assert.rejects(f.cache.commit(f.key.proof(2).verified), /lock timed out/i);
  assert.ok(Date.now() - started < 3000);
  assert.equal((await f.cache.load())?.identity.sequence, 1);
  const inode = (await import("node:fs/promises")).stat;
  const before = (await inode(join(f.namespace, ".knowledge.lock"))).ino;
  const exited = once(holder.child, "exit"); holder.child.send({ release: true }); await exited;
  await f.cache.commit(f.key.proof(2).verified);
  assert.equal((await inode(join(f.namespace, ".knowledge.lock"))).ino, before);
});
test("a process crash after previous replacement preserves verified recovery and releases lock", async t => {
  if (process.platform === "win32") return t.skip("POSIX rename interception; Windows native replacement requires Windows runner");
  const f = await setup(t);
  await f.cache.commit(f.key.proof(2).verified);
  const candidatePath = join(f.directory, "candidate.json");
  await writeFile(candidatePath, JSON.stringify(f.key.proof(3).verified.activation));
  const crash = worker(t);
  const exited = once(crash.child, "exit"); crash.child.send({ action: "crash", options: f.options, candidatePath });
  assert.equal((await exited)[0], 23);
  assert.equal((await f.cache.load())?.identity.sequence, 2);
  await writeFile(join(f.namespace, "active.json"), "damaged");
  assert.equal((await f.cache.load())?.identity.sequence, 2);
  assert.equal((await f.cache.commit(f.key.proof(3).verified)).identity.sequence, 3);
});
test("concurrent key-pinned processes preserve independent proof and advisory state", async t => {
  const f = await setup(t);
  const second = signer();
  const starterPath = join(f.directory, "second-starter.json");
  await writeFile(starterPath, JSON.stringify(second.proof(1).verified.activation));
  const options = { ...f.options, publicKey: second.publicKey, starterPath };
  const secondCache = new KnowledgeCache(options);
  await f.cache.commit(f.key.proof(2).verified);
  await secondCache.commit(second.proof(2).verified);
  await writeFile(join(f.namespace, "status.json"), "first advisory status");
  await writeFile(join(secondCache.directory, "status.json"), "second advisory status");
  const before = await Promise.all(["previous.json", "status.json", ".knowledge.lock"].map(name => readFile(join(f.namespace, name))));
  const candidatePath = join(f.directory, "second-candidate.json");
  await writeFile(candidatePath, JSON.stringify(second.proof(3).verified.activation));
  const other = worker(t);
  const done = other.message(); other.child.send({ action: "commit", options, candidatePath });
  await f.cache.commit(f.key.proof(3).verified);
  assert.equal((await done).sequence, 3);
  assert.equal((await f.cache.load())?.identity.sequence, 3);
  assert.equal((await secondCache.load())?.identity.sequence, 3);
  assert.equal(await readFile(join(f.namespace, "status.json"), "utf8"), before[1].toString());
  assert.equal(await readFile(join(secondCache.directory, "status.json"), "utf8"), "second advisory status");
  // A held old-key lock must not block a new-key writer, or be replaced by it.
  const holder = worker(t); const ready = holder.message(); holder.child.send({ action: "hold", options: f.options }); await ready;
  const names = ["active.json", "previous.json", "status.json", ".knowledge.lock"];
  const retained = await Promise.all(names.map(name => readFile(join(f.namespace, name))));
  const fourthPath = join(f.directory, "second-fourth.json");
  await writeFile(fourthPath, JSON.stringify(second.proof(4).verified.activation));
  const newKeyWriter = worker(t); const fourth = newKeyWriter.message();
  newKeyWriter.child.send({ action: "commit", options, candidatePath: fourthPath });
  assert.equal((await fourth).sequence, 4);
  assert.deepEqual(await Promise.all(names.map(name => readFile(join(f.namespace, name)))), retained);
  assert.equal((await f.cache.load())?.identity.sequence, 3);
});
test("unique temporary files permit parallel advisory replacements", async t => {
  const f = await setup(t);
  await mkdir(f.namespace, { recursive: true });
  const { writeKnowledgeFile } = await import("../electron/knowledgeCache.ts");
  const path = join(f.namespace, "status.json");
  await Promise.all(Array.from({ length: 20 }, (_, index) => writeKnowledgeFile(path, JSON.stringify({ index }))));
  assert.ok(Number.isInteger(JSON.parse(await readFile(path, "utf8")).index));
  assert.deepEqual((await (await import("node:fs/promises")).readdir(f.namespace)).filter(name => name.endsWith(".tmp")), []);
});

test("a second process cannot commit equal-sequence different content", async t => {
  const f = await setup(t);
  await f.cache.commit(f.key.proof(2).verified);
  const candidatePath = join(f.directory, "candidate.json");
  await writeFile(candidatePath, JSON.stringify(f.key.proof(2, "alternate").verified.activation));
  const other = worker(t); const done = other.message();
  other.child.send({ action: "commit", options: f.options, candidatePath });
  assert.match((await done).error ?? "", /equivocation/i);
  assert.equal((await f.cache.load())?.bundle.version, "2");
});

test("loading rejects signed same-sequence identities across active previous and starter", async t => {
  const f = await setup(t);
  await mkdir(f.namespace, { recursive: true });
  const first = f.key.proof(1).verified;
  const conflicting = f.key.proof(1, "conflicting").verified;
  await writeFile(join(f.namespace, "active.json"), JSON.stringify(conflicting.activation));
  await assert.rejects(f.cache.load(), /equivocation/i);
  await writeFile(join(f.namespace, "active.json"), JSON.stringify(first.activation));
  assert.equal((await f.cache.load())?.identity.sequence, 1);
  await writeFile(join(f.namespace, "previous.json"), JSON.stringify(conflicting.activation));
  await assert.rejects(f.cache.load(), /equivocation/i);
  // A higher record does not conceal conflicting signed history at a lower sequence.
  await writeFile(join(f.namespace, "active.json"), JSON.stringify(f.key.proof(3).verified.activation));
  await assert.rejects(f.cache.load(), /equivocation/i);
});
