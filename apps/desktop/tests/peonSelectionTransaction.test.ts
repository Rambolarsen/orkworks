import test from "node:test";
import assert from "node:assert/strict";
import {
  createPeonSelectionTransaction,
  peonErrorFromBody,
  StaleGenerationError,
  type PeonSelectionTransport,
  type PeonVerificationRequest,
} from "../electron/peonSelectionTransaction.ts";

const verificationResponse = (generation: number) => ({
  ok: true,
  provider: "opencode" as const,
  capabilities: { connectivity: true, modelDiscovery: true, providerDefault: true, testInference: true },
  models: ["some-model"],
  ollamaBaseUrl: null,
  generation,
});

function transportWithVerify(
  verifyImpl: (request: PeonVerificationRequest) => Promise<unknown>,
): PeonSelectionTransport {
  return {
    verify: async (request) => (await verifyImpl(request)) as Awaited<ReturnType<PeonSelectionTransport["verify"]>>,
    discover: async () => [],
    apply: async () => {
      throw new Error("apply should not be called");
    },
    getApplied: async () => {
      throw new Error("getApplied should not be called");
    },
  };
}

test("verify retries once with a resynced generation after a stale-generation rejection", async () => {
  const requestedGenerations: number[] = [];
  const transaction = createPeonSelectionTransaction(
    transportWithVerify(async (request) => {
      requestedGenerations.push(request.generation);
      if (requestedGenerations.length === 1) {
        throw new StaleGenerationError("stale", 999001);
      }
      return verificationResponse(request.generation);
    }),
  );

  const result = await transaction.verify("opencode");

  assert.deepEqual(requestedGenerations, [1, 999002]);
  assert.equal(result.generation, 999002);
});

test("verify does not retry for non-stale errors", async () => {
  const requestedGenerations: number[] = [];
  const transaction = createPeonSelectionTransaction(
    transportWithVerify(async (request) => {
      requestedGenerations.push(request.generation);
      throw new Error("provider failed");
    }),
  );

  await assert.rejects(() => transaction.verify("opencode"), /provider failed/);
  assert.deepEqual(requestedGenerations, [1]);
});

test("verify surfaces the error when the retry is also rejected", async () => {
  const requestedGenerations: number[] = [];
  const transaction = createPeonSelectionTransaction(
    transportWithVerify(async (request) => {
      requestedGenerations.push(request.generation);
      throw new StaleGenerationError("still stale", 999001);
    }),
  );

  await assert.rejects(() => transaction.verify("opencode"), /still stale/);
  assert.equal(requestedGenerations.length, 2);
});

test("verify does not resync or retry when superseded by a newer local verification", async () => {
  let releaseFirst!: () => void;
  const gate = new Promise<void>((resolve) => { releaseFirst = resolve; });
  const requestedGenerations: number[] = [];
  const transaction = createPeonSelectionTransaction(
    transportWithVerify(async (request) => {
      requestedGenerations.push(request.generation);
      if (request.generation === 1) {
        await gate;
        throw new StaleGenerationError("stale", 5);
      }
      return verificationResponse(request.generation);
    }),
  );

  const first = transaction.verify("opencode");
  const second = await transaction.verify("opencode");
  releaseFirst();

  await assert.rejects(() => first, /stale/);
  assert.equal(second.generation, 2);
  assert.deepEqual(requestedGenerations, [1, 2]);
});

test("verify does not retry a stale error without a usable remote generation", async () => {
  const requestedGenerations: number[] = [];
  const transaction = createPeonSelectionTransaction(
    transportWithVerify(async (request) => {
      requestedGenerations.push(request.generation);
      throw new StaleGenerationError("stale", null);
    }),
  );

  await assert.rejects(() => transaction.verify("opencode"), /stale/);
  assert.deepEqual(requestedGenerations, [1]);
});

test("peonErrorFromBody builds a StaleGenerationError from a stale sidecar error body", () => {
  const error = peonErrorFromBody(
    { error: { code: "stale_generation", message: "boom" }, currentGeneration: 999001 },
    "fallback",
  );

  assert.ok(error instanceof StaleGenerationError);
  assert.equal((error as StaleGenerationError).currentGeneration, 999001);
  assert.equal(error.message, "boom");
});

test("peonErrorFromBody builds a plain error for non-stale bodies", () => {
  const error = peonErrorFromBody(
    { error: { code: "provider_failure", message: "boom" }, currentGeneration: 999001 },
    "fallback",
  );

  assert.ok(!(error instanceof StaleGenerationError));
  assert.equal(error.message, "boom");
});

test("peonErrorFromBody handles string errors and missing fields", () => {
  assert.equal(peonErrorFromBody({ error: "plain" }, "fallback").message, "plain");
  assert.equal(peonErrorFromBody({}, "fallback").message, "fallback");
});

test("peonErrorFromBody treats an unsafe-integer currentGeneration as absent", () => {
  const error = peonErrorFromBody(
    { error: { code: "stale_generation", message: "boom" }, currentGeneration: 99999999999999999999 },
    "fallback",
  );

  assert.ok(error instanceof StaleGenerationError);
  assert.equal((error as StaleGenerationError).currentGeneration, null);
});
