# M4: Session Metadata Protocol — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `.orkworks/` metadata protocol: directory structure, per-session JSON read/write, file watching, event logging, workspace picker, and right sidebar with grouped session cards.

**Architecture:** New `metadata.rs` and `watcher.rs` modules in orkworksd handle JSON/ndjson I/O and file watching. A `POST /workspace` endpoint accepts a repo path, creates `.orkworks/` dirs, and restarts the sidecar in that directory. The Electron main process exposes a folder picker via IPC. Frontend gains a workspace header in the left sidebar and a rewritten right sidebar showing grouped session cards with metadata badges.

**Tech Stack:** Rust (axum, notify, serde), Electron (dialog), React/TypeScript, xterm.js

---

## File Structure

| Action | Path | Responsibility |
|--------|------|---------------|
| Create | `crates/orkworksd/src/metadata.rs` | Session JSON read/write, event log appending |
| Create | `crates/orkworksd/src/watcher.rs` | `notify` file watcher for `.orkworks/sessions/` |
| Modify | `crates/orkworksd/src/main.rs` | Workspace state, POST /workspace, metadata hooks, watcher init |
| Modify | `crates/orkworksd/Cargo.toml` | Add `notify` dependency |
| Modify | `apps/desktop/electron/main.ts` | `open-workspace` IPC, sidecar restart with cwd |
| Modify | `apps/desktop/electron/preload.ts` | Expose `openWorkspace` to renderer |
| Modify | `apps/desktop/src/api.ts` | Workspace API call, metadata type imports |
| Create | `apps/desktop/src/components/WorkspaceHeader.tsx` | Workspace picker / info in left sidebar |
| Modify | `apps/desktop/src/components/LeftSidebar.tsx` | Add WorkspaceHeader above sessions |
| Rewrite | `apps/desktop/src/components/RightSidebar.tsx` | Grouped session cards with metadata |
| Modify | `apps/desktop/src/App.tsx` | Workspace state, wire RightSidebar props |
| Modify | `apps/desktop/src/App.css` | Workspace header, right sidebar card styles |
| Modify | `apps/desktop/src/components/CenterPanel.tsx` | Disable when no workspace selected |

---

### Task 1: Add `notify` dependency to Cargo.toml

**Files:**
- Modify: `crates/orkworksd/Cargo.toml`

- [ ] **Step 1: Add notify crate**

```toml
notify = { version = "7", features = ["macos_kqueue"] }
```

Add it to the `[dependencies]` section alongside the existing deps.

- [ ] **Step 2: Build to verify**

Run: `cargo build --manifest-path crates/orkworksd/Cargo.toml`
Expected: compiles without errors

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/Cargo.toml crates/orkworksd/Cargo.lock
git commit -m "chore: add notify crate for file watching"
```

---

### Task 2: Create metadata module — session JSON + event log

**Files:**
- Create: `crates/orkworksd/src/metadata.rs`
- Test: inline `#[cfg(test)]` in same file
- Modify: `crates/orkworksd/src/main.rs` — add `mod metadata;`

- [ ] **Step 1: Declare module in main.rs**

At the top of `crates/orkworksd/src/main.rs`, after the existing `use` statements, add:

```rust
mod metadata;
```

- [ ] **Step 2: Write metadata.rs with types and write functions**

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub label: String,
    pub workspace: String,
    pub task: String,
    pub harness: String,
    pub model: String,
    pub cwd: String,
    pub status: String,
    pub phase: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "lastActivity")]
    pub last_activity: String,
    #[serde(rename = "metadataSource")]
    pub metadata_source: String,
    #[serde(rename = "metadataConfidence")]
    pub metadata_confidence: f64,
}

#[derive(Debug, Serialize)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: String,
    pub timestamp: String,
    pub status: String,
}

pub struct MetadataStore {
    root: PathBuf,
}

impl MetadataStore {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    pub fn events_dir(&self) -> PathBuf {
        self.root.join("events")
    }

    pub fn write_session(&self, meta: &SessionMetadata) {
        let dir = self.sessions_dir();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!("{}.json", meta.id));
        if let Ok(json) = serde_json::to_string_pretty(meta) {
            let _ = fs::write(&path, json);
        }
    }

    pub fn read_session(&self, id: &str) -> Option<SessionMetadata> {
        let path = self.sessions_dir().join(format!("{}.json", id));
        let data = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn append_event(&self, id: &str, event: &Event) {
        let dir = self.events_dir();
        let _ = fs::create_dir_all(&dir);
        let path = dir.join(format!("{}.ndjson", id));
        if let Ok(json) = serde_json::to_string(event) {
            if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) {
                let _ = writeln!(file, "{json}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_session() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let meta = SessionMetadata {
            id: "test-1".into(),
            label: "Test".into(),
            workspace: "/tmp".into(),
            task: "".into(),
            harness: "".into(),
            model: "".into(),
            cwd: "/tmp".into(),
            status: "running".into(),
            phase: "implementation".into(),
            created_at: "now".into(),
            last_activity: "now".into(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
        };
        store.write_session(&meta);
        let read = store.read_session("test-1").unwrap();
        assert_eq!(read.status, "running");
    }

    #[test]
    fn append_and_read_events() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.append_event("test-2", &Event {
            event_type: "session.created".into(),
            timestamp: "now".into(),
            status: "creating".into(),
        });
        store.append_event("test-2", &Event {
            event_type: "session.status".into(),
            timestamp: "later".into(),
            status: "running".into(),
        });
        let path = store.events_dir().join("test-2.ndjson");
        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(contents.lines().count(), 2);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: 13 tests pass (11 existing + 2 new)

- [ ] **Step 4: Commit**

```bash
git add crates/orkworksd/src/metadata.rs crates/orkworksd/src/main.rs
git commit -m "feat: add metadata module for session JSON and event logging"
```

---

### Task 3: Create file watcher module

**Files:**
- Create: `crates/orkworksd/src/watcher.rs`
- Modify: `crates/orkworksd/src/main.rs` — add `mod watcher;`

- [ ] **Step 1: Declare module in main.rs**

```rust
mod watcher;
```

- [ ] **Step 2: Write watcher.rs**

```rust
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use tokio::sync::broadcast;

pub struct MetadataWatcher {
    tx: broadcast::Sender<String>,
}

impl MetadataWatcher {
    pub fn start(sessions_dir: &Path) -> Self {
        let (tx, _) = broadcast::channel::<String>(32);
        let tx_clone = tx.clone();
        let dir = sessions_dir.to_path_buf();

        std::thread::spawn(move || {
            let (watcher_tx, watcher_rx) = mpsc::channel::<Result<Event, notify::Error>>();
            let mut watcher = notify::recommended_watcher(move |res| {
                let _ = watcher_tx.send(res);
            })
            .unwrap();

            let _ = watcher.watch(&dir, RecursiveMode::NonRecursive);

            for res in watcher_rx {
                if let Ok(event) = res {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        for path in &event.paths {
                            if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                                let _ = tx_clone.send(name.to_string());
                            }
                        }
                    }
                }
            }
        });

        Self { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }
}
```

- [ ] **Step 3: Build to verify**

Run: `cargo build --manifest-path crates/orkworksd/Cargo.toml`
Expected: compiles

- [ ] **Step 4: Commit**

```bash
git add crates/orkworksd/src/watcher.rs crates/orkworksd/src/main.rs
git commit -m "feat: add file watcher for .orkworks/sessions/"
```

---

### Task 4: Add workspace endpoint and integrate metadata into session lifecycle

**Files:**
- Modify: `crates/orkworksd/src/main.rs` — new workspace state, POST /workspace, metadata hooks in create/delete/handle

- [ ] **Step 1: Add workspace fields to AppState and isodate helper**

Replace the `AppState` struct and add helper:

```rust
use std::path::PathBuf;

struct WorkspaceState {
    path: PathBuf,
    metadata: metadata::MetadataStore,
    #[allow(dead_code)]
    watcher: watcher::MetadataWatcher,
}

struct AppState {
    sessions: Mutex<HashMap<String, SessionHandle>>,
    workspace: Mutex<Option<WorkspaceState>>,
}

fn iso_now() -> String {
    chrono::Utc::now().to_rfc3339()
}
```

But `chrono` isn't in deps. Use a simple manual ISO-8601 or add `chrono` with the `clock` feature. Add to Cargo.toml:

```toml
chrono = { version = "0.4", features = ["clock"] }
```

- [ ] **Step 2: Update main() to init empty workspace**

```rust
let state = Arc::new(AppState {
    sessions: Mutex::new(HashMap::new()),
    workspace: Mutex::new(None),
});
```

- [ ] **Step 3: Add POST /workspace handler**

Add route:
```rust
.route("/workspace", post(set_workspace))
```

Add handler:
```rust
#[derive(Deserialize)]
struct WorkspaceRequest {
    path: String,
}

#[derive(Serialize)]
struct WorkspaceResponse {
    path: String,
    repo_root: Option<String>,
    branch: Option<String>,
    dirty: Option<bool>,
}

async fn set_workspace(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WorkspaceRequest>,
) -> impl IntoResponse {
    let ws_path = PathBuf::from(&req.path);
    if !ws_path.is_dir() {
        return (axum::http::StatusCode::BAD_REQUEST, "not a directory").into_response();
    }

    let orkworks_dir = ws_path.join(".orkworks");
    let _ = std::fs::create_dir_all(orkworks_dir.join("sessions"));
    let _ = std::fs::create_dir_all(orkworks_dir.join("events"));
    let _ = std::fs::create_dir_all(orkworks_dir.join("capacity"));
    let _ = std::fs::create_dir_all(orkworks_dir.join("skills"));

    let store = metadata::MetadataStore::new(&orkworks_dir);
    let watch_dir = orkworks_dir.join("sessions");
    let watcher = watcher::MetadataWatcher::start(&watch_dir);

    let mut ws = state.workspace.lock().unwrap();
    *ws = Some(WorkspaceState {
        path: ws_path.clone(),
        metadata: store,
        watcher,
    });

    // Basic git context
    let repo_root = git_repo_root(&ws_path);
    let branch = repo_root.as_ref().and_then(|r| git_branch(r));
    let dirty = repo_root.as_ref().map(|r| git_dirty(r)).unwrap_or(false);

    Json(WorkspaceResponse {
        path: req.path,
        repo_root,
        branch,
        dirty: Some(dirty),
    })
    .into_response()
}
```

- [ ] **Step 4: Add git helper functions**

```rust
fn git_repo_root(path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["-C", &path.to_string_lossy(), "rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn git_branch(repo_root: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["-C", repo_root, "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn git_dirty(repo_root: &str) -> bool {
    std::process::Command::new("git")
        .args(["-C", repo_root, "diff", "--quiet"])
        .status()
        .map(|s| !s.success())
        .unwrap_or(false)
}
```

- [ ] **Step 5: Hook metadata write into create_session**

After inserting the handle into sessions, write metadata:

```rust
let now = iso_now();
let ws_guard = state.workspace.lock().unwrap();
if let Some(ref ws) = *ws_guard {
    ws.metadata.write_session(&metadata::SessionMetadata {
        id: id.clone(),
        label: info.label.clone(),
        workspace: ws.path.display().to_string(),
        task: String::new(),
        harness: String::new(),
        model: String::new(),
        cwd: info.cwd.clone(),
        status: "creating".into(),
        phase: String::new(),
        created_at: now.clone(),
        last_activity: now.clone(),
        metadata_source: "process".into(),
        metadata_confidence: 1.0,
    });
    ws.metadata.append_event(&id, &metadata::Event {
        event_type: "session.created".into(),
        timestamp: now,
        status: "creating".into(),
    });
}
drop(ws_guard);
```

- [ ] **Step 6: Hook metadata write into delete_session**

After sending kill signal, write event:

```rust
let now = iso_now();
let ws_guard = state.workspace.lock().unwrap();
if let Some(ref ws) = *ws_guard {
    if let Some(meta) = ws.metadata.read_session(&id) {
        let mut meta = meta;
        meta.status = "killed".to_string();
        meta.last_activity = now.clone();
        ws.metadata.write_session(&meta);
    }
    ws.metadata.append_event(&id, &metadata::Event {
        event_type: "session.killed".into(),
        timestamp: now,
        status: "killed".into(),
    });
}
drop(ws_guard);
```

- [ ] **Step 7: Hook metadata write into set_session_status**

After updating `handle.info.status`, also write metadata:

```rust
fn set_session_status(state: &Arc<AppState>, id: &str, status: &str) {
    {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(handle) = sessions.get_mut(id) {
            handle.info.status = status.to_string();
        }
    }
    let now = iso_now();
    let ws_guard = state.workspace.lock().unwrap();
    if let Some(ref ws) = *ws_guard {
        if let Some(mut meta) = ws.metadata.read_session(id) {
            meta.status = status.to_string();
            meta.last_activity = now.clone();
            ws.metadata.write_session(&meta);
        }
        ws.metadata.append_event(id, &metadata::Event {
            event_type: "session.status".into(),
            timestamp: now,
            status: status.to_string(),
        });
    }
}
```

- [ ] **Step 8: Add metadata read in list_sessions to enrich session info**

Add `metadata_source` and `metadata_confidence` fields to `SessionInfo`:

```rust
#[derive(Clone, Debug, Serialize)]
struct SessionInfo {
    id: String,
    label: String,
    status: String,
    cwd: String,
    created_at: String,
    #[serde(rename = "metadataSource")]
    metadata_source: Option<String>,
    #[serde(rename = "metadataConfidence")]
    metadata_confidence: Option<f64>,
}
```

Update `list_sessions` to include metadata fields:

```rust
async fn list_sessions(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let sessions = state.sessions.lock().unwrap();
    let ws_guard = state.workspace.lock().unwrap();
    let infos: Vec<SessionInfo> = sessions.values().map(|h| {
        let source = ws_guard.as_ref()
            .and_then(|ws| ws.metadata.read_session(&h.info.id))
            .map(|m| m.metadata_source);
        let confidence = ws_guard.as_ref()
            .and_then(|ws| ws.metadata.read_session(&h.info.id))
            .map(|m| m.metadata_confidence);
        SessionInfo {
            id: h.info.id.clone(),
            label: h.info.label.clone(),
            status: h.info.status.clone(),
            cwd: h.info.cwd.clone(),
            created_at: h.info.created_at.clone(),
            metadata_source: source,
            metadata_confidence: confidence,
        }
    }).collect();
    drop(ws_guard);
    Json(infos)
}
```

- [ ] **Step 9: Add needed imports to main.rs**

Add to existing imports:
```rust
use serde::Deserialize;
use std::path::Path;
```

- [ ] **Step 10: Build and test**

Run: `cargo build --manifest-path crates/orkworksd/Cargo.toml && cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: builds, 13 tests pass

- [ ] **Step 11: Commit**

```bash
git add crates/orkworksd/src/main.rs crates/orkworksd/Cargo.toml crates/orkworksd/Cargo.lock
git commit -m "feat: add workspace endpoint and metadata hooks to session lifecycle"
```

---

### Task 5: Electron — workspace picker IPC and sidecar restart

**Files:**
- Modify: `apps/desktop/electron/main.ts`
- Modify: `apps/desktop/electron/preload.ts`

- [ ] **Step 1: Add `open-workspace` IPC handler and sidecar restart in main.ts**

Add dialog import at top:
```typescript
import { app, BrowserWindow, dialog, ipcMain } from "electron";
```

Add after the `get-backend-url` handler:
```typescript
let workspacePath: string | null = null;

function restartSidecar(cwd: string): void {
  if (sidecarProcess) {
    sidecarProcess.kill();
    sidecarProcess = null;
  }
  backendPort = null;
  portPromise = new Promise<number>((resolve) => {
    portResolve = resolve;
  });
  sidecarProcess = spawn(getSidecarPath(), [], {
    cwd,
    stdio: ["ignore", "pipe", "pipe"],
  });
  sidecarProcess.stdout?.on("data", (data: Buffer) => {
    const line = data.toString().trim();
    console.log(`[orkworksd] ${line}`);
    const match = line.match(/ORKWORKSD_PORT=(\d+)/);
    if (match) {
      backendPort = parseInt(match[1], 10);
      console.log(`[main] sidecar ready on port ${backendPort}`);
      if (portResolve) {
        portResolve(backendPort);
        portResolve = null;
      }
    }
  });
  sidecarProcess.stderr?.on("data", (data: Buffer) => {
    console.error(`[orkworksd:err] ${data.toString().trim()}`);
  });
  sidecarProcess.on("exit", (code) => {
    console.log(`[main] sidecar exited with code ${code}`);
    sidecarProcess = null;
  });
  killSidecar = () => {
    if (sidecarProcess) {
      sidecarProcess.kill();
      sidecarProcess = null;
    }
  };
}

ipcMain.handle("open-workspace", async () => {
  const result = await dialog.showOpenDialog({
    properties: ["openDirectory"],
    title: "Select Workspace",
  });
  if (result.canceled || result.filePaths.length === 0) return null;
  const path = result.filePaths[0];
  workspacePath = path;

  const port = await portPromise;
  const resp = await fetch(`http://127.0.0.1:${port}/workspace`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path }),
  });
  if (!resp.ok) return null;
  return resp.json();
});
```

- [ ] **Step 2: Expose openWorkspace in preload.ts**

```typescript
import { contextBridge, ipcRenderer } from "electron";

contextBridge.exposeInMainWorld("orkworks", {
  getBackendUrl: (): Promise<string> => ipcRenderer.invoke("get-backend-url"),
  openWorkspace: (): Promise<unknown> => ipcRenderer.invoke("open-workspace"),
});
```

- [ ] **Step 3: TypeCheck**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/electron/main.ts apps/desktop/electron/preload.ts
git commit -m "feat: add workspace picker IPC and sidecar restart"
```

---

### Task 6: Frontend — API types and workspace call

**Files:**
- Modify: `apps/desktop/src/api.ts`

- [ ] **Step 1: Add workspace types and function to api.ts**

```typescript
export interface SessionInfo {
  id: string;
  label: string;
  status: string;
  cwd: string;
  created_at: string;
  metadataSource?: string;
  metadataConfidence?: number;
}

export interface WorkspaceInfo {
  path: string;
  repo_root: string | null;
  branch: string | null;
  dirty: boolean | null;
}

export async function setWorkspace(
  baseUrl: string,
  path: string,
): Promise<WorkspaceInfo> {
  const resp = await fetch(`${baseUrl}/workspace`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path }),
  });
  if (!resp.ok) throw new Error(`set workspace failed: ${resp.status}`);
  return resp.json();
}
```

- [ ] **Step 2: TypeCheck**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/api.ts
git commit -m "feat: add workspace API types and setWorkspace function"
```

---

### Task 7: Frontend — WorkspaceHeader component

**Files:**
- Create: `apps/desktop/src/components/WorkspaceHeader.tsx`

- [ ] **Step 1: Write WorkspaceHeader.tsx**

```typescript
import type { WorkspaceInfo } from "../api";

interface WorkspaceHeaderProps {
  workspace: WorkspaceInfo | null;
  onOpenWorkspace: () => void;
}

function WorkspaceHeader({ workspace, onOpenWorkspace }: WorkspaceHeaderProps) {
  if (!workspace) {
    return (
      <div className="workspace-header workspace-header--empty">
        <div className="workspace-header-title">Workspace</div>
        <button
          className="workspace-open-button"
          type="button"
          onClick={onOpenWorkspace}
        >
          Open Folder
        </button>
      </div>
    );
  }

  const name = workspace.path.split("/").pop() || workspace.path;

  return (
    <div className="workspace-header">
      <div className="workspace-header-title">
        <span>Workspace</span>
        <button
          className="workspace-switch-button"
          type="button"
          onClick={onOpenWorkspace}
          title="Switch workspace"
        >
          &#x21C4;
        </button>
      </div>
      <div className="workspace-info">
        <div className="workspace-name">{name}</div>
        <div className="workspace-path">{workspace.path}</div>
        {workspace.branch && (
          <div className="workspace-git">
            <span>{workspace.branch}</span>
            <span className="workspace-git-sep">&middot;</span>
            <span className={workspace.dirty ? "workspace-dirty" : "workspace-clean"}>
              {workspace.dirty ? "dirty" : "clean"}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

export default WorkspaceHeader;
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/components/WorkspaceHeader.tsx
git commit -m "feat: add WorkspaceHeader component"
```

---

### Task 8: Frontend — Wire WorkspaceHeader into LeftSidebar

**Files:**
- Modify: `apps/desktop/src/components/LeftSidebar.tsx`

- [ ] **Step 1: Update LeftSidebar props and render**

```typescript
import type { SessionInfo, WorkspaceInfo } from "../api";
import WorkspaceHeader from "./WorkspaceHeader";

interface LeftSidebarProps {
  workspace: WorkspaceInfo | null;
  onOpenWorkspace: () => void;
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
  onCreateSession: () => void;
  onKillSession: (id: string) => void;
}

function LeftSidebar({
  workspace,
  onOpenWorkspace,
  sessions,
  activeSessionId,
  onSelectSession,
  onCreateSession,
  onKillSession,
}: LeftSidebarProps) {
  return (
    <>
      {workspace ? (
        <>
          <WorkspaceHeader workspace={workspace} onOpenWorkspace={onOpenWorkspace} />
          <div className="panel-header">
            <span>Sessions</span>
            <button
              className="session-new-button"
              type="button"
              onClick={onCreateSession}
              title="New session"
            >
              +
            </button>
          </div>
          <div className="panel-content">
            {/* existing session list JSX, unchanged */}
            {sessions.length === 0 ? (
              <p className="empty-state">No active sessions</p>
            ) : (
              <ul className="session-list">
                {sessions.map((s) => (
                  <li
                    key={s.id}
                    className={`session-item ${s.id === activeSessionId ? "session-item--active" : ""}`}
                    onClick={() => onSelectSession(s.id)}
                  >
                    <div className="session-item-main">
                      <span
                        className={`session-status session-status--${s.status}`}
                      />
                      <div className="session-item-info">
                        <span className="session-item-label">{s.label}</span>
                        <span className="session-item-meta">
                          {s.status} &middot; {s.cwd.split("/").pop() || s.cwd}
                        </span>
                      </div>
                    </div>
                    <button
                      className="session-kill-button"
                      type="button"
                      title="Kill session"
                      onClick={(e) => {
                        e.stopPropagation();
                        onKillSession(s.id);
                      }}
                    >
                      &times;
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </>
      ) : (
        <WorkspaceHeader workspace={null} onOpenWorkspace={onOpenWorkspace} />
      )}
    </>
  );
}

export default LeftSidebar;
```

- [ ] **Step 2: TypeCheck**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: will have errors from App.tsx (missing new props) — fix in next task

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/components/LeftSidebar.tsx
git commit -m "feat: wire WorkspaceHeader into LeftSidebar"
```

---

### Task 9: Frontend — Rewrite RightSidebar with grouped session cards

**Files:**
- Rewrite: `apps/desktop/src/components/RightSidebar.tsx`

- [ ] **Step 1: Write RightSidebar.tsx**

```typescript
import type { SessionInfo } from "../api";

interface RightSidebarProps {
  sessions: SessionInfo[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
}

const PRIORITY: Record<string, number> = {
  waiting_for_input: 0,
  blocked: 1,
  failed: 2,
  running: 3,
  creating: 4,
  idle: 5,
};

function needsAttention(status: string): boolean {
  return status === "blocked" || status === "failed" || status === "waiting_for_input";
}

function isLive(status: string): boolean {
  return status === "running" || status === "creating";
}

function borderColor(status: string): string {
  if (status === "running" || status === "creating") return "#4ec94e";
  if (status === "blocked" || status === "waiting_for_input") return "#d4d44e";
  if (status === "failed") return "#cc4444";
  return "#666";
}

function sourceColor(source: string | undefined): string {
  if (source === "agent") return "#4ec94e";
  if (source === "peon") return "#57c7ff";
  return "#858585";
}

function sortSessions(list: SessionInfo[]): SessionInfo[] {
  return [...list].sort((a, b) => {
    const pa = PRIORITY[a.status] ?? 9;
    const pb = PRIORITY[b.status] ?? 9;
    return pa - pb;
  });
}

function RightSidebar({ sessions, activeSessionId, onSelectSession }: RightSidebarProps) {
  const sorted = sortSessions(sessions);
  const live = sorted.filter((s) => isLive(s.status));
  const done = sorted.filter((s) => !isLive(s.status));

  if (sessions.length === 0) return null;

  return (
    <div className="overview-list">
      {live.length > 0 && (
        <div className="overview-group">
          <div className="overview-group-header">
            Working &middot; {live.length}
          </div>
          {live.map((s) => (
            <div
              key={s.id}
              className={`overview-card ${s.id === activeSessionId ? "overview-card--active" : ""}`}
              style={{ borderLeftColor: borderColor(s.status) }}
              onClick={() => onSelectSession(s.id)}
            >
              <div className="overview-card-main">
                {needsAttention(s.status) && (
                  <span className="overview-alert" title="Needs attention">&#x26A0;</span>
                )}
                <span className="overview-card-label">{s.label}</span>
              </div>
              <div className="overview-card-meta">
                {s.status}
              </div>
              {s.metadataSource && (
                <span
                  className="overview-card-badge"
                  style={{ background: sourceColor(s.metadataSource) + "22", color: sourceColor(s.metadataSource) }}
                >
                  {s.metadataSource} &middot; {Math.round((s.metadataConfidence ?? 1) * 100)}%
                </span>
              )}
            </div>
          ))}
        </div>
      )}
      {done.length > 0 && (
        <div className="overview-group">
          <div className="overview-group-header overview-group-header--done">
            Done &middot; {done.length}
          </div>
          {done.map((s) => (
            <div
              key={s.id}
              className={`overview-card overview-card--done ${s.id === activeSessionId ? "overview-card--active" : ""}`}
              style={{ borderLeftColor: borderColor(s.status) }}
              onClick={() => onSelectSession(s.id)}
            >
              <div className="overview-card-main">
                <span className="overview-card-label">{s.label}</span>
              </div>
              <div className="overview-card-meta">
                {s.status}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default RightSidebar;
```

- [ ] **Step 2: TypeCheck**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors in RightSidebar (App.tsx may still have errors)

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/components/RightSidebar.tsx
git commit -m "feat: rewrite RightSidebar with grouped session cards and metadata"
```

---

### Task 10: Frontend — CSS for workspace header, overview cards

**Files:**
- Modify: `apps/desktop/src/App.css`

- [ ] **Step 1: Add workspace header and overview card styles**

Append to `apps/desktop/src/App.css`:

```css
.workspace-header {
  padding: 8px 12px;
  border-bottom: 1px solid #3c3c3c;
}

.workspace-header--empty {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.workspace-header-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #999;
}

.workspace-open-button {
  border: 1px solid #4b6b7f;
  border-radius: 4px;
  padding: 5px 10px;
  color: #d8edf7;
  background: #123241;
  font: inherit;
  cursor: pointer;
  font-size: 12px;
}

.workspace-open-button:hover {
  background: #16465b;
}

.workspace-switch-button {
  border: none;
  background: none;
  color: #999;
  font-size: 16px;
  cursor: pointer;
  padding: 0;
  line-height: 1;
}

.workspace-switch-button:hover {
  color: #d4d4d4;
}

.workspace-info {
  margin-top: 6px;
}

.workspace-name {
  color: #d4d4d4;
  font-size: 12px;
  font-weight: 600;
}

.workspace-path {
  color: #858585;
  font-size: 10px;
  margin-top: 2px;
}

.workspace-git {
  margin-top: 4px;
  font-size: 10px;
  color: #858585;
}

.workspace-git-sep {
  margin: 0 4px;
}

.workspace-clean {
  color: #4ec94e;
}

.workspace-dirty {
  color: #d4d44e;
}

.overview-list {
  flex: 1;
  overflow-y: auto;
}

.overview-group {
  padding: 0;
}

.overview-group-header {
  padding: 8px 12px 4px;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: #4ec94e;
}

.overview-group-header--done {
  color: #666;
}

.overview-card {
  margin: 0 8px 4px;
  padding: 6px 10px;
  background: #2d2d2d;
  border-radius: 4px;
  cursor: pointer;
  border-left: 2px solid #4ec94e;
}

.overview-card:hover {
  background: #333;
}

.overview-card--active {
  background: #37373d;
}

.overview-card--done {
  opacity: 0.6;
}

.overview-card-main {
  display: flex;
  align-items: center;
  gap: 6px;
}

.overview-alert {
  font-size: 12px;
  flex-shrink: 0;
}

.overview-card-label {
  color: #d4d4d4;
  font-size: 12px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.overview-card-meta {
  color: #858585;
  font-size: 10px;
  margin-top: 2px;
}

.overview-card-badge {
  display: inline-block;
  margin-top: 4px;
  font-size: 9px;
  padding: 1px 5px;
  border-radius: 3px;
}
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/App.css
git commit -m "feat: add workspace header and overview card styles"
```

---

### Task 11: Frontend — Wire everything together in App.tsx

**Files:**
- Modify: `apps/desktop/src/App.tsx`

- [ ] **Step 1: Add workspace state and handler, update JSX**

```typescript
import { useCallback, useEffect, useState } from "react";
import LeftSidebar from "./components/LeftSidebar";
import CenterPanel from "./components/CenterPanel";
import RightSidebar from "./components/RightSidebar";
import {
  type SessionInfo,
  type WorkspaceInfo,
  createSession,
  listSessions,
  deleteSession,
  setWorkspace,
} from "./api";

declare global {
  interface Window {
    orkworks: {
      getBackendUrl: () => Promise<string>;
      openWorkspace: () => Promise<WorkspaceInfo | null>;
    };
  }
}

function App() {
  const [backendStatus, setBackendStatus] = useState<string>("connecting…");
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);
  const [workspace, setWorkspaceState] = useState<WorkspaceInfo | null>(null);

  // ... (keep existing useEffect for health check, refreshSessions, stateOrder) ...

  const handleOpenWorkspace = useCallback(async () => {
    try {
      const info = await window.orkworks.openWorkspace();
      if (info) {
        setWorkspaceState(info);
        setBackendStatus("connecting…");
        setSessions([]);
        setActiveSessionId(null);
      }
    } catch {
      /* user cancelled */
    }
  }, []);

  return (
    <div className="app-shell">
      <div className="titlebar">
        <span className="titlebar-text">OrkWorks</span>
        <span
          className={`status-badge ${backendStatus === "connected" ? "ok" : "warn"}`}
        >
          {backendStatus}
        </span>
      </div>
      <div className="app-layout">
        <aside className="panel left-sidebar">
          <LeftSidebar
            workspace={workspace}
            onOpenWorkspace={handleOpenWorkspace}
            sessions={sessions}
            activeSessionId={activeSessionId}
            onSelectSession={handleSelectSession}
            onCreateSession={handleCreateSession}
            onKillSession={handleKillSession}
          />
        </aside>
        <main className="panel center-panel">
          <CenterPanel
            backendStatus={backendStatus}
            sessionId={activeSessionId}
          />
        </main>
        <aside className="panel right-sidebar">
          <RightSidebar
            sessions={sessions}
            activeSessionId={activeSessionId}
            onSelectSession={handleSelectSession}
          />
        </aside>
      </div>
    </div>
  );
}

export default App;
```

Keep all existing handlers and effects (checkHealth, refreshSessions, stateOrder, handleCreateSession, handleSelectSession, handleKillSession) unchanged.

- [ ] **Step 2: TypeCheck**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/App.tsx
git commit -m "feat: wire workspace state and RightSidebar into App"
```

---

### Task 12: Integration test — full build and verify

**Files:**
- None (verification only)

- [ ] **Step 1: Build Rust**

Run: `cargo build --manifest-path crates/orkworksd/Cargo.toml && cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: builds, all tests pass

- [ ] **Step 2: TypeCheck frontend**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 3: Start and test manually**

```bash
pkill -f "target/debug/orkworksd" 2>/dev/null
./crates/orkworksd/target/debug/orkworksd > /tmp/m4_test_out.txt 2>&1 &
sleep 1
PORT=$(grep -o 'ORKWORKSD_PORT=[0-9]*' /tmp/m4_test_out.txt | cut -d= -f2)

# Set workspace (uses current dir)
curl -s -X POST "http://127.0.0.1:$PORT/workspace" \
  -H "Content-Type: application/json" \
  -d "{\"path\":\"$(pwd)\"}" | python3 -m json.tool

# Create session
SESSION=$(curl -s -X POST "http://127.0.0.1:$PORT/sessions")
ID=$(echo "$SESSION" | python3 -c "import sys,json; print(json.load(sys.stdin)['id'])")
echo "Session: $ID"

# Check .orkworks/ was created
ls -la .orkworks/sessions/$ID.json && echo "Session JSON exists"
ls -la .orkworks/events/$ID.ndjson && echo "Event log exists"
cat .orkworks/sessions/$ID.json | python3 -m json.tool

# Kill session
curl -s -X DELETE "http://127.0.0.1:$PORT/sessions/$ID"
cat .orkworks/sessions/$ID.json | python3 -c "import sys,json; d=json.load(sys.stdin); print('Status:', d['status'])"

pkill -f "target/debug/orkworksd"
```

Expected: `.orkworks/` dirs created, session JSON written, event log appended, status updated on kill.

- [ ] **Step 4: Commit any fixes**

If manual test reveals issues, fix and commit.

- [ ] **Step 5: Final commit**

```bash
git add -A && git commit -m "feat: M4 session metadata protocol — integration complete"
git push
```
