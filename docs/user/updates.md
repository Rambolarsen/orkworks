# Update OrkWorks

Packaged OrkWorks builds check a fixed GitHub release feed for this repository.
Stable builds use stable releases. Nightly builds use only exact `nightly`
prereleases and do not fall back to the stable channel. Development builds do
not install updates and show updates as unavailable.

## Check, download, and install manually

Open **Settings** and select **Updates**, or choose **Check for updates** from
the application menu. OrkWorks checks only when you ask it to. When an update
is available, downloading is also a manual action.

In-app installation is currently unavailable on both Windows and macOS, so
Settings does not offer a restart-and-install action. Download and install the
signed installer manually from the
[OrkWorks releases page](https://github.com/Rambolarsen/orkworks/releases).
Downloading an update in Settings does not install it. OrkWorks never installs
an update automatically when you quit. Failed checks and downloads can be
retried.

On Windows, the updater can schedule app quit before an asynchronous installer
failure is known, and its public API cannot guarantee safe recovery and retry.
On macOS, it cannot prove native verification before stopping the backend while
keeping automatic installation disabled. The native install paths remain
blocked; use a signed release installer instead.

Windows publisher verification follows the release pipeline's exact certificate
`SimpleName` contract. The upstream verifier's successful common-name message is
accepted; mismatch, skipped verification, and missing-path warnings are rejected.
Cached downloads that skip verification cannot establish verification in the
current app process. Signature verification alone does not enable installation.

## Release validation

Source and fixture tests do not prove that an installed application can apply
a real update. The [release pipeline specification](/specs/release-pipeline)
defines the credential-backed release validation. Validation with real signed
macOS and Windows artifacts remains required to complete issue #511.
