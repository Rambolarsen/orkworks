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
      if (scenario.startsWith('preferences-')) {
        if (scenario === 'preferences-migration') {
          await waitFor('document.querySelector(".shell-preferences-notice [type=button]")');
          assert.match(await evaluate('document.querySelector("[role=status]").textContent'),/saved panel arrangement is retained/);
          assert.equal(await evaluate('fixture.preferenceWrites'),0,'hydration never saves');
          await evaluate('document.querySelector(".shell-preferences-notice button").click()');
          await waitFor('fixture.preferenceWrites===1');
          await evaluate('fixture.remountPreferences()');
          await waitFor('fixture.preferenceReads>=2');
          await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
          assert.equal(await evaluate('!!document.querySelector(".shell-preferences-notice")'),false,'acknowledged migration does not repeat');
        } else {
          await waitFor('document.querySelector("[role=alert]")');
          await evaluate('document.querySelector(".shell-preferences-notice button").click()');
          await waitFor('fixture.rebuildCalls===1');
          assert.deepEqual(await evaluate('fixture.noticeCommands'),['reset-layout'],'recovery notice requests the shared shell command');
          assert.equal(await evaluate('!!document.querySelector("[role=alert]")'),true,'cancel keeps recovery available');
          await evaluate('fixture.cancelRebuild=false;document.querySelector(".shell-preferences-notice button").click()');
          await waitFor('fixture.rebuildCalls===2&&!document.querySelector("[role=alert]")');
          assert.equal(await evaluate('fixture.resetCalls'),0,'invalid records use the confirmed rebuild API');
        }
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario !== 'baseline') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        if (scenario === 'sidebar-controls') {
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=sessions] [data-shell-command=capacity]")'),true,'workspace navigation belongs to the sidebar');
          const anchors = await evaluate(`(()=>{
            const sidebar=document.querySelector('[data-shell-region=sessions]');
            const list=document.getElementById('sessions-list').parentElement;
            const footer=sidebar.querySelector('footer');
            const capacity=sidebar.querySelector('[data-shell-command=capacity]');
            list.scrollTop=list.scrollHeight;
            return {capacity:capacity.getBoundingClientRect().bottom,list:list.getBoundingClientRect().top,
              listBottom:list.getBoundingClientRect().bottom,footer:footer.getBoundingClientRect().top,
              footerBottom:footer.getBoundingClientRect().bottom,sidebarBottom:sidebar.getBoundingClientRect().bottom,
              scrolled:list.scrollTop>0};
          })()`);
          assert.ok(anchors.capacity<=anchors.list,'navigation stays above history');
          assert.ok(anchors.listBottom<=anchors.footer,'history cannot cover Settings');
          assert.ok(anchors.footerBottom<=anchors.sidebarBottom,'footer stays in the viewport');
          assert.ok(anchors.scrolled,'long history actually scrolls');
          await evaluate('document.querySelector("[data-shell-region=sessions] [data-shell-settings]").click()');
          assert.equal(await evaluate('fixture.settingsOpened'),1);
          await evaluate('document.querySelector("[data-shell-region=sessions] [data-shell-create]").click()');
          assert.equal(await evaluate('fixture.sessionsCreated'),1);
          await evaluate('document.querySelector(".titlebar-switch-button").click()');
          await waitFor('document.querySelector(".workspace-history-other-folder")');
          const picker = await evaluate(`(()=>{const r=document.querySelector('.workspace-history-panel').getBoundingClientRect();const sidebar=document.querySelector('[data-shell-region=sessions]').getBoundingClientRect();return {x:r.x,right:r.right,y:r.y,bottom:r.bottom,sidebarRight:sidebar.right};})()`);
          assert.ok(picker.y>=0&&picker.bottom<=700&&picker.x>=0&&picker.right<=picker.sidebarRight,'workspace switcher stays inside the sidebar viewport');
          await evaluate('document.querySelector(".titlebar-switch-button").click()');
          await click('capacity');
          await waitFor('document.querySelector("[data-shell-region=inspector]")');
          assert.equal(await evaluate('fixture.selections'),0,'sidebar utility navigation preserves selected session');
          await click('capacity');
          await click('sessions');
          await waitFor('!document.querySelector("[data-shell-region=sessions]")');
          await evaluate('document.querySelector(".shell-navigation-menu summary").click()');
          assert.ok(await evaluate('document.querySelector(".shell-navigation-menu [data-shell-command=capacity]").getClientRects().length>0'),'hidden sidebar retains reachable navigation');
          await evaluate('const summary=document.querySelector(".shell-navigation-menu summary");summary.focus();summary.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
          assert.equal(await evaluate('document.querySelector(".shell-navigation-menu").open'),false,'Escape dismisses the fallback menu');
          assert.equal(await evaluate('document.activeElement.tagName'),'SUMMARY','menu dismissal restores trigger focus');
          await evaluate('document.querySelector(".shell-navigation-menu summary").click()');
          await click('capacity');
          await waitFor('document.querySelector("[data-shell-region=inspector]")');
          assert.equal(await evaluate('document.querySelector(".shell-navigation-menu").open'),false,'choosing a utility dismisses the menu');
          await evaluate('fixture.command("capacity")');
          await waitFor('!document.querySelector("[data-shell-region=inspector]")');
          assert.equal(await evaluate('document.activeElement.tagName'),'SUMMARY','utility return focuses the dismissed menu trigger');
          for (const action of ['[data-shell-settings]', '[data-shell-create]']) {
            await evaluate('document.querySelector(".shell-navigation-menu summary").click()');
            await evaluate('document.querySelector(".shell-navigation-menu '+action+'").click()');
            assert.equal(await evaluate('document.querySelector(".shell-navigation-menu").open'),false,'dialog actions dismiss the menu');
          }
          await win.setContentSize(640,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="compact"');
          await click('sessions');
          await waitFor('document.getElementById("sessions-list")===document.activeElement');
          assert.ok(await evaluate('document.querySelector("[data-shell-region=sessions] [data-shell-settings]").getBoundingClientRect().bottom<=innerHeight'),'compact footer remains reachable');
          await evaluate('document.querySelector(".titlebar-switch-button").focus();document.querySelector(".titlebar-switch-button").click()');
          await waitFor('document.querySelector(".workspace-history-other-folder")');
          await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=sessions]")'),true,'Escape closes the workspace picker without leaving compact Sessions');
        } else if (scenario === 'sessions-toggle') {
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
          assert.equal(await evaluate('document.activeElement.dataset.shellCommand'),'sessions','repeated Sessions returns focus to its remounted invoker');
          for (const close of ['back','escape']) {
            await click('sessions');
            await waitFor('document.getElementById("sessions-list")===document.activeElement');
            if (close==='back') await evaluate('document.querySelector("[data-shell-return]").click()');
            else await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
            await waitFor('document.querySelector("[data-shell-region=terminal]")');
            assert.equal(await evaluate('document.activeElement.dataset.shellCommand'),'sessions','closing compact Sessions restores its invoker');
          }
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
        } else if (scenario === 'reset-confirmation') {
          await win.setContentSize(1000,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="medium"');
          await evaluate('fixture.command("capacity")');
          await waitFor('document.querySelector("[data-shell-region=utility]")');
          await evaluate('fixture.command("reset-layout")');
          await waitFor('typeof fixture.finishReset==="function"');
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=utility]")'),true,'pending confirmation preserves current page');
          await evaluate('fixture.finishReset(false);void 0');
          await evaluate('new Promise(resolve=>requestAnimationFrame(resolve))');
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=utility]")'),true,'cancel preserves current page');
          await evaluate('fixture.finishReset=null;fixture.command("reset-layout")');
          await waitFor('typeof fixture.finishReset==="function"');
          await evaluate('fixture.finishReset(true);void 0');
          await waitFor('!document.querySelector("[data-shell-region=utility]")');
          await waitFor('fixture.runtime()?.terminal.textarea===document.activeElement');
        } else if (scenario === 'utility-scope') {
          for (const width of [1280,1000,640]) {
            await win.setContentSize(width,700);
            await waitFor(`document.querySelector(".shell-layout").dataset.mode==="${width>=1180?'wide':width>=860?'medium':'compact'}"`);
            for (const destination of ['capacity','recommendations']) {
              await evaluate('fixture.command('+JSON.stringify(destination)+')');
              await waitFor('document.querySelector("[data-shell-page-heading]")');
              assert.match(await evaluate('document.querySelector("[data-shell-page-heading]").parentElement.textContent'),/Workspace: \/tmp\/test/);
              if(destination==='capacity')assert.match(await evaluate('document.querySelector("[data-shell-page-heading]").parentElement.textContent'),/All coding tools/);
              await evaluate('fixture.command('+JSON.stringify(destination)+')');
              await waitFor('!document.querySelector("[data-shell-page-heading]")');
            }
          }
        } else if (scenario === 'empty-terminal-return') {
          await win.setContentSize(1000,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="medium"');
          await evaluate('fixture.select(null)');
          await waitFor('fixture.activeSessionId===null');
          await evaluate('const heading=document.querySelector("[data-shell-terminal-focus]");heading.focus();fixture.command("capacity");void 0');
          await waitFor('document.querySelector("[data-shell-region=utility]")');
          await evaluate('fixture.command("capacity")');
          await waitFor('!document.querySelector("[data-shell-region=utility]")');
          assert.equal(await evaluate('document.activeElement===document.querySelector("[data-shell-terminal-focus]")'),true);
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
  for (const scenario of ['sidebar-controls','baseline','focus','sessions-toggle','sessions-browse','backend-loss','utility-sessions-focus','hidden-resize','empty-terminal-return','preferences-migration','preferences-recovery','reset-confirmation','utility-scope']) test('fixed shell: '+scenario, async () => {
    const require = createRequire(import.meta.url);
    const root = fileURLToPath(new URL('../', import.meta.url));
    const directory = mkdtempSync(join(tmpdir(), 'orkworks-shell-'));
    try {
      const bundle = await build({
        stdin: { contents: `
          import React, {useState} from 'react';
          import {createRoot} from 'react-dom/client';
          import ApplicationShell from './src/components/ApplicationShell';
          import {WorkspaceHistoryDropdown} from './src/components/WorkspaceHistoryDropdown';
          import ShellPreferencesNotice from './src/components/ShellPreferencesNotice';
          import {useShellPreferences} from './src/useShellPreferences';
          import './src/styles/tokens.css';
          import './src/App.css';
          import {getTerminal, getLiveTerminalCount} from './src/terminalStore';
          const fixture = window.fixture = {selections:0,prompts:0,settingsOpened:0,sessionsCreated:0,errors:[],ready:true};
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
          window.orkworks = {notifyPanelVisibility:()=>{},getBackendUrl:()=>Promise.resolve('http://127.0.0.1:12345'),getWorkspaceHistory:async()=>({pinned:[],recent:['/tmp/test','/tmp/other'],diagnostic:null})};
          fixture.noticeCommands=[];fixture.preferenceWrites=0;fixture.preferenceReads=0;fixture.rebuildCalls=0;fixture.resetCalls=0;fixture.cancelRebuild=true;
          fixture.snapshot={preferences:{sessionsWidth:240,inspectorWidth:320,sessionsVisible:true,density:'low'},revision:0,diagnostic:${JSON.stringify(scenario)}==='preferences-recovery'?'corrupt_record':null};
          Object.assign(window.orkworks,{
            getShellLayout:async()=>{fixture.preferenceReads++;return {...fixture.snapshot}},
            getLayout:async()=>'{}',
            saveShellLayout:async preferences=>{fixture.preferenceWrites++;fixture.snapshot={preferences,revision:fixture.snapshot.revision+1,diagnostic:null};return {ok:true}},
            resetShellLayout:async()=>{fixture.resetCalls++;return {ok:false}},
            rebuildShellLayout:async()=>{fixture.rebuildCalls++;if(fixture.cancelRebuild)return {ok:false,diagnostic:'user_cancelled'};fixture.snapshot.diagnostic=null;return {ok:true}}
          });
          function Preferences() {const prefs=useShellPreferences();return <ShellPreferencesNotice {...prefs} onCommand={command=>{fixture.noticeCommands.push(command);void prefs.reset()}}/>;}
          function PreferencesHarness() {const [generation,setGeneration]=useState(0);fixture.remountPreferences=()=>setGeneration(n=>n+1);return <Preferences key={generation}/>;}
          function Harness() {
            const [inspector,setInspector]=useState(null);
            const [backendStatus,setBackendStatus]=useState('connected');
            const [commandRequest,setCommandRequest]=useState(null);
            fixture.command=command=>setCommandRequest(old=>({command,sequence:(old?.sequence||0)+1}));
            fixture.setBackendStatus=setBackendStatus;
            const [activeSessionId,setActiveSessionId]=useState('coding');
            fixture.activeSessionId=activeSessionId;
            fixture.select=setActiveSessionId;
            const [preferences,setPreferences]=useState({sessionsWidth:240,inspectorWidth:320,sessionsVisible:true,density:'low'});
            return <ApplicationShell sessions={(${JSON.stringify(scenario)}==='sidebar-controls'?['coding','other',...Array.from({length:80},(_,i)=>'history-'+i)]:['coding','other']).map((id,i)=>({id,name:id,harnessId:'codex',harness:'codex',lifecycle:'alive',status:'running',label:id,createdAt:'2026-10-10T10:00:00Z',lastActivityAt:i?'2026-10-10T09:00:00Z':'2026-10-10T10:00:00Z'}))}
              workspace={{path:'/tmp/test'}} workspaceControl={<div className="shell-workspace-control"><span className="titlebar-text">test</span><WorkspaceHistoryDropdown triggerLabel="⇄" triggerClassName="titlebar-switch-button" triggerAriaLabel="Switch workspace" currentWorkspacePath="/tmp/test" isSwitching={false} onOpenPath={()=>{}} onOpenOtherFolder={()=>{}} onError={()=>{}}/></div>} onOpenSettings={()=>fixture.settingsOpened++} onCreateSession={()=>fixture.sessionsCreated++} activeSessionId={activeSessionId} workspaceGeneration={0}
              backendStatus={backendStatus} harnesses={[]} debugSettings={{showSessionIds:false}}
              commandRequest={commandRequest} preferences={preferences} onPreferencesChange={setPreferences} onResetPreferences={()=>new Promise(resolve=>{fixture.finishReset=resolve})}
              inspector={inspector} onInspect={setInspector} unreadIds={new Set()} acknowledgedIds={new Set()}
              onSelectSession={id=>{fixture.selections++;setActiveSessionId(id)}} onFocusTerminal={()=>fixture.runtime()?.terminal.focus()} onBackendUnavailable={()=>{}} />;
          }
          createRoot(document.getElementById('root')).render(${JSON.stringify(scenario)}.startsWith('preferences-')?<PreferencesHarness/>:<Harness/>);
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
