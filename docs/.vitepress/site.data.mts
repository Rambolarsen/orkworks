import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { codingTools, fetchPublicRelease } from "./site-facts.mjs";
export default {
  watch: ["../../crates/orkworksd/resources/harnesses-v2.json"],
  async load() {
    const registry = JSON.parse(
      readFileSync(
        resolve(
          import.meta.dirname,
          "../../crates/orkworksd/resources/harnesses-v2.json",
        ),
        "utf8",
      ),
    );
    // Local builds and pull requests stay offline. Deployment explicitly refreshes releases.
    const checkedReleases = process.env.DOCS_FETCH_RELEASES === "1";
    let release = null;
    if (checkedReleases) {
      release = await fetchPublicRelease();
    }
    return { tools: codingTools(registry), release, checkedReleases };
  },
};
