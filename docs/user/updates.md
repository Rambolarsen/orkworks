# Update OrkWorks

Packaged OrkWorks builds check a fixed GitHub release feed for this repository.
Stable builds use stable releases. Nightly builds use only exact `nightly`
prereleases and do not fall back to the stable channel. Development builds do
not install updates and show updates as unavailable.

## Check, download, and install

Open **Settings** and select **Updates**, or choose **Check for updates** from
the application menu. OrkWorks checks only when you ask it to. When an update
is available, downloading is also a manual action.

After the download completes, choose **Restart and install**. OrkWorks asks for
confirmation before installation and may warn that restarting will end live
sessions. It does not install an update automatically when you quit.

Native signature and checksum verification must succeed before OrkWorks starts
the installer. A failed download or installation attempt remains retryable. If
installation fails, OrkWorks attempts to recover its backend; if that recovery
also fails, restart OrkWorks before continuing.

## Release validation

Source and fixture tests do not prove that an installed application can apply
a real update. The [release pipeline specification](/specs/release-pipeline)
defines the credential-backed release validation. Validation with real signed
macOS and Windows artifacts remains required to complete issue #511.
