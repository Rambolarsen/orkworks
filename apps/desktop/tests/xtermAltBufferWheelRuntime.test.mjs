import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

if (process.versions.electron) {
  const { app, BrowserWindow } = await import('electron');
  app.disableHardwareAcceleration();
  app.whenReady().then(async () => {
    const win = new BrowserWindow({ show: true, width: 500, height: 300 });
    try {
      await win.loadFile(process.env.ORKWORKS_XTERM_WHEEL_FIXTURE);
      const evaluate = (source) => win.webContents.executeJavaScript(source);
      const waitFor = async (source) => {
        const deadline = Date.now() + 5000;
        while (Date.now() < deadline) {
          if (await evaluate(source)) return;
          await new Promise(resolve => setTimeout(resolve, 10));
        }
        throw new Error(`Timed out: ${source}`);
      };

      await waitFor('window.wheelFixture?.ready === true');
      const passive = await evaluate('wheelFixture.run(false)');
      assert.equal(passive.active, 'alternate');
      assert.deepEqual(passive.small, ['\u001b[B']);
      assert.deepEqual(passive.large, ['\u001b[B']);
      assert.deepEqual(passive.zero, []);

      const mouse = await evaluate('wheelFixture.run(true)');
      assert.equal(mouse.active, 'alternate');
      assert.equal(mouse.small.length, 1, `small wheel delta should produce a mouse report (${JSON.stringify(mouse)})`);
      assert.match(mouse.small[0], /^\x1b\[(?:M|<)/);
      assert.equal(mouse.large.length, 1, 'large wheel delta should produce a mouse report');
      assert.match(mouse.large[0], /^\x1b\[(?:M|<)/);
      assert.deepEqual(mouse.zero, [], 'zero wheel delta should produce no mouse report');

      const normal = await evaluate('wheelFixture.runNormal()');
      assert.equal(normal.active, 'normal');
      assert.ok(normal.bufferLength > normal.rows, 'normal buffer should contain scrollback');
      assert.deepEqual(normal.small, [], 'normal-buffer mouse tracking should preserve wheel dampening');
      assert.equal(normal.large.length, 1, 'large normal-buffer wheel delta should pass the accumulator');
      assert.match(normal.large[0], /^\x1b\[(?:M|<)/);
    } finally {
      win.destroy();
    }
    app.quit();
  }).catch(error => { console.error(error); app.exit(1); });
} else {
  const { default: test } = await import('node:test');
  const { spawnSync } = await import('node:child_process');
  const { createRequire } = await import('node:module');
  const { mkdtempSync, writeFileSync, rmSync } = await import('node:fs');
  const { tmpdir } = await import('node:os');
  const { join } = await import('node:path');
  const { build } = await import('esbuild');

  test('xterm forwards fractional wheel input in alternate-buffer modes', async () => {
    const require = createRequire(import.meta.url);
    const root = fileURLToPath(new URL('../', import.meta.url));
    const directory = mkdtempSync(join(tmpdir(), 'orkworks-xterm-wheel-'));
    try {
      const bundle = await build({
        stdin: { contents: `
          import { Terminal } from '@xterm/xterm';
          const terminal = new Terminal({ cols: 40, rows: 8, scrollback: 20 });
          terminal.open(document.getElementById('terminal'));
          const outputs = [];
          terminal.onData(data => outputs.push(data));
          const write = data => new Promise(resolve => terminal.write(data, resolve));
          const wheel = deltaY => {
            const element = terminal.element;
            const rect = element.getBoundingClientRect();
            element.dispatchEvent(new WheelEvent('wheel', {
              bubbles: true, cancelable: true, deltaY, clientX: rect.left + 30, clientY: rect.top + 30,
            }));
          };
          const capture = deltaY => {
            outputs.length = 0;
            wheel(deltaY);
            return outputs.splice(0);
          };
          window.wheelFixture = {
            ready: true,
            async run(mouseTracking) {
              terminal.reset();
              await write('\\x1b[?1049h' + (mouseTracking ? '\\x1b[?1000h\\x1b[?1006h' : ''));
              const small = capture(1);
              const large = capture(120);
              const zero = capture(0);
              return { small, large, zero, active: terminal.buffer.active.type };
            },
            async runNormal() {
              terminal.reset();
              const scrollback = Array.from({ length: 16 }, (_, index) => 'line ' + index).join('\\r\\n');
              await write(scrollback + '\\r\\n\\x1b[?1000h\\x1b[?1006h');
              const buffer = terminal.buffer.active;
              const small = capture(1);
              const large = capture(120);
              return { small, large, active: buffer.type, bufferLength: buffer.length, rows: terminal.rows };
            },
          };
        `, resolveDir: root },
        bundle: true, write: false, format: 'iife', platform: 'browser',
      });
      const fixture = join(directory, 'index.html');
      writeFileSync(fixture, '<div id="terminal" style="width:480px;height:240px"></div><script>' + bundle.outputFiles[0].text + '</script>');
      const electron = require('electron');
      const env = { ...process.env, ORKWORKS_XTERM_WHEEL_FIXTURE: fixture };
      delete env.ELECTRON_RUN_AS_NODE;
      const args = [fileURLToPath(import.meta.url)];
      if (process.platform === 'linux') args.unshift('--no-sandbox');
      const headlessLinux = process.platform === 'linux' && !env.DISPLAY;
      if (headlessLinux) args.unshift('-a', electron);
      const result = spawnSync(headlessLinux ? 'xvfb-run' : electron, args,
        { env, encoding: 'utf8', timeout: 30000, windowsHide: true });
      assert.ifError(result.error);
      assert.equal(result.status, 0, `Electron exit status ${result.status}, signal ${result.signal}\n${result.stdout}${result.stderr}`);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
}
