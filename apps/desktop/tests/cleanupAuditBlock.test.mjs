import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { build } from "esbuild";
import * as React from "react";

const require = createRequire(import.meta.url);
const compiled = await build({
  entryPoints: [new URL("../src/components/CleanupAuditBlock.tsx", import.meta.url).pathname],
  bundle: true, write: false, platform: "node", format: "cjs", packages: "external",
});

const recommendation = {
  id: "cleanup-card",
  audit: {
    entries: [
      { id: "rec-a", title: "Stale card", criteria: ["under_eligible", "stale"] },
      { id: "rec-b", title: "Noisy card", criteria: ["noise"] },
    ],
    scanned: 4,
    healthy: 2,
    staleAfterDays: 14,
  },
};

function nodes(tree) {
  if (Array.isArray(tree)) return tree.flatMap(nodes);
  if (!tree || typeof tree !== "object") return [];
  if (typeof tree.type === "function") return nodes(tree.type(tree.props));
  return [tree, ...nodes(tree.props?.children)];
}
function text(tree) {
  if (Array.isArray(tree)) return tree.map(text).join("");
  if (tree && typeof tree === "object") return text(typeof tree.type === "function" ? tree.type(tree.props) : tree.props?.children);
  return tree == null || typeof tree === "boolean" ? "" : String(tree);
}

function render(props) {
  const module = { exports: {} };
  new Function("require", "module", "exports", compiled.outputFiles[0].text)(
    (id) => (id === "react" ? React : require(id)), module, module.exports,
  );
  return module.exports.default(props);
}

test("audit block shows counts, flagged titles, and criteria badges", () => {
  const tree = render({ recommendation });
  const section = nodes(tree).find((node) => node.type === "section");
  assert.match(text(section), /4 proposed scanned · 2 healthy · stale window 14 days/);
  const details = nodes(section).find((node) => node.type === "details");
  assert.match(text(details), /2 flagged recommendations/);
  assert.match(text(details), /Stale card/);
  assert.match(text(details), /Noisy card/);
  assert.match(text(details), /Under Eligible/);
  assert.match(text(details), /Stale/);
  assert.match(text(details), /Noise/);
});

test("entries stay collapsed until the details element is opened", () => {
  const tree = render({ recommendation });
  const details = nodes(tree).find((node) => node.type === "details");
  assert.equal(details.props.open, undefined);
});

test("audit block renders nothing without an audit payload", () => {
  assert.equal(render({ recommendation: { id: "cleanup-card", audit: null } }), null);
});
