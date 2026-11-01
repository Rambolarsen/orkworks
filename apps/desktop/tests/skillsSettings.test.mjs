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
      const search = (value) => evaluate(`(()=>{const el=document.querySelector('input[type="search"]');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value").set.call(el,${JSON.stringify(value)});el.dispatchEvent(new Event("input",{bubbles:true}))})()`);
      const filter = (label, value) => evaluate(`(()=>{const el=document.querySelector('select[aria-label="'+${JSON.stringify(label)}+'"]');el.value=${JSON.stringify(value)};el.dispatchEvent(new Event("change",{bubbles:true}))})()`);
      await wait('document.querySelectorAll("[data-skill-source]").length===4');
      assert.equal(await evaluate(`document.querySelector('input[aria-label="Changed only"]')?.checked`), false);
      await click('input[aria-label="Changed only"]');
      await wait('document.querySelectorAll("[data-skill-source]").length===0');
      assert.match(await evaluate('document.querySelector("[data-skill-counts]").innerText'), /0 skills.*0 sources/s);
      assert.match(await evaluate('document.body.innerText'), /No changed skills match the current filters/);
      await click('input[aria-label="Changed only"]');
      await wait('document.querySelectorAll("[data-skill-source]").length===4');
      assert.match(await evaluate('document.querySelector("[data-skill-counts]").innerText'), /2 skills.*4 sources/s);
      assert.equal(await evaluate('document.querySelectorAll("[data-skill-source]").length'), 4);
      assert.equal(await evaluate('document.querySelectorAll("[data-skill-plugin]").length'), 2);
      assert.equal(await evaluate(`document.querySelector('[data-skill-plugin="alpha@local"] [data-skill-row]').dataset.skillRow`), "review");
      assert.equal(await evaluate(`document.querySelector('[data-skill-plugin="beta@local"] [data-skill-row]').dataset.skillRow`), "build");
      assert.equal(await evaluate('document.querySelectorAll("[data-local-skills] [data-skill-source]").length'), 2);
      assert.equal(await evaluate('document.querySelectorAll("[data-skill-overlap]").length'), 2);
      assert.equal(await evaluate(`document.querySelector('[data-skill-row="build"] [data-skill-overlap]')`), null);
      assert.equal(await evaluate('document.querySelectorAll("[data-skill-row] details:not([open])").length'), 3);
      assert.equal(await evaluate(`document.querySelector('[data-skill-row="review"]').innerText.includes("/repo/.agents")`), false);
      assert.equal(await evaluate('!!document.querySelector("img[data-injected]")'), false);
      if (process.env.ORKWORKS_SKILLS_CAPTURE) {
        const { writeFileSync } = await import("node:fs");
        writeFileSync(process.env.ORKWORKS_SKILLS_CAPTURE + ".collapsed.png", (await win.webContents.capturePage()).toPNG());
      }
      await evaluate(`document.querySelector('[data-skill-row="review"] summary').focus()`);
      assert.equal(await evaluate('document.activeElement.matches("[data-skill-row] summary")'), true);
      await click('[data-local-skills] [data-skill-row="review"] summary');
      await click('[data-skill-plugin="alpha@local"] [data-skill-row="review"] summary');
      await wait(`document.querySelector('[data-skill-row="review"] details[open]')`);
      assert.match(await evaluate(`document.querySelector('[data-skill-source="workspace-review"]').innerText`), /Workspace/);
      assert.match(await evaluate(`document.querySelector('[data-skill-source="personal-review"]').innerText`), /Personal/);
      assert.match(await evaluate(`document.querySelector('[data-skill-source="plugin-one"]').innerText`), /alpha@local/);
      assert.match(await evaluate(`document.querySelector('[data-skill-source="plugin-one"]').innerText`), /1.0/);
      assert.match(await evaluate(`document.querySelector('[data-skill-source="plugin-one"]').innerText`), /Plugin-specific/);
      await click('[data-skill-source="workspace-review"] [data-skill-detail]');
      await wait('document.querySelector("[data-skill-content]")');
      assert.match(await evaluate('document.querySelector("[data-skill-content]").innerText'), /workspace-review/);
      await click('[data-skill-source="plugin-one"] [data-skill-detail]');
      await wait('document.querySelector("[data-skill-content]").innerText.includes("plugin-one")');
      assert.equal(await evaluate(`!!document.querySelector('[data-skill-source="workspace-review"] [data-skill-content]')`), false);
      assert.equal(await evaluate(`!!document.querySelector('[data-skill-source="plugin-one"] [data-skill-content]')`), true);
      if (process.env.ORKWORKS_SKILLS_CAPTURE) {
        const { writeFileSync } = await import("node:fs");
        writeFileSync(process.env.ORKWORKS_SKILLS_CAPTURE + ".expanded.png", (await win.webContents.capturePage()).toPNG());
      }
      for (const width of [1000, 700, 360]) {
        win.setContentSize(width, 850);
        await evaluate("new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))");
        assert.ok(await evaluate("document.documentElement.scrollWidth<=innerWidth+1"), `grouped sources have no overflow at ${width}`);
      }
      win.setContentSize(1000, 850);
      await click('[data-skill-plugin="alpha@local"] [data-skill-row="review"] summary');
      assert.equal(await evaluate(`document.querySelector('[data-skill-row="review"]').innerText.includes("Read-only instructions")`), false);
      await search("alpha@local");
      await wait('document.querySelectorAll("[data-skill-source]").length===1');
      assert.equal(await evaluate('document.querySelectorAll("[data-skill-overlap]").length'), 1, 'overlap remains discoverable when the local source is filtered out');
      assert.equal(await evaluate('document.querySelector("[data-skill-row] strong").innerText'), "review");
      assert.match(await evaluate('document.querySelector("[data-skill-counts]").innerText'), /1 skill.*1 source/s);
      await search("");
      await filter("Coding tool", "codex");
      await wait('document.querySelectorAll("[data-skill-source]").length===2');
      assert.equal(await evaluate('document.querySelectorAll("[data-skill-row]").length'), 1);
      await filter("Scope", "workspace");
      await wait('document.querySelectorAll("[data-skill-source]").length===1');
      assert.equal(await evaluate('document.querySelector("[data-skill-source]").dataset.skillSource'), "workspace-review");
      await filter("Scope", "personal");
      await wait('document.querySelector("[data-skill-source]").dataset.skillSource === "personal-review"');
      await filter("Coding tool", "");
      await wait('document.querySelectorAll("[data-skill-source]").length===3');
      assert.equal(await evaluate('document.querySelectorAll("[data-skill-row]").length'), 3);
      await filter("Scope", "");
      await wait('document.querySelectorAll("[data-skill-source]").length===4');
      await search("missing");
      await wait('document.querySelectorAll("[data-skill-row]").length===0');
      assert.match(await evaluate('document.querySelector("[data-skill-counts]").innerText'), /0 skills.*0 sources/s);
      await search("");
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
      await click('[data-provenance]');
      await click('.skills-actions button');
      await wait('document.querySelectorAll("[data-skill-plugin]").length===3');
      assert.match(await evaluate('document.querySelector("[data-skill-counts]").innerText'), /5 skills.*7 sources/s);
      assert.equal(await evaluate('document.querySelectorAll("[data-skill-overlap]").length'), 0, 'APM ownership and aliases, including aliases across scopes, do not establish manual/plugin duplication');
      assert.equal(await evaluate('document.querySelectorAll("[data-skill-source=multi]").length'), 2, 'a source can belong to two plugins without inflating global counts');
      assert.equal(await evaluate(`document.querySelectorAll('[data-skill-plugin="Plugin (name unknown)"] [data-skill-source]').length`), 1);
      assert.equal(await evaluate('document.querySelector("[data-local-skills] [data-skill-source]").dataset.skillSource'), 'apm-local');
      assert.equal(await evaluate('document.querySelectorAll("[data-skill-modified]").length'), 1, 'only the row containing a changed source gets a cue');
      await click('input[aria-label="Changed only"]');
      await wait('document.querySelectorAll("[data-skill-source]").length===1');
      assert.match(await evaluate('document.querySelector("[data-skill-counts]").innerText'), /1 skill.*1 source/s);
      assert.equal(await evaluate('document.querySelector("[data-skill-source]").dataset.skillSource'), 'apm-local');
      assert.equal(await evaluate('document.querySelectorAll("[data-skill-plugin]").length'), 0);
      await filter('Scope', 'workspace');
      await wait('document.querySelectorAll("[data-skill-source]").length===0');
      await filter('Scope', 'personal');
      await wait('document.querySelectorAll("[data-skill-source]").length===1');
      await filter('Coding tool', 'claude');
      await wait('document.querySelectorAll("[data-skill-source]").length===0');
      await filter('Coding tool', 'codex');
      await wait('document.querySelectorAll("[data-skill-source]").length===1');
      await search('alpha@local');
      await wait('document.querySelectorAll("[data-skill-source]").length===0');
      await search('owner/package');
      await wait('document.querySelectorAll("[data-skill-source]").length===1');
      await click('[data-local-skills] [data-skill-row] summary');
      await click('[data-skill-source="apm-local"] [data-skill-detail]');
      await wait('document.querySelector("[data-skill-content]").innerText.includes("apm-local")');
      await click('[data-local-skills] [data-skill-row] summary');
      await search('');
      await filter('Scope', '');
      await filter('Coding tool', '');
      await click('input[aria-label="Changed only"]');
      await wait('document.querySelectorAll("[data-skill-source]").length===8');

      await click('[data-skill-plugin="alpha@local"] [data-skill-row="apm-skill"] summary');
      assert.match(await evaluate('document.querySelector("[data-skill-source=apm-plugin] [data-apm-integrity]").innerText'), /unavailable.*No recorded SHA-256/s);
      await click('[data-skill-plugin="alpha@local"] [data-skill-row="multi-skill"] summary');
      assert.match(await evaluate('document.querySelector("[data-skill-source=multi] [data-apm-integrity]").innerText'), /matches APM record/);
      assert.match(await evaluate('document.querySelector("[data-skill-source=multi] h4").innerText'), /APM.*sources\/very-long-package-name/);
      for (const width of [1000, 700, 360]) {
        win.setContentSize(width, 850);
        await evaluate("new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))");
        assert.ok(await evaluate("document.querySelector('.settings-content').scrollWidth <= document.querySelector('.settings-content').clientWidth + 1"), `long APM package heading has no horizontal scrolling at ${width}`);
      }
      if (process.env.ORKWORKS_SKILLS_CAPTURE) {
        const { writeFileSync } = await import("node:fs");
        await evaluate(`document.querySelector('[data-skill-source="multi"]').scrollIntoView({block:"center"});new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))`);
        writeFileSync(process.env.ORKWORKS_SKILLS_CAPTURE + ".apm.png", (await win.webContents.capturePage()).toPNG());
      }
      win.setContentSize(1000, 850);
      await click('[data-local-skills] [data-skill-row] summary');
      assert.match(await evaluate('document.querySelector("[data-skill-source=apm-local] h4").innerText'), /APM.*owner\/package/);
      assert.match(await evaluate('document.querySelector("[data-skill-source=apm-local] [data-apm-integrity]").innerText'), /differs from APM record/);
      await search('owner/package');
      await wait('document.querySelectorAll("[data-skill-source]").length===1');
      assert.equal(await evaluate('document.querySelector("[data-skill-source]").dataset.skillSource'), 'apm-local');
      if (process.env.ORKWORKS_SKILLS_CAPTURE) {
        const { writeFileSync } = await import("node:fs");
        await evaluate(`document.querySelector('[data-skill-source="apm-local"]').scrollIntoView({block:"center"});new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))`);
        writeFileSync(process.env.ORKWORKS_SKILLS_CAPTURE + ".hash.png", (await win.webContents.capturePage()).toPNG());
      }
      await evaluate('document.querySelector(".settings-content").scrollTop = 0');
      await wait('document.querySelector("[data-apm-settings]")');
      assert.match(await evaluate('document.querySelector("[data-apm-settings]").innerText'), /0.33.0/);
      assert.match(await evaluate('document.querySelector("[data-apm-settings]").innerText'), /\/usr\/local\/bin\/apm/);
      const apmPath = value => evaluate(`(()=>{const el=document.querySelector('input[aria-label="APM executable path"]');Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,"value").set.call(el,${JSON.stringify(value)});el.dispatchEvent(new Event("input",{bubbles:true}))})()`);
      await apmPath('/custom/unsupported-apm');
      await click('[data-save-apm]');
      await wait('document.querySelector("[data-apm-settings]").innerText.includes("0.34.0")');
      assert.equal(await evaluate('document.querySelector("[data-add-skill]").disabled'), true);
      assert.equal(await evaluate('document.querySelector("[data-install-skill]")'), null);
      await apmPath('');
      await click('[data-save-apm]');
      await wait('document.querySelector("[data-apm-settings]").innerText.includes("0.33.0")');
      assert.equal(await evaluate('document.querySelector("[data-add-skill]").disabled'), false);
      await click('[data-check-apm]');
      await wait('!document.querySelector("[data-check-apm]").disabled');
      for (const width of [1000, 700, 360]) {
        win.setContentSize(width, 850);
        await evaluate("new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))");
        assert.ok(await evaluate("document.querySelector('.settings-content').scrollWidth <= document.querySelector('.settings-content').clientWidth + 1"), `APM settings do not overflow at ${width}`);
      }
      if (process.env.ORKWORKS_SKILLS_CAPTURE) {
        const { writeFileSync } = await import("node:fs");
        await evaluate('document.querySelector("[data-apm-settings]").scrollIntoView({block:"start"});new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))');
        writeFileSync(process.env.ORKWORKS_SKILLS_CAPTURE + ".settings.png", (await win.webContents.capturePage()).toPNG());
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
  test("skills group by plugin, flag local overlap, filter sources and read source-specific details", async () => {
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
