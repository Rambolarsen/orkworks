import React, { useState } from "react";
import { createRoot } from "react-dom/client";
import SkillsSettings from "../../src/components/SkillsSettings";
import "../../src/styles/tokens.css";
import "../../src/App.css";
const skill = {
  name: "review",
  description: "Review code <img data-injected src=x>",
  paths: ["/repo/.agents/skills/review"],
  resolvedPath: "/repo/.agents/skills/review",
  tools: ["codex"],
  management: "local",
};
const inventory = {
  workspace: "/repo",
  skills: [
    { ...skill, id: "workspace-review", scope: "workspace" },
    {
      ...skill,
      id: "personal-review",
      scope: "personal",
      paths: ["/home/.agents/skills/review"],
    },
  ],
  roots: [],
  diagnostics: ["Coverage is limited; native loading is unknown."],
  apm: { available: true, message: "APM found" },
};
Object.assign(window, {
  orkworks: {
    getSkillInventory: async () => inventory,
    readSkillDetail: async () =>
      "---\nname: review\n---\nRead-only instructions.",
    pickSkillSource: async () => "/source/review",
    previewSkillAdd: async (request: any) => ({
      id: "preview",
      scope: request.scope,
      source: request.source,
      target: "codex",
      destination: request.scope === "personal" ? "/home" : "/repo",
      available: true,
      summary: "A reviewed local skill addition.",
      paths: ["/repo/apm.yml"],
      command: "apm install /source/review",
    }),
    installSkillAdd: async () => ({
      ok: true,
      message: "Skill installed. Reload your coding tool if needed.",
    }),
  },
});
function Fixture() {
  const [path, setPath] = useState<string | null>("/repo");
  return (
    <>
      <button data-switch onClick={() => setPath(null)}>
        Close workspace
      </button>
      <div className="settings-content" style={{height:"calc(100vh - 36px)",overflow:"auto",padding:20}}><SkillsSettings currentWorkspacePath={path} /></div>
    </>
  );
}
createRoot(document.getElementById("root")!).render(<Fixture />);
