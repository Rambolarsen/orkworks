import { useEffect, useRef, useState } from "react";
import type {
  SkillInventory,
  SkillPreview,
  SkillScope,
  SkillTool,
} from "../skillTypes";

const toolNames: Record<SkillTool, string> = {
  codex: "Codex",
  claude: "Claude Code",
  copilot: "GitHub Copilot CLI",
  opencode: "OpenCode",
};
const message = (error: unknown) =>
  error instanceof Error ? error.message : "Could not complete the request.";
export default function SkillsSettings({
  currentWorkspacePath,
}: {
  currentWorkspacePath: string | null;
}) {
  const [inventory, setInventory] = useState<SkillInventory | null>(null);
  const [query, setQuery] = useState("");
  const [tool, setTool] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");
  const [error, setError] = useState("");
  const [adding, setAdding] = useState(false);
  const [scope, setScope] = useState<SkillScope>(
    currentWorkspacePath ? "workspace" : "personal",
  );
  const [source, setSource] = useState("");
  const [preview, setPreview] = useState<SkillPreview | null>(null);
  const [detail, setDetail] = useState<{ name: string; text: string } | null>(
    null,
  );
  const generation = useRef(0);
  const load = async () => {
    const request = ++generation.current;
    setBusy(true);
    setError("");
    try {
      const result = await window.orkworks.getSkillInventory();
      if (generation.current === request) setInventory(result);
    } catch (error) {
      if (generation.current === request) setError(message(error));
    } finally {
      if (generation.current === request) setBusy(false);
    }
  };
  useEffect(() => {
    if (!currentWorkspacePath) setScope("personal");
    setInventory(null);
    setDetail(null);
    setPreview((value) => (value?.scope === "workspace" ? null : value));
    void load();
    return () => {
      generation.current++;
    };
  }, [currentWorkspacePath]);
  const perform = async (action: () => Promise<void>) => {
    const request = generation.current;
    setBusy(true);
    setError("");
    try {
      await action();
    } catch (error) {
      if (request === generation.current) setError(message(error));
    } finally {
      if (request === generation.current) setBusy(false);
    }
  };
  const review = () =>
    perform(async () => {
      const request = generation.current;
      const result = await window.orkworks.previewSkillAdd({
        scope,
        source,
        target: "codex",
        expectedWorkspace: currentWorkspacePath,
      });
      if (request === generation.current) setPreview(result);
    });
  const install = () =>
    perform(async () => {
      if (!preview?.id) return;
      const result = await window.orkworks.installSkillAdd(preview.id);
      setStatus(result.message);
      setPreview(null);
      // Even a failed APM operation may have written files.
      await load();
    });
  const filtered = (inventory?.skills || []).filter(
    (skill) =>
      (!tool || skill.tools.includes(tool as SkillTool)) &&
      [skill.name, skill.description, ...skill.paths]
        .join(" ")
        .toLowerCase()
        .includes(query.toLowerCase()),
  );
  return (
    <section className="skills-settings" aria-label="Skills">
      <div className="skills-heading">
        <div>
          <h2>Skills</h2>
          <p>
            See where your skills come from and add them to a workspace or your
            personal setup.
          </p>
        </div>
        <div className="skills-actions">
          <button
            className="ui-button ui-button--md ui-button--secondary"
            disabled={busy}
            onClick={() => void load()}
          >
            Refresh
          </button>
          <button
            data-add-skill
            className="ui-button ui-button--md ui-button--primary"
            disabled={busy || !inventory?.apm?.available}
            onClick={() => setAdding(!adding)}
          >
            Add skill
          </button>
        </div>
      </div>
      {inventory?.apm && !inventory.apm.available && (
        <p>{inventory.apm.message}</p>
      )}
      <div role="status" aria-live="polite">
        {busy ? "Working…" : status}
      </div>
      {error && <p role="alert">{error}</p>}
      <div className="skills-filters">
        <label>
          Search skills
          <input
            className="ui-input"
            aria-label="Search skills"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
        <label>
          Coding tool
          <select
            className="ui-input"
            value={tool}
            onChange={(event) => setTool(event.target.value)}
          >
            <option value="">All coding tools</option>
            {Object.entries(toolNames).map(([id, name]) => (
              <option key={id} value={id}>
                {name}
              </option>
            ))}
          </select>
        </label>
      </div>
      {adding && (
        <section className="skills-add" aria-label="Add skill">
          <h3>Add a skill</h3>
          <label>
            Install for
            <select
              className="ui-input"
              disabled={busy}
              value={scope}
              onChange={(event) => {
                setScope(event.target.value as SkillScope);
                setPreview(null);
              }}
            >
              <option value="workspace" disabled={!currentWorkspacePath}>
                Workspace
              </option>
              <option value="personal">Personal</option>
            </select>
          </label>
          <p>
            {scope === "workspace"
              ? currentWorkspacePath || "Open a workspace first."
              : "Personal skills may be available across your projects."}
          </p>
          <label>
            Source
            <input
              className="ui-input"
              disabled={busy}
              value={source}
              placeholder="GitHub owner/repository or choose a local skill"
              onChange={(event) => {
                setSource(event.target.value);
                setPreview(null);
              }}
            />
          </label>
          <div className="skills-actions">
            <button
              data-pick-skill
              className="ui-button ui-button--md ui-button--secondary"
              disabled={busy}
              onClick={() =>
                void perform(async () => {
                  const path = await window.orkworks.pickSkillSource();
                  if (path) {
                    setSource(path);
                    setPreview(null);
                  }
                })
              }
            >
              Choose folder…
            </button>
            <button
              data-review-skill
              className="ui-button ui-button--md ui-button--secondary"
              disabled={
                busy ||
                !source ||
                (scope === "workspace" && !currentWorkspacePath)
              }
              onClick={() => void review()}
            >
              Review addition
            </button>
          </div>
          <p>
            Initial install support: local single-skill directories for Codex
            through APM 0.33. Other sources show a terminal command when a
            complete review is unavailable.
          </p>
          {preview && (
            <div className="skills-preview">
              <h4>
                {preview.available
                  ? "Review addition"
                  : "Installation unavailable"}
              </h4>
              <p>
                <strong>
                  {preview.scope === "personal" ? "Personal" : "Workspace"}
                </strong>{" "}
                · {preview.destination}
              </p>
              <p>{preview.available ? preview.summary : preview.reason}</p>
              {preview.available ? (
                <>
                  <ul>
                    {preview.paths.map((path) => (
                      <li key={path}>
                        <code>{path}</code>
                      </li>
                    ))}
                  </ul>
                  <button
                    data-install-skill
                    className="ui-button ui-button--md ui-button--primary"
                    disabled={busy}
                    onClick={() => void install()}
                  >
                    Install skill…
                  </button>
                </>
              ) : (
                <>
                  <p>
                    Run this yourself in the intended directory if you want APM
                    to perform the broader operation:
                  </p>
                  <pre>{preview.command}</pre>
                </>
              )}
            </div>
          )}
        </section>
      )}
      {(["workspace", "personal"] as const).map((group) => (
        <section
          key={group}
          data-skill-scope={group}
          className="skills-group"
          aria-label={`${group === "workspace" ? "Workspace" : "Personal"} skills`}
        >
          <h3>
            {group === "workspace" ? "Workspace" : "Personal"}{" "}
            <span>
              {filtered.filter((skill) => skill.scope === group).length}
            </span>
          </h3>
          <p>
            {group === "workspace"
              ? currentWorkspacePath || "Open a workspace to see its skills."
              : "Skills found in your user configuration and registered plugin locations."}
          </p>
          {!filtered.some((skill) => skill.scope === group) && (
            <p>
              {inventory
                ? "No matching skills found in the inspected roots."
                : "Loading skills…"}
            </p>
          )}
          {filtered
            .filter((skill) => skill.scope === group)
            .map((skill) => (
              <article key={skill.id} data-skill-row className="skills-row">
                <div className="skills-row-heading">
                  <strong>{skill.name}</strong>
                  <span className="skills-badge">
                    {group === "workspace" ? "Workspace" : "Personal"}
                  </span>
                  <span>
                    {skill.management === "apm"
                      ? "APM record"
                      : skill.management === "plugin"
                        ? "Plugin"
                        : "Local file"}
                  </span>
                </div>
                <p>{skill.description}</p>
                <p className="skills-tool-coverage">
                  Found ·{" "}
                  {skill.tools.length
                    ? skill.tools.map((tool) => toolNames[tool]).join(", ")
                    : "Coding-tool discovery unknown"}
                  {skill.version ? ` · ${skill.version}` : ""}
                </p>
                {skill.paths.map((path) => (
                  <p className="skills-path" key={path}>
                    {path}
                  </p>
                ))}
                {!skill.paths.includes(skill.resolvedPath) && (
                  <p className="skills-path">
                    Resolves to {skill.resolvedPath}
                  </p>
                )}
                {skill.packageSource && (
                  <p className="skills-path">Package: {skill.packageSource}</p>
                )}
                <button
                  data-skill-detail
                  className="ui-button ui-button--sm ui-button--secondary"
                  disabled={busy}
                  onClick={() =>
                    void perform(async () => {
                      const request = generation.current;
                      const text = await window.orkworks.readSkillDetail(
                        skill.id,
                      );
                      if (request === generation.current)
                        setDetail({ name: skill.name, text });
                    })
                  }
                >
                  Read skill
                </button>
              </article>
            ))}
        </section>
      ))}
      {detail && (
        <section className="skills-detail">
          <div className="skills-heading">
            <h3>{detail.name}</h3>
            <button className="ui-button ui-button--md ui-button--secondary" onClick={() => setDetail(null)}>
              Close details
            </button>
          </div>
          <pre data-skill-content>{detail.text}</pre>
        </section>
      )}
      {inventory && (
        <details className="skills-coverage">
          <summary>Discovery coverage and source roots</summary>
          <p>
            Found on disk does not confirm that a coding tool loaded or used a
            skill. Duplicate names retain both scopes; effective precedence is
            unknown.
          </p>
          <ul>
            {inventory.diagnostics.map((diagnostic, index) => (
              <li key={index}>{diagnostic}</li>
            ))}
          </ul>
          <ul>
            {inventory.roots.map((root, index) => (
              <li key={index}>
                {root.scope}: <code>{root.path}</code>
              </li>
            ))}
          </ul>
        </details>
      )}
    </section>
  );
}
