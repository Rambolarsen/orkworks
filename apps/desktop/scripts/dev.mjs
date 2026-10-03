import { spawn } from "child_process";
import { createServer } from "vite";
import { resolve } from "path";
import { createViteServerOptions, electronSpawnConfig } from "./devConfig.mjs";

import { prepareMacDevBundle } from "./macDevBundle.mjs";

const root = resolve(import.meta.dirname, "..");

async function main() {
  const executable = prepareMacDevBundle(root);
  const server = await createServer(createViteServerOptions(root));

  await server.listen();

  const urls = server.resolvedUrls;
  const url = urls?.local?.[0] ?? "http://localhost:5173";
  console.log(`[dev] vite dev server at ${url}`);

  const electronConfig = electronSpawnConfig(root, url, executable);
  const electron = spawn(electronConfig.command, electronConfig.args, electronConfig.options);

  electron.on("exit", (code) => {
    server.close();
    process.exit(code ?? 0);
  });
}

main();
