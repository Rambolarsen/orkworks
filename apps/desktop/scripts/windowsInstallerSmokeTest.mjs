import { execFileSync as defaultExecFileSync } from "node:child_process";
import fs from "node:fs";
import { readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { createReleaseArtifactExpectation } from "./verifyReleaseArtifact.mjs";

const packageJsonPath = join(import.meta.dirname, "..", "package.json");
const WINDOWS_UNINSTALL_REGISTRY_ROOTS = [
  "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
  "HKLM\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
];

function registryOutputHasDisplayName(output, productName) {
  const expectedName = productName.toLowerCase();
  return String(output).split(/\r?\n/).some((line) => {
    const match = line.match(/^\s*DisplayName\s+REG_\S+\s+(.*)$/i);
    return match && match[1].toLowerCase().includes(expectedName);
  });
}

function readPackageJson() {
  return JSON.parse(readFileSync(packageJsonPath, "utf8"));
}

function assertNonEmptyFile(fsModule, path, label) {
  try {
    const stats = fsModule.statSync(path);
    if (!stats.isFile() || stats.size <= 0) throw new Error("wrong file type or empty file");
  } catch (error) {
    throw new Error(`Windows installer smoke test failed: ${label} missing at ${path}`, {
      cause: error,
    });
  }
}

function assertDirectory(fsModule, path, label) {
  try {
    if (!fsModule.statSync(path).isDirectory()) throw new Error("wrong file type");
  } catch (error) {
    throw new Error(`Windows installer smoke test failed: ${label} missing at ${path}`, {
      cause: error,
    });
  }
}

export function createWindowsInstallerExpectation({
  version,
  releaseDir,
  installDir,
  productName,
  expectedPublisher,
}) {
  const packaged = createReleaseArtifactExpectation("win32", "x64", version, releaseDir);
  const scriptsDir = join(installDir, "resources", "scripts");
  return {
    installerPath: packaged.installerPath,
    installDir,
    appPath: join(installDir, `${productName}.exe`),
    uninstallerPath: join(installDir, `Uninstall ${productName}.exe`),
    sidecarPath: join(installDir, "resources", "orkworksd.exe"),
    expectedPublisher,
    scriptsDir,
    scriptPaths: packaged.scriptPaths.map((path) => join(scriptsDir, basename(path))),
  };
}

function readAuthenticodeSignature(path, execFileSync = defaultExecFileSync) {
  const script = [
    "$signature = Get-AuthenticodeSignature -LiteralPath $args[0]",
    "[pscustomobject]@{ status = [string]$signature.Status; subject = [string]$signature.SignerCertificate.Subject } | ConvertTo-Json -Compress",
  ].join("; ");
  let output;
  try {
    output = execFileSync("powershell.exe", [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      script,
      path,
    ], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "inherit"],
      shell: false,
      windowsHide: true,
    });
  } catch (error) {
    throw new Error(`Windows installer smoke test failed during Authenticode verification: ${path}`, {
      cause: error,
    });
  }
  try {
    return JSON.parse(output);
  } catch (error) {
    throw new Error(`Windows installer smoke test failed: invalid Authenticode result for ${path}`, {
      cause: error,
    });
  }
}

export function verifyInstalledWindowsApp(
  expectation,
  fsModule = fs,
  signatureVerifier = readAuthenticodeSignature,
) {
  assertNonEmptyFile(fsModule, expectation.appPath, "installed application");
  assertNonEmptyFile(fsModule, expectation.sidecarPath, "installed Rust sidecar");
  if (expectation.expectedPublisher) {
    for (const path of [expectation.appPath, expectation.sidecarPath]) {
      const signature = signatureVerifier(path);
      if (signature?.status !== "Valid") {
        throw new Error(
          `Windows installer smoke test failed: Authenticode status ${signature?.status ?? "unknown"} for ${path}`,
        );
      }
      if (
        typeof signature.subject !== "string"
        || signature.subject !== expectation.expectedPublisher
      ) {
        throw new Error(
          `Windows installer smoke test failed: publisher mismatch for ${path}; expected ${expectation.expectedPublisher}`,
        );
      }
    }
  }
  assertDirectory(fsModule, expectation.scriptsDir, "installed hook scripts");
  for (const scriptPath of expectation.scriptPaths) {
    assertNonEmptyFile(fsModule, scriptPath, "installed hook script");
  }
}

export function isWindowsInstallationRegistered(productName, execFileSync = defaultExecFileSync) {
  for (const registryRoot of WINDOWS_UNINSTALL_REGISTRY_ROOTS) {
    try {
      const output = execFileSync("reg.exe", [
        "query",
        registryRoot,
        "/s",
        "/v",
        "DisplayName",
        "/f",
        productName,
        "/d",
      ], {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
        shell: false,
        windowsHide: true,
      });
      if (registryOutputHasDisplayName(output, productName)) return true;
    } catch (error) {
      if (error?.status === 1) continue;
      throw new Error(`Windows installer smoke test failed during registry probe: ${registryRoot}`, {
        cause: error,
      });
    }
  }
  return false;
}

export async function waitForDirectoryRemoval(
  installDir,
  {
    existsSync = fs.existsSync,
    now = () => performance.now(),
    sleep = (milliseconds) => new Promise((resolveSleep) => setTimeout(resolveSleep, milliseconds)),
    timeoutMs = 30_000,
    pollIntervalMs = 250,
  } = {},
) {
  const deadline = now() + timeoutMs;
  while (existsSync(installDir)) {
    if (now() >= deadline) {
      throw new Error(`Windows installer smoke test failed: installation directory remained after ${timeoutMs}ms: ${installDir}`);
    }
    await sleep(pollIntervalMs);
  }
}

function createDefaultInstallDir(env = process.env) {
  const baseDir = env.ORKWORKS_INSTALLER_SMOKE_DIR || env.RUNNER_TEMP || tmpdir();
  return join(baseDir, `orkworks-installer-smoke-${process.pid}-${Date.now()}`);
}

function runInstallerProcess(execFileSync, executablePath, args, operation) {
  try {
    execFileSync(executablePath, args, {
      stdio: "inherit",
      shell: false,
      windowsVerbatimArguments: true,
    });
  } catch (error) {
    throw new Error(`Windows installer smoke test failed during ${operation}: ${executablePath}`, {
      cause: error,
    });
  }
}

export async function runWindowsInstallerSmokeTest(options = {}) {
  const packageJson = readPackageJson();
  const {
    platform = process.platform,
    arch = process.arch,
    version = packageJson.version,
    releaseDir = resolve(import.meta.dirname, "..", "release"),
    installDir = createDefaultInstallDir(),
    productName = packageJson.productName,
    expectedPublisher,
    fsModule = fs,
    execFileSync = defaultExecFileSync,
    signatureVerifier,
    registryProbe,
    waitForDirectoryRemoval: waitForDirectoryRemovalFn = waitForDirectoryRemoval,
    waitOptions = {},
  } = options;

  if (platform !== "win32" || arch !== "x64") {
    throw new Error(`Unsupported Windows installer smoke-test target: ${platform}/${arch}`);
  }

  const expectation = createWindowsInstallerExpectation({
    version,
    releaseDir,
    installDir,
    productName,
    expectedPublisher,
  });

  if (fsModule.existsSync(installDir)) {
    throw new Error(`Windows installer smoke test failed: pre-existing installation directory: ${installDir}`);
  }

  const hasRegisteredInstallation = registryProbe
    ? registryProbe(productName)
    : isWindowsInstallationRegistered(productName, execFileSync);
  if (hasRegisteredInstallation) {
    throw new Error(`Windows installer smoke test failed: registered ${productName} installation exists`);
  }

  assertNonEmptyFile(fsModule, expectation.installerPath, "installer");
  runInstallerProcess(
    execFileSync,
    expectation.installerPath,
    ["/S", `/D=${expectation.installDir}`],
    "silent install",
  );
  verifyInstalledWindowsApp(expectation, fsModule, signatureVerifier);
  assertNonEmptyFile(fsModule, expectation.uninstallerPath, "uninstaller");
  runInstallerProcess(execFileSync, expectation.uninstallerPath, ["/S"], "silent uninstall");
  await waitForDirectoryRemovalFn(expectation.installDir, waitOptions);

  return expectation;
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  const expectedPublisher = process.env.WIN_EXPECTED_PUBLISHER;
  if (!expectedPublisher) {
    throw new Error("Windows installer smoke test failed: WIN_EXPECTED_PUBLISHER is required");
  }
  await runWindowsInstallerSmokeTest({ expectedPublisher });
}
