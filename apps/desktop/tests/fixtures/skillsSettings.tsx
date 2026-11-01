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
    { ...skill, id: "plugin-one", name: "review", description: "Plugin-specific review instructions.", scope: "personal", management: "plugin", plugins: ["alpha@local"], version: "1.0", paths: ["/cache/alpha/1/skills/review"], resolvedPath: "/cache/alpha/1/skills/review", tools: ["claude"] },
    { ...skill, id: "plugin-two", name: "build", scope: "personal", management: "plugin", plugins: ["beta@local"], tools: ["claude"] },
  ],
  roots: [],
  diagnostics: ["Coverage is limited; native loading is unknown."],
  apm: { available: true, message: "APM found" },
};
const edgeSkills = [
  { ...skill, id: "apm-local", name: "apm-skill", scope: "personal", management: "apm", packageSource: "owner/package", apmIntegrity: { status: "differs" } },
  { ...skill, id: "apm-plugin", name: "apm-skill", scope: "personal", management: "apm", apmIntegrity: { status: "unavailable", reason: "No recorded SHA-256 hash." }, plugins: ["alpha@local"], resolvedPath: "/plugin/apm-skill", paths: ["/plugin/apm-skill"] },
  { ...skill, id: "alias", name: "alias-skill", scope: "personal", management: "plugin", plugins: ["alpha@local"], paths: ["/local/alias", "/plugin/alias"] },
  { ...skill, id: "multi", name: "multi-skill", scope: "personal", management: "apm", apmIntegrity: { status: "matches" }, plugins: ["alpha@local", "beta@local"], packageSource: "/sources/" + "very-long-package-name/".repeat(20) },
  { ...skill, id: "unknown", name: "unknown-skill", scope: "personal", management: "plugin" },
];
edgeSkills.push(
  { ...skill, id: "cross-local", name: "cross-alias", scope: "workspace", management: "local", paths: ["/repo/.agents/skills/cross-alias"], resolvedPath: "/shared/plugin/cross-alias" },
  { ...skill, id: "cross-plugin", name: "cross-alias", scope: "personal", management: "plugin", plugins: ["alpha@local"], paths: ["/shared/plugin/cross-alias"], resolvedPath: "/shared/plugin/cross-alias" },
);
let useEdgeSkills = false;
let workspaceOpen = true;
let installs = 0;
Object.assign(window, {
  orkworks: {
    getSkillInventory: async () => ({...inventory, workspace: workspaceOpen ? "/repo" : null, skills: (useEdgeSkills ? edgeSkills : inventory.skills).filter(skill => useEdgeSkills || workspaceOpen || skill.scope === "personal")}),
    readSkillDetail: async (id: string) =>
      `Read-only instructions for ${id}.`,
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
      <button data-provenance onClick={() => { useEdgeSkills = true; }}>Provenance cases</button>
      <button data-switch onClick={() => {workspaceOpen=false;setPath(null);}}>
        Close workspace
      </button>
      <div className="settings-content" style={{height:"calc(100vh - 36px)",overflow:"auto",padding:20}}><SkillsSettings currentWorkspacePath={path} /></div>
    </>
  );
}
createRoot(document.getElementById("root")!).render(<Fixture />);
