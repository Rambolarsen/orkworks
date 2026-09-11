import { test } from "node:test";
import assert from "node:assert/strict";
import { approveInferenceAdapter, readInferenceTrust, revokeInferenceAdapter, type InferenceAdapterView, type TrustContext } from "../electron/inferenceTrust.ts";
import { inferenceTrustActions } from "../src/inferenceTrust.ts";

const revision = { documentRevision: "a".repeat(64), generation: "9007199254740993", digest: "b".repeat(64) };
const adapter: InferenceAdapterView = { id: "custom", name: "Custom", state: "approval_required", resolvedPath: "/tools/adapter", revision,
  definition: { kind: "command", command: "adapter", args: ["--model", "{model}"], input: "stdin", output: "result-json-v1", timeoutSecs: 60 } };
const request = { harnessId: "custom", expectedRevision: revision };
test("Settings distinguishes executable approval from unavailable and busy states", () => {
  assert.deepEqual(inferenceTrustActions(adapter, false), { canApprove: true, canRevoke: true });
  assert.deepEqual(inferenceTrustActions({ ...adapter, state: "approved" }, false), { canApprove: false, canRevoke: true });
  assert.deepEqual(inferenceTrustActions({ ...adapter, state: "unavailable", resolvedPath: null }, false), { canApprove: false, canRevoke: true });
  assert.deepEqual(inferenceTrustActions(adapter, true), { canApprove: false, canRevoke: false });
});
function fixture(confirm: TrustContext["confirm"] = async () => true) {
  const calls: Array<{ url: string; init?: RequestInit }> = [];
  const context: TrustContext = { port: 4321, token: "test-authority", isCurrent: () => true, confirm,
    fetcher: async (url, init) => { calls.push({ url, init }); return new Response(JSON.stringify(init?.method === "POST" ? { ok: true } : { adapters: [adapter] })); } };
  return { context, calls };
}
test("approval uses fixed privileged endpoint and exact reviewed revision after native confirmation", async () => {
  let detail = "";
  const { context, calls } = fixture(async (text) => { detail = text; return true; });
  assert.equal(await approveInferenceAdapter(request, context), true);
  assert.equal(calls.length, 2);
  assert.ok(calls.every((call) => call.url === "http://127.0.0.1:4321/settings/taskmaster/inference"));
  assert.equal((calls[1].init?.headers as Record<string, string>)["x-orkworks-open-plan-token"], "test-authority");
  assert.deepEqual(JSON.parse(calls[1].init?.body as string), { ...request, action: "approve" });
  assert.match(detail, /\/tools\/adapter/);
  assert.match(detail, /credential/);
  assert.match(detail, /not sandboxed/);
  assert.match(detail, /hooks or plugins/);
  assert.equal((await readInferenceTrust(context))[0].revision.generation, "9007199254740993");
});
test("declining approval or changing sidecar during confirmation cannot post a grant", async () => {
  const declined = fixture(async () => false);
  assert.equal(await approveInferenceAdapter(request, declined.context), false);
  assert.equal(declined.calls.filter((call) => call.init?.method === "POST").length, 0);
  let current = true;
  const changed = fixture(async () => { current = false; return true; });
  changed.context.isCurrent = () => current;
  await assert.rejects(approveInferenceAdapter(request, changed.context), /changed/);
  assert.equal(changed.calls.filter((call) => call.init?.method === "POST").length, 0);
});
test("malformed IPC input is rejected before any network request", async () => {
  for (const raw of [null, { ...request, url: "https://example.invalid" }, { ...request, expectedRevision: { ...revision, generation: 1 } }, { ...request, expectedRevision: { ...revision, digest: null } }]) {
    const { context, calls } = fixture();
    await assert.rejects(approveInferenceAdapter(raw, context), /Invalid/);
    assert.equal(calls.length, 0);
  }
});
test("stale adapter revision prevents confirmation and revocation needs no executable", async () => {
  let confirmations = 0;
  const { context, calls } = fixture(async () => { confirmations += 1; return true; });
  await assert.rejects(approveInferenceAdapter({ ...request, expectedRevision: { ...revision, digest: "c".repeat(64) } }, context), /changed/);
  assert.equal(confirmations, 0);
  await revokeInferenceAdapter({ ...request, expectedRevision: { ...revision, digest: null } }, context);
  assert.equal(JSON.parse(calls.at(-1)?.init?.body as string).action, "revoke");
});
