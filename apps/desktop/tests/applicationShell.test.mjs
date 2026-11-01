import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';

if (process.versions.electron) {
  const { app, BrowserWindow } = await import('electron');
  app.disableHardwareAcceleration();
  app.whenReady().then(async () => {
    const win = new BrowserWindow({ show: false, width: 1280, height: 700 });
    const evaluate = async (source) => {
      try { return await win.webContents.executeJavaScript(source); }
      catch (error) { throw new Error(`Renderer evaluation failed: ${source}`, { cause: error }); }
    };
    const waitFor = async (source) => {
      const deadline = Date.now() + 5000;
      while (Date.now() < deadline) {
        if (await evaluate(source)) return;
        await new Promise(resolve => setTimeout(resolve, 10));
      }
      throw new Error(`Timed out: ${source}\n${await evaluate("document.body.innerText")}\n${JSON.stringify(await evaluate("fixture.errors"))}`);
    };
    const fresh = async () => {
      await win.loadFile(process.env.ORKWORKS_SHELL_FIXTURE);
      await waitFor('window.fixture?.ready');
    };
    try {
      await fresh();
      const scenario = process.env.ORKWORKS_SHELL_SCENARIO;
      const click = async command => evaluate('(()=>{const b=document.querySelector("[data-shell-command=' + command + ']");b.focus();b.click()})()');
      if (scenario === 'app-review-branch-documents') {
        await waitFor('document.querySelector("[data-shell-selected-session]") || fixture.sessionList.length===2');
        await click('details');
        await waitFor('document.querySelector("[data-document-kind=design]")');
        assert.deepEqual(await evaluate('[...document.querySelectorAll(".branch-document-label")].map(el=>el.textContent)'), ['Design / specification','Implementation plan']);
        await evaluate('document.querySelector("[data-document-kind=design]").click()');
        await waitFor('document.querySelector(".review-plan-content")?.textContent.includes("Design document")');
        assert.deepEqual(await evaluate('fixture.documentKinds'), ['design']);
        assert.equal(await evaluate('fixture.prompts'),0);
        await click('details');
        await evaluate('document.querySelector("[data-document-kind=implementation]").click()');
        await waitFor('document.querySelector(".review-plan-content")?.textContent.includes("Implementation document")');
        assert.deepEqual(await evaluate('fixture.documentKinds'), ['design','implementation']);
        await evaluate('document.querySelector("[data-review-refresh]").click()');
        await waitFor('fixture.documentKinds.length===3');
        assert.equal(await evaluate('fixture.documentKinds[2]'),'implementation');
        await evaluate('document.querySelector("[data-review-request]").click()');
        await waitFor('fixture.prompts===1');
        assert.equal(await evaluate('fixture.promptKind'),'implementation');
        await evaluate('fixture.finishPrompt()');
        if (!await evaluate('!!document.querySelector("[data-document-kind=design]")')) await click('details');
        await waitFor('document.querySelector("[data-document-kind=design]")');
        await evaluate('fixture.deferRead=true;document.querySelector("[data-document-kind=design]").click()');
        await waitFor('fixture.documentKinds.length===4 && !!fixture.resolveRead');
        await evaluate('fixture.oldBranchRead=fixture.resolveRead;fixture.deferRead=false;fixture.sessionList[0]={...fixture.sessionList[0],branch:"next-work",planDocuments:{...fixture.sessionList[0].planDocuments,branch:"next-work"}}');
        await waitFor('fixture.documentKinds.length===5 && document.querySelector(".review-plan-content")?.textContent.includes("Implementation document")');
        await evaluate('fixture.oldBranchRead("# Stale prior branch design")');
        assert.equal(await evaluate('document.body.textContent.includes("Stale prior branch design")'),false);
        await evaluate('fixture.currentImplementation="# Newer implementation";fixture.sessionList[0]={...fixture.sessionList[0],planDocuments:{...fixture.sessionList[0].planDocuments,steps:fixture.sessionList[0].planDocuments.steps.map(step=>step.kind==="implementation"?{...step,title:"Newer implementation",selectionId:"b".repeat(64)}:step)}}');
        await waitFor('fixture.documentKinds.length===6 && document.querySelector(".review-plan-content")?.textContent.includes("Newer implementation")');
        assert.equal(await evaluate('fixture.documentSelectionIds.at(-1)'), 'b'.repeat(64));
        await evaluate('document.querySelector("[data-review-request]").click()');
        await waitFor('fixture.prompts===2');
        assert.equal(await evaluate('fixture.promptSelectionId'), 'b'.repeat(64));
        await evaluate('fixture.finishPrompt()');
        if (process.env.ORKWORKS_CAPTURE_BRANCH_DOCUMENTS) {
          const {writeFileSync}=await import('node:fs');
          if (!await evaluate('!!document.querySelector("[data-document-kind=design]")')) await click('details');
          await waitFor('document.querySelector("[data-document-kind=design]")');
          writeFileSync(process.env.ORKWORKS_CAPTURE_BRANCH_DOCUMENTS,(await win.webContents.capturePage()).toPNG());
          win.setSize(640,700);
          await waitFor('window.innerWidth===640 && document.querySelector(".shell-layout")?.dataset.mode==="compact"');
          if (!await evaluate('!!document.querySelector("[data-document-kind=design]")')) await click('details');
          await waitFor('document.querySelector("[data-document-kind=design]")');
          writeFileSync(process.env.ORKWORKS_CAPTURE_BRANCH_DOCUMENTS.replace('.png','-narrow.png'),(await win.webContents.capturePage()).toPNG());
        }
        await evaluate('fixture.unreadable=true;fixture.sessionList[0]={...fixture.sessionList[0],planDocuments:{...fixture.sessionList[0].planDocuments,steps:fixture.sessionList[0].planDocuments.steps.map(step=>step.kind==="implementation"?{...step,pinned:true,available:false}:step)}}');
        await waitFor('document.body.innerText.includes("This plan is no longer available.")');
        assert.equal(await evaluate('!!document.querySelector(".review-plan-content")'),false);
        await evaluate('fixture.readsBeforeRetry=fixture.documentKinds.length;document.querySelector("[data-review-refresh]").click()');
        await waitFor('fixture.documentKinds.length===fixture.readsBeforeRetry+1 && document.body.innerText.includes("This plan is no longer available.")');
        assert.equal(await evaluate('fixture.documentKinds.at(-1)'), 'implementation');
        await evaluate('fixture.unreadable=false;fixture.sessionList[0]={...fixture.sessionList[0],planDocuments:{...fixture.sessionList[0].planDocuments,steps:fixture.sessionList[0].planDocuments.steps.map(step=>({...step,available:true}))}}');
        await waitFor('document.querySelector(".review-plan-content")?.textContent.includes("Newer implementation")');
        await evaluate('fixture.unreadable=true;fixture.sessionList[0]={...fixture.sessionList[0],hasOpenablePlan:false,planDocuments:{...fixture.sessionList[0].planDocuments,steps:fixture.sessionList[0].planDocuments.steps.map(step=>({...step,available:false,pinned:false,selectionId:null}))}}');
        await waitFor('document.body.innerText.includes("This plan is no longer available.")');
        assert.equal(await evaluate('document.body.innerText.includes("Loading plan…")'),false);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'session-menu') {
        await evaluate('fixture.setSessionState({hasOpenablePlan:true,repoRoot:"/tmp/test",branch:"feature",changedFiles:3})');
        await click('details');
        await waitFor('document.querySelector("nav.session-menu")');
        assert.equal(await evaluate('!!document.querySelector(".session-detail details[open]")'), false, 'session information starts collapsed');
        assert.deepEqual(await evaluate('[...document.querySelectorAll(".session-menu-label")].map(el=>el.textContent)'), ['Plan','Changes','Activity']);
        if (process.env.ORKWORKS_CAPTURE_SESSION_MENU) {
          const {writeFileSync}=await import('node:fs');
          writeFileSync(process.env.ORKWORKS_CAPTURE_SESSION_MENU,(await win.webContents.capturePage()).toPNG());
        }
        await evaluate('document.querySelector("[data-session-activity]").click()');
        await waitFor('document.querySelector("[data-session-menu-back]")===document.activeElement');
        await waitFor('document.querySelector(".detail-task-history")?.textContent.includes("Checkpoint coding")');
        await evaluate('document.querySelector("[data-session-menu-back]").click()');
        await waitFor('document.querySelector("[data-session-activity]")===document.activeElement');
        await evaluate('document.querySelector(".session-info summary").click()');
        await waitFor('document.querySelector(".session-info").open');
        await evaluate('document.querySelector("[data-session-activity]").click();fixture.select("other")');
        await waitFor('document.querySelector("[data-session-activity]")');
        assert.equal(await evaluate('document.querySelector(".session-info").open'), false, 'switching subjects resets disclosures and activity');
        await evaluate('document.querySelector("[data-session-activity]").click()');
        await waitFor('document.querySelector(".detail-task-history")?.textContent.includes("Checkpoint other")');
        assert.equal(await evaluate('document.querySelector(".detail-task-history").textContent.includes("Checkpoint coding")'), false);
        await evaluate('document.querySelector("[data-session-menu-back]").click()');
        await evaluate('document.querySelector("[data-session-plan]").click()');
        await waitFor('document.querySelector("[data-shell-region=review]")');
        assert.equal(await evaluate('fixture.prompts'), 0, 'opening a plan never requests execution or independent review');
        await evaluate('fixture.setSessionState({});fixture.command("terminal");fixture.command("details",true)');
        await waitFor('document.querySelector("[data-session-plan]")?.disabled');
        assert.equal(await evaluate('document.querySelector("[data-review-changes]").disabled'), true, 'unavailable destinations explain their disabled state');
        assert.deepEqual(await evaluate('fixture.errors'), []);
        return;
      } else if (scenario === 'session-title') {
        await click('details');
        await evaluate('document.querySelector(".session-info summary").click()');
        await waitFor('document.querySelector(".session-title-current button")');
        await evaluate('document.querySelector(".session-title-current button").click()');
        await waitFor('document.querySelector(".session-title-editor input")');
        await evaluate('document.querySelector(".session-title-editor input").select()');
        await win.webContents.insertText('Repair login redirect');
        await evaluate('document.querySelector(".session-title-editor form").requestSubmit()');
        await waitFor('document.querySelector(".session-title-current").textContent.includes("Repair login redirect")');
        assert.deepEqual(await evaluate('fixture.titleRequests'), [{sessionId:'coding',workspaceIdentity:'/tmp/test',action:'set',title:'Repair login redirect'}]);
        await evaluate('document.querySelector(".session-title-current button").click()');
        await evaluate('Array.from(document.querySelectorAll(".session-title-editor button")).find(b=>b.textContent==="Cancel").click()');
        assert.equal(await evaluate('fixture.titleRequests.length'),1,'cancel never changes the title');
        await evaluate('document.querySelector(".session-title-current button").click()');
        await evaluate('Array.from(document.querySelectorAll(".session-title-editor button")).find(b=>b.textContent==="Use automatic title").click()');
        await waitFor('document.querySelector(".session-title-current").textContent.includes("Automatic login topic")');
        assert.equal(await evaluate('fixture.titleRequests.at(-1).action'),'automatic');
        await evaluate('fixture.deferTitle=true;document.querySelector(".session-title-current button").click()');
        await evaluate('document.querySelector(".session-title-editor input").select()');
        await win.webContents.insertText('Old session title');
        await evaluate('document.querySelector(".session-title-editor form").requestSubmit()');
        await waitFor('!!fixture.finishTitle');
        await evaluate('fixture.select("other")');
        await waitFor('!document.querySelector(".session-info").open');
        await evaluate('document.querySelector(".session-info summary").click()');
        await waitFor('document.querySelector(".session-title-current").textContent.includes("other")');
        await evaluate('fixture.finishTitle()');
        assert.equal(await evaluate('document.querySelector(".session-title-current").textContent.includes("Old session title")'),false);
        assert.deepEqual(await evaluate('fixture.errors'), []);
        return;
      } else if (scenario === 'app-skills-picker') {
        await waitFor('document.querySelector(".workspace-picker-screen")');
        assert.equal(await evaluate('Array.from(document.querySelectorAll(".workspace-picker-screen button")).some(button => button.textContent.trim() === "Skills")'), false);
        assert.equal(await evaluate('!!document.querySelector(".workspace-picker-screen .workspace-history-other-folder")'), true);
        assert.deepEqual(await evaluate('fixture.errors'), []);
        return;
      } else if (scenario === 'app-startup-composer') {
        await waitFor('!!fixture.releaseSessions');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=new-session]")'), false,
          'automatic creation waits for session restoration to settle');
        await evaluate('fixture.releaseSessions()');
        await waitFor('!!document.querySelector("[data-session-id=created]")');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=new-session]")'), true,
          'loading session records without a restored selection keeps the composer visible');
        assert.equal(await evaluate('fixture.posts'), 0, 'startup never starts a session');
        await evaluate('Array.from(document.querySelectorAll("[data-shell-region=new-session] button")).find(button=>button.textContent==="Cancel").click()');
        await waitFor('!document.querySelector("[data-shell-region=new-session]")');
        assert.equal(await evaluate('document.querySelector(".shell-context h1").textContent'), 'Terminal');
        await evaluate('document.querySelector("[data-session-id=created] .session-row-select").click()');
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=new-session]")'), false);
        assert.deepEqual(await evaluate('fixture.errors'), []);
        return;
      } else if (scenario === 'app-review-terminal-links') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await evaluate('fixture.deferSelection=true;fixture.selectedDocument={kind:"design",selectionId:"d".repeat(64),branch:"sibling"}');
        await waitFor('document.querySelector("[data-shell-command=changes]")?.disabled===false');
        const activatePath = async (historical = false) => {
          const point = await evaluate(`(()=>{
            const screen=document.querySelector('.terminal-container .xterm-screen');
            const rect=screen.getBoundingClientRect();
            const term=fixture.runtime()?.terminal;
            return {x:Math.round(rect.x+rect.width/${historical ? 80 : 'term.cols'}*3),y:Math.round(rect.y+rect.height/${historical ? 24 : 'term.rows'}*0.5)};
          })()`);
          win.webContents.sendInputEvent({type:'mouseMove',...point});
          await waitFor('document.querySelector(".xterm-cursor-pointer")');
          win.webContents.sendInputEvent({type:'mouseDown',button:'left',clickCount:1,...point});
          win.webContents.sendInputEvent({type:'mouseUp',button:'left',clickCount:1,...point});
        };
        await evaluate('fixture.output("specs/clicked.md\\r\\n")');
        await waitFor('fixture.runtime().terminal.buffer.active.getLine(0)?.translateToString(true)==="specs/clicked.md"');
        await activatePath();
        await waitFor('fixture.planSelections.length===1');
        await waitFor('!!fixture.finishSelection');
        await evaluate('fixture.sessionList=fixture.sessionList.map(session=>session.id==="created"?{...session,hasOpenablePlan:true,planDocuments:{branch:"feature",sharedSessions:1,steps:[{kind:"design",available:true,pinned:true,selectionId:"d".repeat(64)},{kind:"implementation",available:true,pinned:false,selectionId:"a".repeat(64)}]}}:session)');
        await new Promise(resolve=>setTimeout(resolve,600));
        await evaluate('fixture.finishSelection();fixture.deferSelection=false');
        assert.deepEqual(await evaluate('fixture.planSelections'), [['created','specs/clicked.md']]);
        await waitFor('fixture.reads.length===1');
        await waitFor('document.querySelector("[data-shell-region=review]")?.textContent.includes("Clicked document")');
        assert.deepEqual(await evaluate('fixture.planSelections'), [['created','specs/clicked.md']]);
        assert.equal(await evaluate('fixture.prompts'), 0, 'reading never requests an independent review');
        assert.equal(await evaluate('fixture.documentKinds.at(-1)'), 'design');
        await evaluate('document.querySelector("[data-review-request]").click()');
        await waitFor('fixture.prompts===1');
        assert.equal(await evaluate('fixture.promptKind'), 'design');
        assert.equal(await evaluate('fixture.promptSelectionId'), 'd'.repeat(64));
        await evaluate('fixture.finishPrompt();fixture.selectedDocument=undefined');
        await evaluate('fixture.sessionList=fixture.sessionList.map(session=>session.id==="created"?{...session,planDocuments:{...session.planDocuments,steps:session.planDocuments.steps.map(step=>step.kind==="design"?{...step,selectionId:"b".repeat(64)}:step)}}:session)');
        await waitFor('fixture.reads.length===2 && document.querySelector(".review-plan-content")?.textContent.includes("Clicked document")');
        assert.equal(await evaluate('fixture.documentSelectionIds.at(-1)'), 'd'.repeat(64), 'session-branch polling retains the explicitly opened sibling');
        await evaluate('document.querySelector("[data-review-return]").click()');
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await evaluate('fixture.selectionFailure=true');
        await activatePath();
        await waitFor('document.body.textContent.includes("Document is unavailable")');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false);
        await evaluate('fixture.selectionFailure=false;fixture.deferSelection=true');
        await activatePath();
        await waitFor('!!fixture.finishSelection');
        await evaluate('document.querySelector("[data-session-id=other] .session-row-select").click()');
        await waitFor('document.querySelector(".shell-context h1").textContent==="other"');
        await evaluate('fixture.finishSelection()');
        await waitFor('document.querySelector(".historical-terminal-size")');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false,'late selection cannot open another session');
        assert.deepEqual(await evaluate('fixture.reads'), ['created','created']);
        await evaluate('fixture.deferSelection=false');
        await activatePath(true);
        await waitFor('document.querySelector("[data-shell-region=review]")?.textContent.includes("Other session plan")');
        assert.deepEqual(await evaluate('fixture.planSelections.at(-1)'), ['other','specs/history.md']);
        assert.deepEqual(await evaluate('fixture.errors'), []);
        return;
      } else if (scenario === 'open-session-app') {
        await waitFor('document.querySelector("[data-session-app-trigger]").textContent.includes("ChatGPT")');
        assert.deepEqual(await evaluate('fixture.nativeOpened'), [], 'rendering never opens an application');
        await evaluate('document.querySelector("[data-session-app-trigger]").click()');
        await waitFor('document.querySelector(".session-app-menu").textContent.includes("Native ID not captured")');
        assert.equal(await evaluate('document.querySelector("[data-session-app-open]").disabled'), true);
        await evaluate('fixture.nativeMissing=false;fixture.setSessionState({resume:{harnessSessionId:"12345678-1234-1234-1234-123456789abc"}})');
        await waitFor('!document.querySelector("[data-session-app-open]").disabled');
        await evaluate('document.querySelector("[data-session-app-trigger]").click()');
        await waitFor('!document.querySelector(".session-app-popover")');
        await evaluate('fixture.deferNative=true;document.querySelector("[data-session-app-trigger]").click()');
        await waitFor('!!fixture.finishNative');
        assert.equal(await evaluate('document.querySelector("[data-session-app-open]").disabled'), false, 'opening uses the prefetched availability while rechecking');
        await evaluate('fixture.deferNative=false;fixture.nativeUnavailable=true;fixture.setSessionState({harnessId:"claude-code",lifecycle:"alive"})');
        await waitFor('document.querySelector(".session-app-menu").textContent.includes("Close this session first")');
        await evaluate('fixture.finishNative()');
        assert.equal(await evaluate('document.querySelector("[data-session-app-open]").disabled'), true, 'late inspection cannot restore the previous destination');
        await evaluate('fixture.nativeUnavailable=false;fixture.setSessionState({harnessId:"claude-code",lifecycle:"dead"})');
        await waitFor('!document.querySelector("[data-session-app-open]").disabled');
        await evaluate('fixture.setSessionState({harnessId:"codex",lifecycle:"alive"})');
        await waitFor('document.querySelector("[data-session-app-trigger]").textContent.includes("ChatGPT")');
        await waitFor('document.querySelector("[data-session-app-open]")&&!document.querySelector("[data-session-app-open]").disabled');
        assert.match(await evaluate('document.querySelector("[data-session-app-open]").textContent'), /ChatGPT/);
        await evaluate('document.querySelector("[data-session-app-open]").click()');
        await waitFor('fixture.nativeOpened.length===1');
        assert.deepEqual(await evaluate('fixture.nativeOpened'), [['/real/test','coding']]);
        await evaluate('fixture.select("other")');
        await waitFor('document.querySelector(".shell-context h1").textContent==="other"');
        await evaluate('fixture.nativeFailure=true;document.querySelector("[data-session-app-trigger]").click()');
        await waitFor('document.querySelector("[data-session-app-open]")&&!document.querySelector("[data-session-app-open]").disabled');
        await evaluate('document.querySelector("[data-session-app-open]").click()');
        await waitFor('document.querySelector(".session-app-menu [role=alert]")');
        assert.match(await evaluate('document.querySelector(".session-app-menu").textContent'), /App refused/);
        await evaluate('document.querySelector("[data-session-app-trigger]").dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
        await waitFor('!document.querySelector(".session-app-popover")');
        assert.equal(await evaluate('document.activeElement.hasAttribute("data-session-app-trigger")'),true);
        await evaluate('fixture.nativeUnavailable=true;document.querySelector("[data-session-app-trigger]").click()');
        await waitFor('document.querySelector(".session-app-menu").textContent.includes("Close this session first")');
        assert.equal(await evaluate('document.querySelector("[data-session-app-open]").disabled'),true);
        await evaluate('fixture.advanceWorkspace()');
        await waitFor('!document.querySelector(".session-app-popover")');
        await win.setContentSize(360,700);
        await waitFor('document.querySelector(".shell-layout").dataset.mode==="compact"');
        await evaluate('fixture.nativeUnavailable=false;document.querySelector("[data-session-app-trigger]").click()');
        await waitFor('document.querySelector("[data-session-app-open]")&&!document.querySelector("[data-session-app-open]").disabled');
        assert.ok(await evaluate('(()=>{const r=document.querySelector(".session-app-popover").getBoundingClientRect();return r.x>=0&&r.right<=innerWidth})()'), 'menu stays within compact viewport');
        if (process.env.ORKWORKS_CAPTURE_SESSION_APP) {
          const {writeFileSync}=await import('node:fs');
          writeFileSync(process.env.ORKWORKS_CAPTURE_SESSION_APP,(await win.webContents.capturePage()).toPNG());
        }
        await evaluate('fixture.setCreating(true)');
        await waitFor('!document.querySelector("[data-session-app-trigger]")');
        await evaluate('fixture.setCreating(false);fixture.select(null)');
        await waitFor('document.querySelector(".shell-context h1").textContent==="Terminal"');
        assert.equal(await evaluate('!!document.querySelector("[data-session-app-trigger]")'),false);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'sidebar-icon-rail') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        const sockets = await evaluate('fixture.socketCount');
        assert.equal(await evaluate('document.querySelectorAll("[data-shell-command=sessions]").length'), 1, 'expanded sidebar has one toggle');
        assert.equal(await evaluate('!!document.querySelector(".shell-sidebar-top [data-shell-collapse]")'), true, 'collapse control lives at the top');
        const expandedWidth = await evaluate('document.querySelector("[data-shell-region=sessions]").getBoundingClientRect().width');
        await click('sessions');
        await waitFor('document.querySelector("[data-shell-region=rail]")');
        assert.equal(await evaluate('document.querySelectorAll("[data-shell-command=sessions]").length'), 1, 'collapsed sidebar has one toggle');
        const rail = await evaluate(`(()=>{const r=document.querySelector('[data-shell-region=rail]');return {width:r.getBoundingClientRect().width,buttons:[...r.querySelectorAll('button')].map(b=>({name:b.getAttribute('aria-label'),title:b.title,text:b.innerText,icon:!!b.querySelector('svg')}))};})()`);
        assert.ok(rail.width < expandedWidth && rail.width >= 44, 'collapse frees space and retains usable icon targets');
        assert.ok(rail.buttons.length >= 4);
        assert.ok(rail.buttons.every(b=>b.name && b.title && !b.text.trim() && b.icon), 'rail buttons are named, discoverable icons');
        await evaluate('document.querySelector("[data-shell-region=rail] [data-shell-command=recommendations]").click()');
        await waitFor('document.querySelector("[data-shell-region=utility]")');
        await evaluate('document.querySelector("[data-shell-return]").click()');
        await evaluate('document.querySelector("[data-shell-region=rail] [data-shell-settings]").click()');
        assert.equal(await evaluate('fixture.settingsOpened'),1);
        await evaluate('document.querySelector("[data-shell-region=rail] [data-shell-command=sessions]").click()');
        await waitFor('document.querySelector("[data-shell-region=sessions]")');
        assert.equal(await evaluate('document.querySelector("[data-shell-region=sessions]").getBoundingClientRect().width'),expandedWidth,'expanding restores the saved width');
        assert.equal(await evaluate('fixture.selections'),0);
        assert.equal(await evaluate('fixture.socketCount'),sockets,'collapse preserves the live terminal');
        await click('sessions');
        await win.setContentSize(640,700);
        await waitFor('document.querySelector(".shell-layout").dataset.mode==="compact"');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=rail]")'),false);
        await click('sessions');
        await waitFor('document.getElementById("sessions-list")===document.activeElement');
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-changes') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        const sockets=await evaluate('fixture.socketCount');
        await click('details');
        await waitFor('document.querySelector("[data-review-changes]")');
        await evaluate('fixture.deferChanges=true;fixture.deferBases=true;fixture.captureCount=0;document.querySelector("[data-review-changes]").click()');
        await waitFor('!!fixture.finishChanges && !!fixture.finishBases');
        await evaluate('fixture.deferBases=false;fixture.finishBases()');
        await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
        assert.equal(await evaluate('fixture.captureCount'),1,'default base must not restart uncommitted capture');
        await evaluate('fixture.deferChanges=false;fixture.finishChanges()');
        await waitFor('document.querySelector(".git-changes-patch")?.textContent.includes("working edit")');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=utility] .git-changes")'),true,'Changes occupies the main window');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),false);
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=sessions]")'),true);
        assert.equal(await evaluate('document.querySelector(".git-changes-details").open'),false,'technical details start collapsed');
        await evaluate('document.querySelector("[data-diff-mode=split]").click()');
        await waitFor('document.querySelector(".git-diff--split")');
        await evaluate('document.querySelector("[data-diff-mode=unified]").click()');
        await evaluate('document.querySelector(".git-changes-filter input").focus()');
        await win.webContents.insertText('missing');
        await waitFor('document.querySelector(".git-changes-file-list").textContent.includes("No files match")');
        assert.match(await evaluate('document.querySelector(".git-changes-patch").textContent'),/working edit/,'filtering preserves the selected patch');
        await evaluate('document.querySelector(".git-changes-filter input").select()');
        await win.webContents.insertText('change.ts');
        await waitFor('document.querySelectorAll(".git-changes-files button").length===1');
        assert.match(await evaluate('document.querySelector(".git-changes").textContent'),/HEAD.*working tree/);
        await evaluate('(()=>{const s=document.querySelector("[data-git-comparison]");s.value="branch";s.dispatchEvent(new Event("change",{bubbles:true}))})()');
        await waitFor('document.querySelector(".git-changes-patch")?.textContent.includes("committed edit")');
        assert.match(await evaluate('document.querySelector(".git-changes").textContent'),/committed only/);
        assert.doesNotMatch(await evaluate('document.querySelector(".git-changes").textContent'),/working edit/);
        // A delayed refresh from the old comparison must never overwrite the new one.
        await evaluate('fixture.deferChanges=true;document.querySelector("[data-git-refresh]").click()');
        await waitFor('!!fixture.finishChanges');
        await evaluate('(()=>{const s=document.querySelector("[data-git-comparison]");fixture.deferChanges=false;s.value="uncommitted";s.dispatchEvent(new Event("change",{bubbles:true}))})()');
        await waitFor('document.querySelector(".git-changes-patch")?.textContent.includes("working edit")');
        await evaluate('fixture.finishChanges()');
        assert.doesNotMatch(await evaluate('document.querySelector(".git-changes").textContent'),/committed edit/);
        for(const width of [1280,1000,640]){
          await win.setContentSize(width,700);
          await waitFor(`document.querySelector(".shell-layout").dataset.mode==="${width>=1180?'wide':width>=860?'medium':'compact'}"`);
          await waitFor('document.querySelector(".git-changes")');
          assert.equal(await evaluate('!!document.querySelector(".shell-toolbar [data-shell-command=terminal]")'),true,'Changes exposes Terminal in the shared header');
          assert.equal(await evaluate('(()=>{const p=document.querySelector(".git-changes");return p.scrollWidth<=p.clientWidth+1})()'),true,'Changes fits the content width');
          assert.equal(await evaluate('(()=>{const p=document.querySelector(".git-changes-file-content");return p.getBoundingClientRect().height>80})()'),true,'diff retains reading space');
          await evaluate('fixture.output("changes panel output\\r\\n")');
          assert.equal(await evaluate('fixture.socketCount'),sockets);
        }
        win.webContents.setZoomFactor(2);
        await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
        assert.equal(await evaluate('document.querySelector(".git-changes").scrollWidth<=document.querySelector(".git-changes").clientWidth+1'),true,'controls fit at 200% zoom');
        win.webContents.setZoomFactor(1);
        // A binding change inside the same repository invalidates the open capture,
        // including a deferred old response and a failed replacement refresh.
        await evaluate('fixture.deferChanges=true;document.querySelector("[data-git-refresh]").click()');
        await waitFor('!!fixture.finishChanges');
        await evaluate('fixture.oldWorktreeResponse=fixture.finishChanges;fixture.deferChanges=false;fixture.changeFailure=true;fixture.worktree="/tmp/feature";fixture.sessionList[0]={...fixture.sessionList[0],worktreeBinding:{root:"/tmp/feature",commonDir:"/tmp/test/.git"},gitContextSource:"agent"}');
        await waitFor('document.querySelector(".git-changes [role=alert]")?.textContent.includes("fixture refresh failed")');
        assert.equal(await evaluate('!!document.querySelector(".git-changes-patch")'),false);
        await evaluate('fixture.oldWorktreeResponse()');
        await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
        assert.equal(await evaluate('!!document.querySelector(".git-changes-patch")'),false,'late capture from the previous worktree stays discarded');
        await evaluate('fixture.changeFailure=false;document.querySelector("[data-git-refresh]").click()');
        await waitFor('document.querySelector(".git-changes-patch")');
        assert.match(await evaluate('document.querySelector(".git-changes-subject").textContent'),/feature/);
        assert.match(await evaluate('document.querySelector(".git-changes").textContent'),/Reported task worktree/);
        await evaluate('fixture.changePatch="@@ -0,0 +1,1100 @@\\n"+Array.from({length:1100},(_,i)=>"+const row"+i+" = "+i+";").join("\\n");document.querySelector("[data-git-refresh]").click()');
        await waitFor('document.querySelector("[data-diff-more]")');
        assert.equal(await evaluate('document.querySelectorAll(".git-diff-line").length'),999,'the hunk and code rows share the initial 1000-row batch');
        await evaluate('document.querySelector("[data-diff-more]").click()');
        await waitFor('document.querySelector(".git-changes-patch").textContent.includes("row1099")');
        assert.equal(await evaluate('!!document.querySelector("[data-diff-more]")'),false,'the last batch is reachable');
        // Refresh a surviving file while reading beyond the initial display batch.
        await evaluate('fixture.changePatch = fixture.changePatch.replaceAll(";", "; "+"x".repeat(400));document.querySelector("[data-git-refresh]").click()');
        await waitFor('document.querySelector(".git-changes-patch")?.textContent.includes("xxxx")');
        if (await evaluate('!!document.querySelector("[data-diff-more]")')) await evaluate('document.querySelector("[data-diff-more]").click()');
        await evaluate('(()=>{const p=document.querySelector(".git-changes-file-content");p.scrollTop=26000;p.scrollLeft=200})()');
        await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
        const readingPosition=await evaluate('(()=>{const p=document.querySelector(".git-changes-file-content");return [p.scrollTop,p.scrollLeft]})()');
        assert.ok(readingPosition[0]>25000 && readingPosition[1]>0,'fixture must scroll vertically beyond row 1000 and horizontally');
        await evaluate('document.querySelector("[data-git-refresh]").click()');
        await waitFor('!document.querySelector("[data-git-refresh]").disabled && document.querySelector(".git-changes-patch")?.textContent.includes("xxxx")');
        assert.deepEqual(await evaluate('(()=>{const p=document.querySelector(".git-changes-file-content");return [p.scrollTop,p.scrollLeft]})()'),readingPosition,'same-file Refresh preserves both reading axes and expanded rows');
        await evaluate('fixture.changeFailure=true;document.querySelector("[data-git-refresh]").click()');
        await waitFor('document.querySelector(".git-changes [role=alert]")?.textContent.includes("fixture refresh failed")');
        assert.deepEqual(await evaluate('(()=>{const p=document.querySelector(".git-changes-file-content");return [p.scrollTop,p.scrollLeft]})()'),readingPosition,'failed Refresh retains the old reading position');
        await evaluate('fixture.changeFailure=false');
        await evaluate('fixture.deferPatch=true;fixture.finishPatch=null;document.querySelector("[data-git-refresh]").click()');
        await waitFor('!!fixture.finishPatch && !document.querySelector(".git-changes-patch")');
        await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
        await evaluate('fixture.deferPatch=false;fixture.finishPatch()');
        await waitFor('document.querySelector(".git-changes-patch")?.textContent.includes("row1099")');
        assert.deepEqual(await evaluate('(()=>{const p=document.querySelector(".git-changes-file-content");return [p.scrollTop,p.scrollLeft]})()'),readingPosition,'delayed replacement patch restores the reading position');
        await evaluate('document.querySelector(".git-changes-filter input").focus();document.querySelector(".git-changes-filter input").select()');
        await win.webContents.insertText('.ts');
        await waitFor('document.querySelectorAll(".git-changes-files button").length===2');
        await evaluate('Array.from(document.querySelectorAll(".git-changes-files button")).find(b=>b.textContent.includes("other.ts")).click()');
        await waitFor('document.querySelector("[data-diff-more]") && document.querySelector(".git-changes-file-header h3").textContent==="other.ts"');
        assert.equal(await evaluate('document.querySelectorAll(".git-diff-line").length'),999,'switching files resets the initial display batch');
        assert.deepEqual(await evaluate('(()=>{const p=document.querySelector(".git-changes-file-content");return [p.scrollTop,p.scrollLeft]})()'),[0,0],'switching files resets the reading position');
        assert.doesNotMatch(await evaluate('document.querySelector(".git-changes-patch").textContent'),/row1099/);
        await evaluate('fixture.partialChanges=true;document.querySelector("[data-git-refresh]").click()');
        await waitFor('document.querySelector(".git-changes-count")?.textContent.includes("Totals unavailable")');
        assert.match(await evaluate('document.querySelector(".git-changes-status").textContent'),/Partial comparison/);
        await evaluate('(()=>{const p=document.querySelector(".git-changes-file-content");p.scrollTop=1000;p.scrollLeft=100})()');
        await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
        await evaluate('fixture.omitOtherFile=true;document.querySelector("[data-git-refresh]").click()');
        await waitFor('document.querySelector(".git-changes-file-header h3")?.textContent==="change.ts" && document.querySelector(".git-changes-patch")');
        assert.equal(await evaluate('document.activeElement===document.querySelector(".git-changes-file-list h3")'),true,'missing selected file focuses the file-list heading');
        assert.deepEqual(await evaluate('(()=>{const p=document.querySelector(".git-changes-file-content");return [p.scrollTop,p.scrollLeft]})()'),[0,0],'fallback file starts at the beginning');
        await evaluate('(()=>{const p=document.querySelector(".git-changes-file-content");p.scrollTop=1000;p.scrollLeft=100})()');
        await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
        await evaluate('fixture.emptyChanges=true;document.querySelector("[data-git-refresh]").click()');
        await waitFor('!document.querySelector(".git-changes-file-content") && !document.querySelector("[data-git-refresh]").disabled');
        await evaluate('fixture.emptyChanges=false;document.querySelector("[data-git-refresh]").click()');
        await waitFor('document.querySelector(".git-changes-patch")');
        assert.deepEqual(await evaluate('(()=>{const p=document.querySelector(".git-changes-file-content");return [p.scrollTop,p.scrollLeft]})()'),[0,0],'an empty comparison clears the old reading position');
        for (const selectComparison of [
          '(()=>{const s=document.querySelector("[data-git-comparison]");s.value="branch";s.dispatchEvent(new Event("change",{bubbles:true}))})()',
          '(()=>{const s=document.querySelector(".git-changes-toolbar label:nth-child(2) select");s.value="refs/heads/release";s.dispatchEvent(new Event("change",{bubbles:true}))})()',
        ]) {
          await evaluate('(()=>{const p=document.querySelector(".git-changes-file-content");p.scrollTop=1000;p.scrollLeft=100})()');
          await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
          await evaluate(selectComparison);
          await waitFor('document.querySelector(".git-changes-patch") && !document.querySelector("[data-git-refresh]").disabled');
          assert.deepEqual(await evaluate('(()=>{const p=document.querySelector(".git-changes-file-content");return [p.scrollTop,p.scrollLeft]})()'),[0,0],'comparison and base switches reset the reading position');
        }
        if (process.env.ORKWORKS_CAPTURE_CHANGES) {
          await win.setContentSize(1440,900);
          await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
          const {writeFileSync} = await import('node:fs');
          writeFileSync(process.env.ORKWORKS_CAPTURE_CHANGES,(await win.webContents.capturePage()).toPNG());
        }
        await evaluate('document.querySelector(".shell-toolbar [data-shell-command=terminal]").click()');
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await waitFor('document.activeElement?.classList.contains("xterm-helper-textarea") || document.activeElement?.hasAttribute("data-shell-terminal-focus")');
        await click('changes');
        await waitFor('document.querySelector(".git-changes")');
        await evaluate('document.querySelector(".git-changes h2").focus()');
        win.webContents.sendInputEvent({type:'keyDown',keyCode:'Escape'});
        win.webContents.sendInputEvent({type:'keyUp',keyCode:'Escape'});
        await waitFor('!document.querySelector(".git-changes") && fixture.runtime()?.terminal.element?.isConnected');
        await win.setContentSize(1280,700);
        await waitFor('document.querySelector(".shell-layout").dataset.mode==="wide"');
        await click('changes');
        await waitFor('document.querySelector(".git-changes")');
        await waitFor('document.querySelector("[data-session-id=other] .session-row-select")');
        await evaluate('document.querySelector("[data-session-id=other] .session-row-select").click()');
        await waitFor('!document.querySelector(".git-changes") && document.querySelector("[data-shell-region=terminal]")');
        assert.equal(await evaluate('fixture.input.length'),0);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-archived-stale') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        const sockets = await evaluate('fixture.socketCount');
        await evaluate('(()=>{const select=document.querySelector(".session-state-filter select");select.value="archived";select.dispatchEvent(new Event("change",{bubbles:true}))})()');
        await waitFor('document.querySelector("[data-session-id=other]")');
        await evaluate('document.querySelector("[data-session-id=other] .session-row-select").click()');
        await waitFor('document.querySelector(".historical-terminal-size")');
        assert.equal(await evaluate('fixture.socketCount'), sockets, 'archived history must not attach a live terminal');
        await click('details');
        await waitFor('document.querySelector("[data-shell-region=inspector]")');
        assert.equal(await evaluate('Array.from(document.querySelectorAll("button")).some(button=>button.textContent.includes("Request independent review"))'), false);
        await click('review');
        await waitFor('document.querySelector(".review-plan-content")');
        assert.equal(await evaluate('!!document.querySelector("[data-review-request]")'), false);
        assert.equal(await evaluate('fixture.prompts'),0);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'recommendation-status-error') {
        await evaluate('fixture.command("recommendations",true)');
        await waitFor('fixture.statusReads > 0 && !!document.querySelector(".recommendations-panel .recommendation-error")');
        assert.equal(await evaluate('fixture.errors.length'),0);
        return;
      } else if (scenario === 'recommendations-main' || scenario === 'skills-main') {
        const destination = scenario === 'skills-main' ? 'skills' : 'recommendations';
        const title = destination === 'skills' ? 'Skills' : 'Recommendations';
        if (destination === 'skills') assert.equal(await evaluate('document.querySelector("[data-shell-command=recommendations]").nextElementSibling?.dataset.shellCommand'), 'skills');
        for (const width of [1280,1000,640]) {
          await win.setContentSize(width,700);
          await waitFor(`document.querySelector(".shell-layout").dataset.mode==="${width>=1180?'wide':width>=860?'medium':'compact'}"`);
          await evaluate('fixture.command('+JSON.stringify(destination)+',true)');
          await waitFor('document.querySelector("[data-shell-page-heading]")');
          assert.equal(await evaluate('!!document.querySelector("main [data-shell-region=utility]")'),true,'the utility occupies the main view at every width');
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=terminal], [data-shell-region=inspector], [data-shell-pin]")'),false);
          assert.equal(await evaluate('document.querySelector("[data-shell-terminal-focus]").textContent'),title);
          assert.equal(await evaluate('fixture.selections'),0,'opening a utility does not select a session');
          if(width>=860)assert.equal(await evaluate('!!document.querySelector("[data-shell-region=sessions]")'),true);
          if (destination === 'skills') {
            assert.equal(await evaluate('!!document.querySelector("main [data-shell-region=utility] > .shell-region-header")'),false,'Skills omits the duplicate title/return row');
            assert.equal(await evaluate('document.activeElement === document.querySelector(".skills-settings h2")'),true);
            await click('terminal');
          } else {
            await evaluate('document.querySelector("[data-shell-return]").click()');
          }
          await waitFor('document.querySelector("[data-shell-region=terminal]")');
          await evaluate('fixture.command('+JSON.stringify(destination)+',true)');
          await waitFor('document.querySelector("[data-shell-page-heading]")');
          await evaluate('document.querySelector("[data-shell-page-heading]").dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
          await waitFor('document.querySelector("[data-shell-region=terminal]")');
        }
        await win.setContentSize(1280,700);
        await waitFor('document.querySelector(".shell-layout").dataset.mode==="wide"');
        await evaluate('fixture.openReview()');
        await waitFor('document.querySelector("[data-shell-region=review]")');
        await evaluate('fixture.command('+JSON.stringify(destination)+',true)');
        await waitFor('document.querySelector("main [data-shell-region=utility]")');
        if (destination === 'skills') {
          await evaluate('fixture.command("skills")');
        } else {
          assert.equal(await evaluate('document.querySelector("[data-shell-return]").textContent'),'Back to Review');
          await evaluate('document.querySelector("[data-shell-return]").click()');
        }
        await waitFor('document.querySelector("[data-shell-region=review]")');
        await evaluate('fixture.command("terminal")');
        await waitFor('document.querySelector("[data-shell-region=terminal]")');
        await evaluate('fixture.command('+JSON.stringify(destination)+',true)');
        await waitFor('document.querySelector("[data-shell-page-heading]")');
        await evaluate('Array.from(document.querySelectorAll("[data-session-id]")).find(el=>el.textContent.includes("other")).click()');
        await waitFor('document.querySelector("[data-shell-region=terminal]")');
        assert.equal(await evaluate('fixture.activeSessionId'),'other');
        await evaluate('fixture.command('+JSON.stringify(destination)+',true)');
        await waitFor('document.querySelector("[data-shell-page-heading]")');
        await evaluate('fixture.advanceWorkspace()');
        await waitFor('document.querySelector("[data-shell-region=terminal]")');
        if (destination === 'skills') {
          await evaluate('fixture.command("skills")');
          await waitFor('document.querySelector("[data-add-skill]")?.disabled === false');
          await evaluate('document.querySelector("[data-add-skill]").click()');
          await waitFor('document.querySelector("[data-pick-skill]")');
          await evaluate('document.querySelector("[data-pick-skill]").click()');
          await waitFor('!document.querySelector("[data-review-skill]").disabled');
          await evaluate('document.querySelector("[data-review-skill]").click()');
          await waitFor('fixture.skillRequest');
          assert.equal(await evaluate('fixture.skillRequest.expectedWorkspace'), '/real/test');
          await evaluate('fixture.command("skills")');
          await waitFor('!document.querySelector("main .skills-settings")');
          await evaluate('fixture.select(null);fixture.setEmpty(true)');
          await evaluate('fixture.command("skills")');
          await waitFor('document.querySelector("main .skills-settings")');
          assert.equal(await evaluate('fixture.activeSessionId'), null);
          await evaluate('fixture.command("skills")');
          await waitFor('!document.querySelector("main .skills-settings")');
        }
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-cancel-workspace') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await click('review');
        await waitFor('document.querySelector(".review-plan-content")');
        await click('sessions');
        await waitFor('document.querySelector(".shell-navigation-menu summary")');
        const openPicker = async () => {
          await evaluate('document.querySelector(".shell-navigation-menu summary").click()');
          await waitFor('document.querySelector(".titlebar-switch-button")');
          await evaluate('document.querySelector(".titlebar-switch-button").click()');
          await waitFor('document.querySelector(".workspace-history-other-folder")');
        };
        await openPicker();
        await evaluate('document.querySelector(".workspace-history-other-folder").click()');
        await waitFor('!!fixture.cancelWorkspacePicker && document.querySelector("[data-shell-command=review]").disabled');
        assert.equal(await evaluate('document.querySelector(".review-plan-content")?.textContent'),'Current plan','pending picker preserves the validated snapshot');
        assert.equal(await evaluate('document.querySelector("[data-review-refresh]").disabled'),true);
        assert.equal(await evaluate('document.querySelector("[data-review-request]").disabled'),true);
        await evaluate('fixture.cancelWorkspacePicker(null)');
        await waitFor('!document.querySelector("[data-shell-command=review]").disabled');
        assert.equal(await evaluate('document.querySelector(".review-plan-content")?.textContent'),'Current plan','cancelled picker preserves Review');
        await openPicker();
        await evaluate('document.querySelector(".workspace-history-row-open:not(:disabled)").click()');
        await waitFor('!!fixture.rejectRememberedWorkspace && document.querySelector("[data-shell-command=review]").disabled');
        await evaluate('fixture.rejectRememberedWorkspace(new Error("Workspace is unavailable"))');
        await waitFor('!document.querySelector("[data-shell-command=review]").disabled');
        assert.equal(await evaluate('document.querySelector(".review-plan-content")?.textContent'),'Current plan','prevalidation failure preserves Review');
        assert.deepEqual(await evaluate('fixture.reads'),['created'],'aborted attempts do not reload the plan');
        assert.deepEqual(await evaluate('fixture.navigation'),['review'],'aborted attempts do not alter navigation recency');
        await evaluate('fixture.unreadable=true;document.querySelector("[data-review-refresh]").click()');
        await waitFor('document.querySelector(".empty-state-action")?.textContent==="Retry"');
        await openPicker();
        await evaluate('fixture.cancelWorkspacePicker=null;document.querySelector(".workspace-history-other-folder").click()');
        await waitFor('!!fixture.cancelWorkspacePicker && document.querySelector("[data-shell-command=review]").disabled');
        assert.equal(await evaluate('!!document.querySelector(".empty-state-action:not(:disabled)")'),false,'Retry cannot read the old workspace during a switch attempt');
        await evaluate('fixture.cancelWorkspacePicker(null)');
        await waitFor('!document.querySelector("[data-shell-command=review]").disabled');
        await evaluate('fixture.unreadable=false;fixture.deferRead=true;document.querySelector(".empty-state-action").click()');
        await waitFor('!!fixture.resolveRead');
        await evaluate('fixture.lifecycle({state:"opening"})');
        await evaluate('fixture.lifecycle({state:"ready",workspace:{path:"/tmp/new",workspaceIdentity:"/tmp/new",lastActiveSessionId:"created",activeHarnessIds:["generic-shell"]},historyDiagnostic:null})');
        await waitFor('document.querySelector("[data-shell-region=terminal]") && !document.querySelector("[data-shell-command=review]").disabled');
        await evaluate('fixture.resolveRead("# Old workspace plan")');
        await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
        assert.equal(await evaluate('!!document.querySelector(".review-plan-content")'),false,'actual workspace change rejects the old read even with a reused session ID');
        assert.equal(await evaluate('fixture.prompts'),0);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-create-persistence') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await click('review');
        await waitFor('document.querySelector(".review-plan-content")');
        assert.deepEqual(await evaluate('fixture.navigation'),['review']);
        await evaluate('fixture.createdId="new-created";fixture.menu({action:"new-session"})');
        await waitFor('document.querySelector(".new-session-confirm")');
        await evaluate('fixture.failCreation=true;document.querySelector(".new-session-confirm").click()');
        await waitFor('fixture.posts===1&&!document.querySelector(".new-session-confirm").disabled');
        assert.deepEqual(await evaluate('fixture.navigation'),['review'],'failed creation preserves saved Review');
        await evaluate('fixture.failCreation=false;document.querySelector(".new-session-confirm").click()');
        await waitFor('document.querySelector("[data-shell-region=terminal]") && document.querySelector(".shell-context h1")?.textContent==="new-created"');
        await waitFor('fixture.navigation.length===2');
        assert.deepEqual(await evaluate('fixture.navigation'),['review','terminal'],'completed creation saves Terminal exactly once');
        await evaluate('fixture.menu({action:"focus",panelId:"terminal"})');
        await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
        assert.deepEqual(await evaluate('fixture.navigation'),['review','terminal'],'later focus-only Terminal stays read-only');
        await evaluate('fixture.lifecycle({state:"opening"})');
        await evaluate('fixture.lifecycle({state:"ready",workspace:{path:"/tmp/test",workspaceIdentity:"/tmp/test",lastActiveSessionId:"new-created",activeHarnessIds:["generic-shell"]},historyDiagnostic:null})');
        await waitFor('document.querySelector(".shell-context h1")?.textContent==="new-created"');
        await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false,'reopening does not apply an old Review destination to the new session');
        assert.equal(await evaluate('fixture.prompts'),0);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-terminal-noop') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await evaluate('fixture.menu({action:"focus",panelId:"terminal"})');
        await waitFor('document.activeElement?.classList.contains("xterm-helper-textarea")');
        assert.deepEqual(await evaluate('fixture.navigation'),[],'focusing the current Terminal cannot change recency');
        await evaluate('fixture.deferRead=true;fixture.resolveRead=null');
        await click('review');
        await waitFor('!!fixture.resolveRead');
        await evaluate('fixture.menu({action:"focus",panelId:"terminal"})');
        await waitFor('document.activeElement?.classList.contains("xterm-helper-textarea")');
        await evaluate('fixture.resolveRead("# Cancelled entry")');
        await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
        assert.deepEqual(await evaluate('fixture.navigation'),[],'cancelling pending entry is not a completed central navigation');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-native-focus') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await evaluate('fixture.runtime().terminal.focus();fixture.menu({action:"focus",panelId:"review"})');
        await waitFor('document.querySelector(".review-plan-content")');
        await evaluate('document.querySelector("[data-review-return]").click()');
        await waitFor('document.activeElement?.classList.contains("xterm-helper-textarea")');
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-shared-request') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await click('review');
        await waitFor('document.querySelector(".review-plan-content")');
        await click('details');
        await waitFor('document.querySelector("[data-shell-region=inspector]")');
        await evaluate('document.querySelector(".session-info summary").click()');
        await evaluate(`fixture.clickBoth=()=>{document.querySelector('[data-review-request]').click();Array.from(document.querySelectorAll('.resume-option')).find(button=>button.textContent.includes('Request independent review')).click()};fixture.clickBoth()`);
        assert.equal(await evaluate('fixture.prompts'),1,'two visible controls share one in-flight request');
        await evaluate('fixture.finishPrompt()');
        await waitFor('!document.querySelector("[data-review-request]").disabled');
        await evaluate(`Array.from(document.querySelectorAll('.resume-option')).find(button=>button.textContent.includes('Request independent review')).click();document.querySelector('[data-review-request]').click()`);
        assert.equal(await evaluate('fixture.prompts'),2,'settled requests allow a later request from either control');
        await evaluate('fixture.rejectPrompt(new Error("Request failed"))');
        await waitFor('!document.querySelector("[data-review-request]").disabled');
        await evaluate('fixture.clickBoth()');
        assert.equal(await evaluate('fixture.prompts'),3,'failed requests also release the shared guard');
        await evaluate('fixture.oldPrompt=fixture.finishPrompt;fixture.lifecycle({state:"opening"})');
        await evaluate('fixture.lifecycle({state:"ready",workspace:{path:"/tmp/new",workspaceIdentity:"/tmp/new",lastActiveSessionId:"created",activeHarnessIds:["generic-shell"]},historyDiagnostic:null})');
        await waitFor('document.querySelector(".shell-workspace-control [title]")?.title==="/tmp/new" && fixture.runtime()?.terminal.element?.isConnected');
        await click('review');
        await waitFor('document.querySelector(".review-plan-content")');
        await evaluate('document.querySelector("[data-review-request]").click()');
        assert.equal(await evaluate('fixture.prompts'),4,'a new workspace generation does not inherit an old pending guard');
        await evaluate('fixture.oldPrompt()');
        await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
        await click('details');
        await waitFor('document.querySelector("[data-shell-region=inspector]")');
        await evaluate('if (!document.querySelector(".session-info").open) document.querySelector(".session-info summary").click()');
        await evaluate(`Array.from(document.querySelectorAll('.resume-option')).find(button=>button.textContent.includes('Request independent review')).click()`);
        assert.equal(await evaluate('fixture.prompts'),4,'old settlement cannot release the new generation guard');
        await evaluate('fixture.finishPrompt()');
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-composer-origin') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await click('review');
        await waitFor('document.querySelector(".review-plan-content")');
        await evaluate('fixture.menu({action:"new-session"})');
        await waitFor('document.querySelector("[data-shell-region=new-session]")');
        await evaluate('fixture.menu({action:"focus",panelId:"review"})');
        await waitFor('!document.querySelector("[data-shell-region=new-session]") && document.querySelector("[data-shell-region=review]")');
        await click('details');
        await waitFor('document.querySelector("[data-shell-region=inspector]")');
        await evaluate('fixture.menu({action:"new-session"})');
        await waitFor('document.querySelector("[data-shell-region=new-session]") && !document.querySelector("[data-shell-region=inspector]")');
        await evaluate(`Array.from(document.querySelectorAll('[data-shell-region=new-session] button')).find(button=>button.textContent==='Cancel').click()`);
        await waitFor('document.querySelector("[data-shell-region=review]") && document.querySelector("[data-shell-region=inspector]")');
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-pending-navigation-recovery') {
        for (const outcome of ['resolve','reject']) {
          if (outcome==='reject') await fresh();
          await waitFor('!!fixture.releaseSessions && document.querySelector("[data-rebuild-navigation]")');
          await evaluate('fixture.cancelNavigationRebuild=false;fixture.deferNavigationRebuild=true;document.querySelector("[data-rebuild-navigation]").click()');
          await waitFor('!!fixture.finishNavigationRebuild');
          await evaluate('fixture.releaseSessions()');
          await waitFor('document.querySelector("[data-session-id]")');
          await evaluate(`Array.from(document.querySelectorAll('[data-session-id]')).find(row=>row.textContent.includes('created')).click()`);
          await waitFor('!!fixture.resolveNewNavigation');
          await evaluate('fixture.finishNavigationRebuild()');
          await waitFor('!document.querySelector("[data-rebuild-navigation]")');
          await evaluate(outcome==='resolve' ? 'fixture.resolveNewNavigation({lastCentralSurface:"terminal",revision:0,diagnostic:"corrupt_record"})' : 'fixture.rejectNewNavigation(new Error("Read during rebuild failed"))');
          await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
          assert.equal(await evaluate('document.body.innerText.includes("Saved navigation could not be read") || document.body.innerText.includes("Navigation could not be saved or restored")'),false,'reads begun during rebuild cannot undo completion');
          assert.deepEqual(await evaluate('fixture.errors'),[]);
        }
        return;
      } else if (scenario === 'app-review-delayed-navigation-recovery') {
        for (const outcome of ['resolve','reject']) {
          if (outcome==='reject') await fresh();
          await waitFor('!!fixture.resolveOldNavigation && !!fixture.releaseSessions');
          await evaluate('fixture.releaseSessions()');
          await waitFor('document.querySelector("[data-session-id]")');
          await evaluate(`Array.from(document.querySelectorAll('[data-session-id]')).find(row=>row.textContent.includes('created')).click()`);
          await waitFor('document.querySelector("[data-rebuild-navigation]")');
          await evaluate('fixture.navigationBeforeRecovery=fixture.navigation.length');
          await evaluate('fixture.cancelNavigationRebuild=false;document.querySelector("[data-rebuild-navigation]").click()');
          await waitFor('fixture.navigationRebuilds===1&&!document.querySelector("[data-rebuild-navigation]")');
          await evaluate(outcome==='resolve' ? 'fixture.resolveOldNavigation({lastCentralSurface:"terminal",revision:0,diagnostic:"corrupt_record"})' : 'fixture.rejectOldNavigation(new Error("Old read failure"))');
          await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
          assert.equal(await evaluate('document.body.innerText.includes("Saved navigation could not be read") || document.body.innerText.includes("Navigation could not be saved or restored")'),false,'old diagnostic cannot undo confirmed rebuild');
          assert.equal(await evaluate('fixture.navigation.length'),await evaluate('fixture.navigationBeforeRecovery'));
          assert.deepEqual(await evaluate('fixture.errors'),[]);
        }
        return;
      } else if (scenario === 'app-review-stale-read-restore') {
        await waitFor('!!fixture.rejectRead');
        await evaluate('fixture.menu({action:"focus",panelId:"detail"})');
        await waitFor('document.querySelector("[data-shell-region=inspector]") && document.activeElement?.matches("[data-shell-page-heading]")');
        await evaluate('fixture.rejectRead(new Error("Superseded restore failure"))');
        await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),true);
        assert.equal(await evaluate('document.activeElement?.matches("[data-shell-page-heading]")'),true);
        assert.equal(await evaluate('document.body.innerText.includes("Review could not be restored") || document.body.innerText.includes("no longer available")'),false);
        assert.deepEqual(await evaluate('fixture.navigation'),[]);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-unreadable-restore') {
        await waitFor('document.body.innerText.includes("Review could not be restored")');
        await waitFor('document.querySelector("[data-shell-region=terminal]")');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false);
        assert.deepEqual(await evaluate('fixture.reads'),['created']);
        assert.deepEqual(await evaluate('fixture.navigation'),[]);
        assert.equal(await evaluate('fixture.prompts'),0);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-empty-navigation-recovery') {
        await waitFor('document.querySelector("[data-rebuild-navigation]")');
        assert.equal(await evaluate('fixture.reads.length'),0);
        await evaluate('fixture.cancelNavigationRebuild=false;document.querySelector("[data-rebuild-navigation]").click()');
        await waitFor('fixture.navigationRebuilds===1&&!document.querySelector("[data-rebuild-navigation]")');
        assert.deepEqual(await evaluate('fixture.navigation'),[]);
        assert.equal(await evaluate('fixture.prompts'),0);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-composer-return') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await click('review');
        await waitFor('document.querySelector(".review-plan-content")?.textContent.includes("Current plan")');
        await evaluate('fixture.originalRuntime=fixture.runtime();fixture.navigationBeforeComposer=fixture.navigation.length');
        for (const width of [1280,1000,640]) {
          win.setContentSize(width,700);
          await waitFor(`innerWidth===${width} && document.querySelector('.shell-layout').clientWidth===${width}`);
          await evaluate('fixture.menu({action:"new-session"})');
          await waitFor('document.querySelector("[data-shell-region=new-session]") && !document.querySelector("[data-shell-region=new-session]").hidden');
          await evaluate(`Array.from(document.querySelectorAll('[data-shell-region=new-session] button')).find(button=>button.textContent==='Cancel').click()`);
          await waitFor('document.querySelector(".review-plan-content")?.textContent.includes("Current plan")');
          assert.equal(await evaluate('fixture.navigation.length'),await evaluate('fixture.navigationBeforeComposer'),'transient composer never persists Terminal');
        }
        await evaluate('document.querySelector("[data-review-return]").click()');
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        assert.equal(await evaluate('fixture.runtime()===fixture.originalRuntime'),true);
        assert.equal(await evaluate('fixture.posts'),0);
        assert.equal(await evaluate('fixture.prompts'),0);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-initial-error') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await evaluate('fixture.unreadable=true');
        await click('review');
        await waitFor('document.querySelector("[data-shell-region=review]")?.textContent.includes("no longer available")');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=terminal]")'),false);
        assert.deepEqual(await evaluate('fixture.navigation'),[],'failed reads cannot persist a validated Review destination');
        await evaluate(`fixture.unreadable=false;const retry=Array.from(document.querySelectorAll('[data-shell-region=review] button')).find(button=>button.textContent==='Retry');retry.focus();retry.click()`);
        await waitFor('document.querySelector(".review-plan-content")?.textContent.includes("Current plan")');
        assert.deepEqual(await evaluate('fixture.navigation'),[],'Retry remains read-only');
        await waitFor('document.activeElement?.matches("[data-shell-review-heading]")');
        await evaluate('document.querySelector("[data-review-return]").click()');
        await waitFor('document.querySelector("[data-shell-region=terminal]")');
        assert.equal(await evaluate('document.body.innerText.includes("no longer available")'),false);
        await evaluate('fixture.deferRead=true;fixture.resolveRead=null');
        await click('review');
        await waitFor('!!fixture.resolveRead');
        await evaluate('fixture.menu({action:"focus",panelId:"detail"});fixture.rejectRead(new Error("Superseded failure"))');
        await waitFor('document.querySelector("[data-shell-region=inspector]")');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false,'superseded failure cannot open Review');
        assert.equal(await evaluate('document.body.innerText.includes("no longer available")'),false,'superseded failure cannot leak an alert into Details');
        await evaluate('fixture.resolveRead=null');
        await click('review');
        await waitFor('!!fixture.resolveRead');
        await evaluate('fixture.rejectOldRead=fixture.rejectRead;fixture.lifecycle({state:"opening"})');
        await evaluate('fixture.deferRead=false;fixture.lifecycle({state:"ready",workspace:{path:"/tmp/new",workspaceIdentity:"/tmp/new",lastActiveSessionId:"created",activeHarnessIds:["generic-shell"]},historyDiagnostic:null})');
        await waitFor('document.querySelector(".shell-workspace-control [title]")?.title==="/tmp/new" && fixture.runtime()?.terminal.element?.isConnected');
        await evaluate('fixture.rejectOldRead(new Error("Previous workspace failure"))');
        await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false,'stale rejection cannot attach to a reused session ID');
        assert.equal(await evaluate('document.body.innerText.includes("no longer available")'),false);
        assert.equal(await evaluate('fixture.navigation.includes("review")'),false);
        assert.equal(await evaluate('fixture.prompts'),0);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-native-race') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await evaluate('fixture.deferRead=true;fixture.originalDigest=crypto.subtle.digest.bind(crypto.subtle);crypto.subtle.digest=()=>Promise.resolve(new ArrayBuffer(32));void 0');
        await click('review');
        await waitFor('!!fixture.resolveRead');
        await evaluate('fixture.menu({action:"focus",panelId:"detail"});fixture.resolveRead("# Superseded native read")');
        await waitFor('document.querySelector("[data-shell-region=inspector]")');
        await evaluate('document.querySelector("[data-shell-return]").click()');
        await waitFor('!document.querySelector("[data-shell-region=inspector]")');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false,'native-menu intent fences completion before shell effects run');
        assert.equal(await evaluate('fixture.navigation.includes("review")'),false);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-details-request') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await click('details');
        await evaluate('document.querySelector(".session-info summary").click()');
        await waitFor('Array.from(document.querySelectorAll(".resume-option")).some(button=>button.textContent.includes("Request independent review"))');
        await evaluate(`(()=>{const button=Array.from(document.querySelectorAll('.resume-option')).find(button=>button.textContent.includes('Request independent review'));button.click();button.click()})()`);
        assert.equal(await evaluate('fixture.prompts'),1,'Details guards clicks before React rerenders');
        await evaluate('fixture.finishPrompt()');
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-navigation-recovery') {
        await waitFor('document.body.textContent.includes("Saved navigation could not be read")');
        assert.equal(await evaluate('fixture.navigation.length'),0,'corrupt navigation is preserved during hydration');
        await evaluate('document.querySelector("[data-rebuild-navigation]").click();document.querySelector("[data-rebuild-navigation]").click()');
        await waitFor('fixture.navigationRebuilds===1&&!document.querySelector("[data-rebuild-navigation]").disabled');
        assert.equal(await evaluate('!!document.querySelector("[data-rebuild-navigation]")'),true,'cancel preserves recovery');
        await evaluate('fixture.cancelNavigationRebuild=false;document.querySelector("[data-rebuild-navigation]").click()');
        await waitFor('fixture.navigationRebuilds===2&&!document.querySelector("[data-rebuild-navigation]")');
        assert.equal(await evaluate('fixture.prompts'),0);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-opening-navigation') {
        await waitFor('document.querySelector("[data-shell-region=review]")');
        await evaluate('fixture.lifecycle({state:"opening"})');
        await evaluate('document.querySelector("[data-shell-command=details]").focus();fixture.menu({action:"focus",panelId:"terminal"})');
        await waitFor('document.activeElement?.matches("[data-shell-terminal-focus]")');
        await evaluate('fixture.lifecycle({state:"ready",workspace:{path:"/tmp/other",workspaceIdentity:"/tmp/other",lastActiveSessionId:"created",activeHarnessIds:["generic-shell"]},historyDiagnostic:null})');
        await waitFor('document.querySelector(".shell-context h1")?.textContent==="created"');
        await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false,'navigation during opening supersedes saved Review');
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-missing-restore') {
        await waitFor('document.querySelector("[data-shell-region=terminal]")');
        await waitFor('document.body.textContent.includes("Review could not be restored")');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false);
        assert.equal(await evaluate('fixture.reads.length'),0);
        assert.equal(await evaluate('fixture.prompts'),0);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-stale-restore') {
        await waitFor('!!fixture.resolveNavigation');
        await evaluate('fixture.menu({action:"focus",panelId:"terminal"})');
        await evaluate('fixture.resolveNavigation()');
        await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false);
        assert.equal(await evaluate('fixture.reads.length'),0,'late hydration cannot override user navigation');
        return;
      } else if (scenario === 'app-review-history') {
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await evaluate(`Array.from(document.querySelectorAll('[data-session-id]')).find(row=>row.textContent.includes('other')).click()`);
        await waitFor(`document.querySelector('[aria-label="Saved terminal output"]')`);
        await click('review');
        await waitFor('document.querySelector(".review-plan-content")');
        assert.equal(await evaluate('!!document.querySelector("[data-review-request]")'),false,'remembered sessions cannot receive prompts');
        await evaluate('document.querySelector("[data-review-return]").click()');
        await waitFor(`document.querySelector('[aria-label="Saved terminal output"]')`);
        assert.equal(await evaluate('fixture.prompts'),0);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review-restore') {
        await waitFor('document.querySelector("[data-shell-region=review]")?.textContent.includes("Current plan")');
        assert.equal(await evaluate('fixture.prompts'),0);
        assert.deepEqual(await evaluate('fixture.navigation'),[],'restoring presentation does not write defaults');
        await evaluate('document.querySelector("[data-review-return]").click()');
        await waitFor('document.querySelector("[data-shell-region=terminal]")');
        await waitFor('document.activeElement?.classList.contains("xterm-helper-textarea")');
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-review') {
        const resizeReview = async width => {
          win.setContentSize(width,700);
          const mode = width >= 1180 ? 'wide' : width >= 860 ? 'medium' : 'compact';
          await waitFor(`innerWidth===${width} && document.querySelector('.shell-layout').clientWidth===${width} && document.querySelector('.shell-layout').dataset.mode==='${mode}'`);
        };
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await waitFor('document.querySelector("[data-shell-command=review]")');
        await evaluate('fixture.originalRuntime=fixture.runtime();fixture.writesBefore=fixture.activeWrites');
        await click('review');
        await waitFor('document.querySelector("[data-shell-region=review]")?.textContent.includes("Current plan")');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=terminal]")'),false);
        await waitFor('document.activeElement?.matches("[data-shell-review-heading]")');
        await evaluate('fixture.readsBeforeReselect=fixture.reads.length;fixture.navigationBeforeReselect=fixture.navigation.length');
        await click('review');
        await evaluate('fixture.menu({action:"focus",panelId:"review"})');
        await waitFor('document.activeElement?.matches("[data-shell-review-heading]")');
        assert.equal(await evaluate('fixture.reads.length'),await evaluate('fixture.readsBeforeReselect'),'reselecting Review focuses without rereading');
        assert.equal(await evaluate('fixture.navigation.length'),await evaluate('fixture.navigationBeforeReselect'),'reselecting Review cannot change recency');
        for (const width of [1000,640,360,1280]) {
          await resizeReview(width);
          await waitFor('innerWidth==='+width);
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),true);
          assert.equal(await evaluate('document.querySelectorAll(".review-plan-content").length'),1);
          assert.ok(await evaluate('(()=>{const b=document.querySelector("[data-review-return]").getBoundingClientRect();return b.width>0&&b.right<=innerWidth})()'));
        }
        await win.webContents.setZoomFactor(2);
        await waitFor('innerWidth===640 && document.querySelector(".shell-layout").clientWidth===640 && document.querySelector(".shell-layout").dataset.mode==="compact"');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),true);
        assert.ok(await evaluate('(()=>{const b=document.querySelector("[data-review-return]").getBoundingClientRect();return b.width>0&&b.right<=innerWidth&&b.bottom<=innerHeight&&document.documentElement.scrollWidth<=innerWidth})()'),'return remains reachable at 200% zoom');
        await win.webContents.setZoomFactor(1);
        await waitFor('innerWidth===1280 && document.querySelector(".shell-layout").clientWidth===1280 && document.querySelector(".shell-layout").dataset.mode==="wide"');
        await evaluate('fixture.output("output while reading\\r\\n")');
        if (process.env.ORKWORKS_CAPTURE_REVIEW) {
          const {writeFileSync} = await import('node:fs');
          writeFileSync(process.env.ORKWORKS_CAPTURE_REVIEW,(await win.webContents.capturePage()).toPNG());
        }
        assert.equal(await evaluate('fixture.prompts'),0);
        assert.equal(await evaluate('fixture.activeWrites===fixture.writesBefore'),true,'reading does not select/acknowledge');
        await evaluate('document.querySelector("[data-shell-review-heading]").focus();document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        await waitFor('document.activeElement?.dataset.shellCommand==="review"');
        assert.equal(await evaluate('fixture.runtime()===fixture.originalRuntime'),true);
        assert.equal(await evaluate('fixture.socketCount'),1);
        assert.match(await evaluate('fixture.runtime().terminal.buffer.active.getLine(0).translateToString()'),/output while reading/);
        await click('details');
        await waitFor('document.querySelector("[data-session-plan]")');
        await evaluate('document.querySelector("[data-session-plan]").click()');
        await waitFor('document.querySelector("[data-shell-region=review]")');
        await evaluate('fixture.navigationBeforeRefresh=fixture.navigation.length;fixture.plan="# Revised plan";document.querySelector("[data-review-refresh]").click()');
        await waitFor('document.querySelector(".review-change-notice[role=status]")?.textContent.includes("Plan changed")');
        await waitFor('document.querySelector(".review-plan-content")?.textContent.includes("Revised plan")');
        assert.equal(await evaluate('fixture.prompts'),0,'refresh does not request review');
        assert.equal(await evaluate('fixture.navigation.length'),await evaluate('fixture.navigationBeforeRefresh'),'refresh cannot update navigation recency');
        await evaluate('fixture.deferRead=true;fixture.resolveRead=null;document.querySelector("[data-review-refresh]").click()');
        await waitFor('!!fixture.resolveRead');
        await click('details');
        await waitFor('document.querySelector("[data-shell-region=inspector]")');
        await evaluate('fixture.resolveRead("# Revised plan")');
        await waitFor('!document.querySelector("[data-review-refresh]").disabled');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),true,'refresh cannot close a newer inspection');
        await evaluate('fixture.deferRead=false;document.querySelector("[data-shell-return]").click()');
        await waitFor('document.querySelector(".review-plan-content")?.textContent.includes("Revised plan")');
        await resizeReview(640);
        await click('details');
        await waitFor('document.querySelector("[data-shell-region=utility]")');
        assert.match(await evaluate('document.querySelector("[data-shell-return]").textContent'),/Back to Review/);
        await evaluate('document.querySelector("[data-shell-return]").click()');
        await waitFor('document.querySelector("[data-shell-region=review]")');
        await resizeReview(1280);
        await evaluate('document.querySelector("[data-review-return]").click()');
        await waitFor('document.activeElement?.classList.contains("xterm-helper-textarea")');
        await click('review');
        await waitFor('document.querySelector("[data-shell-region=review]")');

        await evaluate('document.querySelector("[data-review-request]").click();document.querySelector("[data-review-request]").click()');
        await waitFor('fixture.prompts===1');
        assert.equal(await evaluate('fixture.promptSession'),'created');
        await evaluate('fixture.finishPrompt();fixture.unreadable=true;document.querySelector("[data-review-refresh]").click()');
        await waitFor('document.querySelector("[data-shell-region=review]")?.textContent.includes("no longer available")');
        assert.equal(await evaluate('!!document.querySelector(".review-plan-content")'),false,'unreadable refresh removes stale content');
        await evaluate(`fixture.unreadable=false;const retry=Array.from(document.querySelectorAll('[data-shell-region=review] button')).find(button=>button.textContent==='Retry');retry.focus();retry.click()`);
        await waitFor('document.querySelector(".review-plan-content")?.textContent.includes("Revised plan")');
        assert.equal(await evaluate('fixture.prompts'),1,'Retry reads content without submitting another prompt');
        await evaluate('fixture.unreadable=false;document.querySelector("[data-review-return]").click()');
        await waitFor('document.querySelector("[data-shell-region=terminal]")');
        await evaluate('fixture.deferRead=true');
        await click('review');
        await waitFor('!!fixture.resolveRead');
        await click('terminal');
        await evaluate('fixture.resolveRead("# Too late")');
        await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false,'return fences pending entry');
        await evaluate('fixture.resolveRead=null');
        await click('review');
        await waitFor('!!fixture.resolveRead');
        await click('details');
        await evaluate('fixture.resolveRead("# Superseded by inspection")');
        await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false,'pending entry cannot replace newer inspection');
        await click('details');
        await evaluate('fixture.deferRead=false;fixture.menu({action:"focus",panelId:"review"})');
        await waitFor('document.querySelector("[data-shell-region=review]")');
        await evaluate(`Array.from(document.querySelectorAll('[data-session-id]')).find(row=>row.textContent.includes('other')).click()`);
        await waitFor('document.querySelector("[data-shell-region=terminal]")');
        assert.equal(await evaluate('!!document.querySelector(".review-plan-content")'),false,'selection removes the previous subject');
        await evaluate('fixture.menu({action:"focus",panelId:"review"})');
        await waitFor('document.querySelector(".review-plan-content")?.textContent.includes("Other session plan")');
        assert.equal(await evaluate('fixture.reads.at(-1)'),'other','Review reads the exact selected session');
        assert.equal(await evaluate('document.querySelector(".review-plan-content").textContent.includes("Revised plan")'),false);
        await evaluate('document.querySelector("[data-review-return]").click()');
        await waitFor('document.querySelector("[data-shell-region=terminal]")');
        await evaluate('fixture.deferRead=true;fixture.resolveRead=null;fixture.menu({action:"focus",panelId:"review"})');
        await waitFor('!!fixture.resolveRead');
        await evaluate('fixture.lifecycle({state:"opening"})');
        await evaluate('fixture.resolveRead("# Previous workspace document")');
        await evaluate('fixture.deferRead=false;fixture.lifecycle({state:"ready",workspace:{path:"/tmp/new",workspaceIdentity:"/tmp/new",lastActiveSessionId:"other",activeHarnessIds:["generic-shell"]},historyDiagnostic:null})');
        await waitFor('document.querySelector(".shell-context h1")?.textContent==="other"');
        await evaluate('new Promise(resolve=>setTimeout(resolve,50))');
        assert.equal(await evaluate('!!document.querySelector("[data-shell-region=review]")'),false);
        assert.equal(await evaluate('document.body.textContent.includes("Previous workspace document")'),false,'old generation bytes never paint after workspace switch');
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario === 'app-new-session') {
        await waitFor('document.querySelector("[data-shell-region=new-session] textarea")');
        assert.equal(await evaluate('fixture.posts'),0,'empty workspace never launches automatically');
        assert.equal(await evaluate('!!document.querySelector("[role=dialog]")'),false);
        await evaluate(`(()=>{const prompt=document.querySelector('#nsd-prompt');Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,'value').set.call(prompt,'My initial task');prompt.dispatchEvent(new Event('input',{bubbles:true}));})()`);
        await evaluate('fixture.failCreation=true;document.querySelector(".new-session-confirm").click()');
        await waitFor('fixture.posts===1&&!document.querySelector(".new-session-confirm").disabled');
        assert.equal(await evaluate('document.querySelector("#nsd-prompt").value'),'My initial task','failed creation retains the prompt');
        await evaluate('fixture.lifecycle({state:"ready",workspace:{path:"/tmp/other",workspaceIdentity:"/tmp/other",lastActiveSessionId:null,activeHarnessIds:["generic-shell"]},historyDiagnostic:null})');
        await waitFor('document.querySelector("#nsd-prompt").value===""');
        await evaluate('document.querySelector(".new-session-cancel").click()');
        await waitFor('!document.querySelector("[data-shell-region=new-session]")');
        await evaluate('fixture.menu({action:"new-session"})');
        await waitFor('document.querySelector("#nsd-prompt")');
        await evaluate('fixture.failCreation=false;document.querySelector(".new-session-confirm").click()');
        await waitFor('document.querySelector("[data-shell-region=terminal]")');
        await waitFor('fixture.runtime()?.terminal.element?.isConnected');
        assert.equal(await evaluate('fixture.posts'),2);
        assert.deepEqual(await evaluate('fixture.errors'),[]);
        return;
      } else if (scenario.startsWith('preferences-')) {
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
        if (scenario === 'inspector-dismissal') {
          const nativeClick = async selector => {
            const point = await evaluate('(()=>{const r=document.querySelector('+JSON.stringify(selector)+').getBoundingClientRect();return {x:Math.round(r.left+r.width/2),y:Math.round(r.top+r.height/2)}})()');
            win.webContents.sendInputEvent({type:'mouseDown',button:'left',clickCount:1,...point});
            win.webContents.sendInputEvent({type:'mouseUp',button:'left',clickCount:1,...point});
            await evaluate('new Promise(resolve=>requestAnimationFrame(resolve))');
          };
          for (const command of ['actions']) {
            await click(command);
            await waitFor('document.querySelector("[data-shell-region=inspector]")');
            await nativeClick('[data-shell-region=inspector] h2');
            assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),true,command+' content keeps the inspector open');
            await nativeClick('.shell-inspector-host [role=separator]');
            assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),true,command+' resize handle keeps the inspector open');
            await nativeClick('[data-shell-terminal-focus]');
            assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),false,'outside click hides unpinned '+command);
            assert.equal(await evaluate('document.activeElement.hasAttribute("data-shell-terminal-focus")'),true,'outside click retains its focus target');
            await click(command);
            await waitFor('document.querySelector("[data-shell-pin]")');
            await evaluate('document.querySelector("[data-shell-pin]").click()');
            await waitFor('document.querySelector("[data-shell-pin]").getAttribute("aria-pressed")==="true"');
            await nativeClick('[data-shell-terminal-focus]');
            assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),true,'outside click preserves pinned '+command);
            await evaluate('document.querySelector("[data-shell-pin]").click()');
            await waitFor('document.querySelector("[data-shell-pin]").getAttribute("aria-pressed")==="false"');
            await evaluate('(()=>{const button=document.querySelector('+JSON.stringify('[data-shell-command='+command+']')+');button.dispatchEvent(new PointerEvent("pointerdown",{bubbles:true}));button.click()})()');
            await waitFor('!document.querySelector("[data-shell-region=inspector]")');
            await win.setContentSize(1000,700);
            await waitFor('document.querySelector(".shell-layout").dataset.mode==="medium"');
            await click(command);
            await waitFor('document.querySelector("[data-shell-region=utility]")');
            await nativeClick('[data-shell-terminal-focus]');
            assert.equal(await evaluate('!!document.querySelector("[data-shell-region=utility]")'),true,'outside click preserves responsive '+command+' page');
            await evaluate('document.querySelector("[data-shell-return]").click()');
            await waitFor('document.querySelector("[data-shell-region=terminal]")');
            await win.setContentSize(1280,700);
            await waitFor('document.querySelector(".shell-layout").dataset.mode==="wide"');
          }
          assert.equal(await evaluate('fixture.selections'),0);
          assert.equal(await evaluate('fixture.input.length'),0);
        } else if (scenario === 'inspector-overlay') {
          const mainWidth = await evaluate('document.querySelector(".shell-main").getBoundingClientRect().width');
          await click('details');
          await waitFor('document.querySelector("[data-shell-region=inspector]")');
          assert.equal(await evaluate('document.querySelector(".shell-main").getBoundingClientRect().width'), mainWidth, 'overlay must not shrink central view');
          assert.equal(await evaluate('document.querySelector("[data-shell-region=inspector]").getBoundingClientRect().right'), await evaluate('document.querySelector(".shell-main").getBoundingClientRect().right'));
          assert.match(await evaluate('getComputedStyle(document.querySelector("[data-shell-region=inspector]")).backgroundColor'), /0\.96/);
          const assertCornerControls = async () => {
            const geometry = await evaluate(`(()=>{
              const pane=document.querySelector('[data-shell-region=inspector]');
              const header=pane.querySelector('header');
              const rect=element=>{const r=element.getBoundingClientRect();return {top:r.top,right:r.right,bottom:r.bottom}};
              return {header:rect(header),pin:rect(header.querySelector('[data-shell-pin]')),close:rect(header.querySelector('[data-shell-return]'))};
            })()`);
            assert.ok(Math.abs(geometry.pin.top - geometry.close.top) <= 1, 'pin and close share the top row');
            assert.ok(geometry.close.top - geometry.header.top <= 12, 'controls stay at the top of the header');
            assert.ok(geometry.header.right - geometry.close.right <= 16, 'close stays at the right edge');
            assert.ok(geometry.pin.right < geometry.close.right, 'pin precedes close');
          };
          await assertCornerControls();
          await evaluate('fixture.command("actions",true)');
          await waitFor('document.querySelector("[data-shell-region=inspector] h2").textContent==="Actions"');
          await evaluate('document.querySelector(".shell-utility-scope").textContent="Workspace: /Users/developer/workspace/a-project-with-a-long-directory-name"');
          await assertCornerControls();
          await evaluate('fixture.command("details",true)');
          await waitFor('document.querySelector("[data-shell-region=inspector] h2").textContent==="Details"');
          // Overflowing detail content scrolls below the stationary controls.
          await evaluate(`(()=>{const pane=document.querySelector('[data-shell-region=inspector]');const content=document.createElement('div');content.className='panel-content';content.style.overflowY='auto';content.innerHTML='<div style="height:2000px">Long details</div>';pane.append(content);content.scrollTop=300})()`);
          assert.equal(await evaluate('document.querySelector("[data-shell-region=inspector] > .panel-content").scrollTop'),300);
          await assertCornerControls();
          await evaluate('document.querySelector("[data-shell-region=inspector] > .panel-content").remove()');
          const outsideClick = async () => {
            const point = await evaluate('(()=>{const r=document.querySelector("[data-shell-terminal-focus]").getBoundingClientRect();return {x:Math.round(r.left+10),y:Math.round(r.top+10)}})()');
            win.webContents.sendInputEvent({type:'mouseDown',button:'left',clickCount:1,...point});
            win.webContents.sendInputEvent({type:'mouseUp',button:'left',clickCount:1,...point});
            await evaluate('new Promise(resolve=>requestAnimationFrame(resolve))');
          };
          const separatorPoint = await evaluate('(()=>{const r=document.querySelector(".shell-inspector-host [role=separator]").getBoundingClientRect();return {x:Math.round(r.left+r.width/2),y:Math.round(r.top+40)}})()');
          win.webContents.sendInputEvent({type:'mouseDown',button:'left',clickCount:1,...separatorPoint});
          win.webContents.sendInputEvent({type:'mouseUp',button:'left',clickCount:1,...separatorPoint});
          await evaluate('new Promise(resolve=>requestAnimationFrame(resolve))');
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),true,'resize handle preserves unpinned Details');
          await outsideClick();
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),false,'outside click hides unpinned Details');
          assert.notEqual(await evaluate('document.activeElement.dataset.shellCommand'),'details','outside click does not restore invoker focus');
          const togglePoint = await evaluate('(()=>{const r=document.querySelector("[data-shell-command=details]").getBoundingClientRect();return {x:Math.round(r.left+r.width/2),y:Math.round(r.top+r.height/2)}})()');
          win.webContents.sendInputEvent({type:'mouseDown',button:'left',clickCount:1,...togglePoint});
          win.webContents.sendInputEvent({type:'mouseUp',button:'left',clickCount:1,...togglePoint});
          await evaluate('new Promise(resolve=>requestAnimationFrame(resolve))');
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),true,'Details toggle opens normally after outside dismissal');
          // The overlay can cover the toolbar toggle; exercise its event sequence directly.
          await evaluate('(()=>{const button=document.querySelector("[data-shell-command=details]");button.dispatchEvent(new PointerEvent("pointerdown",{bubbles:true}));button.click()})()');
          await waitFor('!document.querySelector("[data-shell-region=inspector]")');
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),false,'Details toggle closes without reopening');
          await click('details');
          await waitFor('document.querySelector("[data-shell-region=inspector]")');
          // A real click on non-interactive inspector content must keep Escape in its region.
          const panelPoint = await evaluate('(()=>{const r=document.querySelector("[data-shell-region=inspector]").getBoundingClientRect();return {x:Math.round(r.left+40),y:Math.round(r.bottom-40)}})()');
          win.webContents.sendInputEvent({type:'mouseDown',button:'left',clickCount:1,...panelPoint});
          win.webContents.sendInputEvent({type:'mouseUp',button:'left',clickCount:1,...panelPoint});
          await evaluate('new Promise(resolve=>requestAnimationFrame(resolve))');
          assert.equal(await evaluate('!!document.activeElement.closest("[data-shell-region=inspector]")'),true,'clicking inspector content retains focus in the inspector');
          win.webContents.sendInputEvent({type:'keyDown',keyCode:'Escape'});
          win.webContents.sendInputEvent({type:'keyUp',keyCode:'Escape'});
          await waitFor('!document.querySelector("[data-shell-region=inspector]")');
          assert.equal(await evaluate('document.activeElement.dataset.shellCommand'),'details');
          await click('details');
          await waitFor('document.querySelector("[data-shell-region=inspector]")');
          await evaluate('document.querySelector("[data-shell-region=inspector] h2").focus();document.activeElement.addEventListener("keydown",event=>event.preventDefault(),{once:true});document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true,cancelable:true}))');
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),true,'handled Escape belongs to nested control');
          await evaluate('document.querySelector(".shell-inspector-host [role=separator]").dispatchEvent(new KeyboardEvent("keydown",{key:"End",bubbles:true}))');
          await waitFor('document.querySelector(".shell-inspector-host [role=separator]").getAttribute("aria-valuenow")==="420"');
          assert.equal(await evaluate('document.querySelector(".shell-main").getBoundingClientRect().width'),mainWidth,'overlay resizing preserves central width');
          if (process.env.ORKWORKS_CAPTURE_INSPECTOR) {
            const {writeFileSync}=await import('node:fs');
            writeFileSync(process.env.ORKWORKS_CAPTURE_INSPECTOR+'-overlay.png',(await win.webContents.capturePage()).toPNG());
          }
          await evaluate('document.querySelector("[data-shell-command=details]").focus();document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),true,'Escape outside inspector preserves it');
          await evaluate('document.querySelector("[data-shell-pin]").click()');
          await waitFor('document.querySelector("[data-shell-pin]").getAttribute("aria-pressed")==="true"');
          await outsideClick();
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),true,'outside click preserves pinned Details');
          assert.ok(await evaluate('document.querySelector(".shell-main").getBoundingClientRect().width') < mainWidth);
          assert.equal(await evaluate('getComputedStyle(document.querySelector("[data-shell-region=inspector]")).backgroundColor'), await evaluate('getComputedStyle(document.querySelector("[data-shell-region=sessions]")).backgroundColor'));
          await assertCornerControls();
          if (process.env.ORKWORKS_CAPTURE_INSPECTOR) {
            const {writeFileSync}=await import('node:fs');
            writeFileSync(process.env.ORKWORKS_CAPTURE_INSPECTOR+'-pinned.png',(await win.webContents.capturePage()).toPNG());
          }
          await evaluate('document.querySelector("[data-shell-pin]").focus();document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
          await waitFor('!document.querySelector("[data-shell-region=inspector]")');
          assert.equal(await evaluate('document.activeElement.dataset.shellCommand'),'details');
          await click('details');
          await waitFor('document.querySelector("[data-shell-pin]")');
          assert.equal(await evaluate('document.querySelector("[data-shell-pin]").getAttribute("aria-pressed")'),'true');
          await evaluate('document.querySelector("[data-shell-pin]").click()');
          await waitFor('document.querySelector("[data-shell-pin]").getAttribute("aria-pressed")==="false"');
          assert.equal(await evaluate('document.querySelector(".shell-main").getBoundingClientRect().width'),mainWidth);
          await evaluate('document.querySelector("[data-shell-page-heading]").focus();document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
          await waitFor('!document.querySelector("[data-shell-region=inspector]")');
          assert.equal(await evaluate('document.activeElement.dataset.shellCommand'),'details');
          assert.equal(await evaluate('fixture.input.length'),0);
          await click('recommendations');
          await waitFor('document.querySelector(".recommendations-panel")');
          assert.equal(await evaluate('!!document.querySelector("main .recommendations-panel")'),true,'Recommendations occupies the main pane');
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'),false);
          await evaluate('document.querySelector("[data-shell-page-heading]").focus();document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
          await waitFor('!document.querySelector("[data-shell-region=inspector]")');
          assert.equal(await evaluate('document.activeElement.dataset.shellCommand'),'recommendations');
          await click('details');
          await waitFor('document.querySelector("[data-shell-pin]")');
          await evaluate('document.querySelector("[data-shell-pin]").click()');
          await waitFor('document.querySelector("[data-shell-pin]").getAttribute("aria-pressed")==="true"');
          await evaluate('document.querySelector(".shell-inspector-host [role=separator]").focus();document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
          await waitFor('!document.querySelector("[data-shell-region=inspector]")');
          assert.equal(await evaluate('document.activeElement.dataset.shellCommand'),'details');
        } else if (scenario === 'actions-sources') {
          await waitFor('document.querySelector("[data-actions-counts]")?.textContent.includes("Recommendations: 2")');
          await click('actions');
          await waitFor('document.querySelector(".actions-overview")');
          assert.deepEqual(await evaluate('Array.from(document.querySelectorAll("[data-action-key]")).map(el=>el.dataset.actionKey)'),['session:coding','session:other','recommendation:r2','recommendation:r1']);
          await evaluate('fixture.setSessionSourceStatus("unavailable")');
          await waitFor('document.querySelector("[data-actions-counts]").textContent.includes("Session attention: unavailable")');
          await click('next-action');
          await waitFor('document.activeElement.dataset.actionKey === "recommendation:r2"');
          await evaluate('document.querySelector("[data-actions-detail] button").click()');
          assert.deepEqual(await evaluate('fixture.openedRecommendations'),['r2']);
          await evaluate('fixture.setSessionState({harnessId:"generic-shell",harness:"generic-shell"});fixture.command("recommendations",true)');
          await waitFor('document.querySelector("main [data-shell-region=utility] h2")?.textContent==="Recommendations"');
          await click('actions');

          await evaluate('fixture.partial=true;fixture.setBackendStatus("unreachable")');
          await waitFor('document.querySelector("[data-actions-counts]").textContent.includes("Recommendations: unavailable")');
          await evaluate('fixture.setBackendStatus("connected")');
          await waitFor('document.querySelector("[data-actions-counts]").textContent.includes("Recommendations: 2 known (partial)")');
          await click('next-action');
          await waitFor('document.activeElement.dataset.actionKey === "recommendation:r2"');
          assert.equal(await evaluate('document.querySelector("[data-actions-detail] h3")?.textContent'),'r2','known partial-source items remain inspectable');
          await evaluate('fixture.pauseRecommendations=true;fixture.setBackendStatus("unreachable")');
          await waitFor('document.querySelector("[data-actions-counts]").textContent.includes("Recommendations: unavailable")');
          await evaluate('fixture.setBackendStatus("connected")');
          await waitFor('fixture.pendingRecommendations.length===1');
          await evaluate('fixture.advanceWorkspace()');
          await waitFor('fixture.pendingRecommendations.length===2');
          await evaluate('fixture.pendingRecommendations[0]({recommendations:[fixture.recommendation("old")],diagnostics:[]})');
          await evaluate('new Promise(resolve=>requestAnimationFrame(resolve))');
          assert.equal(await evaluate('Array.from(document.querySelectorAll("[data-action-key]")).some(el=>el.dataset.actionKey==="recommendation:old")'),false);
          await evaluate('fixture.pendingRecommendations[1]({recommendations:[fixture.recommendation("new")],diagnostics:[]})');
          await waitFor('document.querySelector("[data-actions-counts]").textContent.includes("Recommendations: 1")');
          assert.equal(await evaluate('fixture.selections'),0);
          assert.equal(await evaluate('fixture.input.length'),0);
          assert.deepEqual(await evaluate('fixture.errors'),[]);
        } else if (scenario === 'actions') {
          await waitFor('document.querySelector("[data-actions-counts]")?.textContent.includes("Session attention: 1")');
          const attentionBefore = await evaluate('document.querySelectorAll(".session-row--unread.session-row--loud").length');
          assert.equal(attentionBefore,1);
          await click('next-action');
          await waitFor('document.activeElement.dataset.actionKey === "session:other"');
          assert.equal(await evaluate('fixture.selections'),0);
          assert.equal(await evaluate('fixture.input.length'),0);
          assert.equal(await evaluate('fixture.activeSessionId'),'coding');
          assert.equal(await evaluate('document.querySelectorAll(".session-row--unread.session-row--loud").length'),attentionBefore,'inspection preserves unread and unacknowledged attention');
          assert.equal(await evaluate('fixture.prompts'),0);
          assert.match(await evaluate('document.querySelector(".actions-overview").textContent'),/Needs You/);
          if (process.env.ORKWORKS_CAPTURE_ACTIONS) {
            const {writeFileSync}=await import('node:fs');
            writeFileSync(process.env.ORKWORKS_CAPTURE_ACTIONS+'-wide.png',(await win.webContents.capturePage()).toPNG());
          }
          await evaluate('document.querySelector("[data-shell-return]").click()');
          await waitFor('document.activeElement.dataset.shellCommand === "next-action"');
          await click('sessions');
          await waitFor('!document.querySelector("[data-shell-region=sessions]")');
          assert.match(await evaluate('document.querySelector("[data-actions-counts]").textContent'),/Session attention: 1/);
          await win.setContentSize(640,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="compact"');
          await click('next-action');
          await waitFor('document.activeElement.dataset.actionKey === "session:other"');
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=utility]")'),true);
          if (process.env.ORKWORKS_CAPTURE_ACTIONS) {
            const {writeFileSync}=await import('node:fs');
            writeFileSync(process.env.ORKWORKS_CAPTURE_ACTIONS+'-compact.png',(await win.webContents.capturePage()).toPNG());
          }
          await evaluate('document.querySelector("[data-action-key=\\"session:coding\\"]").click()');
          await waitFor('document.querySelector("[data-actions-detail] h3")?.textContent=== "coding"');
          await win.setContentSize(1280,700);
          await waitFor('document.querySelector("[data-shell-region=inspector]")');
          assert.equal(await evaluate('document.querySelector("[data-actions-detail] h3")?.textContent'), 'coding', 'responsive host changes retain the inspected identity');
          await evaluate('fixture.setSessionState({attention:"working"})');
          await waitFor('!document.querySelector("[data-actions-detail]")');
          assert.equal(await evaluate('fixture.selections'),0);
          assert.match(await evaluate('document.querySelector(".actions-overview").textContent'),/unavailable or changed/);
          await evaluate('fixture.setSessionState({})');
          await evaluate('new Promise(resolve=>requestAnimationFrame(resolve))');
          assert.equal(await evaluate('!!document.querySelector("[data-actions-detail]")'),false,'stale inspection cannot reactivate when its revision returns');
          await evaluate('document.querySelector("[data-action-key=\\"session:coding\\"]").click()');
          await waitFor('document.querySelector("[data-actions-detail]")');
          await evaluate('fixture.setEmpty(true)');
          await waitFor('!document.querySelector("[data-actions-detail]")');
          assert.match(await evaluate('document.querySelector(".actions-overview").textContent'),/coding.*unavailable or changed/);
          await evaluate('fixture.setEmpty(false);fixture.advanceWorkspace()');
          await waitFor('!document.querySelector("[data-actions-detail]")');
          await evaluate('document.querySelector("[data-shell-return]").click()');
          await waitFor('document.querySelector("[data-shell-region=terminal]")');
          await waitFor('document.activeElement.hasAttribute("data-shell-terminal-focus") || document.activeElement.classList.contains("xterm-helper-textarea")');
          assert.deepEqual(await evaluate('fixture.errors'),[]);
        } else if (scenario === 'resume-session') {
          assert.equal(await evaluate('!!document.querySelector("[data-shell-resume]")'),false,'live sessions have no resume action');
          await evaluate('fixture.setSessionState({lifecycle:"dead",status:"ended",memoryState:"resumable",resumeStrategy:"exact",resumeOptions:[{strategy:"exact",available:true,label:"Exact",preferred:true},{strategy:"latest_cwd",available:true,label:"Latest in folder"},{strategy:"latest_repo",available:false,label:"Latest in repo",reason:"Not supported"}]})');
          await waitFor('document.querySelector("[data-shell-resume]")');
          await evaluate('document.querySelector("[data-shell-resume]").click()');
          assert.deepEqual(await evaluate('fixture.resumed'),[{id:'coding',strategy:'exact'}],'main button resumes the preferred exact session');
          await evaluate('fixture.setResuming(["coding"])');
          await waitFor('document.querySelector("[data-shell-resume]").disabled');
          assert.match(await evaluate('document.querySelector("[data-shell-resume]").textContent'),/Resuming/);
          assert.match(await evaluate('document.querySelector("[data-session-id=coding]").textContent'),/Resuming/);
          assert.equal(await evaluate('document.querySelector("[data-resume-toggle]").disabled'),true);
          await evaluate('document.querySelector("[data-shell-resume]").click()');
          assert.equal(await evaluate('fixture.resumed.length'),1,'pending resume suppresses duplicate clicks');
          await click('details');
          await waitFor('document.querySelector(".session-detail .resume-split-button")');
          assert.equal(await evaluate('[...document.querySelectorAll(".session-detail .resume-split-button button")].every(button=>button.disabled)'),true,'Details uses the same pending state');
          await evaluate('fixture.setResuming([]);fixture.setPreferences(old=>({...old,inspectorWidth:280}))');
          await waitFor('!document.querySelector("[data-shell-resume]").disabled');
          await evaluate('document.querySelector(".session-detail [data-resume-toggle]").click()');
          await waitFor('document.querySelector(".session-detail [data-resume-options]")');
          assert.ok(await evaluate('(()=>{const panel=document.querySelector(".session-detail").getBoundingClientRect();const menu=document.querySelector(".session-detail [data-resume-options]").getBoundingClientRect();return menu.left>=panel.left&&menu.right<=panel.right})()'),'Details choices stay inside the narrow inspector scrollport');
          await evaluate('document.querySelector(".session-detail [data-resume-strategy=latest_cwd]").click()');
          await waitFor('document.querySelector("[data-resume-warning]")?.open');
          await evaluate('const cancel=document.querySelector("[data-resume-warning] button");cancel.dispatchEvent(new PointerEvent("pointerdown",{bubbles:true}));cancel.click()');
          await waitFor('!document.querySelector("[data-resume-warning]")');
          assert.equal(await evaluate('!!document.querySelector(".session-detail")'),true,'warning interaction preserves its Details inspector');
          assert.equal(await evaluate('fixture.resumed.length'),1,'cancelling the Details warning does not resume');
          await click('details');
          await evaluate('fixture.resumed=[];document.querySelector("[data-resume-toggle]").click()');
          await waitFor('document.querySelector("[data-resume-options]")');
          assert.deepEqual(await evaluate('fixture.resumed'),[],'opening choices does not resume');
          if (process.env.ORKWORKS_CAPTURE_RESUME) {
            await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
            const {writeFileSync}=await import('node:fs');
            writeFileSync(process.env.ORKWORKS_CAPTURE_RESUME+'-menu.png',(await win.webContents.capturePage()).toPNG());
          }
          assert.equal(await evaluate('document.querySelector("[data-resume-strategy=latest_repo]").disabled'),true);
          assert.match(await evaluate('document.querySelector("[data-resume-options]").textContent'),/Not supported/);
          await evaluate('document.querySelector("[data-resume-strategy=exact]").click()');
          assert.deepEqual(await evaluate('fixture.resumed'),[{id:'coding',strategy:'exact'}]);
          await waitFor('!document.querySelector("[data-resume-options]")');
          await evaluate('document.querySelector("[data-resume-toggle]").click()');
          await waitFor('document.querySelector("[data-resume-options]")');
          await evaluate('document.querySelector("[data-resume-options]").dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true,cancelable:true}))');
          await waitFor('!document.querySelector("[data-resume-options]")');
          assert.equal(await evaluate('document.activeElement.hasAttribute("data-resume-toggle")'),true);
          await win.setContentSize(360,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="compact"');
          await evaluate('document.querySelector("[data-resume-toggle]").click()');
          await waitFor('document.querySelector("[data-resume-options]")');
          assert.ok(await evaluate('(()=>{const r=document.querySelector("[data-resume-options]").getBoundingClientRect();return r.x>=0&&r.right<=innerWidth})()'),'choices stay inside compact viewport');
          await evaluate('fixture.select("other")');
          await waitFor('document.querySelector(".shell-context h1").textContent==="other"');
          assert.equal(await evaluate('!!document.querySelector("[data-resume-options]")'),false,'changing session closes stale choices');
          await evaluate('document.querySelector("[data-resume-toggle]").click()');
          await waitFor('document.querySelector("[data-resume-options]")');
          await evaluate('document.querySelector("[data-resume-strategy=latest_cwd]").click()');
          await waitFor('document.querySelector("[data-resume-warning]")?.open');
          assert.deepEqual(await evaluate('fixture.resumed'),[{id:'coding',strategy:'exact'}],'fallback requires confirmation');
          assert.match(await evaluate('document.querySelector("[data-resume-warning]").textContent'),/different session/);
          assert.equal(await evaluate('document.activeElement.textContent'),'Cancel');
          if (process.env.ORKWORKS_CAPTURE_RESUME) {
            await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
            const {writeFileSync}=await import('node:fs');
            writeFileSync(process.env.ORKWORKS_CAPTURE_RESUME+'-warning.png',(await win.webContents.capturePage()).toPNG());
          }
          await evaluate('document.activeElement.click()');
          await waitFor('!document.querySelector("[data-resume-warning]")');
          assert.equal(await evaluate('fixture.resumed.length'),1,'Cancel does not resume');
          await evaluate('document.querySelector("[data-resume-toggle]").click()');
          await waitFor('document.querySelector("[data-resume-options]")');
          await evaluate('document.querySelector("[data-resume-strategy=latest_cwd]").click()');
          await waitFor('document.querySelector("[data-resume-warning]")?.open');
          await evaluate('document.querySelector("[data-resume-confirm]").click()');
          assert.deepEqual(await evaluate('fixture.resumed'),[{id:'coding',strategy:'exact'},{id:'other',strategy:'latest_cwd'}]);
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=utility]")'),false,'choices do not navigate to Details');
          await evaluate('fixture.setSessionState({lifecycle:"dead",memoryState:"resumable",resumeStrategy:"latest_cwd",resumeOptions:[{strategy:"latest_cwd",available:true,label:"Latest in folder",preferred:true}]})');
          await waitFor('document.querySelector("[data-shell-resume]").textContent.includes("Latest in folder")');
          await evaluate('document.querySelector("[data-shell-resume]").click()');
          await waitFor('document.querySelector("[data-resume-warning]")?.open');
          assert.equal(await evaluate('fixture.resumed.length'),2,'main-button fallback also requires confirmation');
          await evaluate('fixture.select("coding")');
          await waitFor('!document.querySelector("[data-resume-warning]")');
          await evaluate('fixture.setSessionState({lifecycle:"dead",memoryState:"unsupported",resumeStrategy:"none",resumeOptions:[{strategy:"exact",available:false,label:"Exact",reason:"Missing ID"}]})');
          await waitFor('!document.querySelector("[data-shell-resume]")');
          assert.deepEqual(await evaluate('fixture.errors'),[]);
        } else if (scenario === 'new-session') {
          await evaluate('fixture.originalRuntime=fixture.runtime();document.querySelector("[data-shell-create]").click()');
          await waitFor('document.querySelector("[data-shell-region=new-session] textarea")');
          assert.equal(await evaluate('!!document.querySelector("[role=dialog]")'),false);
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=sessions]")'),true);
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=terminal]")'),false);
          await waitFor('document.activeElement.id==="nsd-prompt"');
          await evaluate('document.querySelector(".new-session-cancel").focus();fixture.command("new-session")');
          await waitFor('document.activeElement.id==="nsd-prompt"');
          await click('recommendations');
          await waitFor('document.querySelector("[data-shell-region=new-session]").hidden');
          await evaluate('document.querySelector("[data-shell-create]").focus();document.querySelector("[data-shell-create]").click()');
          await waitFor('!document.querySelector("[data-shell-region=new-session]").hidden');
          await waitFor('document.activeElement.id==="nsd-prompt"');
          await evaluate('document.querySelector(".new-session-cancel").focus();fixture.setSessionState({status:"waiting"})');
          await waitFor('document.querySelector(".new-session-cancel")===document.activeElement');
          if (process.env.ORKWORKS_CAPTURE_COMPOSER) {
            await evaluate('new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))');
            const {writeFileSync} = await import('node:fs');
            writeFileSync(process.env.ORKWORKS_CAPTURE_COMPOSER+'-wide.png',(await win.webContents.capturePage()).toPNG());
          }
          await evaluate('fixture.output("while composing\\r\\n")');
          await evaluate('document.querySelector(".new-session-cancel").click()');
          await waitFor('fixture.runtime()?.terminal.element?.isConnected');
          assert.equal(await evaluate('fixture.runtime()===fixture.originalRuntime'),true);
          assert.equal(await evaluate('fixture.socketCount'),1);
          await evaluate('document.querySelector("[data-shell-create]").click()');
          await waitFor('document.querySelector("[data-shell-region=new-session]")');
          await click('terminal');
          await waitFor('document.querySelector("[data-shell-region=terminal]")');
          await evaluate('document.querySelector("[data-shell-create]").click()');
          await waitFor('document.querySelector("[data-shell-region=new-session]")');
          await waitFor('document.activeElement.id==="nsd-prompt"');
          await evaluate(`(()=>{const prompt=document.querySelector('#nsd-prompt');Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,'value').set.call(prompt,'Keyboard task');prompt.dispatchEvent(new Event('input',{bubbles:true}));})()`);
          for (const modifiers of [{shiftKey:true},{ctrlKey:true},{metaKey:true},{altKey:true},{isComposing:true}]) {
            assert.equal(await evaluate(`document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Enter",bubbles:true,cancelable:true,...${JSON.stringify(modifiers)}}))`),true,'modified or composing Enter keeps its default behavior');
            assert.equal(await evaluate('!!fixture.submitted'),false,'modified or composing Enter does not start a session');
          }
          assert.equal(await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Enter",bubbles:true,cancelable:true}))'),false,'Enter submits instead of inserting a newline');
          await waitFor('fixture.submitted');
          assert.equal(await evaluate('fixture.submitted.initialPrompt'),'Keyboard task');
          assert.equal(await evaluate('document.querySelector(".new-session-confirm").disabled'),true,'pending creation cannot be submitted twice');
          await evaluate('fixture.resolveCreate()');
          await waitFor('document.querySelector("[data-shell-region=terminal]")');
          await click('details');
          await waitFor('document.querySelector("[data-shell-region=inspector]")');
          await evaluate('document.querySelector("[data-shell-create]").click()');
          await waitFor('document.querySelector("[data-shell-region=new-session]")&&!document.querySelector("[data-shell-region=inspector]")');
          await evaluate('document.querySelector(".new-session-cancel").click()');
          await waitFor('document.querySelector("[data-shell-region=inspector]")');
          await click('details');
          await evaluate('document.querySelector("[data-shell-create]").click()');
          await waitFor('document.querySelector("[data-shell-region=new-session]")');
          await evaluate(`(()=>{const prompt=document.querySelector('#nsd-prompt');const set=Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,'value').set;set.call(prompt,'Keep this draft');prompt.dispatchEvent(new Event('input',{bubbles:true}));})()`);
          await win.setContentSize(640,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="compact"');
          await click('sessions');
          await waitFor('document.getElementById("sessions-list")===document.activeElement');
          await click('sessions');
          await waitFor('document.querySelector("[data-shell-region=new-session]")?.getClientRects().length');
          await waitFor('document.activeElement.id==="nsd-prompt"');
          assert.equal(await evaluate('document.querySelector("#nsd-prompt").value'),'Keep this draft','compact session browsing preserves the draft');
          const bounds = await evaluate(`(()=>{const prompt=document.querySelector('#nsd-prompt').getBoundingClientRect();const model=document.querySelector('#nsd-model').getBoundingClientRect();const start=document.querySelector('.new-session-confirm').getBoundingClientRect();return {promptRight:prompt.right,modelRight:model.right,startBottom:start.bottom,width:innerWidth,height:innerHeight};})()`);
          assert.ok(bounds.promptRight<=bounds.width && bounds.modelRight<=bounds.width && bounds.startBottom<=bounds.height,'compact composer controls stay inside the viewport');
          if (process.env.ORKWORKS_CAPTURE_COMPOSER) {
            await evaluate('new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))');
            const {writeFileSync} = await import('node:fs');
            writeFileSync(process.env.ORKWORKS_CAPTURE_COMPOSER+'-compact.png',(await win.webContents.capturePage()).toPNG());
          }
          await evaluate('fixture.advanceWorkspace()');
          await waitFor('document.querySelector("#nsd-prompt").value===""');
          assert.deepEqual(await evaluate('fixture.errors'),[]);
        } else if (scenario === 'sidebar-controls') {
          assert.equal(await evaluate('!!document.querySelector("[data-shell-region=sessions] [data-shell-command=recommendations]")'),true,'workspace navigation belongs to the sidebar');
          const anchors = await evaluate(`(()=>{
            const sidebar=document.querySelector('[data-shell-region=sessions]');
            const list=document.getElementById('sessions-list').parentElement;
            const footer=sidebar.querySelector('footer');
            const recommendations=sidebar.querySelector('[data-shell-command=recommendations]');
            list.scrollTop=list.scrollHeight;
            return {recommendations:recommendations.getBoundingClientRect().bottom,list:list.getBoundingClientRect().top,
              listBottom:list.getBoundingClientRect().bottom,footer:footer.getBoundingClientRect().top,
              footerBottom:footer.getBoundingClientRect().bottom,sidebarBottom:sidebar.getBoundingClientRect().bottom,
              scrolled:list.scrollTop>0};
          })()`);
          assert.ok(anchors.recommendations<=anchors.list,'navigation stays above history');
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
          await click('recommendations');
          await waitFor('document.querySelector("[data-shell-region=utility]")');
          assert.equal(await evaluate('fixture.selections'),0,'sidebar utility navigation preserves selected session');
          await click('recommendations');
          await click('recommendations');
          await waitFor('document.querySelector("[data-shell-region=utility]")');
          await click('sessions');
          await waitFor('!document.querySelector("[data-shell-region=sessions]")');
          await evaluate('fixture.command("recommendations")');
          await waitFor('!document.querySelector("[data-shell-region=utility]")');
          assert.ok(await evaluate('document.activeElement===document.querySelector("[data-shell-region=rail] [data-shell-command=recommendations]")'),'collapsed sidebar returns focus to its visible rail control');
          await click('sessions');
          await waitFor('document.querySelector("[data-shell-region=sessions]")');
          await click('recommendations');
          await waitFor('document.querySelector("[data-shell-region=utility]")');
          await win.setContentSize(640,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="compact"');
          await waitFor('document.querySelector("[data-shell-region=utility]")');
          await evaluate('document.querySelector("[data-shell-return]").click()');
          await waitFor('document.querySelector("[data-shell-region=terminal]")');
          assert.equal(await evaluate('document.activeElement.tagName'),'SUMMARY','responsive target loss returns to the visible workspace summary');
          await win.setContentSize(1280,700);
          await waitFor('document.querySelector("[data-shell-region=sessions]")');
          await click('sessions');
          await waitFor('!document.querySelector("[data-shell-region=sessions]")');
          await evaluate('document.querySelector(".shell-navigation-menu summary").click()');
          assert.equal(await evaluate(`(()=>{const picker=document.createElement('div');picker.className='workspace-picker-screen';document.body.append(picker);const r=document.querySelector('.shell-navigation-popover').getBoundingClientRect();const covered=document.elementFromPoint(r.x+10,r.y+10)?.closest('.workspace-picker-screen')===picker;picker.remove();return covered;})()`),true,'full-screen picker covers an open shell menu');
          assert.ok(await evaluate('document.querySelector(".shell-navigation-menu [data-shell-command=recommendations]").getClientRects().length>0'),'hidden sidebar retains reachable navigation');
          await evaluate('const summary=document.querySelector(".shell-navigation-menu summary");summary.focus();summary.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
          assert.equal(await evaluate('document.querySelector(".shell-navigation-menu").open'),false,'Escape dismisses the fallback menu');
          assert.equal(await evaluate('document.activeElement.tagName'),'SUMMARY','menu dismissal restores trigger focus');
          await evaluate('document.querySelector(".shell-navigation-menu summary").click()');
          await evaluate('const button=document.querySelector(".shell-navigation-menu [data-shell-command=recommendations]");button.focus();button.click()');
          await waitFor('document.querySelector("[data-shell-region=utility]")');
          assert.equal(await evaluate('document.querySelector(".shell-navigation-menu").open'),false,'choosing a utility dismisses the menu');
          await evaluate('fixture.command("recommendations")');
          await waitFor('!document.querySelector("[data-shell-region=utility]")');
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
        } else if (scenario === 'workspace-navigation') {
          await click('sessions');
          await waitFor('!document.querySelector("[data-shell-region=sessions]")');
          const openPicker = async () => {
            await evaluate('document.querySelector(".shell-navigation-menu summary").click()');
            await evaluate('document.querySelector(".titlebar-switch-button").click()');
            await waitFor('document.querySelector(".workspace-history-row-open:not(:disabled)")');
          };
          await openPicker();
          await evaluate('document.querySelector(".workspace-history-row-action").click()');
          await waitFor('fixture.pins===1');
          assert.equal(await evaluate('document.querySelector(".shell-navigation-menu").open'),true,'pinning preserves outer menu');
          await evaluate('const row=document.querySelector(".workspace-history-row-open:not(:disabled)");row.focus();row.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
          await waitFor('!document.querySelector(".workspace-history-panel")');
          assert.equal(await evaluate('document.activeElement.className'),'titlebar-switch-button','nested Escape returns focus to workspace switcher');
          assert.equal(await evaluate('document.querySelector(".shell-navigation-menu").open'),true,'nested Escape preserves outer disclosure');
          await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true}))');
          assert.equal(await evaluate('document.querySelector(".shell-navigation-menu").open'),false,'second Escape dismisses outer disclosure');
          await openPicker();
          await evaluate('document.querySelector(".workspace-history-row-open:not(:disabled)").click()');
          await waitFor('fixture.workspaceOpens===1');
          assert.equal(await evaluate('document.querySelector(".shell-navigation-menu").open'),false,'workspace selection dismisses outer menu');
          await openPicker();
          await evaluate('const other=document.querySelector(".workspace-history-other-folder");other.focus();other.click()');
          assert.equal(await evaluate('document.querySelector(".shell-navigation-menu").open'),false,'Open other workspace dismisses outer menu even if dialog is cancelled');
          assert.equal(await evaluate('document.activeElement.tagName'),'SUMMARY','cancelled workspace dialog retains a visible focus target');
          await evaluate('document.querySelector(".shell-navigation-menu summary").click();fixture.advanceWorkspace()');
          await waitFor('fixture.workspaceGeneration===2');
          assert.equal(await evaluate('document.querySelector(".shell-navigation-menu").open'),false,'programmatic workspace generation also dismisses menu');
        } else if (scenario === 'short-sidebar') {
          await win.setContentSize(1280,500);
          win.webContents.setZoomFactor(2);
          await evaluate('document.getElementById("root").style.height="calc(100vh - 38px)"');
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="compact"');
          await click('sessions');
          await waitFor('document.querySelector("[data-shell-region=sessions]")');
          assert.equal(await evaluate('getComputedStyle(document.querySelector("[data-shell-region=sessions]")).overflowY'),'auto','short sidebar offers user scrolling');
          for (const selector of ['[data-shell-command=recommendations]', '[data-shell-settings]']) {
            const visible=await evaluate(`(()=>{const b=document.querySelector('[data-shell-region=sessions] ${selector}');b.scrollIntoView({block:'center'});const r=b.getBoundingClientRect();return r.top>=0&&r.bottom<=innerHeight&&document.elementFromPoint(r.x+r.width/2,r.y+r.height/2)?.closest('button')===b;})()`);
            assert.ok(visible,'short zoomed sidebar keeps '+selector+' reachable');
          }
          assert.ok(await evaluate('document.querySelector("[data-shell-region=sessions]").scrollTop>0'),'footer reached by scrolling sidebar');
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
          await evaluate('fixture.setEmpty(true)');
          await waitFor('!document.getElementById("sessions-list")');
          await click('sessions');
          await waitFor('document.querySelector("[data-shell-region=sessions]")');
          assert.equal(await evaluate('document.activeElement.textContent'),'Sessions','empty compact Sessions focuses its heading');
          await evaluate('document.querySelector("[data-shell-return]").click()');
          await waitFor('document.querySelector("[data-shell-region=terminal]")');
          assert.equal(await evaluate('document.activeElement.dataset.shellCommand'),'sessions','empty page returns to Sessions invoker');
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
        } else if (scenario === 'region-focus') {
          // Removing the shell's focus styling must fail even though DOM focus still moves.
          const cue = selector => evaluate(`(()=>{const s=getComputedStyle(document.querySelector(${JSON.stringify(selector)}));return s.borderTopStyle!=='none'&&parseFloat(s.borderTopWidth)>=2&&s.borderTopColor!=='rgba(0, 0, 0, 0)'})()`);
          const assertFocusedRegion = async expected => {
            for (const selector of ['[data-shell-region=sessions]', '.shell-main', '[data-shell-region=inspector]']) {
              if (await evaluate(`!!document.querySelector(${JSON.stringify(selector)})`)) {
                assert.equal(await cue(selector), selector===expected, `visible region focus: ${selector}`);
              }
            }
          };
          const before = await evaluate('document.querySelector(".shell-main").getBoundingClientRect().toJSON()');
          await evaluate('document.getElementById("sessions-list").focus()');
          await assertFocusedRegion('[data-shell-region=sessions]');
          await evaluate('document.activeElement.dispatchEvent(new KeyboardEvent("keydown",{key:"Enter",bubbles:true}))');
          await waitFor('fixture.runtime()?.terminal.textarea===document.activeElement');
          await assertFocusedRegion('.shell-main');
          assert.deepEqual(await evaluate('document.querySelector(".shell-main").getBoundingClientRect().toJSON()'),before,'focus cue does not resize terminal');
          // Wait for compositing, then check painted pixels: terminal children can hide an ancestor outline.
          const assertPaintedCue = async selector => {
            await evaluate('new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))');
            const region = await evaluate(`document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect().toJSON()`);
            const painted = await win.webContents.capturePage();
            const size = painted.getSize(1);
            const bitmap = painted.toBitmap({scaleFactor:1});
            const viewport = await evaluate('({width:innerWidth,height:innerHeight})');
            const pixel = (x,y) => {
              const offset=(Math.floor(y*size.height/viewport.height)*size.width+Math.floor(x*size.width/viewport.width))*4;
              return [...bitmap.subarray(offset,offset+3)];
            };
            const top = pixel(region.x+region.width/2,region.y+1);
            assert.notDeepEqual(top,pixel(region.x+region.width/2,region.y+5),'focus cue contrasts with content');
            assert.deepEqual(pixel(region.x+1,region.y+region.height/2),top,'content cannot cover the left focus edge');
            assert.deepEqual(pixel(region.right-1,region.y+region.height/2),top,'content cannot cover the right focus edge');
            assert.deepEqual(pixel(region.x+region.width/2,region.bottom-1),top,'bottom focus edge remains visible');
          };
          await assertPaintedCue('.shell-main');

          await click('details');
          await waitFor('document.querySelector("[data-shell-region=inspector]")');
          await evaluate('document.querySelector("[data-shell-region=inspector] [data-shell-page-heading]").focus()');
          await assertFocusedRegion('[data-shell-region=inspector]');
          await evaluate('document.activeElement.blur()');
          await assertFocusedRegion(null);
          assert.equal(await evaluate('fixture.activeSessionId'),'coding','focus changes preserve session selection');
          assert.equal(await evaluate('fixture.selections'),0);
          assert.deepEqual(await evaluate('fixture.input'),[],'focus changes send no terminal input');
          await win.setContentSize(640,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="compact"');
          await waitFor('document.querySelector("[data-shell-region=utility]")');
          await evaluate('document.querySelector("[data-shell-region=utility] [data-shell-page-heading]").focus()');
          await assertFocusedRegion('.shell-main');
          await click('sessions');
          await waitFor('document.getElementById("sessions-list")===document.activeElement');
          await assertFocusedRegion('[data-shell-region=sessions]');
          await win.setContentSize(1280,360);
          await evaluate('document.getElementById("root").style.height="calc(100vh - 38px)"');
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="wide"');
          await evaluate('document.querySelector("[data-shell-settings]").focus();const sidebar=document.querySelector("[data-shell-region=sessions]");sidebar.scrollTop=sidebar.scrollHeight');
          assert.ok(await evaluate('document.querySelector("[data-shell-region=sessions]").scrollTop>0'),'short sidebar actually scrolls');
          await assertFocusedRegion('[data-shell-region=sessions]');
          await assertPaintedCue('[data-shell-region=sessions]');
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
          await evaluate('fixture.command("recommendations")');
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
            for (const destination of ['recommendations','actions']) {
              await evaluate('fixture.command('+JSON.stringify(destination)+')');
              await waitFor('document.querySelector("[data-shell-page-heading]")');
              assert.match(await evaluate('document.querySelector("[data-shell-page-heading]").parentElement.textContent'),/Workspace: \/tmp\/test/);
              await evaluate('fixture.command('+JSON.stringify(destination)+')');
              await waitFor('!document.querySelector("[data-shell-page-heading]")');
            }
          }
        } else if (scenario === 'empty-terminal-return') {
          await win.setContentSize(1000,700);
          await waitFor('document.querySelector(".shell-layout").dataset.mode==="medium"');
          await evaluate('fixture.select(null)');
          await waitFor('fixture.activeSessionId===null');
          await evaluate('const heading=document.querySelector("[data-shell-terminal-focus]");heading.focus();fixture.command("actions");void 0');
          await waitFor('document.querySelector("[data-shell-region=utility]")');
          await evaluate('fixture.command("actions")');
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
      assert.equal(await evaluate('!!document.querySelector("[data-shell-region=inspector]")'), false, 'resizing Sessions dismisses unpinned Details while the resize completes');
      await evaluate('(()=>{const button=document.querySelector("[data-shell-command=details]");button.focus();button.click()})()');
      await waitFor('document.querySelector("[data-shell-region=inspector] h2").textContent === "Details"');
      assert.equal(await evaluate('document.querySelectorAll("[data-shell-region=inspector]").length'), 1);
      await evaluate('(()=>{const button=document.querySelector("[data-shell-command=details]");button.focus();button.click()})()');
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
  for (const scenario of ['app-review-branch-documents','session-menu','session-title','app-skills-picker','app-startup-composer','app-review-terminal-links','recommendation-status-error','app-review-changes','open-session-app','sidebar-icon-rail','app-review-archived-stale','recommendations-main','skills-main','inspector-dismissal','inspector-overlay','actions-sources','actions','app-review-cancel-workspace','app-review-create-persistence','app-review-terminal-noop','app-review-native-focus','app-review-shared-request','app-review-composer-origin','app-review-pending-navigation-recovery','app-review-delayed-navigation-recovery','app-review-stale-read-restore','app-review-unreadable-restore','app-review-empty-navigation-recovery','app-review-composer-return','app-review-initial-error','app-review-native-race','app-review-details-request','app-review-navigation-recovery','app-review-opening-navigation','app-review-missing-restore','app-review-stale-restore','app-review-history','app-review-restore','app-review','resume-session','app-new-session','new-session','workspace-navigation','short-sidebar','sidebar-controls','baseline','region-focus','focus','sessions-toggle','sessions-browse','backend-loss','utility-sessions-focus','hidden-resize','empty-terminal-return','preferences-migration','preferences-recovery','reset-confirmation','utility-scope']) test('fixed shell: '+scenario, async () => {
    const require = createRequire(import.meta.url);
    const root = fileURLToPath(new URL('../', import.meta.url));
    const directory = mkdtempSync(join(tmpdir(), 'orkworks-shell-'));
    try {
      const bundle = await build({
        stdin: { contents: `
          import React, {useState} from 'react';
          import {createRoot} from 'react-dom/client';
          import ApplicationShell from './src/components/ApplicationShell';
          import App from './src/App';
          import NewSessionDialog from './src/components/NewSessionDialog';
          import {WorkspaceHistoryDropdown} from './src/components/WorkspaceHistoryDropdown';
          import ShellPreferencesNotice from './src/components/ShellPreferencesNotice';
          import {useShellPreferences} from './src/useShellPreferences';
          import './src/styles/tokens.css';
          import './src/App.css';
          import {getTerminal, getLiveTerminalCount} from './src/terminalStore';
          const fixture = window.fixture = {selections:0,prompts:0,settingsOpened:0,sessionsCreated:0,errors:[],ready:true};
          window.addEventListener('error', e => fixture.errors.push(e.error?.stack || e.message));
          window.addEventListener('unhandledrejection', e => fixture.errors.push(String(e.reason)));
          fixture.socketCount=0;fixture.input=[];fixture.pins=0;fixture.workspaceOpens=0;
          window.WebSocket=class {
            static OPEN=1; readyState=1; bufferedAmount=0;
            constructor(url){if(String(url).includes("/workspace/updates")){queueMicrotask(()=>this.onerror?.());return;}fixture.socketCount++;fixture.socket=this;queueMicrotask(()=>this.onopen?.());}
            send(payload){const message=JSON.parse(payload);if(message.type==='input')fixture.input.push(message.data);}
            close(){}
          };
          fixture.output=text=>fixture.socket.onmessage({data:new TextEncoder().encode(text).buffer});
          fixture.runtime=()=>getTerminal(fixture.activeSessionId||'coding');
          fixture.runtimeCount=()=>getLiveTerminalCount();
          window.orkworks = {getSkillInventory:async()=>({skills:[],diagnostics:[],roots:[],apm:{available:true,compatible:true,path:"/usr/local/bin/apm",version:"0.33.0",executableOverride:null,message:"APM 0.33.0 supports reviewed installation."}}),pickSkillSource:async()=>'/tmp/skill',previewSkillAdd:async request=>{fixture.skillRequest=request;return null},notifyPanelVisibility:()=>{},getBackendUrl:()=>Promise.resolve('http://127.0.0.1:12345'),getWorkspaceHistory:async()=>({pinned:[],recent:['/tmp/test','/tmp/other'],diagnostic:null})};
          fixture.nativeOpened=[];fixture.nativeMissing=${JSON.stringify(scenario)}==='open-session-app';
          window.orkworks.inspectSessionApp=async()=>{
            const value=fixture.nativeMissing?{label:'ChatGPT',available:false,reason:'Native ID not captured'}:fixture.nativeUnavailable?{label:'Claude',available:false,reason:'Close this session first'}:{label:fixture.nativeHarness==='claude-code'?'Claude':'ChatGPT',available:true};
            if(fixture.deferNative)await new Promise(resolve=>fixture.finishNative=resolve);
            return value;
          };
          window.orkworks.openSessionApp=async(...args)=>{if(fixture.nativeFailure)throw new Error('App refused');fixture.nativeOpened.push(args)};
          fixture.titleRequests=[];
          window.orkworks.sessionTitle=async request=>{fixture.titleRequests.push(request);if(fixture.deferTitle)await new Promise(resolve=>fixture.finishTitle=resolve);return {title:request.action==='automatic'?'Automatic login topic':request.title,automatic:request.action!=='set'}};
          window.orkworks.pinWorkspacePath=async()=>{fixture.pins++;return {pinned:[],recent:['/tmp/test','/tmp/other'],diagnostic:null}};
          window.orkworks.readGitChanges=async request=>{
            if(request.kind==='bases'){if(fixture.deferBases)await new Promise(resolve=>fixture.finishBases=resolve);return {refs:['refs/heads/main','refs/heads/release'],defaultRef:'refs/heads/main',limited:false,unavailableReason:null};}
            if(request.kind==='file'){const result={snapshotId:request.snapshotId,fileId:request.fileId,patch:fixture.changePatch||(request.snapshotId.includes('branch')?'+committed edit':'+working edit'),notice:null};if(fixture.deferPatch)await new Promise(resolve=>fixture.finishPatch=resolve);return result;}
            fixture.captureCount=(fixture.captureCount||0)+1;
            if(fixture.changeFailure)throw new Error('fixture refresh failed');
            const result={snapshotId:request.comparison+'-'+request.sessionId+'-'+fixture.captureCount,sessionId:request.sessionId,worktree:fixture.worktree||'/tmp/test',gitContextSource:fixture.worktree?'agent':'launch',branch:'feature',comparison:request.comparison,baseRef:request.baseRef||null,baseOid:'bbbbbbbb',baselineOid:'aaaaaaaa',headOid:'cccccccc',capturedAt:'2026-10-24T10:00:00Z',partial:!!fixture.partialChanges,notice:null,files:[{id:'file',path:'src/change.ts',oldPath:null,status:'Modified',additions:1,deletions:0,notice:null}]};
            if(${JSON.stringify(scenario)}==='app-review-changes')result.files.push({id:'other-file',path:'src/other.ts',oldPath:null,status:'Modified',additions:1,deletions:0,notice:null});
            if(fixture.omitOtherFile)result.files=result.files.filter(f=>f.id!=='other-file');
            if(fixture.emptyChanges)result.files=[];
            if(fixture.deferChanges)await new Promise(resolve=>fixture.finishChanges=resolve);
            return result;
          };
          fixture.noticeCommands=[];fixture.preferenceWrites=0;fixture.preferenceReads=0;fixture.rebuildCalls=0;fixture.resetCalls=0;fixture.cancelRebuild=true;
          fixture.snapshot={preferences:{sessionsWidth:240,inspectorWidth:320,sessionsVisible:true,density:'low'},revision:0,diagnostic:${JSON.stringify(scenario)}==='preferences-recovery'?'corrupt_record':null};
          Object.assign(window.orkworks,{
            getShellLayout:async()=>{fixture.preferenceReads++;return {...fixture.snapshot}},
            getLayout:async()=>'{}',
            saveShellLayout:async preferences=>{fixture.preferenceWrites++;fixture.snapshot={preferences,revision:fixture.snapshot.revision+1,diagnostic:null};return {ok:true}},
            resetShellLayout:async()=>{fixture.resetCalls++;return {ok:false}},
            rebuildShellLayout:async()=>{fixture.rebuildCalls++;if(fixture.cancelRebuild)return {ok:false,diagnostic:'user_cancelled'};fixture.snapshot.diagnostic=null;return {ok:true}}
          });
          if ((${JSON.stringify(scenario)}.startsWith('app-'))) {
            fixture.posts=0;fixture.navigation=[];fixture.navigationRebuilds=0;fixture.cancelNavigationRebuild=true;fixture.navigationDiagnostic=${JSON.stringify(scenario)}.includes('navigation-recovery')?'corrupt_record':null;fixture.activeWrites=0;fixture.reads=[];fixture.navigationReads=0;
            fixture.plan='# Current plan';fixture.planSelections=[];fixture.documentKinds=[];fixture.documentSelectionIds=[];
            if(${JSON.stringify(scenario)}==='app-review-terminal-links')fixture.plan='# Clicked document';fixture.savedSurface=${JSON.stringify(scenario)}.includes('restore')||${JSON.stringify(scenario)}==='app-review-opening-navigation'?'review':'terminal';
            const reviewSession=id=>({id,label:id,harness:'generic-shell',harnessId:'generic-shell',lifecycle:'alive',status:'running',cwd:'/tmp/test',created_at:'2026-10-13T10:00:00Z',memoryState:'live',resumeStrategy:'none',hasOpenablePlan:true,repoRoot:'/tmp/test',branch:'feature'});
            fixture.sessionList=${JSON.stringify(scenario)}.startsWith('app-review')||${JSON.stringify(scenario)}==='app-startup-composer'?[reviewSession('created'),reviewSession('other')]:[];
            if(${scenario === 'app-review-branch-documents'})for(const session of fixture.sessionList)session.planDocuments={branch:'feature',sharedSessions:2,steps:[{kind:'design',title:'Design document',available:true,pinned:false,source:'git_history',selectionId:'d'.repeat(64)},{kind:'implementation',title:'Implementation document',available:true,pinned:false,source:'agent_reported',selectionId:'a'.repeat(64)}]};
            if(${JSON.stringify(scenario)}==='app-review-terminal-links'){fixture.sessionList[0].hasOpenablePlan=false;fixture.sessionList[1].lifecycle='dead';fixture.sessionList[1].status='ended';}
            if(${JSON.stringify(scenario)}==='app-review-empty-navigation-recovery')fixture.sessionList=[];
            if(${JSON.stringify(scenario)}==='app-review-stale-read-restore')fixture.deferRead=true;
            if(${JSON.stringify(scenario)}==='app-review-unreadable-restore')fixture.unreadable=true;
            if(${JSON.stringify(scenario)}==='app-review-missing-restore')fixture.sessionList[0].hasOpenablePlan=false;
            if(${JSON.stringify(scenario)}==='app-review-archived-stale')fixture.sessionList[1].sessionState='archived';
            if(${JSON.stringify(scenario)}==='app-review-history'){fixture.sessionList[1].lifecycle='dead';fixture.sessionList[1].status='ended';}
            fixture.runtime=()=>getTerminal('created');
            Object.assign(window.orkworks,{
              onBackendLifecycle:callback=>{fixture.lifecycle=callback;queueMicrotask(()=>callback({state:'ready',workspace:${JSON.stringify(scenario)}==='app-skills-picker'?null:{path:'/tmp/test',workspaceIdentity:'/tmp/test',lastActiveSessionId:${JSON.stringify(scenario)}.startsWith('app-review')?'created':null,activeHarnessIds:['generic-shell']},historyDiagnostic:null}));return ()=>{}},
              onUpdateStatus:()=>()=>{},onMenuCommand:callback=>{fixture.menu=callback;return ()=>{}},
              getInitialWorkspace:async()=>({workspace:null,historyDiagnostic:null}),
              openWorkspace:async()=>new Promise(resolve=>{fixture.cancelWorkspacePicker=resolve}),
              openRememberedWorkspace:async()=>new Promise((resolve,reject)=>{fixture.rejectRememberedWorkspace=reject}),
              getSettings:async()=>({debug:{showSessionIds:false,rendererHealthLogMs:0}}),
              getProviderModels:async()=>({models:[]}),getWorkspaceNavigation:async()=>{if(${JSON.stringify(scenario)}==='app-review-pending-navigation-recovery'&&fixture.navigationReads++){return new Promise((resolve,reject)=>{fixture.resolveNewNavigation=resolve;fixture.rejectNewNavigation=reject})}if(${JSON.stringify(scenario)}==='app-review-delayed-navigation-recovery'&&!fixture.navigationReads++){return new Promise((resolve,reject)=>{fixture.resolveOldNavigation=resolve;fixture.rejectOldNavigation=reject})}if(${JSON.stringify(scenario)}==='app-review-stale-restore')await new Promise(resolve=>fixture.resolveNavigation=resolve);return {lastCentralSurface:fixture.savedSurface,revision:0,diagnostic:fixture.navigationDiagnostic}},
              completeWorkspaceNavigation:async surface=>{if(["corrupt_record","unsupported_version"].includes(fixture.navigationDiagnostic))return {ok:false,diagnostic:fixture.navigationDiagnostic};fixture.navigation.push(surface);if(${JSON.stringify(scenario)}==='app-review-create-persistence')fixture.savedSurface=surface;return {ok:true}},
              rebuildWorkspaceNavigation:async()=>{fixture.navigationRebuilds++;if(fixture.deferNavigationRebuild)await new Promise(resolve=>fixture.finishNavigationRebuild=resolve);if(fixture.cancelNavigationRebuild)return {ok:false,diagnostic:'user_cancelled'};fixture.navigationDiagnostic=null;return {ok:true}},
              selectTerminalPlan:async (id,path)=>{fixture.planSelections.push([id,path]);if(fixture.selectionFailure)throw new Error('Document is unavailable');if(fixture.deferSelection)await new Promise(resolve=>fixture.finishSelection=resolve);return fixture.selectedDocument},
              getPlanContent:async (id,kind,selectionId)=>{fixture.reads.push(id);if(kind){fixture.documentKinds.push(kind);fixture.documentSelectionIds.push(selectionId)}if(fixture.deferRead)return new Promise((resolve,reject)=>{fixture.resolveRead=resolve;fixture.rejectRead=reject});if(fixture.unreadable)throw new Error('Missing plan');if(${scenario === 'app-review-branch-documents'})return kind==='design'?'# Design document':fixture.currentImplementation||'# Implementation document';return id==='other'?'# Other session plan':fixture.plan},
              requestPlanReview:async (id,kind,selectionId)=>{fixture.prompts++;fixture.promptSession=id;fixture.promptKind=kind;fixture.promptSelectionId=selectionId;await new Promise((resolve,reject)=>{fixture.finishPrompt=resolve;fixture.rejectPrompt=reject})},
            });
            window.fetch=async (url,options={})=>{
              const path=new URL(url).pathname;
              const response=value=>({ok:true,status:200,json:async()=>value});
              if(path==='/sessions' && options.method==='POST'){
                fixture.posts++;
                if(fixture.failCreation)return {ok:false,status:500};
                const created={id:fixture.createdId||'created',label:fixture.createdId||'created',hasOpenablePlan:${JSON.stringify(scenario)}==='app-review-create-persistence',harness:'generic-shell',harnessId:'generic-shell',lifecycle:'alive',status:'running',cwd:'/tmp',created_at:'2026-10-13T10:00:00Z',memoryState:'live',resumeStrategy:'none'};
                fixture.sessionList=[created];return response(created);
              }
              if(path.endsWith('/summary-log'))return response({entries:[]});
              if(path==='/sessions/other/terminal-output')return response({lines:[${JSON.stringify(scenario)}==='app-review-terminal-links'?'specs/history.md':'saved output'],cols:80,rows:24});
              if(path==='/sessions'){if(['app-startup-composer','app-review-pending-navigation-recovery','app-review-delayed-navigation-recovery'].includes(${JSON.stringify(scenario)})&&!fixture.sessionsReleased)await new Promise(resolve=>fixture.releaseSessions=()=>{fixture.sessionsReleased=true;resolve()});return response(fixture.sessionList);}
              if(path==='/harnesses')return response({harnesses:[],documentRevision:null});
              if(path==='/providers')return response({providers:[],appliedRevision:null});
              if(path==='/workspace/active-session'){fixture.activeWrites++;return response({})}
              throw new Error('Unexpected fixture request '+path);
            };
          }
          if (${JSON.stringify(scenario)}==='recommendation-status-error') {
            fixture.statusReads=0;
            window.orkworks.getTaskmasterRunStatus=async()=>{fixture.statusReads++;return {workspacePath:'/tmp/test',state:'idle'}};
            window.fetch=async()=>({ok:false,status:500,json:async()=>({})});
          }
          if (${JSON.stringify(scenario)}==='actions-sources') {
            fixture.openedRecommendations=[];fixture.pendingRecommendations=[];
            fixture.recommendation=id=>({id,title:id,summary:'Summary '+id,status:'proposed',updatedAt:'2026-10-14',rollupMemberIds:[]});
            window.fetch=async url=>{
              if(new URL(url).pathname!=='/taskmaster/recommendations')throw new Error('Unexpected Actions request');
              const body=fixture.pauseRecommendations ? await new Promise(resolve=>fixture.pendingRecommendations.push(resolve)) : {recommendations:[fixture.recommendation('r2'),fixture.recommendation('r1')],diagnostics:fixture.partial?[{code:'partial',message:'missing source'}]:[]};
              return {ok:true,json:async()=>body};
            };
          }
          if (${JSON.stringify(scenario)}==='session-menu') {
            window.fetch=async url=>({ok:true,json:async()=>({entries:[{timestamp:'2026-10-31T10:00:00Z',summary:'Checkpoint '+new URL(url).pathname.split('/')[2],source:'agent_hook'}]})});
          }
          function Preferences() {const prefs=useShellPreferences();return <ShellPreferencesNotice {...prefs} onCommand={command=>{fixture.noticeCommands.push(command);void prefs.reset()}}/>;}
          function PreferencesHarness() {const [generation,setGeneration]=useState(0);fixture.remountPreferences=()=>setGeneration(n=>n+1);return <Preferences key={generation}/>;}
          function Harness() {
            const [sessionState,setSessionState]=useState({});fixture.setSessionState=next=>{fixture.nativeHarness=next.harnessId;setSessionState(next)};fixture.resumed??=[];const [resumingSessionIds,setResumingSessionIds]=useState(new Set());fixture.setResuming=ids=>setResumingSessionIds(new Set(ids));
            const [sessionsSourceStatus,setSessionSourceStatus]=useState('ready');fixture.setSessionSourceStatus=setSessionSourceStatus;
            const [inspector,setInspector]=useState(null);
            const [reviewing,setReviewing]=useState(false);fixture.openReview=()=>setReviewing(true);
            const [creating,setCreating]=useState(false);fixture.setCreating=setCreating;
            const [empty,setEmpty]=useState(false);fixture.setEmpty=setEmpty;
            const [generation,setGeneration]=useState(0);fixture.workspaceGeneration=generation;fixture.advanceWorkspace=()=>{if(${JSON.stringify(scenario)}.endsWith('-main'))setInspector(null);setGeneration(n=>n+1)};
            const [backendStatus,setBackendStatus]=useState('connected');
            const [commandRequest,setCommandRequest]=useState(null);
            fixture.command=(command,reveal=false)=>setCommandRequest(old=>({command,reveal,sequence:(old?.sequence||0)+1}));
            fixture.setBackendStatus=setBackendStatus;
            const [activeSessionId,setActiveSessionId]=useState('coding');
            fixture.activeSessionId=activeSessionId;
            fixture.select=setActiveSessionId;
            const [preferences,setPreferences]=useState({sessionsWidth:240,inspectorWidth:320,sessionsVisible:true,density:'low'});fixture.setPreferences=setPreferences;
            return <ApplicationShell centralSurface={reviewing ? {kind:"review",sessionId:activeSessionId,artifactId:"plan"} : {kind:"terminal",sessionId:activeSessionId}} onOpenReview={()=>setReviewing(true)} onCloseReview={()=>setReviewing(false)} reviewContent={<p>Current plan</p>} sessions={empty ? [] : ((${JSON.stringify(scenario)}==='sidebar-controls'?['coding','other',...Array.from({length:80},(_,i)=>'history-'+i)]:['coding','other']).map((id,i)=>({id,name:id,cwd:'/tmp/test',harnessId:'codex',harness:'codex',lifecycle:'alive',status:'running',attention:(${JSON.stringify(scenario)}==='actions' && id==='other'?'needs_you':'idle'),...sessionState,label:id,createdAt:'2026-10-10T10:00:00Z',lastActivityAt:i?'2026-10-10T09:00:00Z':'2026-10-10T10:00:00Z'})))}
              workspace={{path:'/tmp/test',workspaceIdentity:['open-session-app','skills-main'].includes(${JSON.stringify(scenario)})?'/real/test':'/tmp/test'}} workspaceControl={<div className="shell-workspace-control"><span className="titlebar-text">test</span><WorkspaceHistoryDropdown triggerLabel="⇄" triggerClassName="titlebar-switch-button" triggerAriaLabel="Switch workspace" currentWorkspacePath="/tmp/test" isSwitching={false} onOpenPath={()=>{fixture.workspaceOpens++;setGeneration(n=>n+1)}} onOpenOtherFolder={()=>{}} onError={()=>{}}/></div>} onOpenSettings={()=>fixture.settingsOpened++} onCreateSession={()=>{fixture.sessionsCreated++;if(${JSON.stringify(scenario)}==='new-session'){setCreating(true);fixture.command('new-session')}}} activeSessionId={activeSessionId} workspaceGeneration={generation}
              newSessionContent={creating ? <NewSessionDialog key={generation} harnesses={[]} providerRuntime={null} onConfirm={async opts=>{fixture.submitted=opts;fixture.finishCreate=new Promise(resolve=>fixture.resolveCreate=resolve);await fixture.finishCreate;setCreating(false);fixture.command("terminal")}} onCancel={()=>fixture.command("cancel-new-session")}/> : null}
              onCancelNewSession={()=>setCreating(false)} backendStatus={backendStatus} sessionsSourceStatus={sessionsSourceStatus} taskmasterReady={['actions-sources','recommendation-status-error'].includes(${JSON.stringify(scenario)})} onOpenRecommendation={id=>fixture.openedRecommendations.push(id)} onFixWithAi={()=>fixture.prompts++} onRunCleanup={()=>fixture.prompts++} harnesses={[]} debugSettings={{showSessionIds:false}}
              commandRequest={commandRequest} preferences={preferences} onPreferencesChange={setPreferences} onResetPreferences={()=>new Promise(resolve=>{fixture.finishReset=resolve})}
              inspector={inspector} onInspect={setInspector} unreadIds={new Set(${JSON.stringify(scenario)}==='actions'?['other']:[])} acknowledgedIds={new Set()}
              visibleSubjectId={['resume-session','session-title','session-menu'].includes(${JSON.stringify(scenario)})?activeSessionId:undefined} resumingSessionIds={resumingSessionIds} onResumeSession={(id,strategy)=>fixture.resumed.push({id,strategy})} onSelectSession={id=>{fixture.selections++;if(${JSON.stringify(scenario)}.endsWith('-main'))setInspector(null);setActiveSessionId(id)}} onFocusTerminal={()=>fixture.runtime()?.terminal.focus()} onBackendUnavailable={()=>{}} />;
          }
          createRoot(document.getElementById('root')).render(${JSON.stringify(scenario)}.startsWith('preferences-')?<PreferencesHarness/>:${JSON.stringify(scenario)}.startsWith('app-')?<App/>:<Harness/>);
        `, resolveDir: root, loader: 'tsx' },
        bundle: true, write: false, outdir: directory, format: 'iife', platform: 'browser',
        define: { 'process.env.NODE_ENV': '"development"' },
        plugins: [{ name: 'lifecycle-fixture', setup(builder) {
          builder.onResolve({filter:/icon-dark\.svg\?no-inline$/},()=>({path:'icon',namespace:'fixture-icon'}));
          builder.onLoad({filter:/.*/,namespace:'fixture-icon'},()=>({contents:'export default \"data:image/svg+xml,<svg/>\"',loader:'js'}));
          builder.onResolve({filter:/^@xterm\/addon-webgl$/},()=>({path:'webgl',namespace:'no-gpu'}));
          builder.onLoad({filter:/.*/,namespace:'no-gpu'},()=>({contents:'export class WebglAddon { constructor(){throw new Error("GPU unavailable in headless fixture")} }',loader:'js'}));
          // Panel bodies connect to the backend/PTY; retain real React shell and terminal ownership.
          builder.onResolve({ filter: /^\.\/(SessionDetailPanel|RecommendationsPanel)$/ },
            args => ((scenario.startsWith('app-review') || ['resume-session','session-title','session-menu'].includes(scenario)) && args.path === './SessionDetailPanel') || (['inspector-overlay','recommendation-status-error'].includes(scenario) && args.path === './RecommendationsPanel') ? undefined : ({ path: args.path, namespace: 'empty-panel' }));
          builder.onLoad({ filter: /.*/, namespace: 'empty-panel' },
            () => ({ contents: 'export default function Panel(){return null}', loader: 'tsx', resolveDir: root }));

        } }],
      });
      const fixture = join(directory, 'index.html');
      writeFileSync(fixture, '<style>' + (bundle.outputFiles.find(f=>f.path.endsWith('.css'))?.text||'') + '</style><div id="root" style="height:650px;display:'+(scenario.startsWith('app-')?'block':'flex')+'"></div><script>' + bundle.outputFiles.find(f=>!f.path.endsWith('.css')).text + '</script>');
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
