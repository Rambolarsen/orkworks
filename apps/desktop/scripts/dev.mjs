import { spawn } from "child_process";
import { createServer } from "vite";
import { resolve } from "path";
import { createViteServerOptions, electronSpawnConfig } from "./devConfig.mjs";

const root = resolve(import.meta.dirname, "..");

async function main() {
  const server = await createServer(createViteServerOptions(root));

  await server.listen();

  const urls = server.resolvedUrls;
  const url = urls?.local?.[0] ?? "http://localhost:5173";
  console.log(`[dev] vite dev server at ${url}`);

  const electronConfig = electronSpawnConfig(root, url);
  const electron = spawn(electronConfig.command, electronConfig.args, electronConfig.options);

  electron.on("exit", (code) => {
    server.close();
    process.exit(code ?? 0);
  });
}

main();
