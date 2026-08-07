import { resolve } from "path";

export function createViteServerOptions(root) {
  return {
    configFile: resolve(root, "vite.config.ts"),
    root,
    server: {
      port: 5173,
      strictPort: true,
    },
  };
}

export function electronSpawnConfig(root, url) {
  // On Windows, use the .CMD wrapper; on other platforms use the direct executable
  const pnpmCommand = process.platform === "win32" ? "pnpm.CMD" : "pnpm";
  
  return {
    command: pnpmCommand,
    args: ["exec", "electron", "."],
    options: {
      cwd: root,
      env: { ...process.env, VITE_DEV_SERVER_URL: url },
      stdio: "inherit",
      shell: true,
    },
  };
}
