import assert from "node:assert/strict";
import { test } from "node:test";
import { generateKeyPairSync, createHash, sign } from "node:crypto";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { KnowledgeUpdates, synchronizeKnowledge, validateKnowledgeBundle } from "../electron/knowledgeUpdates.ts";

const keys = generateKeyPairSync("ed25519");
const publicKey = keys.publicKey.export({ type: "spki", format: "pem" }).toString();
function bundle(version: string) {
  const content = "Document the command that verifies a change and its applicable scope.";
  return { formatVersion: 1, version, sequence: Number(version), publishedAt: "2026-09-09T00:00:00Z", pages: [{
    id: "concepts/verification.md", title: "Verification", type: "concept", status: "active",
    content, sha256: createHash("sha256").update(content).digest("hex"), relatedIds: [] as string[],
  }] };
}
function signed(value: unknown) {
  const payload = JSON.stringify(value);
  return JSON.stringify({ payload, signature: sign(null, Buffer.from(payload), keys.privateKey).toString("base64") });
}
async function setup() {
  const directory = await mkdtemp(join(tmpdir(), "orkworks-knowledge-test-"));
  const starterPath = join(directory, "starter.json");
  await writeFile(starterPath, JSON.stringify(bundle("1")));
  const remote = signed(bundle("2"));
  const manifest = signed({ formatVersion: 1, bundles: [{ formatVersion: 1, sequence: 2, version: "2", path: "bundles/2.json", sha256: createHash("sha256").update(remote).digest("hex") }] });
  const calls: string[] = [];
  const fetcher = async (url: string) => { calls.push(url); return new Response(url.endsWith("manifest.json") ? manifest : remote); };
  const options = { directory: join(directory, "cache"), starterPath, publicKey, feedUrl: "https://example.org/knowledge/manifest.json", fetcher, now: () => 1_800_000_000_000 };
  return { directory, options, calls };
}

test("a rejected older local snapshot still checks and activates the signed feed", async () => {
  const fixture = await setup();
  try {
    const activated: string[] = [];
    await synchronizeKnowledge(new KnowledgeUpdates(fixture.options), async (value) => {
      if (value.version === "1") throw new Error("knowledge bundle sequence is older than the active bundle");
      activated.push(value.version);
    }, () => true);
    assert.deepEqual(activated, ["2"]);
    assert.equal(fixture.calls.length, 2);
  } finally { await rm(fixture.directory, { recursive: true, force: true }); }
});

test("knowledge synchronization propagates other activation failures", async () => {
  const fixture = await setup();
  try {
    await assert.rejects(synchronizeKnowledge(new KnowledgeUpdates(fixture.options), async () => {
      throw new Error("unauthorized");
    }, () => true), /unauthorized/);
    assert.deepEqual(fixture.calls, []);
  } finally { await rm(fixture.directory, { recursive: true, force: true }); }
});

test("knowledge synchronization stops at every stale workspace boundary", async () => {
  for (const stage of ["load", "activate", "check"]) {
    let current = true;
    let checks = 0;
    const activated: string[] = [];
    await synchronizeKnowledge({
      load: async () => { if (stage === "load") current = false; return bundle("1"); },
      check: async () => { checks++; if (stage === "check") current = false; return bundle("2"); },
    }, async (value) => {
      activated.push(value.version);
      if (stage === "activate") current = false;
    }, () => current);
    assert.deepEqual(activated, stage === "load" ? [] : ["1"], stage);
    assert.equal(checks, stage === "check" ? 1 : 0, stage);
  }
});

test("verified update survives restart and unchanged checks respect the interval", async () => {
  const fixture = await setup();
  try {
    const updates = new KnowledgeUpdates(fixture.options);
    assert.equal((await updates.load()).version, "1");
    assert.equal((await updates.check()).version, "2");
    assert.equal((await new KnowledgeUpdates(fixture.options).load()).version, "2");
    await updates.check();
    assert.equal(fixture.calls.length, 2);
    assert.equal(updates.status().lastError, null);
  } finally { await rm(fixture.directory, { recursive: true, force: true }); }
});

test("a tampered signed response retains the starter and reports failure", async () => {
  const fixture = await setup();
  try {
    const updates = new KnowledgeUpdates({ ...fixture.options, fetcher: async () => new Response(signed({ formatVersion: 1, bundles: [] }).replace('"signature":"', '"signature":"AAAA')) });
    assert.equal((await updates.check()).version, "1");
    assert.match(updates.status().lastError ?? "", /signature/i);
  } finally { await rm(fixture.directory, { recursive: true, force: true }); }
});

test("offline startup preserves a previously verified snapshot", async () => {
  const fixture = await setup();
  try {
    await new KnowledgeUpdates(fixture.options).check();
    const updates = new KnowledgeUpdates({ ...fixture.options, now: () => fixture.options.now() + 7 * 3_600_000, fetcher: async () => { throw new Error("offline"); } });
    assert.equal((await updates.check()).version, "2");
    assert.match(updates.status().lastError ?? "", /offline/);
  } finally { await rm(fixture.directory, { recursive: true, force: true }); }
});

test("incompatible newer releases do not displace a compatible cached snapshot", async () => {
  const fixture = await setup();
  try {
    const updates = new KnowledgeUpdates({ ...fixture.options, fetcher: async () => new Response(signed({ formatVersion: 1, bundles: [{ formatVersion: 2, sequence: 3, version: "3", path: "bundles/3.json", sha256: "a".repeat(64) }] })) });
    assert.equal((await updates.check()).version, "1");
  } finally { await rm(fixture.directory, { recursive: true, force: true }); }
});

test("corrupt active cache falls back to previous verified snapshot", async () => {
  const fixture = await setup();
  try {
    await new KnowledgeUpdates(fixture.options).check();
    const activePath = join(fixture.options.directory, "active.json");
    const active = await readFile(activePath, "utf8");
    await writeFile(join(fixture.options.directory, "previous.json"), active);
    await writeFile(activePath, "broken");
    assert.equal((await new KnowledgeUpdates(fixture.options).load()).version, "2");
  } finally { await rm(fixture.directory, { recursive: true, force: true }); }
});

test("content limits measure UTF-8 bytes rather than characters", () => {
  const value = bundle("1");
  value.pages[0].content = "é".repeat(40_000);
  value.pages[0].sha256 = createHash("sha256").update(value.pages[0].content).digest("hex");
  assert.throws(() => validateKnowledgeBundle(value), /Invalid knowledge page/);
});

test("knowledge page IDs preserve safe Markdown names and relationships", () => {
  for (const id of ["concepts/Review notes.v2.md", "概念/検証.md", ".guides/review.md", "x".repeat(253) + ".md"]) {
    const value = bundle("1");
    value.pages[0].id = id;
    value.pages.push({ ...value.pages[0], id: "index.md", relatedIds: [id] });
    const parsed = validateKnowledgeBundle(value);
    assert.equal(parsed.pages[0].id, id);
    assert.deepEqual(parsed.pages[1].relatedIds, [id]);
  }
});

test("knowledge page IDs reject unsafe paths and exceedance of the byte limit", () => {
  for (const id of ["/guide.md", "../guide.md", "a/../guide.md", "./guide.md", "a//guide.md",
    "C:/guide.md", "C:guide.md", "a\\guide.md", "\\\\server\\guide.md", "https://host/guide.md",
    "guide.txt", "guide.md\n", "a\u0000.md", "a\u001b.md", "a\u007f.md", "a\u0085.md",
    "x".repeat(254) + ".md", "é".repeat(127) + ".md"]) {
    const value = bundle("1");
    value.pages[0].id = id;
    assert.throws(() => validateKnowledgeBundle(value), /Invalid knowledge page/, id);
  }
});

test("disabled updates load the starter without making network requests", async () => {
  const fixture = await setup();
  try {
    const updates = new KnowledgeUpdates(fixture.options);
    updates.setEnabled(false);
    assert.equal((await updates.check()).version, "1");
    assert.equal(fixture.calls.length, 0);
  } finally { await rm(fixture.directory, { recursive: true, force: true }); }
});

test("disabling updates cancels a pending download without replacing knowledge", async () => {
  const fixture = await setup();
  try {
    let started!: () => void;
    const ready = new Promise<void>((resolve) => { started = resolve; });
    let signal: AbortSignal | undefined;
    const updates = new KnowledgeUpdates({ ...fixture.options, fetcher: async (_url, init) => {
      signal = init?.signal as AbortSignal;
      started();
      return new Promise<Response>((_resolve, reject) => {
        signal!.addEventListener("abort", () => reject(new Error("cancelled")), { once: true });
      });
    } });
    const pending = updates.check();
    await ready;
    updates.setEnabled(false);
    assert.equal((await pending).version, "1");
    assert.equal(signal?.aborted, true);
  } finally { await rm(fixture.directory, { recursive: true, force: true }); }
});
