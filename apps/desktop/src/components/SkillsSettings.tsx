import { useEffect, useRef, useState } from "react";
import { ChevronRight, Copy, Package, Plus, RefreshCw } from "lucide-react";
import type {
  SkillEntry,
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
function skillNameGroups(skills: SkillEntry[]) {
  const groups = new Map<string, SkillEntry[]>();
  for (const skill of skills) {
    const sources = groups.get(skill.name) || [];
    sources.push(skill);
    groups.set(skill.name, sources);
  }
  return [...groups].sort(([a], [b]) => a.localeCompare(b));
}
export default function SkillsSettings({
  currentWorkspacePath,
}: {
  currentWorkspacePath: string | null;
}) {
  const [inventory, setInventory] = useState<SkillInventory | null>(null);
  const [query, setQuery] = useState("");
  const [tool, setTool] = useState("");
  const [filterScope, setFilterScope] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");
  const [error, setError] = useState("");
  const [adding, setAdding] = useState(false);
  const [scope, setScope] = useState<SkillScope>(
    currentWorkspacePath ? "workspace" : "personal",
  );
  const [source, setSource] = useState("");
  const [preview, setPreview] = useState<SkillPreview | null>(null);
  const [detail, setDetail] = useState<{ id: string; text: string } | null>(
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
    setStatus("");
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
      const request = generation.current;
      const result = await window.orkworks.installSkillAdd(preview.id);
      if (request === generation.current) {
        setStatus(result.message);
        setPreview(null);
      }
      // Even a failed APM operation may have written files.
      await load();
    });
  const filtered = (inventory?.skills || []).filter(
    (skill) =>
      (!filterScope || skill.scope === filterScope) &&
      (!tool || skill.tools.includes(tool as SkillTool)) &&
      [skill.name, skill.description, ...(skill.plugins || []), ...skill.paths]
        .join(" ")
        .toLowerCase()
        .includes(query.toLowerCase()),
  );
  const grouped = skillNameGroups(filtered);
  const pluginEntries = new Map<string, SkillEntry[]>();
  const localSkills: SkillEntry[] = [];
  for (const skill of filtered) {
    const plugins = [...new Set(
      skill.plugins?.length ? skill.plugins
        : skill.management === "plugin" ? ["Plugin (name unknown)"] : [],
    )];
    if (!plugins.length) localSkills.push(skill);
    for (const plugin of plugins) {
      const entries = pluginEntries.get(plugin) || [];
      entries.push(skill);
      pluginEntries.set(plugin, entries);
    }
  }
  const pluginGroups = [...pluginEntries].sort(([a], [b]) => a.localeCompare(b));
  const overlappingNames = new Set(
    skillNameGroups(inventory?.skills || [])
      .filter(([, sources]) =>
        sources.some((source) => source.management === "local" && !source.plugins?.length)
        && sources.some((source) => source.management === "plugin" || source.plugins?.length),
      )
      .map(([name]) => name),
  );
  const renderRows = (entries: SkillEntry[]) =>
    skillNameGroups(entries).map(([name, sources]) => (
      <article key={name} data-skill-row={name} className="skills-row">
        <details>
          <summary className="skills-row-heading">
            <ChevronRight className="skills-chevron" size={15} aria-hidden="true" />
            <strong>{name}</strong>
            <span className="skills-badge">
              {sources.length} {sources.length === 1 ? "source" : "sources"}
            </span>
            {overlappingNames.has(name) && (
              <span data-skill-overlap className="skills-badge skills-badge--overlap" title="This name has separate local and plugin sources. Local means ownership is unrecorded; it does not prove manual installation. Other sources may be hidden by filters.">
                <Copy size={12} aria-hidden="true" /> Local + plugin
              </span>
            )}
          </summary>
          <div className="skills-row-body">
            {sources.map((skill) => (
              <section key={skill.id} data-skill-source={skill.id} className="skills-source" aria-label={`${name} source: ${skill.paths[0]}`}>
                <h4>
                  <span className="skills-badge">{skill.scope === "workspace" ? "Workspace" : "Personal"}</span>
                  <span>{skill.plugins?.join(" · ") || (skill.management === "apm" ? "APM record" : skill.management === "plugin" ? "Plugin (name unknown)" : "Local file")}</span>
                  {skill.plugins?.length && skill.management === "apm" ? <span className="skills-badge">APM record</span> : null}
                </h4>
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
                        setDetail({ id: skill.id, text });
                    })
                  }
                >
                  Read skill
                </button>
                {detail?.id === skill.id && (
                  <section className="skills-detail" aria-label={`${skill.name} instructions`}>
                    <div className="skills-heading">
                      <h4>SKILL.md</h4>
                      <button className="ui-button ui-button--sm ui-button--secondary" onClick={() => setDetail(null)}>Close text</button>
                    </div>
                    <pre data-skill-content>{detail.text}</pre>
                  </section>
                )}
              </section>
            ))}
          </div>
        </details>
      </article>
    ));
  return (
    <section className="skills-settings" aria-label="Skills">
      <div className="skills-heading">
        <div>
          <h2 data-shell-page-heading tabIndex={-1}>Skills</h2>
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
            <RefreshCw size={14} aria-hidden="true" /> Refresh
          </button>
          <button
            data-add-skill
            className="ui-button ui-button--md ui-button--primary"
            disabled={busy || !inventory?.apm?.available}
            aria-expanded={adding}
            onClick={() => setAdding(!adding)}
          >
            <Plus size={14} aria-hidden="true" /> {adding ? "Cancel addition" : "Add skill"}
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
            placeholder="Find a skill by name, plugin, description, or path…"
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
        <label>
          Coding tool
          <select
            className="ui-input"
            aria-label="Coding tool"
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
        <label>
          Scope
          <select
            className="ui-input"
            aria-label="Scope"
            value={filterScope}
            onChange={(event) => setFilterScope(event.target.value)}
          >
            <option value="">All scopes</option>
            <option value="workspace">Workspace</option>
            <option value="personal">Personal</option>
          </select>
        </label>
      </div>
      <p className="skills-group-caption">
        Skills are grouped by recorded plugin, with other files under Local skills.
        Expand a skill to inspect its sources. Local + plugin marks a name found in
        both, even if another source is hidden by filters. Matching names may have
        different instructions. Found does not mean loaded or enabled.
      </p>
      <p className="skills-group-caption">
        {currentWorkspacePath
          ? `Workspace: ${currentWorkspacePath}`
          : "Open a workspace to include its skills."}
      </p>
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
      <section className="skills-group" aria-label="Discovered skills">
        <h3 data-skill-counts>
          {grouped.length} {grouped.length === 1 ? "skill" : "skills"}
          <span>· {filtered.length} {filtered.length === 1 ? "source" : "sources"}</span>
        </h3>
        {!grouped.length && (
          <p>{inventory ? "No matching skills found in the inspected roots." : "Loading skills…"}</p>
        )}
        {!!pluginGroups.length && (
          <section className="skills-installation-group" aria-label="Plugins">
            <h3><Package size={16} aria-hidden="true" /> Plugins <span>{pluginGroups.length}</span></h3>
            {pluginGroups.map(([plugin, entries]) => (
              <section key={plugin} data-skill-plugin={plugin} className="skills-plugin" aria-label={`Plugin: ${plugin}`}>
                <h4 className="skills-plugin-heading"><Package size={14} aria-hidden="true" />{plugin}<span className="skills-badge">{skillNameGroups(entries).length} {skillNameGroups(entries).length === 1 ? "skill" : "skills"}</span></h4>
                {renderRows(entries)}
              </section>
            ))}
          </section>
        )}
        {!!localSkills.length && (
          <section data-local-skills className="skills-installation-group" aria-label="Local skills">
            <h3>Local skills</h3>
            <p className="skills-group-caption">No recorded plugin ownership. These may include manually installed or APM-generated files.</p>
            {renderRows(localSkills)}
          </section>
        )}
      </section>
      {inventory && (
        <details className="skills-coverage">
          <summary>Discovery coverage and source roots</summary>
          <p>
            Found on disk does not confirm that a coding tool loaded or used a
            skill. Sources retain their scopes, paths and versions; effective
            precedence is unknown.
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
