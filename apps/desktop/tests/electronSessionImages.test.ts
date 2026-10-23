import test from "node:test";
import assert from "node:assert/strict";
import { normalizeImageCreation, submitImageSession } from "../electron/sessionImages.ts";
const png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";
const request = () => ({harnessId:"codex", imageWorkspacePath:"/tmp/work", initialPrompt:"inspect", initialImages:[{name:"a.png",mimeType:"image/png",dataBase64:png}]});
test("main rejects oversized, malformed, and mismatched image payloads", () => {
  assert.equal(normalizeImageCreation(request()).initialImages.length, 1);
  assert.throws(() => normalizeImageCreation({...request(), initialImages: Array(9).fill(request().initialImages[0])}));
  assert.throws(() => normalizeImageCreation({...request(), initialImages:[{...request().initialImages[0],dataBase64:"not base64"}]}));
  assert.throws(() => normalizeImageCreation({...request(), initialImages:[{...request().initialImages[0],mimeType:"text/html"}]}));
  assert.throws(() => normalizeImageCreation({...request(), initialPrompt:"x".repeat(65537)}));
});
test("old sidecars cannot silently discard images", async () => {
  let posted = false;
  const fetchImpl = async (_url: string, init?: RequestInit) => {
    if (init?.method === "POST") posted = true;
    return new Response("ok", {status:404});
  };
  await assert.rejects(() => submitImageSession(1234,"secret",normalizeImageCreation(request()),undefined,fetchImpl), /image context/i);
  assert.equal(posted, false);
});
test("main negotiates support then sends only bounded creation data with authentication", async () => {
  const calls: Array<{url:string; init?:RequestInit}> = [];
  const fetchImpl = async (url:string, init?:RequestInit) => {
    calls.push({url,init});
    return new Response(JSON.stringify(init?.method === "POST" ? {id:"created"} : {version:1,supportedHarnesses:["codex"]}), {status:200});
  };
  assert.deepEqual(await submitImageSession(1234,"secret",normalizeImageCreation(request()),undefined,fetchImpl), {id:"created"});
  assert.equal(calls[1].url,"http://127.0.0.1:1234/sessions");
  assert.equal((calls[1].init?.headers as Record<string,string>)["x-orkworks-open-plan-token"],"secret");
  assert.equal(JSON.parse(calls[1].init?.body as string).expectedWorkspacePath,"/tmp/work");
});
