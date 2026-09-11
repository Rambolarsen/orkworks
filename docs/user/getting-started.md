# Try OrkWorks

Bring one project and one coding tool. OrkWorks gives you a place to run the
session, follow its progress, and see when it needs you.

OrkWorks is in early alpha. These guides follow current source; an older
installer may not include every feature described here.

## Get the app

<DownloadLinks />

The release pipeline targets Apple Silicon macOS and Windows x64. Alpha
installers are unsigned. When a public installer is available, read its notes
on the [releases page](https://github.com/Rambolarsen/orkworks/releases) for
platform-specific installation details. Linux and Intel macOS are
source/local-build paths.

## Run from source

Install Git, Node.js 22, pnpm, and stable Rust with Cargo. Native build tools
are also needed: Xcode Command Line Tools on macOS, the C++ build tools on
Windows, or your Linux distribution’s compiler toolchain.

Install and authenticate at least one [coding tool](/docs/user/coding-tools)
and confirm it works in your normal terminal first.

```bash
git clone https://github.com/Rambolarsen/orkworks.git
cd orkworks/apps/desktop
corepack enable
pnpm install --frozen-lockfile
pnpm dev
```

Use the pnpm version declared by the desktop package. If your Node
installation does not include Corepack, install pnpm using its
[installation guide](https://pnpm.io/installation) first.

The first start builds the Rust backend before opening the desktop app, so
it can take longer than subsequent starts. Keep the development terminal open
while using the app.

## Start your first session

1. Open or add a workspace: a Git repository you want to work in.
2. Open **Settings** and review **Coding tools**. Enable the tool you installed
   and check its detection status.
3. Create a new session, choose the tool and working directory, and give it a
   small task you can verify.
4. Work in the embedded terminal as usual. The sessions list and Details panel
   keep the session’s context nearby.
5. To enable AI observation, configure a model provider and Peon selection in
   Settings. Check that the provider works before relying on its summaries.

Add another session when you have separate work. Select a session to focus
its terminal; switching does not stop the others while the backend is alive.

## Your data and model providers

OrkWorks stores session metadata and recent terminal history locally under
`~/.orkworks/`. App settings are stored in the operating system’s application
data directory.

Local storage does not mean all inference stays local. Your coding tools
communicate with their configured services. When enabled, Peon sends recent
terminal context to its configured model provider, which may be remote or
local. Choose providers appropriate for the data in your workspace.

## If you get stuck

- **Coding tool not detected:** confirm the command works in your terminal,
  then check its command path in Settings.
- **No useful summaries:** check the configured model provider and Peon
  selection. Optional coding-tool integrations can improve direct signals.
- **Source build fails:** check Node, pnpm, Rust, and native build tools first.

For a reproducible problem, [open an issue](https://github.com/Rambolarsen/orkworks/issues)
with your OS, version or commit, and the error. Remove credentials and private
terminal content before sharing logs.

Next: [Follow your sessions](/docs/user/sessions) or
[meet Taskmaster](/docs/user/taskmaster).
