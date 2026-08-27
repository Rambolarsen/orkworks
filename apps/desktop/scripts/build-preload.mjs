import { resolve } from "path";
import * as esbuild from "esbuild";

import { preloadBuildOptions } from "./preloadBuildConfig.mjs";

const root = resolve(import.meta.dirname, "..");

await esbuild.build(preloadBuildOptions(root));
