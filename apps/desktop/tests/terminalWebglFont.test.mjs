import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

// Characterizes the installed WebGL addon, including our pnpm patch. The
// regression kept 28px cells but silently rasterized glyphs at the 10px default.
if (process.versions.electron) {
  const { app, BrowserWindow } = await import('electron');
  app.setPath('userData', process.env.ORKWORKS_FONT_PROFILE);
  app.commandLine.appendSwitch('force-device-scale-factor', '2');
  if (process.platform === 'linux') app.commandLine.appendSwitch('use-angle', 'swiftshader');
  app.whenReady().then(async () => {
    try {
      const win = new BrowserWindow({ show: false, width: 800, height: 600 });
      await win.loadFile(process.env.ORKWORKS_FONT_FIXTURE);
      const samples = await win.webContents.executeJavaScript(`new Promise(resolve => {
        const samples = [];
        const original = CanvasRenderingContext2D.prototype.fillText;
        CanvasRenderingContext2D.prototype.fillText = function(text, ...args) {
          if (text === 'M') samples.push(this.font);
          return original.call(this, text, ...args);
        };
        const terminal = new Terminal({fontSize:14, fontFamily:'monospace', cols:40, rows:5});
        terminal.open(document.getElementById('terminal'));
        terminal.loadAddon(new WebglAddon.WebglAddon());
        terminal.write('M\\x1b[1mM\\x1b[3mM\\x1b[22mM', () => {
          requestAnimationFrame(() => requestAnimationFrame(() => {
            CanvasRenderingContext2D.prototype.fillText = original;
            terminal.dispose();
            resolve(samples);
          }));
        });
      })`);
      assert.ok(samples.length >= 4, 'normal, bold, bold italic and italic glyphs were drawn');
      for (const font of samples) assert.match(font, /28px monospace$/, `glyph font: ${font}`);
      assert.ok(samples.some(font => font.includes('bold')));
      assert.ok(samples.some(font => font.includes('italic')));
      app.exit(0);
    } catch (error) {
      console.error(error);
      app.exit(1);
    }
  });
} else {
  const { default: test } = await import('node:test');
  const { mkdtempSync, writeFileSync, rmSync } = await import('node:fs');
  const { tmpdir } = await import('node:os');
  const { join } = await import('node:path');
  const { createRequire } = await import('node:module');
  const { spawn } = await import('node:child_process');
  const require = createRequire(import.meta.url);
  for (const format of ['js', 'mjs']) test(`WebGL ${format} glyphs use the requested Retina font for normal, bold and italic text`, {timeout:30000}, async () => {
    const directory = mkdtempSync(join(tmpdir(), 'orkworks-webgl-font-'));
    const fixture = join(directory, 'index.html');
    const xterm = require.resolve('@xterm/xterm').replace(/\.js$/, `.${format}`);
    const webgl = require.resolve('@xterm/addon-webgl').replace(/\.js$/, `.${format}`);
    const scripts = format === 'js'
      ? `<script src="${xterm}"></script><script src="${webgl}"></script>`
      : `<script type="module">import {Terminal} from '${xterm}'; import {WebglAddon} from '${webgl}'; window.Terminal=Terminal; window.WebglAddon={WebglAddon};</script>`;
    writeFileSync(fixture, `<div id="terminal"></div>${scripts}`);
    try {
      const env = {...process.env, ORKWORKS_FONT_PROFILE:join(directory,'profile'), ORKWORKS_FONT_FIXTURE:fixture};
      delete env.ELECTRON_RUN_AS_NODE;
      const args = [fileURLToPath(import.meta.url)];
      let executable = require('electron');
      if (process.platform === 'linux' && !process.env.DISPLAY) {
        args.unshift(executable, '--no-sandbox');
        executable = 'xvfb-run';
        args.unshift('-a');
      }
      const child = spawn(executable, args, {env, stdio:['ignore','pipe','pipe']});
      let output = '';
      child.stdout.on('data', data => { output += data; });
      child.stderr.on('data', data => { output += data; });
      const timer = setTimeout(() => child.kill('SIGKILL'), 25000);
      try {
        const code = await new Promise((resolve, reject) => {
          child.once('error', reject);
          child.once('exit', resolve);
        });
        assert.equal(code, 0, output);
      } finally { clearTimeout(timer); }
    } finally { rmSync(directory, {recursive:true, force:true}); }
  });
}
