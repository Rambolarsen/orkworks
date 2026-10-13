import fsExt from "fs-ext";
import { openSync, closeSync } from "node:fs";
import fs from "node:fs/promises";
import { syncBuiltinESMExports } from "node:module";
import { join } from "node:path";
import { KnowledgeCache } from "../electron/knowledgeCache.ts";
import { verifyKnowledgeActivation } from "../electron/knowledgeProof.ts";
process.once("message", async (message: { action: string; options: { directory: string; publicKey: string; starterPath: string }; candidatePath?: string }) => {
  try {
    const cache = new KnowledgeCache(message.options);
    if (message.action === "hold") {
      await fs.mkdir(cache.directory, { recursive: true });
      const fd = openSync(join(cache.directory, ".knowledge.lock"), "a+", 0o600);
      fsExt.flockSync(fd, "exnb");
      process.send?.({ ready: true });
      process.once("message", () => { closeSync(fd); process.exit(0); });
      return;
    }
    const candidate = verifyKnowledgeActivation(await fs.readFile(message.candidatePath!), message.options.publicKey);
    if (message.action === "slow") {
      await cache.load();
      process.send?.({ ready: true });
      await new Promise<void>(resolve => process.once("message", () => resolve()));
    }
    if (message.action === "crash") {
      // Stop at the real atomic replacement boundary after previous was flushed.
      const original = fs.rename;
      fs.rename = async (source, target) => {
        if (String(target) === join(cache.directory, "active.json")) process.exit(23);
        return original(source, target);
      };
      syncBuiltinESMExports();
    }
    const selected = await cache.commit(candidate);
    process.send?.({ sequence: selected.identity.sequence });
    process.disconnect();
  } catch (error) {
    process.send?.({ error: (error as Error).message });
    process.disconnect();
  }
});
