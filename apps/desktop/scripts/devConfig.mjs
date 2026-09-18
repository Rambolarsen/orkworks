import { resolve } from "path";

const DEFAULT_DEV_PORT = 5173;

function resolveDevPort() {
  const raw = process.env.ORKWORKS_DEV_PORT;
  if (raw === undefined || raw.trim() === "") return DEFAULT_DEV_PORT;
  const port = Number(raw);
  if (!Number.isInteger(port) || port < 1 || port > 65535) return DEFAULT_DEV_PORT;
  return port;
}

export function createViteServerOptions(root) {
  return {
    configFile: resolve(root, "vite.config.mjs"),
    root,
    server: {
      port: resolveDevPort(),
      strictPort: true,
    },
  };
}

export function electronSpawnConfig(root, url) {
  // On Windows, .CMD wrapper scripts require shell:true to run at all; Node's
  // shell:true only escapes args when they aren't split from the command, so
  // fold the command+args into one string and pass an empty args array to
  // avoid DEP0190 (unescaped shell arg concatenation) while keeping the
  // fixed, non-user-controlled argv unaffected.
  const isWindows = process.platform === "win32";
  const pnpmCommand = isWindows ? "pnpm.CMD" : "pnpm";
  const args = ["exec", "electron", "."];

  return {
    command: isWindows ? [pnpmCommand, ...args].join(" ") : pnpmCommand,
    args: isWindows ? [] : args,
    options: {
      cwd: root,
      env: { ...process.env, VITE_DEV_SERVER_URL: url },
      stdio: "inherit",
      shell: isWindows,
    },
  };
}
