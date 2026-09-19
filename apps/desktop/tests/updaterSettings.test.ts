import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";
import * as React from "react";
import { ModuleKind, ScriptTarget, transpileModule } from "typescript";

const settings = await readFile(new URL("../src/components/SettingsModal.tsx", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const css = await readFile(new URL("../src/App.css", import.meta.url), "utf8");
const require = createRequire(import.meta.url);
const compiledSettings = await build({
  entryPoints: [fileURLToPath(new URL("../src/components/SettingsModal.tsx", import.meta.url))],
  bundle: true,
  write: false,
  platform: "node",
  format: "cjs",
  packages: "external",
});

const initialSettings = {
  version: 1,
  hotkeys: {
    newSession: "CmdOrCtrl+N",
    toggleSessionsPanel: "CmdOrCtrl+Shift+S",
    toggleDetailPanel: "CmdOrCtrl+Shift+D",
    toggleTerminalPanel: "CmdOrCtrl+Shift+T",
    toggleCapacityPanel: "CmdOrCtrl+Shift+C",
    toggleRecommendationsPanel: "CmdOrCtrl+Shift+R",
    resetLayout: null,
  },
  defaultHotkeys: {
    newSession: "CmdOrCtrl+N",
    toggleSessionsPanel: "CmdOrCtrl+Shift+S",
    toggleDetailPanel: "CmdOrCtrl+Shift+D",
    toggleTerminalPanel: "CmdOrCtrl+Shift+T",
    toggleCapacityPanel: "CmdOrCtrl+Shift+C",
    toggleRecommendationsPanel: "CmdOrCtrl+Shift+R",
    resetLayout: null,
  },
  retention: { maxSessions: 20, maxAgeDays: 30 },
  debug: { showSessionIds: false, rendererHealthLogMs: 0 },
  providers: {
    version: 1,
    revision: 0,
    peonModel: null,
    ollamaBaseUrl: "http://127.0.0.1:11434",
    providers: [],
  },
};

function settingsFixture() {
  const values: unknown[] = [];
  const refs: Array<{ current: unknown }> = [];
  const dependencies: Array<readonly unknown[] | undefined> = [];
  const effects = new Map<number, () => void>();
  let stateIndex = 0;
  let refIndex = 0;
  let effectIndex = 0;

  const hooks = {
    ...React,
    useState(initial: unknown) {
      const index = stateIndex++;
      if (!(index in values)) values[index] = typeof initial === "function" ? initial() : initial;
      return [values[index], (next: unknown) => {
        values[index] = typeof next === "function"
          ? (next as (current: unknown) => unknown)(values[index])
          : next;
      }];
    },
    useRef(initial: unknown) {
      const index = refIndex++;
      if (!(index in refs)) refs[index] = { current: initial };
      return refs[index];
    },
    useCallback(callback: unknown) {
      return callback;
    },
    useLayoutEffect() {},
    useEffect(callback: () => void, deps?: readonly unknown[]) {
      const index = effectIndex++;
      const previous = dependencies[index];
      if (!deps || !previous || deps.length !== previous.length || deps.some((value, dependencyIndex) => !Object.is(value, previous[dependencyIndex]))) {
        effects.set(index, callback);
      }
      dependencies[index] = deps;
    },
  };
  const module = { exports: {} as { default: (props: Record<string, unknown>) => unknown } };
  new Function("require", "module", "exports", compiledSettings.outputFiles[0].text)(
    (id: string) => id === "react" ? hooks : require(id),
    module,
    module.exports,
  );

  let parentSection = "tools";
  const props: Record<string, unknown> = {
    initialSection: parentSection,
    initialSettings,
    updateStatus: { state: "never-checked" },
    updateCurrentVersion: "1.0.0",
    updateChannel: "latest",
    onCheckForUpdates() {},
    onDownloadUpdate() {},
    harnesses: [],
    documentRevision: null,
    onRefreshHarnesses: async () => ({ documentRevision: "", harnesses: [] }),
    activeHarnessIds: [],
    providerRuntime: null,
    onClose() {},
    onSaved() {},
    onSaveActiveHarnesses: async () => ({ ok: true, activeHarnessIds: [] }),
    onSectionChange(section: string) {
      parentSection = section;
      props.initialSection = section;
    },
  };
  const render = () => {
    stateIndex = 0;
    refIndex = 0;
    effectIndex = 0;
    return module.exports.default(props);
  };
  const syncInitialSection = () => {
    const effect = effects.get(0);
    effects.clear();
    effect?.();
  };
  const openUpdatesFromMenu = () => {
    parentSection = "updates";
    props.initialSection = parentSection;
    let tree = render();
    syncInitialSection();
    tree = render();
    return tree;
  };
  return {
    render,
    syncInitialSection,
    openUpdatesFromMenu,
    parentSection: () => parentSection,
  };
}

function nodes(tree: unknown): Array<{ type?: unknown; props?: Record<string, unknown> }> {
  if (Array.isArray(tree)) return tree.flatMap(nodes);
  if (!tree || typeof tree !== "object") return [];
  const node = tree as { type?: unknown; props?: Record<string, unknown> };
  return [node, ...nodes(node.props?.children)];
}

function text(tree: unknown): string {
  if (Array.isArray(tree)) return tree.map(text).join("");
  if (tree && typeof tree === "object") return text((tree as { props?: { children?: unknown } }).props?.children);
  return tree == null || typeof tree === "boolean" ? "" : String(tree);
}

function navButton(tree: unknown, label: string) {
  const button = nodes(tree).find((node) => node.type === "button" && text(node) === label);
  assert.ok(button, `missing ${label} navigation button`);
  return button;
}

test("Settings Updates renders every UpdateStatus state explicitly", () => {
  for (const state of ["unavailable", "never-checked", "checking", "up-to-date", "available", "downloading", "downloaded", "installing", "error"]) {
    assert.match(settings, new RegExp(`case "${state}"`));
  }
});

test("Settings Updates renders version, channel, candidate, notes, and download progress", () => {
  for (const field of [
    /Current version[\s\S]*currentVersion/,
    /Channel[\s\S]*channel === "latest"[\s\S]*Stable[\s\S]*channel === "nightly"[\s\S]*Nightly/,
    /Available version[\s\S]*candidate\.identity\.version/,
    /Release tag[\s\S]*candidate\.identity\.tag/,
    /Release notes[\s\S]*candidate\.releaseNotes/,
    /progress\.percent[\s\S]*progress\.transferred[\s\S]*progress\.total/,
  ]) {
    assert.match(settings, field);
  }
});

test("Settings Updates exposes only supported actions and disables busy actions", () => {
  assert.match(settings, /status\.state === "never-checked"[\s\S]*status\.state === "checking"[\s\S]*status\.state === "up-to-date"[\s\S]*status\.state === "unavailable"[\s\S]*Check for updates/);
  assert.match(settings, /disabled=\{!status \|\| status\.state === "checking" \|\| status\.state === "unavailable"\}/);
  assert.match(settings, /status\?\.state === "available" \|\| status\?\.state === "downloading"[\s\S]*onClick=\{onDownload\} disabled=\{status\.state === "downloading"\}[\s\S]*Download update/);
  assert.doesNotMatch(settings, /Restart and install|onClick=\{onInstall\}/);
  assert.match(settings, /status\?\.state === "error"[\s\S]*status\.message[\s\S]*onClick=\{onCheck\}>Retry/);
});

test("App owns the update subscription and shared update actions", () => {
  assert.match(app, /useEffect\(\(\) => window\.orkworks\.onUpdateStatus/);
  assert.match(app, /window\.orkworks\.checkForUpdates\(\)/);
  assert.match(app, /window\.orkworks\.downloadUpdate\(\)/);
  assert.doesNotMatch(app, /window\.orkworks\.requestUpdateInstall\(\)/);
});

test("downloaded updates direct users to manual signed-release installation without an install button", () => {
  const source = settings.slice(settings.indexOf("function UpdatesSection"), settings.indexOf("function isBareKey"));
  const compiled = transpileModule(source + "\nmodule.exports = UpdatesSection;", {
    compilerOptions: { target: ScriptTarget.ES2022, module: ModuleKind.CommonJS, jsx: 2 },
  }).outputText;
  const module = { exports: null as any };
  new Function("module", "React", "Button", compiled)(module, React, "button");
  const tree = module.exports({
    status: { state: "downloaded", candidate: { identity: { version: "1.1.0", tag: "v1.1.0" } } },
    currentVersion: "1.0.0",
    channel: "latest",
    onCheck() {},
    onDownload() {},
  });

  assert.match(text(tree), /in-app installation is currently unavailable/i);
  assert.match(text(tree), /signed release manually/i);
  assert.doesNotMatch(text(tree), /ready to install|restart and install/i);
  assert.equal(nodes(tree).filter((node) => node.type === "button").length, 0);
});

test("menu check starts while the unrelated settings refresh remains pending", () => {
  const handler = app.slice(app.indexOf('if (action === "check-for-updates")'), app.indexOf('if (action === "open-settings")'));
  assert.ok(handler.includes('openSettings("updates")'));
  const calls: string[] = [];
  new Function("action", "openSettings", "checkForUpdates", handler)(
    "check-for-updates",
    (section: string) => { calls.push(section); return new Promise(() => {}); },
    () => { calls.push("check"); return Promise.resolve(); },
  );
  assert.deepEqual(calls, ["updates", "check"]);
});

test("rendered update status strings have readable ellipses", () => {
  const source = settings.slice(settings.indexOf("function UpdatesSection"), settings.indexOf("function isBareKey"));
  const compiled = transpileModule(source + "\nmodule.exports = UpdatesSection;", {
    compilerOptions: { target: ScriptTarget.ES2022, module: ModuleKind.CommonJS, jsx: 2 },
  }).outputText;
  const module = { exports: null as any };
  new Function("module", "React", "Button", compiled)(module, React, "button");
  for (const [state, expected] of [
    [null, "Loading update status..."],
    ["checking", "Checking for updates..."],
    ["downloading", "Downloading update..."],
    ["installing", "Installing update..."],
  ]) {
    const tree = module.exports({ status: state ? { state, progress: { percent: 1, transferred: 1, total: 100 } } : null });
    const status = nodes(tree).find((node) => node.props?.role === "status");
    assert.equal(text(status), expected);
  }
});

test("an already-open Settings modal follows repeated menu navigation after a local tab change", () => {
  const view = settingsFixture();
  view.render();
  view.syncInitialSection();

  let tree = view.openUpdatesFromMenu();
  assert.match(String(navButton(tree, "Updates").props?.className), /settings-nav-button--active/);

  (navButton(tree, "Hotkeys").props?.onClick as () => void)();
  tree = view.render();
  assert.equal(view.parentSection(), "hotkeys");
  assert.match(String(navButton(tree, "Hotkeys").props?.className), /settings-nav-button--active/);

  tree = view.openUpdatesFromMenu();
  assert.match(String(navButton(tree, "Updates").props?.className), /settings-nav-button--active/);

  assert.match(app, /action === "check-for-updates"[\s\S]*openSettings\("updates"\)[\s\S]*checkForUpdates/);
  assert.match(app, /initialSection=\{settingsSection\}/);
  assert.match(app, /onSectionChange=\{setSettingsSection\}/);
});

test("update errors and manual-install warnings use distinct application colors", () => {
  assert.match(css, /\.updates-error\s*\{\s*color:\s*var\(--state-error\);\s*\}/);
  assert.match(css, /\.updates-warning\s*\{\s*color:\s*var\(--state-warn\);\s*\}/);
});
