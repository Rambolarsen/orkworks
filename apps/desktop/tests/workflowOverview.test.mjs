import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";

if (process.versions.electron) {
  const { app, BrowserWindow } = await import("electron");
  app.disableHardwareAcceleration();
  app.whenReady().then(async () => {
    const win = new BrowserWindow({ show: false, width: 1280, height: 900, webPreferences: { nodeIntegration: false, contextIsolation: true } });
    const evaluate = async source => { try { return await win.webContents.executeJavaScript(source); } catch (error) { throw new Error(`Renderer evaluation failed: ${source}`, { cause: error }); } };
    const wait = async source => {
      const deadline = Date.now() + 5000;
      while (Date.now() < deadline) {
        if (await evaluate(source)) return;
        await new Promise(resolve => setTimeout(resolve, 15));
      }
      throw new Error(`Timed out: ${source}\n${await evaluate("document.body.innerText")}\nACTIVE=${await evaluate("document.activeElement.outerHTML")}`);
    };
    const click = selector => evaluate(`document.querySelector(${JSON.stringify(selector)}).click()`);
    const key = value => evaluate(`document.activeElement.dispatchEvent(new KeyboardEvent('keydown',{key:${JSON.stringify(value)},bubbles:true}))`);
    try {
      await win.loadFile(process.env.ORKWORKS_WORKFLOW_FIXTURE);
      await wait('document.querySelector("[role=treeitem]")');
      assert.match(await evaluate('document.body.innerText'), /Synthetic workflow preview/);
      assert.equal(await evaluate('document.querySelectorAll("[role=tree]").length'), 1);
      assert.equal(await evaluate(`document.querySelectorAll('[role=treeitem][tabindex="0"]').length`), 1);
      assert.equal(await evaluate('document.querySelectorAll(".xterm").length'), 0);
      await evaluate('document.querySelector("[role=treeitem]").focus()');
      await key('ArrowDown'); await key('ArrowDown');
      await wait('document.activeElement.dataset.kind === "task"');
      assert.equal(await evaluate('document.querySelector("[aria-selected=true]").dataset.kind'), 'run', 'focus is not inspection');
      await key(' ');
      await wait('document.querySelector("[aria-selected=true]").dataset.kind === "task"');
      assert.match(await evaluate('document.querySelector("[data-workflow-details]").innerText'), /Delivery unconfirmed/);
      assert.match(await evaluate('document.querySelector("[data-workflow-details]").innerText'), /Reported use/);
      assert.equal(await evaluate('document.querySelector("[aria-selected=true] .workflow-skill").dataset.delivery'), 'unconfirmed', 'reported usage must not confirm loading');
      assert.equal(await evaluate('document.querySelectorAll("[data-workflow-details] .workflow-skill-detail").length'), 4, 'details disclose overflow skills');
      assert.match(await evaluate('document.querySelector("[data-workflow-details]").innerText'), /Evidence unavailable · Usage unknown/);
      assert.equal(await evaluate('document.querySelector("[data-terminal-command]").disabled'), true);
      await key('Enter');
      await wait('document.activeElement.hasAttribute("data-details-heading")');
      await key('Escape');
      await wait('document.activeElement.dataset.kind === "task"');
      await click('[data-view=tree]');
      assert.equal(await evaluate('document.querySelector("[aria-selected=true]").dataset.kind'), 'task');
      // Collapsing a stage with an inspected task is refused even after focus leaves it.
      await evaluate('(()=>{const el=document.querySelector("[role=treeitem][data-kind=stage]");el.focus();el.dispatchEvent(new FocusEvent("focusin",{bubbles:true}))})()');
      await wait('document.querySelector("[data-disclosure]")');
      await click('[data-disclosure]');
      await wait('document.querySelector("[data-collapse-reasons]")');
      assert.match(await evaluate('document.querySelector("[data-collapse-reasons]").innerText'), /Inspect the run/);
      await click('[data-inspect-run]');
      await evaluate('(()=>{const el=document.querySelector("[role=treeitem][data-kind=stage]");el.focus();el.dispatchEvent(new FocusEvent("focusin",{bubbles:true}))})()');
      await key('ArrowLeft');
      await wait('document.querySelector("[role=treeitem][data-kind=stage]").getAttribute("aria-expanded")==="false"');
      assert.match(await evaluate('document.querySelector("[role=treeitem][data-kind=stage]").innerText'), /1 blocker/);
      await key('ArrowRight');
      await key('Home'); await key('End');
      await key(' ');
      await wait('document.querySelector("[data-workflow-details]").innerText.includes("Planned")');
      assert.equal(await evaluate('!!document.querySelector("[data-terminal-command]")'), false);
      // Scenario change resets selection and malicious-looking labels stay text.
      await evaluate('(()=>{const s=document.querySelector("[data-scenario]");s.value="edge";s.dispatchEvent(new Event("change",{bubbles:true}))})()');
      await wait('document.querySelector("[aria-selected=true]").dataset.kind === "run"');
      assert.equal(await evaluate('!!document.querySelector("img[data-injected]")'), false);
      assert.match(await evaluate('document.body.innerText'), /Parent session unavailable/);
      await evaluate('document.querySelector("[role=treeitem]").focus()');
      await key('m');
      await wait('document.activeElement.innerText.includes("Missing session")');
      await key(' ');
      assert.equal(await evaluate('!!document.querySelector("[data-terminal-command]")'), false);
      // Dense content and real layout at logical widths, including text zoom.
      await evaluate('(()=>{const s=document.querySelector("[data-scenario]");s.value="dense";s.dispatchEvent(new Event("change",{bubbles:true}))})()');
      await wait('document.querySelectorAll("[data-kind=task]").length >= 30');
      for (const width of [1280, 1000, 700, 360]) {
        await win.setContentSize(width, 900);
        await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
        assert.ok(await evaluate('document.documentElement.scrollWidth <= innerWidth + 1'), `no horizontal overflow at ${width}`);
      }
      await evaluate('document.documentElement.style.fontSize="200%"');
      await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
      assert.ok(await evaluate('document.documentElement.scrollWidth <= innerWidth + 1'), 'no overflow at 200% text zoom');
      assert.ok(await evaluate('[...document.querySelectorAll("button,select")].every(el=>el.getBoundingClientRect().right<=innerWidth+1)'), 'actions remain in viewport');
      await evaluate('document.documentElement.style.fontSize=""');
      win.webContents.debugger.attach('1.3');
      await win.webContents.debugger.sendCommand('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
      assert.equal(await evaluate('matchMedia("(prefers-reduced-motion: reduce)").matches'), true);
      assert.ok(await evaluate('[...document.querySelectorAll(".workflow-skill")].every(el=>getComputedStyle(el).animationName==="none")'));
      // Capture actual component output for visual inspection outside the test.
      if (process.env.ORKWORKS_WORKFLOW_CAPTURE) {
        const { writeFileSync } = await import('node:fs');
        await evaluate('window.scrollTo(0,0);new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
        writeFileSync(process.env.ORKWORKS_WORKFLOW_CAPTURE + '-compact.png', (await win.webContents.capturePage()).toPNG());
        await win.setContentSize(1280,900);
        await evaluate('(()=>{const s=document.querySelector("[data-scenario]");s.value="mixed";s.dispatchEvent(new Event("change",{bubbles:true}))})()');
        await wait('document.querySelectorAll("[data-kind=task]").length < 30');
        await evaluate('window.scrollTo(0,0);new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
        writeFileSync(process.env.ORKWORKS_WORKFLOW_CAPTURE + '-wide.png', (await win.webContents.capturePage()).toPNG());
      }
      assert.deepEqual(await evaluate('window.previewViolations'), []);
      app.exit(0);
    } catch (error) { console.error(error); app.exit(1); }
  });
} else {
  const { test } = await import('node:test');
  const { mkdtempSync, writeFileSync, rmSync } = await import('node:fs');
  const { join, dirname, resolve } = await import('node:path');
  const { tmpdir } = await import('node:os');
  const { spawnSync } = await import('node:child_process');
  const { createRequire } = await import('node:module');
  test('hierarchy preview preserves inspection, keyboard, collapse, evidence and narrow layout without backend access', async () => {
    const directory = mkdtempSync(join(tmpdir(),'orkworks-workflow-'));
    const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
    const require = createRequire(import.meta.url);
    try {
      const { build } = await import('esbuild');
      const result = await build({ entryPoints:[join(root,'tests/fixtures/workflowPreview.tsx')], bundle:true, write:false, outdir:directory,
        format:'iife', platform:'browser', jsx:'automatic', define:{'process.env.NODE_ENV':'"development"'} });
      const script = result.outputFiles.find(file=>file.path.endsWith('.js')).text;
      const css = result.outputFiles.find(file=>file.path.endsWith('.css'))?.text ?? '';
      const html = join(directory,'index.html');
      writeFileSync(html, `<meta name="viewport" content="width=device-width,initial-scale=1"><style>${css}</style><div id="root"></div><script>window.previewViolations=[];window.fetch=()=>{previewViolations.push('fetch');throw Error('Unexpected fetch')};window.WebSocket=class{constructor(){previewViolations.push('socket');throw Error('Unexpected socket')}};Object.defineProperty(window,'orkworks',{get(){previewViolations.push('bridge');throw Error('Unexpected bridge')}});window.addEventListener('error',e=>previewViolations.push(e.message));</script><script>${script}</script>`);
      const env = {...process.env, ORKWORKS_WORKFLOW_FIXTURE:html}; delete env.ELECTRON_RUN_AS_NODE;
      const electron = require('electron');
      const headless = process.platform === 'linux' && !env.DISPLAY;
      const args = [fileURLToPath(import.meta.url)];
      if(process.platform==='linux') args.unshift('--no-sandbox');
      if(headless) args.unshift('-a',electron);
      const child=spawnSync(headless?'xvfb-run':electron,args,{env,encoding:'utf8',timeout:60000,windowsHide:true});
      assert.ifError(child.error); assert.equal(child.status,0,child.stdout+child.stderr);
    } finally { rmSync(directory,{recursive:true,force:true}); }
  });
}
