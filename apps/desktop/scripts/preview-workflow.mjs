import { build } from "esbuild";
import { createServer } from "node:http";
import { fileURLToPath } from "node:url";

const rawPort = process.env.ORKWORKS_WORKFLOW_PREVIEW_PORT ?? "5276";
if (!/^\d+$/.test(rawPort) || Number(rawPort) > 65535) {
  console.error("Preview port must be an integer from 0 to 65535.");
  process.exit(1);
}
// In-memory output keeps fixtures out of the production build and serves no source files.
const result = await build({
  entryPoints: [fileURLToPath(new URL("../tests/fixtures/workflowPreview.tsx", import.meta.url))],
  bundle: true, write: false, outdir: "workflow-preview", entryNames: "preview", format: "iife",
  platform: "browser", jsx: "automatic", define: { "process.env.NODE_ENV": '"development"' },
});
const artifacts = new Map(result.outputFiles.map(file => ["/" + file.path.split(/[\\/]/).pop(), file.contents]));
artifacts.set("/", Buffer.from('<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>OrkWorks · Workflow preview</title><link rel="stylesheet" href="/preview.css"></head><body><div id="root"></div><script src="/preview.js"></script></body></html>'));
const server = createServer((request, response) => {
  response.setHeader("Content-Security-Policy", "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'");
  response.setHeader("X-Content-Type-Options", "nosniff");
  response.setHeader("Cache-Control", "no-store");
  if (request.method !== "GET" && request.method !== "HEAD") { response.writeHead(405); response.end(); return; }
  const path = (request.url ?? "/").split("?")[0];
  const body = artifacts.get(path);
  if (!body) { response.writeHead(404); response.end(); return; }
  response.setHeader("Content-Type", path.endsWith(".js") ? "text/javascript; charset=utf-8" : path.endsWith(".css") ? "text/css; charset=utf-8" : "text/html; charset=utf-8");
  response.end(request.method === "HEAD" ? undefined : body);
});
server.on("error", error => { console.error(`Workflow preview: ${error.message}`); process.exitCode = 1; });
server.listen(Number(rawPort), "127.0.0.1", () => {
  console.log(`Synthetic workflow preview: http://127.0.0.1:${server.address().port}`);
});
for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => { server.close(); server.closeAllConnections(); });
