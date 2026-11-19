import test from "node:test";
import assert from "node:assert/strict";
import { build } from "vite";

test("production renderer module graph excludes synthetic review and workflow previews", async () => {
  const modules = new Set();
  await build({ logLevel: "silent", build: { write: false }, plugins: [{
    name: "inspect-production-preview-isolation",
    generateBundle(_options, bundle) {
      for (const value of Object.values(bundle)) {
        if (value.type === "chunk") for (const id of Object.keys(value.modules)) modules.add(id.replaceAll("\\", "/"));
      }
    },
  }] });
  assert.ok([...modules].some(id => id.endsWith("/src/App.tsx")), "check the real production graph");
  assert.deepEqual([...modules].filter(id => id.includes("/tests/fixtures/") || id.includes("/components/review/ReviewHandoffPreview") || id.includes("/components/review/reviewHandoffPresentation")), []);
});
