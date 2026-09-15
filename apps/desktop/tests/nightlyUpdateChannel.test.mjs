import assert from "node:assert/strict";
import { createRequire } from "node:module";
import test from "node:test";

import { GitHubProvider } from "electron-updater/out/providers/GitHubProvider.js";

const requireFromUpdater = createRequire(import.meta.resolve("electron-updater/package.json"));
const semver = requireFromUpdater("semver");

function atomFeed(tags) {
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<feed xmlns="http://www.w3.org/2005/Atom">',
    ...tags.map((tag) => [
      "<entry>",
      `<title>${tag}</title>`,
      `<link href="https://github.com/Rambolarsen/orkworks/releases/tag/${tag}"/>`,
      "<content>release notes</content>",
      "</entry>",
    ].join("")),
    "</feed>",
  ].join("");
}

function createProvider(tags) {
  const requests = [];
  const executor = {
    async request(options) {
      requests.push(options.path);
      if (options.path.endsWith(".atom")) return atomFeed(tags);
      if (options.path.endsWith("/nightly.yml")) {
        return [
          "version: 0.2.0-nightly.20260915.22.1",
          "files:",
          "  - url: OrkWorks.exe",
          "    sha512: dGVzdA==",
        ].join("\n");
      }
      throw new Error(`unexpected updater request: ${options.path}`);
    },
  };
  const updater = {
    allowPrerelease: true,
    channel: "nightly",
    currentVersion: new semver.SemVer("0.2.0-nightly.20260914.1.1"),
    fullChangelog: false,
  };
  return {
    provider: new GitHubProvider(
      { provider: "github", owner: "Rambolarsen", repo: "orkworks", channel: "nightly" },
      updater,
      { executor, platform: "win32", isUseMultipleRangeRequest: false },
    ),
    requests,
  };
}

test("custom nightly channel ignores a newer stable release", async () => {
  const newestNightly = "v0.2.0-nightly.20260915.22.1";
  const { provider, requests } = createProvider([
    "v0.3.0",
    newestNightly,
    "v0.2.0-nightly.20260914.21.1",
  ]);

  const update = await provider.getLatestVersion();

  assert.equal(update.tag, newestNightly);
  assert.equal(update.version, newestNightly.slice(1));
  assert(requests.some((path) => path.endsWith(`/${newestNightly}/nightly.yml`)));
  assert(!requests.some((path) => path.endsWith("/latest.yml")));
});

test("custom nightly channel does not select a stable-only feed", async () => {
  const { provider, requests } = createProvider(["v0.3.0", "v0.2.0"]);

  await assert.rejects(() => provider.getLatestVersion(), /No published versions/i);
  assert(!requests.some((path) => path.endsWith("/latest.yml")));
});
