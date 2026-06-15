import { execSync } from "child_process";
import { existsSync, writeFileSync } from "fs";
import { join, resolve } from "path";

const electronPath = resolve(import.meta.dirname, "..", "node_modules", "electron");
const distPath = join(electronPath, "dist");
const pathTxt = join(electronPath, "path.txt");
const platformPath = "Electron.app/Contents/MacOS/Electron";

if (existsSync(join(distPath, platformPath))) {
  process.exit(0);
}

const zipPath = execSync(
  `find ~/Library/Caches/electron -name "electron-*-darwin-arm64.zip" 2>/dev/null | head -1`,
  { encoding: "utf-8" }
).trim();

if (!zipPath) {
  console.warn("[setup-electron] no cached electron zip found, electron postinstall should handle it");
  process.exit(0);
}

console.log(`[setup-electron] extracting ${zipPath} -> ${distPath}`);
execSync(`unzip -qo "${zipPath}" -d "${distPath}"`, { stdio: "inherit" });

if (existsSync(join(distPath, platformPath))) {
  writeFileSync(pathTxt, platformPath);
  console.log("[setup-electron] done");
} else {
  console.error("[setup-electron] extraction failed");
  process.exit(1);
}
