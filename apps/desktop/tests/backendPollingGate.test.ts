import assert from "node:assert/strict";
import test from "node:test";

import {
  shouldEnableSessionPolling,
  type BackendStatus,
} from "../src/backendPollingGate.ts";

test("requires connected backend, workspace, and no workspace switch", () => {
  const cases: Array<[BackendStatus, boolean, boolean, boolean]> = [
    ["connecting…", true, false, false],
    ["connected", false, false, false],
    ["connected", true, true, false],
    ["unreachable", true, false, false],
    ["exhausted", true, false, false],
    ["connected", true, false, true],
  ];

  for (const [status, hasWorkspace, isSwitchingWorkspace, expected] of cases) {
    assert.equal(
      shouldEnableSessionPolling(status, hasWorkspace, isSwitchingWorkspace),
      expected,
      `${status}, workspace=${hasWorkspace}, switching=${isSwitchingWorkspace}`,
    );
  }
});

test("keeps polling disabled through starting and ready until openWorkspace completes", () => {
  let status: BackendStatus = "connected";
  let hasWorkspace = true;
  let isSwitchingWorkspace = false;

  assert.equal(shouldEnableSessionPolling(status, hasWorkspace, isSwitchingWorkspace), true);

  // The sidecar emits starting before the replacement workspace is ready.
  status = "connecting…";
  isSwitchingWorkspace = true;
  assert.equal(shouldEnableSessionPolling(status, hasWorkspace, isSwitchingWorkspace), false);

  // A ready sidecar still has not completed the renderer's openWorkspace flow.
  status = "connected";
  assert.equal(shouldEnableSessionPolling(status, hasWorkspace, isSwitchingWorkspace), false);

  // openWorkspace has now applied the new workspace and cleared the switch.
  hasWorkspace = true;
  isSwitchingWorkspace = false;
  assert.equal(shouldEnableSessionPolling(status, hasWorkspace, isSwitchingWorkspace), true);
});
