import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { transpileModule, ModuleKind } from "typescript";
import { formatTerminalOutputGapNotice } from "../src/terminalProtocol.ts";

const require = createRequire(import.meta.url);

function loadTerminalStoreWithMocks() {
  const writes = [];
  const sockets = [];
  let createdTerminal = null;

  class FakeTerminal {
    constructor() {
      this.rows = 24;
      this.cols = 80;
      this.options = { disableStdin: false, cursorBlink: true };
      createdTerminal = this;
    }
    loadAddon() {}
    attachCustomKeyEventHandler() {}
    onData() {}
    onResize() {}
    resize(cols, rows) {
      this.cols = cols;
      this.rows = rows;
    }
    write(data) {
      if (data instanceof Uint8Array) {
        writes.push(`bin:${new TextDecoder().decode(data)}`);
      } else {
        writes.push(`text:${data}`);
      }
    }
    writeln(line) {
      writes.push(`writeln:${line}`);
    }
    dispose() {}
  }

  class FakeFitAddon {
    fit() {}
  }

  class FakeWebSocket {
    constructor(url) {
      this.url = url;
      this.readyState = 1;
      this.bufferedAmount = 0;
      this.binaryType = "";
      this.onopen = null;
      this.onmessage = null;
      this.onclose = null;
      sockets.push(this);
    }
    send() {}
    close() {}
  }

  const fakeWindow = {
    orkworks: { openExternalLink() {} },
    addEventListener() {},
  };
  const fakeDocument = {
    createElement() {
      return {
        dataset: {},
        style: { cssText: "" },
        isConnected: false,
        clientWidth: 0,
        clientHeight: 0,
        remove() {},
      };
    },
  };
  class FakeResizeObserver {
    disconnect() {}
  }

  const source = readFileSync(
    new URL("../src/terminalStore.ts", import.meta.url),
    "utf8",
  );
  const { outputText } = transpileModule(source, {
    compilerOptions: { module: ModuleKind.CommonJS },
  });

  const mod = { exports: {} };
  const fn = new Function(
    "require",
    "module",
    "exports",
    "window",
    "document",
    "ResizeObserver",
    "WebSocket",
    outputText,
  );
  fn(
    (id) => {
      if (id === "./backendAccess.ts") {
        return { openBackendSocket: (url) => new FakeWebSocket(url) };
      }
      if (id === "@xterm/xterm") return { Terminal: FakeTerminal };
      if (id === "@xterm/addon-fit") return { FitAddon: FakeFitAddon };
      if (id === "@xterm/addon-webgl") {
        return {
          WebglAddon: class {
            constructor() {
              throw new Error("no webgl in unit test");
            }
          },
        };
      }
      if (id === "./terminalSize") return require("../src/terminalSize.ts");
      if (id === "./terminalWheel") return { installTerminalWheelHandler() {} };
      if (id === "./terminalTheme") return { orkworksTerminalOptions: {} };
      if (id === "./api") return { getTerminalOutput: async () => ({ lines: [] }) };
      if (id === "./terminalReplay") return require("../src/terminalReplay.ts");
      if (id === "./terminalLinks") return { terminalLinkHandler: () => () => {} };
      if (id === "./terminalProtocol") return require("../src/terminalProtocol.ts");
      if (id === "./terminalRegistry") return require("../src/terminalRegistry.ts");
      return require(id);
    },
    mod,
    mod.exports,
    fakeWindow,
    fakeDocument,
    FakeResizeObserver,
    FakeWebSocket,
  );

  return {
    store: mod.exports,
    writes,
    sockets,
    getTerminal: () => createdTerminal,
  };
}

function encode(text) {
  return new TextEncoder().encode(text).buffer;
}

test("renderer writes lossless recovered live chunks in order without a gap notice", () => {
  const { store, writes, sockets, getTerminal } = loadTerminalStoreWithMocks();
  const handle = store.ensureTerminal("s-lossless", "http://127.0.0.1:4321");
  const ws = sockets[0];

  ws.onmessage({ data: JSON.stringify({ type: "replay-start", cursor: 0 }) });
  ws.onmessage({ data: encode("initial-0") });
  ws.onmessage({ data: JSON.stringify({ type: "replay-end", cursor: 1 }) });
  ws.onmessage({ data: encode("recovered-1") });
  ws.onmessage({ data: encode("recovered-2") });
  ws.onmessage({ data: encode("live-3") });

  assert.deepEqual(writes, [
    "bin:initial-0",
    "bin:recovered-1",
    "bin:recovered-2",
    "bin:live-3",
  ]);
  assert.equal(handle.ended, false);
  assert.equal(handle.unavailable, false);
  assert.equal(getTerminal().options.disableStdin, false);
});

test("renderer surfaces an explicit output-gap notice and keeps the live session interactive", () => {
  const { store, writes, sockets, getTerminal } = loadTerminalStoreWithMocks();
  const handle = store.ensureTerminal("s-gap", "http://127.0.0.1:4321");
  const ws = sockets[0];

  ws.onmessage({ data: JSON.stringify({ type: "replay-start", cursor: 0 }) });
  ws.onmessage({ data: encode("before-gap") });
  ws.onmessage({ data: JSON.stringify({ type: "replay-end", cursor: 1 }) });

  const gap = {
    type: "output-gap",
    fromCursor: 1,
    toCursor: 4,
    missedChunks: 3,
  };
  ws.onmessage({ data: JSON.stringify(gap) });
  ws.onmessage({ data: encode("retained-4") });
  ws.onmessage({ data: encode("live-5") });

  assert.deepEqual(writes, [
    "bin:before-gap",
    `writeln:${formatTerminalOutputGapNotice(gap)}`,
    "bin:retained-4",
    "bin:live-5",
  ]);
  assert.equal(handle.ended, false);
  assert.equal(handle.unavailable, false);
  assert.equal(getTerminal().options.disableStdin, false);
  assert.equal(getTerminal().options.cursorBlink, true);
});
