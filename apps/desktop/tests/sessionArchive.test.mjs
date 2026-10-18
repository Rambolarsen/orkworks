import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

if (process.versions.electron) {
  const { app, BrowserWindow } = await import('electron');
  app.disableHardwareAcceleration();
  app.whenReady().then(async () => {
    const win = new BrowserWindow({ show: false, width: 460, height: 680 });
    const evaluate = async source => {
      try { return await win.webContents.executeJavaScript(source); }
      catch (error) { throw new Error(`Renderer evaluation failed: ${source}`, { cause: error }); }
    };
    const wait = async source => {
      const deadline = Date.now() + 5000;
      while (Date.now() < deadline) { if (await evaluate(`Boolean(${source})`)) return; await new Promise(resolve => setTimeout(resolve, 10)); }
      throw new Error(`Timed out: ${source}\n${await evaluate('document.body.innerText')}`);
    };
    let exitCode = 0;
    try {
      await win.loadFile(process.env.ORKWORKS_ARCHIVE_FIXTURE);
      await wait('window.fixture?.ready');
      const filter = async value => {
        await evaluate(`(()=>{const s=document.querySelector('select');s.value=${JSON.stringify(value)};s.dispatchEvent(new Event('change',{bubbles:true}))})()`);
      };
      const more = async id => {
        await evaluate(`document.querySelector('[data-session-id="${id}"] .session-row-more').click()`);
        await wait('document.querySelector("[role=menu]")');
      };
      const action = async prefix => {
        await evaluate(`Array.from(document.querySelectorAll('[role=menuitem]')).find(b=>b.textContent.startsWith(${JSON.stringify(prefix)})).click()`);
        await wait('!document.querySelector("[role=menu]")');
      };
      assert.deepEqual(await evaluate('Array.from(document.querySelectorAll("[data-session-id]")).map(e=>e.dataset.sessionId)'), ['live','closed']);
      await evaluate('fixture.selectHidden()');
      await evaluate('document.getElementById("sessions-list").focus();document.getElementById("sessions-list").dispatchEvent(new KeyboardEvent("keydown",{key:"F10",shiftKey:true,bubbles:true}))');
      assert.equal(await evaluate('!!document.querySelector("[role=menu]")'),false,'a hidden selection cannot target another visible row');
      await evaluate('fixture.selectLive()');
      await more('closed');
      assert.equal(await evaluate('fixture.selections.length'),0,'More does not select or acknowledge');
      assert.equal(await evaluate('document.activeElement.textContent'),'Archive');
      await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"End",bubbles:true}))');
      assert.equal(await evaluate('document.activeElement.textContent'),'Delete…');
      await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
      await wait('document.activeElement.classList.contains("session-row-more")');
      await evaluate('fixture.pauseAction=true');
      await more('closed'); await action('Archive');
      await wait('fixture.finishAction');
      assert.equal(await evaluate('document.querySelector("[data-session-id=closed] .session-row-more").disabled'),true);
      await evaluate('fixture.pauseAction=false;fixture.finishAction();fixture.finishAction=null');
      await wait('fixture.calls.length===1');
      assert.deepEqual(await evaluate('fixture.calls[0]'),['closed','archive','token-closed']);
      assert.equal(await evaluate('fixture.selections.length'),0);
      await wait('!document.querySelector("[data-session-id=closed]")');
      assert.equal(await evaluate('document.activeElement.id'), 'sessions-list', 'Archive returns focus after its invoker disappears');
      await filter('archived'); await wait('document.querySelector("[data-session-id=closed]")');
      assert.equal(await evaluate('fixture.selected'),'live','filter preserves selected history');
      await more('closed'); await action('Restore');
      await wait('fixture.calls.length===2');
      await filter('active'); await wait('document.querySelector("[data-session-id=live]")');
      await evaluate('document.querySelector("[data-session-id=live]").dispatchEvent(new MouseEvent("contextmenu",{bubbles:true,clientX:450,clientY:670}))');
      await wait('document.querySelector("[role=menu]")');
      assert.equal(await evaluate('fixture.selections.length'),0,'right-click targets without selection');
      assert.equal(await evaluate('document.querySelector("[role=menu]").getBoundingClientRect().right<=window.innerWidth'),true,'menu clamps at narrow width');
      await action('Delete');
      assert.equal(await evaluate('fixture.calls.length'),2,'cancelled confirmation does not close');
      assert.equal(await evaluate('fixture.confirmations.length'),1);
      await more('live'); await action('Archive');
      await wait('fixture.calls.length===3');
      assert.equal(await evaluate('fixture.calls[2][1]'),'archive');
      assert.equal(await evaluate('fixture.selected'),'live');
      await filter('archived'); await wait('document.querySelector("[data-session-id=live]")');
      await evaluate('document.getElementById("sessions-list").focus();document.getElementById("sessions-list").dispatchEvent(new KeyboardEvent("keydown",{key:"F10",shiftKey:true,bubbles:true}))');
      await wait('document.querySelector("[role=menu]")');
      assert.equal(await evaluate('document.activeElement.textContent'),'Restore to Closed');
      await evaluate('document.body.dispatchEvent(new PointerEvent("pointerdown",{bubbles:true}))');
      await wait('!document.querySelector("[role=menu]")');
      await evaluate('fixture.setActionsEnabled(false)');
      await more('live');
      assert.equal(await evaluate('Array.from(document.querySelectorAll("[role=menuitem]")).every(b=>b.disabled)'),true,'disabled admission disables menu commands');
      await evaluate('document.querySelector("[role=menu]").dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
      await evaluate('fixture.setActionsEnabled(true)');
      await win.webContents.setZoomFactor(1.5);
      await evaluate('new Promise(resolve=>requestAnimationFrame(()=>resolve(true)))');
      await more('live');
      await wait('document.querySelector("[role=menu]")?.getBoundingClientRect().right<=window.innerWidth');
      assert.equal(await evaluate('document.querySelector("[role=menu]").getBoundingClientRect().right<=window.innerWidth'),true,
        JSON.stringify(await evaluate('(()=>{const m=document.querySelector("[role=menu]");return {bounds:m.getBoundingClientRect().toJSON(),viewport:innerWidth,style:m.getAttribute("style")}})()')));
      if (process.env.ORKWORKS_ARCHIVE_SCREENSHOT) {
        const { writeFileSync } = await import('node:fs');
        writeFileSync(process.env.ORKWORKS_ARCHIVE_SCREENSHOT, (await win.capturePage()).toPNG());
      }
      await evaluate('void(window.confirm=()=>true)');
      await action('Delete');
      await wait('fixture.calls.length===4 && !document.querySelector("[data-session-id=live]")');
      assert.equal(await evaluate('document.activeElement.id'), 'sessions-list', 'Delete returns focus to Sessions');
      assert.deepEqual(await evaluate('fixture.errors'),[]);
      console.log('archive UI workflow passed');
    } catch (error) { console.error(error); exitCode = 1; }
    finally { app.exit(exitCode); }
  });
} else {
  const { default: test } = await import('node:test');
  const { createRequire } = await import('node:module');
  const { mkdtempSync, writeFileSync, rmSync } = await import('node:fs');
  const { tmpdir } = await import('node:os');
  const { join } = await import('node:path');
  const { spawnSync } = await import('node:child_process');
  const require = createRequire(import.meta.url);
  test('session state filters and exact-row menu preserve selection and confirm before deletion', async () => {
    const directory = mkdtempSync(join(tmpdir(), 'orkworks-archive-'));
    try {
      const { build } = await import('esbuild');
      const root = fileURLToPath(new URL('..', import.meta.url));
      const bundle = await build({ stdin: { contents: `
        import React,{useState} from 'react';import{createRoot}from'react-dom/client';
        import SessionListPanel from './src/components/SessionListPanel';import './src/styles/tokens.css';import './src/App.css';
        window.fixture={ready:true,calls:[],selections:[],confirmations:[],errors:[],selected:'live'};
        window.addEventListener('error',e=>fixture.errors.push(e.message));
        window.confirm=message=>{fixture.confirmations.push(message);return false};
        const row=(id,state)=>({id,label:id,sessionState:state,sessionActionToken:'token-'+id,lifecycle:state==='active'?'alive':'dead',lifecyclePhase:state==='active'?'active':'ended',status:state==='active'?'running':'ended',created_at:'2026-10-18T10:00:00Z',cwd:'/tmp',memoryState:'remembered',resumeStrategy:'none'});
        function Harness(){const[rows,setRows]=useState([row('live','active'),row('closed','closed'),row('saved','archived')]);const[selected,setSelected]=useState('live');const[enabled,setEnabled]=useState(true);fixture.setActionsEnabled=setEnabled;fixture.selectHidden=()=>setSelected('saved');fixture.selectLive=()=>setSelected('live');
          return <SessionListPanel workspace={{path:'/tmp',workspaceIdentity:'test'}} sessions={rows} activeSessionId={selected} unreadIds={new Set()} acknowledgedIds={new Set()} harnesses={[]}
            actionsEnabled={enabled} onSelectSession={id=>{fixture.selections.push(id);fixture.selected=id;setSelected(id)}} onFocusTerminal={()=>{}} onOpenWorkspace={()=>{}}
            onSessionAction={async(id,action,token)=>{fixture.calls.push([id,action,token]);if(fixture.pauseAction)await new Promise(resolve=>fixture.finishAction=resolve);setRows(rows=>action==='delete'?rows.filter(r=>r.id!==id):rows.map(r=>r.id===id?{...r,sessionState:action==='archive'?'archived':'closed',sessionActionToken:r.sessionActionToken+'-next',lifecycle:'dead',lifecyclePhase:'ended'}:r))}}/>}
        createRoot(document.getElementById('root')).render(<Harness/>);
      `, resolveDir: root, loader: 'tsx' }, bundle: true, write: false, outdir: directory, format: 'iife', platform: 'browser', define: { 'process.env.NODE_ENV': '"development"' } });
      const fixture = join(directory, 'index.html');
      writeFileSync(fixture, '<style>'+(bundle.outputFiles.find(f=>f.path.endsWith('.css'))?.text||'')+'</style><div id="root"></div><script>'+bundle.outputFiles.find(f=>f.path.endsWith('.js')).text+'</script>');
      const electron = require('electron');
      const env = { ...process.env, ORKWORKS_ARCHIVE_FIXTURE: fixture }; delete env.ELECTRON_RUN_AS_NODE;
      const args = [fileURLToPath(import.meta.url)];
      if (process.platform === 'linux') args.unshift('--no-sandbox');
      const headless = process.platform === 'linux' && !env.DISPLAY;
      if (headless) args.unshift('-a',electron);
      const result=spawnSync(headless?'xvfb-run':electron,args,{env,encoding:'utf8',timeout:30000,windowsHide:true});
      assert.ifError(result.error);assert.equal(result.status,0,result.stdout+result.stderr);
    } finally { rmSync(directory,{recursive:true,force:true}); }
  });
}
