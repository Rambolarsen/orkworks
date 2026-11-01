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
    { ...skill, id: "plugin-one", name: "plan", scope: "personal", management: "plugin", plugins: ["alpha@local"], tools: ["claude"] },
    { ...skill, id: "plugin-two", name: "build", scope: "personal", management: "plugin", plugins: ["beta@local"], tools: ["claude"] },
  ],
  roots: [],
  diagnostics: ["Coverage is limited; native loading is unknown."],
  apm: { available: true, message: "APM found" },
};
let workspaceOpen = true;
let installs = 0;
Object.assign(window, {
  orkworks: {
    getSkillInventory: async () => ({...inventory, workspace: workspaceOpen ? "/repo" : null, skills: inventory.skills.filter(skill => workspaceOpen || skill.scope === "personal")}),
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
    installSkillAdd: async () => {
      if (++installs === 1) return {ok:true, message:"Skill installed. Reload your coding tool if needed."};
      return new Promise(resolve => Object.assign(window, {finishSkillInstall: () => resolve({ok:true,message:"Old workspace install complete."})}));
    },
  },
});
function Fixture() {
  const [path, setPath] = useState<string | null>("/repo");
  return (
    <>
      <button data-switch onClick={() => {workspaceOpen=false;setPath(null);}}>
        Close workspace
      </button>
      <div className="settings-content" style={{height:"calc(100vh - 36px)",overflow:"auto",padding:20}}><SkillsSettings currentWorkspacePath={path} /></div>
    </>
  );
}
createRoot(document.getElementById("root")!).render(<Fixture />);
