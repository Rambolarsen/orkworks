import test from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL('../scripts/preview-workflow.mjs', import.meta.url));
function launch(port = '0') {
  const child = spawn(process.execPath, [script], { env: { ...process.env, ORKWORKS_WORKFLOW_PREVIEW_PORT: port }, stdio: ['ignore','pipe','pipe'] });
  let output = ''; let errors = '';
  child.stdout.on('data', chunk => { output += chunk; });
  child.stderr.on('data', chunk => { errors += chunk; });
  const exited = once(child, 'exit');
  return { child, exited, errors: () => errors, url: async () => {
    const deadline = Date.now() + 15000;
    while (Date.now() < deadline) {
      const match = output.match(/http:\/\/127\.0\.0\.1:\d+/);
      if (match) return match[0];
      if (child.exitCode !== null) throw new Error(errors);
      await new Promise(resolve => setTimeout(resolve, 20));
    }
    throw new Error('Preview did not start: ' + errors);
  } };
}
test('preview serves only generated artifacts on loopback and refuses occupied or invalid ports', async () => {
  const server = launch();
  try {
    const url = await server.url();
    const response = await fetch(url);
    assert.equal(response.status,200);
    assert.match(response.headers.get('content-security-policy'), /connect-src 'none'/);
    assert.match(await response.text(), /preview.js/);
    const js = await fetch(url + '/preview.js'); assert.equal(js.status,200);
    assert.match(await js.text(), /Synthetic workflow preview/);
    assert.equal((await fetch(url+'/preview.css')).status,200);
    assert.equal((await fetch(url+'/src/App.tsx')).status,404);
    assert.equal((await fetch(url+'/package.json')).status,404);
    assert.equal((await fetch(url+'/', {method:'POST'})).status,405);
    const conflict = launch(new URL(url).port);
    try { const [code] = await conflict.exited; assert.notEqual(code,0); assert.match(conflict.errors(), /EADDRINUSE/); }
    finally { conflict.child.kill(); }
  } finally { server.child.kill('SIGTERM'); await server.exited; }
  const invalid = launch('not-a-port');
  const [code] = await invalid.exited; assert.notEqual(code,0); assert.match(invalid.errors(),/port/i);
});
