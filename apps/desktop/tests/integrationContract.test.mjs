import test from "node:test";
import assert from "node:assert/strict";
import ts from "typescript";
import { fileURLToPath } from "node:url";

// Only this no-emit test crosses the Electron/renderer compilation boundary.
// Production code continues to own an independent contract on each side.
function contractDiagnostics(source) {
  const filename = fileURLToPath(new URL("./integrationContract.types.ts", import.meta.url));
  const options = {
    strict: true,
    noEmit: true,
    skipLibCheck: true,
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext,
    moduleResolution: ts.ModuleResolutionKind.Bundler,
    allowImportingTsExtensions: true,
    types: ["electron"],
  };
  const host = ts.createCompilerHost(options);
  const getSourceFile = host.getSourceFile.bind(host);
  host.getSourceFile = (path, languageVersion, ...rest) => path === filename
    ? ts.createSourceFile(path, source, languageVersion, true)
    : getSourceFile(path, languageVersion, ...rest);
  return ts.getPreEmitDiagnostics(ts.createProgram([filename], options, host));
}

const contracts = `
import type * as Main from "../electron/activeHarnessIntegration";
import type * as Renderer from "../src/orkworksWindow";
import type * as Presentation from "../src/harnessIntegrationPresentation";
import type * as MainProvider from "../electron/providerTypes";
import type * as RendererProvider from "../src/providerTypes";
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends
  (<T>() => T extends B ? 1 : 2) ? true : false;
type Expect<T extends true> = T;
type Save = Expect<Equal<Main.ActiveHarnessSaveResult, Renderer.ActiveHarnessSaveResult>>;
type Operation = Expect<Equal<Main.ActiveHarnessIntegrationResult, Renderer.ActiveHarnessIntegrationResult>>;
type Key = Expect<Equal<Main.IntegrationKey, Renderer.IntegrationKey>>;
type PresentationSave = Expect<Equal<Presentation.ActiveHarnessSaveResult, Renderer.ActiveHarnessSaveResult>>;
type PeonSave = Expect<Equal<MainProvider.PeonSelectionSaveResult, RendererProvider.PeonSelectionSaveResult>>;
type PeonBridge = Expect<Equal<Awaited<ReturnType<Window["orkworks"]["savePeonSelection"]>>, MainProvider.PeonSelectionSaveResult>>;
`;

test("integration save contracts agree across the Electron boundary", () => {
  const diagnostics = contractDiagnostics(contracts);
  assert.deepEqual(diagnostics.map((item) => ts.flattenDiagnosticMessageText(item.messageText, "\n")), []);
});

test("the contract check rejects nested payload drift", () => {
  const diagnostics = contractDiagnostics(contracts + `
    type Drifted = Omit<Main.ActiveHarnessIntegrationResult, "outcome"> & { outcome: "new_outcome" };
    type Rejected = Expect<Equal<Drifted, Renderer.ActiveHarnessIntegrationResult>>;
  `);
  assert.equal(diagnostics.length, 1);
  assert.equal(diagnostics[0].code, 2344);
});

test("the Peon contract check rejects lost settings fields", () => {
  const diagnostics = contractDiagnostics(contracts + `
    type Success = Extract<MainProvider.PeonSelectionSaveResult, { ok: true }>;
    type Drifted = { ok: true; settings: Omit<Success["settings"], "defaultHotkeys"> };
    type Rejected = Expect<Equal<Drifted, Extract<RendererProvider.PeonSelectionSaveResult, { ok: true }>>>;
  `);
  assert.equal(diagnostics.length, 1);
  assert.equal(diagnostics[0].code, 2344);
});
