import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
if (process.env.ORKWORKS_SKILLS_FIXTURE) {
  const { app, BrowserWindow } = await import("electron");
  app.whenReady().then(async () => {
    const win = new BrowserWindow({
      width: 1000,
      height: 850,
      show: false,
      webPreferences: { contextIsolation: true, nodeIntegration: false },
    });
    try {
      await win.loadFile(process.env.ORKWORKS_SKILLS_FIXTURE);
      win.webContents.on("console-message", (_event, level, text) => {
        if (level >= 2) console.error(text);
      });
      const evaluate = async (code) => {
        try {
          return await win.webContents.executeJavaScript(code, true);
        } catch (error) {
          throw new Error(code + ": " + error.message);
        }
      };
      const wait = async (code) => {
        for (let n = 0; n < 100; n++) {
          if (await evaluate(code)) return;
          await new Promise((r) => setTimeout(r, 20));
        }
        throw Error("Timed out: " + code);
      };
      const click = (selector) =>
        evaluate(`document.querySelector(${JSON.stringify(selector)}).click()`);
      await wait('document.querySelectorAll("[data-skill-row]").length===4');
      assert.deepEqual(await evaluate(`Array.from(document.querySelectorAll('[data-skill-scope="personal"] [data-skill-plugin]')).map(group => [group.dataset.skillPlugin, group.querySelectorAll('[data-skill-row]').length])`), [["alpha@local", 1], ["beta@local", 1], ["", 1]]);
      await evaluate(`(()=>{const el=document.querySelector('input[type="search"]');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value").set.call(el,"alpha@local");el.dispatchEvent(new Event("input",{bubbles:true}))})()`);
      await wait('document.querySelectorAll("[data-skill-row]").length===1');
      assert.equal(await evaluate('document.querySelector("[data-skill-row] strong").innerText'), "plan");
      await evaluate(`(()=>{const el=document.querySelector('input[type="search"]');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value").set.call(el,"");el.dispatchEvent(new Event("input",{bubbles:true}))})()`);
      await wait('document.querySelectorAll("[data-skill-row]").length===4');
      await evaluate(`(()=>{const el=document.querySelector('.skills-filters select');el.value="codex";el.dispatchEvent(new Event("change",{bubbles:true}))})()`);
      await wait('document.querySelectorAll("[data-skill-row]").length===2');
      assert.equal(await evaluate(`Array.from(document.querySelectorAll('[data-skill-plugin]')).some(group => group.dataset.skillPlugin)`), false);
      await evaluate(`(()=>{const el=document.querySelector('.skills-filters select');el.value="";el.dispatchEvent(new Event("change",{bubbles:true}))})()`);
      await wait('document.querySelectorAll("[data-skill-row]").length===4');
      assert.equal(
        await evaluate(
          'document.querySelectorAll("[data-skill-scope]").length',
        ),
        2,
      );
      assert.equal(
        await evaluate('!!document.querySelector("img[data-injected]")'),
        false,
      );
      assert.equal(await evaluate('document.querySelectorAll("[data-skill-row] details:not([open])").length'), 4, 'skills start collapsed');
      assert.equal(await evaluate('document.querySelector("[data-skill-row]").innerText.includes("/repo/.agents")'), false, 'paths do not clutter collapsed rows');
      assert.equal(await evaluate('document.querySelector("[data-skill-row]").innerText.includes("Review code")'), false, 'descriptions stay inside expanded rows');
      if (process.env.ORKWORKS_SKILLS_CAPTURE) {
        const { writeFileSync } = await import("node:fs");
        writeFileSync(process.env.ORKWORKS_SKILLS_CAPTURE + ".collapsed.png", (await win.webContents.capturePage()).toPNG());
      }
      await evaluate('document.querySelector("[data-skill-row] summary").focus()');
      assert.equal(await evaluate('document.activeElement.matches("[data-skill-row] summary")'), true, 'row disclosure is focusable');
      await click("[data-skill-row] summary");
      await wait('document.querySelector("[data-skill-row] details[open]")');
      assert.match(await evaluate('document.querySelector("[data-skill-row]").innerText'), /Review code/);
      await click("[data-skill-detail]");
      await wait('document.querySelector("[data-skill-content]")');
      assert.match(
        await evaluate(
          'document.querySelector("[data-skill-content]").innerText',
        ),
        /Read-only instructions/,
      );
      assert.equal(await evaluate('!!document.querySelector("[data-skill-row] [data-skill-content]")'), true, 'skill text opens within its row');
      if (process.env.ORKWORKS_SKILLS_CAPTURE) {
        const { writeFileSync } = await import("node:fs");
        writeFileSync(process.env.ORKWORKS_SKILLS_CAPTURE + ".expanded.png", (await win.webContents.capturePage()).toPNG());
      }
      await click("[data-skill-row] summary");
      assert.equal(await evaluate('document.querySelector("[data-skill-row]").innerText.includes("Read-only instructions")'), false, 'collapsing hides inline content');
      await evaluate(
        '(()=>{const el=document.querySelector("input[aria-label]");Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value").set.call(el,"missing");el.dispatchEvent(new Event("input",{bubbles:true}))})()',
      );
      await wait('document.querySelectorAll("[data-skill-row]").length===0');
      await evaluate(
        '(()=>{const el=document.querySelector("input[aria-label]");Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value").set.call(el,"");el.dispatchEvent(new Event("input",{bubbles:true}))})()',
      );
      await click("[data-add-skill]");
      await click("[data-pick-skill]");
      await click("[data-review-skill]");
      await wait('document.querySelector("[data-install-skill]")');
      await click("[data-install-skill]");
      await wait(
        'document.querySelector("[role=status]").innerText.includes("Skill installed")',
      );
      await click("[data-review-skill]");
      await wait('document.querySelector("[data-install-skill]")');
      await click("[data-install-skill]");
      await wait('typeof window.finishSkillInstall === "function"');
      await click("[data-switch]");
      await wait('document.body.innerText.includes("Open a workspace")');
      await evaluate('window.finishSkillInstall();new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');
      await wait('document.querySelector("[role=status]").innerText !== "Working…"');
      assert.equal(await evaluate('document.querySelector("[role=status]").innerText.includes("Old workspace")'), false, 'old completion cannot label the new workspace');
      assert.equal(await evaluate('document.querySelectorAll("[data-skill-row]").length'), 3);
      await click("[data-skill-row] summary");
      for (const width of [1000, 700, 360]) {
        win.setContentSize(width, 850);
        await evaluate(
          "new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))",
        );
        assert.ok(
          await evaluate("document.documentElement.scrollWidth<=innerWidth+1"),
          `no overflow at ${width}`,
        );
      }
      if (process.env.ORKWORKS_SKILLS_CAPTURE) {
        const { writeFileSync } = await import("node:fs");
        writeFileSync(
          process.env.ORKWORKS_SKILLS_CAPTURE,
          (await win.webContents.capturePage()).toPNG(),
        );
      }
      app.exit(0);
    } catch (error) {
      console.error(error);
      app.exit(1);
    }
  });
} else {
  const { test } = await import("node:test");
  const { mkdtempSync, writeFileSync, rmSync } = await import("node:fs");
  const { join, dirname, resolve } = await import("node:path");
  const { tmpdir } = await import("node:os");
  const { spawnSync } = await import("node:child_process");
  const { createRequire } = await import("node:module");
  test("skills settings distinguish scope, search safely, show details and add with review", async () => {
    const temp = mkdtempSync(join(tmpdir(), "orkworks-skills-ui-"));
    const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
    const require = createRequire(import.meta.url);
    try {
      const { build } = await import("esbuild");
      const result = await build({
        entryPoints: [join(root, "tests/fixtures/skillsSettings.tsx")],
        bundle: true,
        write: false,
        outdir: temp,
        format: "iife",
        platform: "browser",
        jsx: "automatic",
      });
      const js = result.outputFiles.find((f) => f.path.endsWith(".js")).text;
      const css =
        result.outputFiles.find((f) => f.path.endsWith(".css"))?.text || "";
      const html = join(temp, "index.html");
      writeFileSync(
        html,
        `<meta name="viewport" content="width=device-width,initial-scale=1"><style>${css}</style><div id="root"></div><script>${js}</script>`,
      );
      const env = { ...process.env, ORKWORKS_SKILLS_FIXTURE: html };
      delete env.ELECTRON_RUN_AS_NODE;
      const electron = require("electron"),
        args = [fileURLToPath(import.meta.url)];
      if (process.platform === "linux") args.unshift("--no-sandbox");
      const headless = process.platform === "linux" && !env.DISPLAY;
      if (headless) args.unshift("-a", electron);
      const resultProcess = spawnSync(headless ? "xvfb-run" : electron, args, {
        env,
        encoding: "utf8",
        timeout: 30000,
        windowsHide: true,
      });
      assert.ifError(resultProcess.error);
      assert.equal(
        resultProcess.status,
        0,
        resultProcess.stdout + resultProcess.stderr,
      );
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });
}
