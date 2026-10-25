import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import xterm from "@xterm/xterm";

async function state(chunks) {
  const terminal = new xterm.Terminal({ cols: 80, rows: 24, allowProposedApi: true });
  try {
    for (const chunk of chunks) {
      await new Promise(resolve => terminal.write(new Uint8Array(chunk), resolve));
    }
    return { buffer: terminal.buffer.active.type, modes: terminal.modes };
  } finally {
    terminal.dispose();
  }
}

for (const fixture of JSON.parse(readFileSync(0, "utf8"))) {
  assert.deepEqual(await state(fixture.replay), await state(fixture.continuous), fixture.name);
}
