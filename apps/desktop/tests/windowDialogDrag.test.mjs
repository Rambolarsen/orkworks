import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

if (process.versions.electron) {
  const { app, BrowserWindow } = await import('electron');
  app.disableHardwareAcceleration();
  app.whenReady().then(async () => {
    const win = new BrowserWindow({ width: 900, height: 500, useContentSize: true, show: false });
    try {
      const css = readFileSync(new URL('../src/styles/tokens.css', import.meta.url), 'utf8')
        + readFileSync(new URL('../src/App.css', import.meta.url), 'utf8');
      for (const [backdrop, card] of [
        ['settings-backdrop', 'settings-modal'],
        ['new-session-backdrop', 'new-session-dialog'],
        ['error-boundary', 'error-boundary-card'],
      ]) {
        for (const platform of ['win32', 'darwin', 'linux']) {
          const html = `<!doctype html><html data-platform="${platform}"><style>${css}</style><body>
            <button id="workspace">Workspace switch</button>
            <div class="${backdrop}"><div class="${card}">
              <button id="action">Dialog action</button><div style="height:1000px;flex-shrink:0">Tall content</div>
            </div></div></body></html>`;
          await win.loadURL('data:text/html;charset=utf-8,' + encodeURIComponent(html));
          const actual = await win.webContents.executeJavaScript(`(() => {
            const backdrop = document.querySelector('.${backdrop}');
            const card = document.querySelector('.${card}');
            const strip = getComputedStyle(backdrop, '::before');
            const action = document.querySelector('#action');
            const rect = card.getBoundingClientRect();
            return { content: strip.content, drag: strip.webkitAppRegion, height: strip.height,
              position: strip.position, top: strip.top, cardTop: rect.top,
              actionDrag: getComputedStyle(action).webkitAppRegion,
              headerBlocked: document.elementFromPoint(10, 10) === backdrop,
              cardHeight: rect.height, scrollable: card.scrollHeight > card.clientHeight,
              overflow: getComputedStyle(card).overflowY };
          })()`);
          if (platform === 'win32') {
            assert.equal(actual.content, '""', backdrop + ' must supply a drag strip');
            assert.equal(actual.drag, 'drag');
            assert.equal(actual.position, 'fixed');
            assert.equal(actual.top, '0px');
            assert.equal(actual.height, '38px');
            assert.ok(actual.headerBlocked, 'workspace controls stay blocked by the modal');
            assert.ok(actual.cardTop >= 38, 'dialog content stays below the drag strip');
            assert.notEqual(actual.actionDrag, 'drag', 'dialog action remains clickable');
            if (backdrop === 'new-session-backdrop') {
              assert.ok(actual.cardHeight <= 442 && actual.scrollable);
              assert.equal(actual.overflow, 'auto');
            }
          } else {
            assert.equal(actual.content, 'none', platform + ' must retain its existing behavior');
          }
        }
      }
      console.log('Windows modal drag strip, tall dialog layout, and platform isolation passed');
    } finally {
      win.destroy();
    }
    app.quit();
  }).catch(error => { console.error(error); app.exit(1); });
} else {
  const { default: test } = await import('node:test');
  const { spawnSync } = await import('node:child_process');
  const { createRequire } = await import('node:module');
  test('Windows modal overlays preserve dragging without covering dialog controls', { skip: process.platform !== 'win32' }, () => {
    const electron = createRequire(import.meta.url)('electron');
    const env = { ...process.env };
    delete env.ELECTRON_RUN_AS_NODE;
    const result = spawnSync(electron, [fileURLToPath(import.meta.url)], { env, encoding: 'utf8', timeout: 30000, windowsHide: true });
    assert.equal(result.status, 0, result.stdout + result.stderr);
  });
}
