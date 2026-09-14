import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

if (process.versions.electron) {
  const { app, BrowserWindow } = await import('electron');
  app.disableHardwareAcceleration();
  app.whenReady().then(async () => {
    const win = new BrowserWindow({ show: false, width: 1000, height: 700 });
    const evaluate = (source) => win.webContents.executeJavaScript(source);
    const waitFor = async (source) => {
      const deadline = Date.now() + 5000;
      while (Date.now() < deadline) {
        if (await evaluate(source)) return;
        await new Promise(resolve => setTimeout(resolve, 10));
      }
      throw new Error(`Timed out: ${source}`);
    };
    const fresh = async () => {
      await win.loadFile(process.env.ORKWORKS_DOCKVIEW_FIXTURE);
      await waitFor('window.fixture?.loads.length === 1');
    };
    const restore = async (index) => {
      await evaluate(`fixture.loads[${index}](null)`);
      await waitFor('fixture.apis.at(-1).getPanel("recommendations") != null');
    };
    try {
      // Catch the one-time initialization guard retaining an already-disposed API.
      await fresh();
      await restore(0);
      await evaluate('fixture.replaceDock()');
      await waitFor('fixture.apis.length === 2');
      await evaluate('fixture.selectShell()');
      await new Promise(resolve => setTimeout(resolve, 50));
      assert.deepEqual(await evaluate('fixture.errors'), [], 'session switching must not close disposed panels');
      assert.equal(await evaluate('fixture.apiRef.current === fixture.apis[1]'), true);
      assert.equal(await evaluate('fixture.loads.length'), 2, 'replacement must request its layout');
      await evaluate('fixture.loads[1](null)');
      await waitFor('fixture.apis[1].getPanel("terminal") != null');
      assert.deepEqual(await evaluate('fixture.apis[1].panels.map(p => p.id).sort()'), ['detail', 'sessions', 'terminal']);

      // Catch a delayed read mutating a replaced instance or its shared hidden-panel state.
      await fresh();
      await evaluate('fixture.replaceDock()');
      await waitFor('fixture.loads.length === 2');
      await restore(1);
      await evaluate('fixture.visibility.length = 0; fixture.loads[0](JSON.stringify({v:1,d:fixture.apis[1].toJSON(),hiddenSignalPanels:["review"]}))');
      await new Promise(resolve => setTimeout(resolve, 50));
      assert.equal(await evaluate('fixture.apis[0].totalPanels'), 0);
      assert.deepEqual(await evaluate('[...fixture.hidden.current]'), []);
      assert.deepEqual(await evaluate('fixture.visibility'), []);

      // Catch unmount retaining the API or allowing an unresolved restore to publish.
      await fresh();
      await evaluate('fixture.unmount(); fixture.loads[0](null)');
      await new Promise(resolve => setTimeout(resolve, 50));
      assert.equal(await evaluate('fixture.apiRef.current'), null);
      assert.equal(await evaluate('fixture.apis[0].totalPanels'), 0);
      assert.deepEqual(await evaluate('fixture.visibility'), []);

      // Catch a pending debounced save serializing disposed Dockview state.
      await fresh();
      await restore(0);
      await waitFor('fixture.visibility.length > 0');
      await evaluate('fixture.saves.length = 0; fixture.unmount()');
      await new Promise(resolve => setTimeout(resolve, 650));
      assert.deepEqual(await evaluate('fixture.saves'), []);
      assert.deepEqual(await evaluate('fixture.errors'), []);
      console.log('Dockview replacement, stale restore, and unmount lifecycle checks passed');
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
  test('Dockview follows the current instance across replacement and unmount', async () => {
    const require = createRequire(import.meta.url);
    const root = fileURLToPath(new URL('../', import.meta.url));
    const directory = mkdtempSync(join(tmpdir(), 'orkworks-dockview-'));
    try {
      const bundle = await build({
        stdin: { contents: `
          import React, {useState} from 'react';
          import {createRoot} from 'react-dom/client';
          import DockviewApp from './src/components/DockviewApp';
          const fixture = window.fixture = {
            apiRef: {current: null}, hidden: {current: new Set()},
            apis: [], loads: [], visibility: [], saves: [], errors: [],
          };
          window.addEventListener('error', event => fixture.errors.push(event.error?.stack || event.message));
          window.addEventListener('unhandledrejection', event => fixture.errors.push(String(event.reason)));
          window.orkworks = {
            getLayout: () => new Promise(resolve => fixture.loads.push(resolve)),
            notifyPanelVisibility: (...args) => fixture.visibility.push(args),
            saveLayout: layout => fixture.saves.push(layout),
          };
          function Harness() {
            const [session, setSession] = useState({id:'coding', harnessId:'codex', hasOpenablePlan:false});
            fixture.selectShell = () => setSession({id:'shell', harnessId:'generic-shell', hasOpenablePlan:false});
            return <DockviewApp dockviewApiRef={fixture.apiRef} signalPanelHiddenIdsRef={fixture.hidden}
              sessions={[session]} activeSessionId={session.id} debugSettings={{}} />;
          }
          const root = createRoot(document.getElementById('root'));
          fixture.unmount = () => root.unmount();
          root.render(<Harness/>);
        `, resolveDir: root, loader: 'tsx' },
        bundle: true, write: false, format: 'iife', platform: 'browser',
        define: { 'process.env.NODE_ENV': '"development"' },
        plugins: [{ name: 'lifecycle-fixture', setup(builder) {
          // Panel bodies connect to the backend/PTY; retain real React and Dockview ownership.
          builder.onResolve({ filter: /^\.\/(SessionListPanel|SessionDetailPanel|TerminalPanel|CapacityPanel|RecommendationsPanel|ReviewPanel)$/ },
            () => ({ path: 'panel', namespace: 'empty-panel' }));
          builder.onLoad({ filter: /.*/, namespace: 'empty-panel' },
            () => ({ contents: 'export default function Panel(){return null}', loader: 'js' }));
          builder.onResolve({ filter: /^dockview-react$/ }, args => args.namespace === 'dockview-fixture'
            ? undefined : { path: 'dockview-react', namespace: 'dockview-fixture' });
          builder.onLoad({ filter: /.*/, namespace: 'dockview-fixture' }, () => ({
            contents: `
              import React, {useState} from 'react';
              import {DockviewReact as RealDockview} from 'dockview-react';
              export * from 'dockview-react';
              export function DockviewReact(props) {
                const [key, setKey] = useState(0);
                window.fixture.replaceDock = () => setKey(value => value + 1);
                return <RealDockview key={key} {...props} onReady={event => {
                  window.fixture.apis.push(event.api);
                  props.onReady(event);
                }}/>;
              }
            `, loader: 'tsx', resolveDir: root,
          }));
        } }],
      });
      const fixture = join(directory, 'index.html');
      writeFileSync(fixture, '<div id="root" style="height:600px;display:flex"></div><script>' + bundle.outputFiles[0].text + '</script>');
      const electron = require('electron');
      const env = { ...process.env, ORKWORKS_DOCKVIEW_FIXTURE: fixture };
      delete env.ELECTRON_RUN_AS_NODE;
      const args = [fileURLToPath(import.meta.url)];
      if (process.platform === 'linux') args.unshift('--no-sandbox');
      const headlessLinux = process.platform === 'linux' && !env.DISPLAY;
      if (headlessLinux) args.unshift('-a', electron);
      const result = spawnSync(headlessLinux ? 'xvfb-run' : electron, args,
        { env, encoding: 'utf8', timeout: 30000, windowsHide: true });
      assert.ifError(result.error);
      assert.equal(result.status, 0, result.stdout + result.stderr);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
}
