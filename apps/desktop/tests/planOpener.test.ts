import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm, realpath, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { openSessionPlan } from "../electron/planOpener.ts";

test("opens only the sidecar-validated plan path", async () => {
  const workspace = await mkdtemp(path.join(tmpdir(), "orkworks-plan-"));
  const docs = path.join(workspace, "docs");
  const plan = path.join(docs, "plan.md");
  await mkdir(docs);
  await writeFile(plan, "# plan");
  const requests: string[] = [];
  const opened: string[] = [];
  await openSessionPlan(
    "http://127.0.0.1:4444",
    "session 1",
    "private-token", workspace,
    async (url) => {
      requests.push(url.toString());
      return new Response(JSON.stringify({ path: plan }), { status: 200 });
    },
    async (filePath) => { opened.push(filePath); return ""; },
  );

  assert.deepEqual(requests, ["http://127.0.0.1:4444/sessions/session%201/open-plan"]);
  assert.deepEqual(opened, [await realpath(plan)]);
  await rm(workspace, { recursive: true, force: true });
});

test("reports sidecar and OS-handler failures without exposing a path", async () => {
  await assert.rejects(
    openSessionPlan("http://127.0.0.1:4444", "s", "token", process.cwd(), async () => new Response(null, { status: 409 }), async () => ""),
    /Couldn’t open this plan/,
  );
  const workspace = await mkdtemp(path.join(tmpdir(), "orkworks-plan-"));
  const plan = path.join(workspace, "plan.md");
  await writeFile(plan, "# plan");
  await assert.rejects(openSessionPlan("http://127.0.0.1:4444", "s", "token", workspace, async () => new Response(JSON.stringify({ path: plan })), async () => "OS refused"), /configured application/);
  await rm(workspace, { recursive: true, force: true });
});

test("starts the sidecar with the plan token and retains the restored workspace", async () => {
  const mainSource = await readFile(new URL("../electron/main.ts", import.meta.url), "utf8");

  assert.match(mainSource, /env: \{ \.\.\.process\.env, ORKWORKS_OPEN_PLAN_TOKEN: openPlanToken \}/);
  assert.match(mainSource, /workspacePath = initialWorkspacePath;/);
  assert.match(mainSource, /openPlanToken = randomBytes\(32\)\.toString\("hex"\);/);
});
