import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

if (process.versions.electron) {
  const { app, BrowserWindow } = await import('electron');
  app.disableHardwareAcceleration();
  app.whenReady().then(async () => {
    const win = new BrowserWindow({ show: false, width: 1280, height: 700 });
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
      await win.loadFile(process.env.ORKWORKS_SHELL_FIXTURE);
      await waitFor('window.fixture?.ready');
    };
    try {
      await fresh();
      const scenario = process.env.ORKWORKS_SHELL_SCENARIO;
      const click = async command => evaluate('(()=>{const b=document.querySelector("[data-shell-command=' + command + ']");b.focus();b.click()})()');
      if (scenario !== 'baseline') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        if (scenario === 'sessions-toggle') {
          await click('sessions');
          await waitFor('!document.querySelector("[data-shell-region=sessions]")');
          await click('sessions');
          await waitFor('document.getElementById("sessions-list")===document.activeElement');
          await click('sessions');
          await waitFor('!document.querySelector("[data-shell-region=sessions]")');
          await win.setContentSize(640,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="compact"');
          await click('sessions');
          await waitFor('document.getElementById("sessions-list")===document.activeElement');
          await click('sessions');
          await waitFor('document.querySelector("[data-shell-region=terminal]")');
        } else if (scenario === 'sessions-browse') {
          await win.setContentSize(640,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="compact"');
          await click('sessions');
          await waitFor('document.getElementById("sessions-list")===document.activeElement');
          await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowDown",bubbles:true}))');
          await waitFor('fixture.selections===1');
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=sessions]")'),true,'arrow selection keeps compact Sessions mounted');
          assert.equal(await evaluate('document.activeElement.id'),'sessions-list');
          await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowUp",bubbles:true}))');
          await waitFor('fixture.selections===2');
          await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Enter",bubbles:true}))');
          await waitFor('fixture.runtime()?.terminal.textarea===document.activeElement');
        } else if (scenario === 'focus') {
          await win.setContentSize(640,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="compact"');
          await click('sessions');
          await waitFor('document.getElementById("sessions-list")===document.activeElement');
          await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Enter",bubbles:true}))');
          await waitFor('fixture.runtime()?.terminal.textarea===document.activeElement');
          await click('details');
          await waitFor('document.querySelector("[data-shell-region=utility]")');
          await click('terminal');
          await waitFor('fixture.runtime()?.terminal.textarea===document.activeElement');
        } else if (scenario === 'utility-sessions-focus') {
          await win.setContentSize(1000,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="medium"');
          await click('details');
          await waitFor('document.querySelector("[data-shell-region=utility]")');
          await evaluate('fixture.command("sessions")');
          await waitFor('document.getElementById("sessions-list")===document.activeElement');
          await evaluate('fixture.command("sessions")');
          await waitFor('!document.querySelector("[data-shell-region=sessions]")');
          assert.equal(await evaluate('document.activeElement.hasAttribute("data-shell-page-heading")'),true,'hiding Sessions focuses the remaining utility');
          assert.equal(await evaluate('document.activeElement.textContent'),'Details');
        } else if (scenario === 'hidden-resize') {
          await win.setContentSize(1000,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="medium"');
          await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
          await evaluate('fixture.detachedFits=0;fixture.oldGrid=[fixture.runtime().terminal.cols,fixture.runtime().terminal.rows];const handle=fixture.runtime();const fit=handle.fitAddon.fit.bind(handle.fitAddon);handle.fitAddon.fit=()=>{if(!handle.wrapper.isConnected)fixture.detachedFits++;fit()};void 0');
          await click('details');
          await waitFor('document.querySelector("[data-shell-region=utility]")');
          await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(resolve))))');
          assert.equal(await evaluate('fixture.detachedFits'),0,'detached presentation must not fit the live PTY');
          assert.deepEqual(await evaluate('[fixture.runtime().terminal.cols,fixture.runtime().terminal.rows]'),await evaluate('fixture.oldGrid'));
          await click('terminal');
          await waitFor('fixture.runtime()?.terminal.element?.isConnected');
          await win.setContentSize(1050,700);
          await waitFor('fixture.runtime().terminal.cols>fixture.oldGrid[0]');
        } else if (scenario === 'backend-loss') {
          await win.setContentSize(1000,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="medium"');
          await click('details');
          await waitFor('document.querySelector("[data-shell-region=utility]")');
          await evaluate('fixture.oldRuntime=fixture.runtime();fixture.setBackendStatus("unreachable")');
          await waitFor('fixture.oldRuntime.disposed');
          await evaluate('fixture.setBackendStatus("connected")');
          await click('terminal');
          await waitFor('fixture.runtime()?.terminal.element?.isConnected');
          assert.notEqual(await evaluate('fixture.runtime()===fixture.oldRuntime'),true);
          assert.equal(await evaluate('fixture.socketCount'),2);
          assert.equal(await evaluate('fixture.runtime().unavailable'),false);
        }
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      }
      await waitFor('document.querySelector("[data-shell-region=terminal]")');
      await waitFor('fixture.runtime()?.terminal.element?.isConnected');
      assert.equal(await evaluate('document.querySelectorAll("[data-shell-region]").length'), 2);
      await evaluate('(()=>{const button=document.querySelector("[data-shell-command=details]");button.focus();button.click()})()');
      await waitFor('document.querySelector("[data-shell-region=inspector]")');
      assert.equal(await evaluate('fixture.selections'), 0, 'inspection must not select or acknowledge');
      assert.equal(await evaluate('fixture.prompts'), 0, 'inspection cannot submit a prompt');
      assert.equal(await evaluate('document.querySelectorAll("[role=separator]").length'), 2);
      await evaluate('document.querySelector("[role=separator]").focus()');
      await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"ArrowRight",bubbles:true}))');
      await waitFor('document.querySelector("[role=separator]").getAttribute("aria-valuenow") === "256"');
      await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Home",bubbles:true}))');
      await waitFor('document.querySelector("[role=separator]").getAttribute("aria-valuenow") === "200"');
      const separator = await evaluate('(()=>{const r=document.querySelector("[role=separator]").getBoundingClientRect();return {x:Math.round(r.x+r.width/2),y:Math.round(r.y+r.height/2)}})()');
      win.webContents.sendInputEvent({type:'mouseDown',...separator,button:'left',clickCount:1});
      win.webContents.sendInputEvent({type:'mouseMove',x:separator.x+32,y:separator.y});
      win.webContents.sendInputEvent({type:'mouseUp',x:separator.x+32,y:separator.y,button:'left',clickCount:1});
      await waitFor('document.querySelector("[role=separator]").getAttribute("aria-valuenow") === "232"');
      await evaluate('(()=>{const button=document.querySelector("[data-shell-command=capacity]");button.focus();button.click()})()');
      await waitFor('document.querySelector("[data-shell-region=inspector] h2").textContent === "Capacity"');
      assert.equal(await evaluate('document.querySelectorAll("[data-shell-region=inspector]").length'), 1);
      await evaluate('(()=>{const button=document.querySelector("[data-shell-command=capacity]");button.focus();button.click()})()');
      await waitFor('!document.querySelector("[data-shell-region=inspector]")');
      assert.equal(await evaluate('document.activeElement.dataset.shellCommand'), 'details');

      await evaluate('fixture.originalRuntime=fixture.runtime(); fixture.output("before hiding\\r\\n")');
      await win.setContentSize(1000, 700);
      await evaluate('(()=>{const button=document.querySelector("[data-shell-command=details]");button.focus();button.click()})()');
      await waitFor('document.querySelector("[data-shell-region=utility]")');
      assert.equal(await evaluate('!!document.querySelector("[data-shell-region=sessions]")'), true);
      assert.equal(await evaluate('!!document.querySelector("[data-shell-region=terminal]")'), false);
      await evaluate('fixture.output("output while hidden\\r\\n")');
      await evaluate('(()=>{const button=document.querySelector("[data-shell-return]");button.focus();button.click()})()');
      await waitFor('document.querySelector("[data-shell-region=terminal]")');
      assert.equal(await evaluate('document.activeElement.dataset.shellCommand'), 'details');

      await waitFor('fixture.runtime()?.terminal.element?.isConnected');
      assert.equal(await evaluate('fixture.runtime()===fixture.originalRuntime'),true);
      assert.equal(await evaluate('fixture.socketCount'),1);
      await waitFor('Array.from({length:fixture.runtime().terminal.buffer.active.length},(_,i)=>fixture.runtime().terminal.buffer.active.getLine(i).translateToString()).join("\\n").includes("output while hidden")');
      assert.equal(await evaluate('fixture.input.length'),0,'navigation must not reach the PTY');
      await win.setContentSize(640, 700);
      await waitFor('document.querySelector(".shell-layout").dataset.mode === "compact"');
      await evaluate('(()=>{const button=document.querySelector("[data-shell-command=sessions]");button.focus();button.click()})()');
      await waitFor('document.getElementById("sessions-list") === document.activeElement');
      assert.equal(await evaluate('document.querySelectorAll("[data-shell-region]").length'), 1);
      await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Enter",bubbles:true}))');
      await waitFor('document.querySelector("[data-shell-region=terminal]")');
      assert.equal(await evaluate('fixture.runtime()===fixture.originalRuntime'), true);
      assert.equal(await evaluate('fixture.socketCount'), 1);
      await evaluate('(()=>{const button=document.querySelector("[data-shell-command=details]");button.focus();button.click()})()');
      await waitFor('document.querySelector("[data-shell-region=utility]")');
      await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown", {key:"Escape",bubbles:true}))');
      await waitFor('document.querySelector("[data-shell-region=terminal]")');
      assert.equal(await evaluate('document.activeElement.dataset.shellCommand'), 'details');
      assert.equal(await evaluate('fixture.selections'), 0);
      assert.equal(await evaluate('fixture.prompts'), 0);
      assert.deepEqual(await evaluate('fixture.errors'), []);
      await win.setContentSize(1280, 700);
      await waitFor('document.querySelector(".shell-layout").dataset.mode === "wide"');
      win.webContents.setZoomFactor(2);
      await waitFor('document.querySelector(".shell-layout").dataset.mode === "compact"');
      assert.equal(await evaluate('document.querySelectorAll("[data-shell-region]").length'), 1);
      assert.equal(await evaluate('fixture.runtime()===fixture.originalRuntime'), true);
      win.webContents.setZoomFactor(1);
      await waitFor('document.querySelector(".shell-layout").dataset.mode === "wide"');
      console.log('Fixed shell responsive destinations, resizing, zoom, and focus checks passed');

    } finally {
      win.destroy();
      app.quit();
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
  for (const scenario of ['baseline','focus','sessions-toggle','sessions-browse','backend-loss','utility-sessions-focus','hidden-resize']) test('fixed shell: '+scenario, async () => {
    const require = createRequire(import.meta.url);
    const root = fileURLToPath(new URL('../', import.meta.url));
    const directory = mkdtempSync(join(tmpdir(), 'orkworks-shell-'));
    try {
      const bundle = await build({
        stdin: { contents: `
          import React, {useState} from 'react';
          import {createRoot} from 'react-dom/client';
          import ApplicationShell from './src/components/ApplicationShell';
          import './src/App.css';
          import {getTerminal, getLiveTerminalCount} from './src/terminalStore';
          const fixture = window.fixture = {selections:0,prompts:0,errors:[],ready:true};
          window.addEventListener('error', e => fixture.errors.push(e.error?.stack || e.message));
          window.addEventListener('unhandledrejection', e => fixture.errors.push(String(e.reason)));
          fixture.socketCount=0;fixture.input=[];
          window.WebSocket=class {
            static OPEN=1; readyState=1; bufferedAmount=0;
            constructor(){fixture.socketCount++;fixture.socket=this;queueMicrotask(()=>this.onopen?.());}
            send(payload){const message=JSON.parse(payload);if(message.type==='input')fixture.input.push(message.data);}
            close(){}
          };
          fixture.output=text=>fixture.socket.onmessage({data:new TextEncoder().encode(text).buffer});
          fixture.runtime=()=>getTerminal(fixture.activeSessionId||'coding');
          fixture.runtimeCount=()=>getLiveTerminalCount();
          window.orkworks = {notifyPanelVisibility:()=>{},getBackendUrl:()=>Promise.resolve('http://127.0.0.1:12345')};
          function Harness() {
            const [inspector,setInspector]=useState(null);
            const [backendStatus,setBackendStatus]=useState('connected');
            const [commandRequest,setCommandRequest]=useState(null);
            fixture.command=command=>setCommandRequest(old=>({command,sequence:(old?.sequence||0)+1}));
            fixture.setBackendStatus=setBackendStatus;
            const [activeSessionId,setActiveSessionId]=useState('coding');
            fixture.activeSessionId=activeSessionId;
            const [preferences,setPreferences]=useState({sessionsWidth:240,inspectorWidth:320,sessionsVisible:true,density:'low'});
            return <ApplicationShell sessions={['coding','other'].map((id,i)=>({id,name:id,harnessId:'codex',harness:'codex',lifecycle:'alive',status:'running',label:id,createdAt:'2026-10-10T10:00:00Z',lastActivityAt:i?'2026-10-10T09:00:00Z':'2026-10-10T10:00:00Z'}))}
              workspace={{name:'Test',path:'/tmp/test'}} activeSessionId={activeSessionId} workspaceGeneration={0}
              backendStatus={backendStatus} harnesses={[]} debugSettings={{showSessionIds:false}}
              commandRequest={commandRequest} preferences={preferences} onPreferencesChange={setPreferences} onResetPreferences={()=>{}}
              inspector={inspector} onInspect={setInspector} unreadIds={new Set()} acknowledgedIds={new Set()}
              onSelectSession={id=>{fixture.selections++;setActiveSessionId(id)}} onFocusTerminal={()=>fixture.runtime()?.terminal.focus()} onBackendUnavailable={()=>{}} />;
          }
          createRoot(document.getElementById('root')).render(<Harness/>);
        `, resolveDir: root, loader: 'tsx' },
        bundle: true, write: false, outdir: directory, format: 'iife', platform: 'browser',
        define: { 'process.env.NODE_ENV': '"development"' },
        plugins: [{ name: 'lifecycle-fixture', setup(builder) {
          builder.onResolve({filter:/^@xterm\/addon-webgl$/},()=>({path:'webgl',namespace:'no-gpu'}));
          builder.onLoad({filter:/.*/,namespace:'no-gpu'},()=>({contents:'export class WebglAddon { constructor(){throw new Error("GPU unavailable in headless fixture")} }',loader:'js'}));
          // Panel bodies connect to the backend/PTY; retain real React shell and terminal ownership.
          builder.onResolve({ filter: /^\.\/(SessionDetailPanel|CapacityPanel|RecommendationsPanel)$/ },
            args => ({ path: args.path, namespace: 'empty-panel' }));
          builder.onLoad({ filter: /.*/, namespace: 'empty-panel' },
            () => ({ contents: 'export default function Panel(){return null}', loader: 'tsx', resolveDir: root }));

        } }],
      });
      const fixture = join(directory, 'index.html');
      writeFileSync(fixture, '<style>' + (bundle.outputFiles.find(f=>f.path.endsWith('.css'))?.text||'') + '</style><div id="root" style="height:650px;display:flex"></div><script>' + bundle.outputFiles.find(f=>!f.path.endsWith('.css')).text + '</script>');
      const electron = require('electron');
      const env = { ...process.env, ORKWORKS_SHELL_FIXTURE: fixture, ORKWORKS_SHELL_SCENARIO: scenario };
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
