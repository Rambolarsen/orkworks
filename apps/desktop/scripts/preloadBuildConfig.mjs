import { resolve } from "path";

export function preloadBuildOptions(root) {
  return {
    entryPoints: [resolve(root, "electron/preload.ts")],
    outfile: resolve(root, "dist-electron/preload.js"),
    bundle: true,
    platform: "node",
    format: "cjs",
    target: "node18",
    external: ["electron"],
  };
}
