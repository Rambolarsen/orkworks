import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
if (process.versions.electron) {
  const {app, BrowserWindow} = await import('electron');
  app.disableHardwareAcceleration();
  app.setPath('userData',process.env.ORKWORKS_IMAGE_UI_DIR+'/profile');
  app.whenReady().then(async()=>{
    const win = new BrowserWindow({show:false,width:1280,height:900});
    const evaluate = async source => { try { return await win.webContents.executeJavaScript(source); } catch(error) { throw new Error("Renderer evaluation failed: "+source, {cause:error}); } };
    win.webContents.on("console-message", event => { if (event.level === "error") console.error(event.message); });
    const wait = async source => {const deadline=Date.now()+6000;while(Date.now()<deadline){if(await evaluate(source))return;await new Promise(resolve=>setTimeout(resolve,20));}throw new Error('Timed out '+source+'\n'+await evaluate('document.body.innerText'));};
    try {
      await win.loadFile(process.env.ORKWORKS_IMAGE_UI_DIR+'/index.html');
      await wait('window.fixture && document.querySelector("#nsd-harness").value==="codex"');
      // Drop, paste (including mixed text), and picker all use real browser File/Blob decoding.
      await evaluate('fixture.drop([fixture.file("drop.png")])');
      await wait('document.querySelectorAll(".new-session-image-list li").length===1');
      await evaluate('fixture.paste(fixture.file("paste.png"),"inspect this")');
      await wait('document.querySelectorAll(".new-session-image-list li").length===2');
      assert.equal(await evaluate('document.querySelector("#nsd-prompt").value'),'inspect this');
      await evaluate('fixture.picker([fixture.file("picker.png")])');
      await wait('document.querySelectorAll(".new-session-image-list li").length===3');
      await evaluate('document.querySelector("input[type=file]").addEventListener("click",event=>{event.preventDefault();fixture.pickerClicks++});document.querySelector(".new-session-attach").focus()');
      assert.equal(await evaluate('(()=>{const button=document.querySelector(".new-session-attach");const event=new KeyboardEvent("keydown",{key:"Enter",bubbles:true,cancelable:true});button.dispatchEvent(event);if(!event.defaultPrevented)button.click();return event.defaultPrevented})()'),false);
      await wait("fixture.pickerClicks===1");
      assert.equal(await evaluate('fixture.pickerClicks'),1);
      assert.equal(await evaluate('document.querySelector("input[type=file]").value'),'');
      // Invalid batches must not partially append a valid item.
      await evaluate('fixture.drop([fixture.file("valid.png"),new File(["broken"],"bad.png",{type:"image/png"})])');
      await wait('!!document.querySelector("[role=alert]")');
      assert.equal(await evaluate('document.querySelectorAll(".new-session-image-list li").length'),3);
      // Remove then reselect the same file; ensure URLs are released.
      await evaluate('Array.from(document.querySelectorAll("button[aria-label]")).find(button=>button.getAttribute("aria-label")==="Remove picker.png").click()');
      await wait('document.querySelectorAll(".new-session-image-list li").length===2');
      await evaluate('fixture.picker([fixture.file("picker.png")])');
      await wait('document.querySelectorAll(".new-session-image-list li").length===3');
      await evaluate('fixture.fail=true;document.querySelector(".new-session-confirm").click()');
      await wait('document.querySelector("[role=alert]")?.textContent.includes("fixture creation failed")');
      assert.equal(await evaluate('document.querySelectorAll(".new-session-image-list li").length'),3);
      await evaluate('fixture.fail=false;document.querySelector(".new-session-confirm").click()');
      await wait('fixture.submitted.length===1');
      assert.deepEqual(await evaluate('fixture.submitted[0].initialImages.map(image=>image.name)'),['drop.png','paste.png','picker.png']);
      assert.equal(await evaluate('fixture.submitted[0].imageWorkspacePath'),'/tmp/image-workspace');
      assert.equal(await evaluate('fixture.submitted[0].initialImages.every(image=>image.dataBase64===fixture.png)'),true);
      // Unsupported tool retains the draft and blocks Start.
      await evaluate('(()=>{const select=document.querySelector("#nsd-harness");select.value="generic-shell";select.dispatchEvent(new Event("change",{bubbles:true}))})()');
      await wait('document.querySelector(".new-session-confirm").disabled');
      assert.equal(await evaluate('document.querySelectorAll(".new-session-image-list li").length'),3);
      const {writeFileSync} = await import('node:fs');
      writeFileSync((await import('node:path')).join((await import('node:os')).tmpdir(),'orkworks-image-composer.png'),(await win.webContents.capturePage()).toPNG());
      await evaluate('document.querySelector(".new-session-cancel").click()');
      await wait('!document.querySelector(".new-session-composer")');
      assert.ok(await evaluate('fixture.revoked>=4'));
      // A pending ingestion must not publish into a remounted composer/workspace.
      await evaluate('fixture.show();');
      await wait('!!document.querySelector(".new-session-composer")');
      await evaluate('fixture.delayedDrop()');
      await wait('document.querySelector(".new-session-confirm").disabled');
      await evaluate('fixture.hide();fixture.finishRead();fixture.show()');
      await wait('!!document.querySelector(".new-session-composer")');
      await new Promise(resolve=>setTimeout(resolve,200));
      assert.equal(await evaluate('document.querySelectorAll(".new-session-image-list li").length'),0);
      assert.deepEqual(await evaluate('fixture.errors'),[]);
    } catch(error) {console.error(error);app.exit(1);return;}
    win.destroy();app.quit();
  }).catch(error=>{console.error(error);app.exit(1)});
} else {
  const {default:test}=await import('node:test');
  test('image composer: paste, drop, picker, removal, retries, and cancellation',async()=>{
    const {mkdtempSync,writeFileSync,rmSync}=await import('node:fs');
    const {tmpdir}=await import('node:os');const {join}=await import('node:path');
    const {spawnSync}=await import('node:child_process');const {createRequire}=await import('node:module');
    const {build}=await import('esbuild');const require=createRequire(import.meta.url);
    const directory=mkdtempSync(join(tmpdir(),'orkworks-image-ui-'));
    try {
      const bundle=await build({stdin:{resolveDir:fileURLToPath(new URL('../',import.meta.url)),loader:'tsx',contents:`
        import React,{useState} from 'react';import{createRoot}from'react-dom/client';
        import Composer from './src/components/NewSessionDialog';import './src/styles/tokens.css';import './src/App.css';
        window.fixture={png:'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',submitted:[],revoked:0,pickerClicks:0,errors:[]};
        const fixture=window.fixture;
        window.addEventListener('error',event=>fixture.errors.push(event.message));
        const originalRevoke=URL.revokeObjectURL.bind(URL);URL.revokeObjectURL=url=>{fixture.revoked++;originalRevoke(url)};
        window.orkworks={getBackendUrl:async()=>'http://fixture',getProviderModels:async()=>({models:[]}),getHarnessIntegrationStatus:async()=>({ok:true,status:{toolDetected:true}})};
        window.fetch=async()=>new Response(JSON.stringify({version:1,supportedHarnesses:['codex']}));
        fixture.file=name=>new File([Uint8Array.from(atob(fixture.png),char=>char.charCodeAt(0))],name,{type:'image/png'});
        fixture.drop=files=>{const data=new DataTransfer();files.forEach(file=>data.items.add(file));document.querySelector('.new-session-composer').dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:data}))};
        fixture.paste=(file,text)=>{const data=new DataTransfer();data.items.add(file);if(text)data.setData('text/plain',text);const prompt=document.querySelector('#nsd-prompt');prompt.focus();prompt.dispatchEvent(new ClipboardEvent('paste',{bubbles:true,cancelable:true,clipboardData:data}))};
        fixture.picker=files=>{const data=new DataTransfer();files.forEach(file=>data.items.add(file));const input=document.querySelector('input[type=file]');input.files=data.files;input.dispatchEvent(new Event('change',{bubbles:true}))};
        fixture.delayedDrop=()=>{const file=fixture.file('late.png');const read=file.arrayBuffer.bind(file);file.arrayBuffer=()=>new Promise(resolve=>{fixture.finishRead=async()=>resolve(await read())});fixture.drop([file])};
        function App(){const[visible,setVisible]=useState(true);const[generation,setGeneration]=useState(0);fixture.hide=()=>setVisible(false);fixture.show=()=>{setGeneration(n=>n+1);setVisible(true)};return visible?<Composer key={generation} workspacePath='/tmp/image-workspace' harnesses={[{id:'codex',name:'Codex',launch:{command:'codex',args:[]}},{id:'generic-shell',name:'Shell',launch:{command:'sh',args:[]}}]} providerRuntime={null} onCancel={()=>setVisible(false)} onConfirm={async options=>{if(fixture.fail)throw new Error('fixture creation failed');fixture.submitted.push(options)}}/>:<p>Closed</p>}
        createRoot(document.getElementById('root')).render(<App/>);
      `},bundle:true,write:false,outdir:directory,format:'iife',platform:'browser',define:{'process.env.NODE_ENV':'"development"'}});
      writeFileSync(join(directory,'index.html'),'<style>'+bundle.outputFiles.find(file=>file.path.endsWith('.css')).text+'</style><div id="root"></div><script>'+bundle.outputFiles.find(file=>file.path.endsWith('.js')).text+'</script>');
      const env={...process.env,ORKWORKS_IMAGE_UI_DIR:directory};delete env.ELECTRON_RUN_AS_NODE;
      const electron=require('electron');const headless=process.platform==='linux'&&!env.DISPLAY;
      const args=[fileURLToPath(import.meta.url)];if(process.platform==='linux')args.unshift('--no-sandbox');if(headless)args.unshift('-a',electron);
      const result=spawnSync(headless?'xvfb-run':electron,args,{env,encoding:'utf8',timeout:30000});
      assert.ifError(result.error);assert.equal(result.status,0,result.stdout+result.stderr);
    } finally {rmSync(directory,{recursive:true,force:true});}
  });
}
