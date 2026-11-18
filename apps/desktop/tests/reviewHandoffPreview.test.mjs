import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { javascriptLiteral } from "./helpers/javascriptLiteral.mjs";

if (process.versions.electron) {
  const { app, BrowserWindow } = await import("electron");
  app.disableHardwareAcceleration();
  console.log("handoff fixture: starting");
  app.whenReady().then(async () => {
  console.log("handoff fixture: ready");
  const win = new BrowserWindow({ show: false, width: 1280, height: 1000, webPreferences: { nodeIntegration: false, contextIsolation: true } });
  const evaluate = source => win.webContents.executeJavaScript(`(()=>{const result=eval((${javascriptLiteral(source)}));return result instanceof Node ? "__NODE__" : result})()`);
  const wait = async source => {
    const deadline = Date.now() + 5000;
    while (Date.now() < deadline) {
      if (await evaluate(source)) return;
      await new Promise(resolve => setTimeout(resolve, 20));
    }
    throw Error(`Timeout: ${source}\n${await evaluate('document.body.innerText')}`);
  };
  const click = selector => evaluate(`document.querySelector(${javascriptLiteral(selector)}).click()`);
  const select = value => evaluate(`(()=>{const el=document.querySelector('[data-handoff-scenario]');el.value=${javascriptLiteral(value)};el.dispatchEvent(new Event('change',{bubbles:true}))})()`);
  try {
    await win.loadFile(process.env.ORKWORKS_HANDOFF_FIXTURE);
    await wait('document.querySelector("[data-scenario]")');
    await evaluate('(()=>{const s=document.querySelector("[data-scenario]");s.value="handoff";s.dispatchEvent(new Event("change",{bubbles:true}))})()');
    await wait('document.querySelector("[data-handoff-open]")');
    const open = async () => { await click('[data-handoff-open]'); await wait('document.activeElement.matches("[data-handoff-heading]")'); };
    await open();
    assert.match(await evaluate('document.body.innerText'), /Synthetic review handoff preview/);
    assert.match(await evaluate('document.querySelector("[data-handoff-detail]").innerText'), /S41/);
    assert.match(await evaluate('document.body.innerText'), /S42/);
    assert.equal(await evaluate('document.querySelector("[data-handoff-instruction]").open'), true);
    assert.equal(await evaluate('document.querySelector("[data-handoff-continue]").disabled'), true);
    await click('[data-handoff-instruction] summary');
    assert.equal(await evaluate('document.querySelector("[data-handoff-instruction]").open'), false);
    await click('[data-handoff-instruction] summary');
    await evaluate('document.querySelector("[data-handoff-refresh]").focus()');
    await click('[data-change-plan]');
    await wait('document.querySelector("[data-handoff-status]").innerText.includes("Plan changed")');
    assert.match(await evaluate('document.querySelector("[data-handoff-detail]").innerText'), /version A/);
    assert.equal(await evaluate('document.activeElement.matches("[data-handoff-refresh]")'), true, 'background change cannot steal focus');
    await click('[data-handoff-refresh]');
    await wait('document.querySelector("[data-handoff-detail]").innerText.includes("version B")');
    await wait('document.activeElement.matches("[data-handoff-status]")');
    await click('[data-delay-refresh]');
    await click('[data-handoff-refresh]');
    await click('[data-handoff-refresh]');
    await click('[data-complete-newest]');
    await wait('document.querySelector("[data-handoff-status]").innerText.includes("refreshed")');
    await click('[data-complete-oldest]');
    assert.match(await evaluate('document.querySelector("[data-handoff-detail]").innerText'), /version B/);
    await click('[data-handoff-refresh]');
    await click('[data-handoff-close]');
    await wait('document.activeElement.matches("[data-handoff-open]")');
    await click('[data-complete-oldest]');
    assert.equal(await evaluate('!!document.querySelector("[data-handoff-detail]")'), false);
    await open();
    await click('[data-handoff-refresh]');
    await click('[data-replace-runtime]');
    await click('[data-complete-oldest]');
    assert.equal(await evaluate('!!document.querySelector("[data-handoff-detail]")'), false);
    await open();
    await click('[data-handoff-refresh]');
    await click('[data-replace-workspace]');
    await click('[data-complete-oldest]');
    assert.equal(await evaluate('!!document.querySelector("[data-handoff-detail]")'), false);
    await open();
    await click('[data-remove-recommendation]');
    await wait('document.activeElement.matches("[data-recommendations-heading]")');
    assert.equal(await evaluate('!!document.querySelector("[data-handoff-open]")'), false);
    await select('missing'); await open();
    assert.match(await evaluate('document.querySelector("[data-handoff-detail]").innerText'), /No plan selected/);
    await select('unavailable'); await open();
    assert.match(await evaluate('document.querySelector("[data-handoff-detail]").innerText'), /Plan unavailable/);
    assert.match(await evaluate('document.querySelector("[data-handoff-detail]").innerText'), /Working location unavailable/);
    await select('long'); await open();
    assert.equal(await evaluate('!!document.querySelector("img[data-injected]")'), false);
    assert.match(await evaluate('document.querySelector("[data-handoff-instruction]").innerText'), /<img data-injected/);
    assert.ok(await evaluate('document.querySelector("[data-handoff-instruction] pre").innerText.length > 2000'));
    for (const width of [1280, 700, 360]) {
      win.setContentSize(width, 1000);
      await evaluate('new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');
      assert.ok(await evaluate('document.documentElement.scrollWidth <= innerWidth + 1'), `no page overflow at ${width}`);
    }
    await evaluate('document.documentElement.style.fontSize="200%"');
    await evaluate('new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');
    assert.ok(await evaluate('document.documentElement.scrollWidth <= innerWidth + 1'), 'no page overflow at 200%');
    assert.ok(await evaluate('[...document.querySelectorAll("button,select,summary")].every(el=>el.getBoundingClientRect().right<=innerWidth+1)'), 'all controls fit');
    win.webContents.debugger.attach('1.3');
    await win.webContents.debugger.sendCommand('Emulation.setEmulatedMedia', { features: [{ name:'prefers-reduced-motion',value:'reduce' }] });
    assert.equal(await evaluate('getComputedStyle(document.querySelector("[data-handoff-detail]")).animationName'), 'none');
    await evaluate('document.querySelector("[data-handoff-refresh]").focus()');
    win.webContents.sendInputEvent({ type:'keyDown',keyCode:'Tab' }); win.webContents.sendInputEvent({ type:'keyUp',keyCode:'Tab' });
    await wait('!document.querySelector("[data-handoff-detail]").contains(document.activeElement)');
    await evaluate('document.querySelector("[data-handoff-close]").focus();document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
    await wait('document.activeElement.matches("[data-handoff-open]")');
    assert.equal(await evaluate('document.querySelectorAll(".xterm").length'), 0);
    assert.deepEqual(await evaluate('window.previewViolations'), []);
    if (process.env.ORKWORKS_HANDOFF_CAPTURE) {
      const { writeFileSync } = await import('node:fs');
      await evaluate('document.documentElement.style.fontSize=""');
      await select('current'); await open();
      for (const [name,width] of [['wide',1280],['compact',360]]) {
        win.setContentSize(width,1000);
        await evaluate('window.scrollTo(0,0);new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');
        writeFileSync(`${process.env.ORKWORKS_HANDOFF_CAPTURE}-${name}.png`,(await win.webContents.capturePage()).toPNG());
        await evaluate('document.querySelector("[data-handoff-continue]").scrollIntoView({block:"end"});new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');
        writeFileSync(`${process.env.ORKWORKS_HANDOFF_CAPTURE}-${name}-instruction.png`,(await win.webContents.capturePage()).toPNG());
      }
    }
    app.exit(0);
  } catch(error) { console.error(error); app.exit(1); }
  });
} else {
  const { test } = await import('node:test');
  const { mkdtempSync,writeFileSync,rmSync } = await import('node:fs');
  const { tmpdir } = await import('node:os');
  const { join } = await import('node:path');
  const { createRequire } = await import('node:module');
  const { spawnSync } = await import('node:child_process');
  test('handoff preview preserves identity, stale context, focus and compact layout without capabilities',async()=>{
    const directory=mkdtempSync(join(tmpdir(),'orkworks-review-handoff-'));
    try {
      const { build } = await import('esbuild');
      const result=await build({entryPoints:[fileURLToPath(new URL('./fixtures/workflowPreview.tsx',import.meta.url))],bundle:true,write:false,outdir:directory,format:'iife',platform:'browser',jsx:'automatic',define:{'process.env.NODE_ENV':'"development"'}});
      const js=result.outputFiles.find(f=>f.path.endsWith('.js')).text;
      const css=result.outputFiles.find(f=>f.path.endsWith('.css'))?.text??'';
      const html=join(directory,'index.html');
      writeFileSync(html,`<meta name="viewport" content="width=device-width,initial-scale=1"><style>${css}</style><div id="root"></div><script>window.previewViolations=[];window.fetch=()=>{previewViolations.push('fetch');throw Error('Unexpected fetch')};window.WebSocket=class{constructor(){previewViolations.push('socket');throw Error('Unexpected socket')}};Object.defineProperty(window,'orkworks',{get(){previewViolations.push('bridge');throw Error('Unexpected bridge')}});for(const name of ['localStorage','sessionStorage'])Object.defineProperty(window,name,{get(){previewViolations.push(name);throw Error('Unexpected storage')}});window.addEventListener('error',e=>previewViolations.push(e.message));</script><script>${js}</script>`);
      const electron=createRequire(import.meta.url)('electron');
      const env={...process.env,ORKWORKS_HANDOFF_FIXTURE:html};delete env.ELECTRON_RUN_AS_NODE;
      const headless=process.platform==='linux'&&!env.DISPLAY;
      const args=[fileURLToPath(import.meta.url)];if(process.platform==='linux')args.unshift('--no-sandbox');if(headless)args.unshift('-a',electron);
      const child=spawnSync(headless?'xvfb-run':electron,args,{env,encoding:'utf8',timeout:60000,windowsHide:true});
      assert.ifError(child.error && Object.assign(child.error,{message:child.error.message+"\n"+child.stdout+child.stderr}));assert.equal(child.status,0,child.stdout+child.stderr);
    }finally{rmSync(directory,{recursive:true,force:true});}
  });
}
