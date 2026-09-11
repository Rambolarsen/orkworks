import { test } from "node:test";
import assert from "node:assert/strict";
import { codingTools, publicRelease, fetchPublicRelease } from "./site-facts.mjs";

test("release lookup distinguishes no published release from an unavailable API", async () => {
  const requests = [];
  const result = await fetchPublicRelease(async (url) => {
    requests.push(url);
    return new Response(null, { status: 404 });
  });
  assert.equal(result, null);
  assert.deepEqual(requests, ["https://api.github.com/repos/Rambolarsen/orkworks/releases/latest"]);
  await assert.rejects(fetchPublicRelease(async () => new Response(null, { status: 403 })), /HTTP 403/);
});

test("release lookup projects the latest public release into download data", async () => {
  const result = await fetchPublicRelease(async () => Response.json(release()));
  assert.equal(result.version, "v0.1.0");
  assert.equal(result.downloads[0].label, "macOS · Apple Silicon");
});

test("retired tools and generic shells are not advertised as coding integrations", () => {
  assert.deepEqual(
    codingTools({
      builtins: [
        {
          id: "old",
          name: "Old",
          retired: true,
          launch: { kind: "command-template" },
        },
        { id: "shell", name: "Shell", launch: { kind: "platform-shell" } },
        {
          id: "active",
          name: "Active",
          launch: { kind: "command-template" },
          integration: null,
        },
      ],
    }),
    [{ id: "active", name: "Active" }],
  );
});

const release = (overrides = {}) => ({
  draft: false,
  prerelease: false,
  tag_name: "v0.1.0",
  published_at: "2026-08-15T12:00:00Z",
  html_url: "https://github.com/Rambolarsen/orkworks/releases/tag/v0.1.0",
  assets: [
    {
      name: "OrkWorks-0.1.0-mac-arm64.dmg",
      browser_download_url:
        "https://github.com/Rambolarsen/orkworks/releases/download/v0.1.0/OrkWorks-0.1.0-mac-arm64.dmg",
    },
  ],
  ...overrides,
});

test("drafts and prereleases cannot become the public download", () => {
  assert.equal(
    publicRelease([release({ draft: true }), release({ prerelease: true })]),
    null,
  );
  assert.equal(publicRelease([]), null);
});

test("newest published release is selected, not API order or package version", () => {
  const result = publicRelease([
    release(),
    release({ tag_name: "v0.2.0", published_at: "2026-09-01T12:00:00Z" }),
  ]);
  assert.equal(result.version, "v0.2.0");
  assert.equal(result.downloads[0].label, "macOS · Apple Silicon");
});

test("non-installers and links outside the repository are excluded", () => {
  const result = publicRelease([
    release({
      assets: [
        {
          name: "OrkWorks-0.1.0-win-x64.exe",
          browser_download_url: "https://evil.example/setup.exe",
        },
        {
          name: "debug.zip",
          browser_download_url:
            "https://github.com/Rambolarsen/orkworks/releases/download/v0.1.0/debug.zip",
        },
      ],
    }),
  ]);
  assert.deepEqual(result.downloads, []);
  assert.equal(
    publicRelease([release({ html_url: "javascript:alert(1)" })]),
    null,
  );
});
