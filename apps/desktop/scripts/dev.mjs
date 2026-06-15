import { spawn } from "child_process";
import { createServer } from "vite";
import { resolve } from "path";

const root = resolve(import.meta.dirname, "..");

async function main() {
  const server = await createServer({
    configFile: resolve(root, "vite.config.ts"),
    root,
  });

  await server.listen();

  const urls = server.resolvedUrls;
  const url = urls?.local?.[0] ?? "http://localhost:5173";
  console.log(`[dev] vite dev server at ${url}`);

  const electron = spawn("npx", ["electron", "."], {
    cwd: root,
    env: { ...process.env, VITE_DEV_SERVER_URL: url },
    stdio: "inherit",
  });

  electron.on("exit", (code) => {
    server.close();
    process.exit(code ?? 0);
  });
}

main();
