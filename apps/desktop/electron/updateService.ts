export interface UpdateCandidateIdentity {
  channel: "latest" | "nightly";
  version: string;
  tag: string;
  metadataUrl: string;
  metadataDigest: string;
  payloadDigest: string;
}

export interface UpdateCandidate {
  identity: UpdateCandidateIdentity;
  releaseNotes: string | null;
  publishedAt: string | null;
}

export type UpdateEngineEvent =
  | { type: "update-available"; operationId: number; candidate: UpdateCandidate }
  | { type: "update-not-available"; operationId: number }
  | { type: "download-progress"; operationId: number; percent: number; transferred: number; total: number }
  | { type: "update-downloaded"; operationId: number; candidate: UpdateCandidate }
  | { type: "error"; operation: "check" | "download"; operationId: number; message: string };

export interface UpdateEngine {
  readonly installationUnavailableReason: string | null;
  autoDownload: boolean;
  autoInstallOnAppQuit: boolean;
  allowDowngrade: boolean;
  allowPrerelease: boolean;
  channel: "latest" | "nightly";
  onEvent(listener: (event: UpdateEngineEvent) => void): () => void;
  checkForUpdates(operationId: number): Promise<void>;
  downloadUpdate(operationId: number): Promise<void>;
  quitAndInstall(): Promise<void>;
}

export interface UpdateServiceDependencies {
  isPackaged: boolean;
  platform: NodeJS.Platform;
  currentVersion: string;
  createEngine: () => UpdateEngine;
  now: () => string;
  querySessions: () => Promise<ReadonlyArray<{ lifecycle?: string }>>;
  verifyCandidate: (candidate: UpdateCandidate) => Promise<boolean>;
  confirmInstall: (input: {
    candidate: UpdateCandidate;
    liveSessionCount: number | null;
  }) => Promise<boolean>;
  stopSidecar: (timeoutMs: number) => Promise<void>;
  restartSidecar: () => Promise<void>;
  runInstall?: (install: () => Promise<void>) => Promise<void>;
}

type UpdateChannel = UpdateCandidateIdentity["channel"];
type UpdateProgress = { percent: number; transferred: number; total: number };

export type UpdateStatus =
  | { state: "unavailable"; reason: "development" | "unsupported-platform" | "unsupported-version"; sequence: number }
  | { state: "never-checked"; channel: UpdateChannel; currentVersion: string; sequence: number }
  | { state: "checking"; channel: UpdateChannel; currentVersion: string; sequence: number }
  | { state: "up-to-date"; channel: UpdateChannel; currentVersion: string; checkedAt: string; sequence: number }
  | { state: "available"; currentVersion: string; candidate: UpdateCandidate; sequence: number }
  | { state: "downloading"; currentVersion: string; candidate: UpdateCandidate; progress: UpdateProgress; sequence: number }
  | { state: "downloaded"; currentVersion: string; candidate: UpdateCandidate; sequence: number }
  | { state: "installing"; currentVersion: string; candidate: UpdateCandidate; sequence: number }
  | {
      state: "error";
      currentVersion: string;
      operation: "check" | "download" | "install";
      message: string;
      retryable: true;
      candidate?: UpdateCandidate;
      sequence: number;
    };

type UpdateStatusWithoutSequence = UpdateStatus extends infer Status
  ? Status extends { sequence: number }
    ? Omit<Status, "sequence" | "currentVersion">
    : never
  : never;

export interface UpdateService {
  getStatus(): UpdateStatus;
  check(): Promise<UpdateStatus>;
  download(): Promise<UpdateStatus>;
  requestInstall(): Promise<UpdateStatus>;
  subscribe(listener: (status: UpdateStatus) => void): () => void;
}

const stableVersion = /^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;
const nightlyVersion = /^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)-nightly\.(\d{8})\.([1-9]\d*)\.([1-9]\d*)$/;

export function channelForVersion(version: string): UpdateChannel | null {
  if (stableVersion.test(version)) return "latest";
  const match = nightlyVersion.exec(version);
  if (match) {
    const date = match[1];
    const year = Number(date.slice(0, 4));
    const month = Number(date.slice(4, 6));
    const day = Number(date.slice(6, 8));
    const runId = Number(match[2]);
    const attempt = Number(match[3]);
    const parsedDate = new Date(Date.UTC(year, month - 1, day));
    if (year >= 1000 && year <= 9999
      && Number.isSafeInteger(runId)
      && Number.isSafeInteger(attempt)
      && attempt <= 99
      && parsedDate.getUTCFullYear() === year
      && parsedDate.getUTCMonth() === month - 1
      && parsedDate.getUTCDate() === day) {
      return "nightly";
    }
  }
  return null;
}

function unavailableStatus(reason: "development" | "unsupported-platform" | "unsupported-version"): UpdateService {
  const status: UpdateStatus = { state: "unavailable", reason, sequence: 0 };
  return {
    getStatus: () => status,
    check: () => Promise.resolve(status),
    download: () => Promise.resolve(status),
    requestInstall: () => Promise.resolve(status),
    subscribe(listener) {
      listener(status);
      return () => undefined;
    },
  };
}

function sameCandidate(left: UpdateCandidate, right: UpdateCandidate): boolean {
  return left.identity.channel === right.identity.channel
    && left.identity.version === right.identity.version
    && left.identity.tag === right.identity.tag
    && left.identity.metadataUrl === right.identity.metadataUrl
    && left.identity.metadataDigest === right.identity.metadataDigest
    && left.identity.payloadDigest === right.identity.payloadDigest;
}

export function createUpdateService(dependencies: UpdateServiceDependencies): UpdateService {
  if (!dependencies.isPackaged) return unavailableStatus("development");
  if (dependencies.platform !== "darwin" && dependencies.platform !== "win32") {
    return unavailableStatus("unsupported-platform");
  }

  const channel = channelForVersion(dependencies.currentVersion);
  if (channel === null) return unavailableStatus("unsupported-version");
  const selectedChannel: UpdateChannel = channel;

  const engine = dependencies.createEngine();
  engine.autoDownload = false;
  engine.autoInstallOnAppQuit = false;
  engine.allowDowngrade = false;
  engine.allowPrerelease = selectedChannel === "nightly";
  engine.channel = selectedChannel;

  if (
    engine.autoDownload !== false ||
    engine.autoInstallOnAppQuit !== false ||
    engine.allowDowngrade !== false ||
    engine.allowPrerelease !== (selectedChannel === "nightly") ||
    engine.channel !== selectedChannel
  ) {
    throw new Error("Updater engine rejected the required safety configuration");
  }

  let status: UpdateStatus = {
    state: "never-checked",
    channel: selectedChannel,
    currentVersion: dependencies.currentVersion,
    sequence: 0,
  };
  let candidate: UpdateCandidate | null = null;
  let downloadedCandidate: UpdateCandidate | null = null;
  let operationSequence = 0;
  let activeOperation: { kind: "check" | "download" | "install"; sequence: number } | null = null;
  let checkPromise: Promise<UpdateStatus> | null = null;
  let downloadPromise: Promise<UpdateStatus> | null = null;
  let installPromise: Promise<UpdateStatus> | null = null;
  const listeners = new Set<(nextStatus: UpdateStatus) => void>();

  function publish(nextStatus: UpdateStatusWithoutSequence): UpdateStatus {
    status = {
      ...nextStatus,
      currentVersion: dependencies.currentVersion,
      sequence: status.sequence + 1,
    } as UpdateStatus;
    for (const listener of listeners) listener(status);
    return status;
  }

  function eventMatches(operation: "check" | "download", operationId: number): boolean {
    return activeOperation?.kind === operation && activeOperation.sequence === operationId;
  }

  engine.onEvent((event) => {
    switch (event.type) {
      case "update-available":
        if (!eventMatches("check", event.operationId)) return;
        if (event.candidate.identity.channel !== selectedChannel) return;
        if (channelForVersion(event.candidate.identity.version) !== selectedChannel
          || event.candidate.identity.tag !== `v${event.candidate.identity.version}`) {
          activeOperation = null;
          candidate = null;
          downloadedCandidate = null;
          publish({ state: "error", operation: "check", retryable: true,
            message: "The release version and tag do not match the selected update channel." });
          return;
        }
        candidate = event.candidate;
        downloadedCandidate = null;
        activeOperation = null;
        publish({ state: "available", candidate });
        return;
      case "update-not-available":
        if (!eventMatches("check", event.operationId)) return;
        candidate = null;
        downloadedCandidate = null;
        activeOperation = null;
        publish({
          state: "up-to-date",
          channel: selectedChannel,
          checkedAt: dependencies.now(),
        });
        return;
      case "download-progress":
        if (!eventMatches("download", event.operationId) || candidate === null || status.state !== "downloading") return;
        publish({
          state: "downloading",
          candidate,
          progress: { percent: event.percent, transferred: event.transferred, total: event.total },
        });
        return;
      case "update-downloaded":
        if (!eventMatches("download", event.operationId)) return;
        if (event.candidate.identity.channel !== selectedChannel) return;
        if (candidate === null || !sameCandidate(candidate, event.candidate)) return;
        downloadedCandidate = event.candidate;
        activeOperation = null;
        publish({ state: "downloaded", candidate });
        return;
      case "error":
        if (!eventMatches(event.operation, event.operationId)) return;
        activeOperation = null;
        publish({
          state: "error",
          operation: event.operation,
          message: event.message,
          retryable: true,
          ...(candidate === null ? {} : { candidate }),
        });
    }
  });

  function check(): Promise<UpdateStatus> {
    if (installPromise !== null) return installPromise;
    if (checkPromise !== null) return checkPromise;

    const sequence = ++operationSequence;
    let resolveOperation!: (result: UpdateStatus) => void;
    const operation = new Promise<UpdateStatus>((resolve) => { resolveOperation = resolve; });
    checkPromise = operation;
    activeOperation = { kind: "check", sequence };
    publish({ state: "checking", channel: selectedChannel });
    void (async () => {
      try {
        await engine.checkForUpdates(sequence);
        if (activeOperation?.kind === "check" && activeOperation.sequence === sequence) {
          activeOperation = null;
          candidate = null;
          publish({
            state: "up-to-date",
            channel: selectedChannel,
            checkedAt: dependencies.now(),
          });
        }
      } catch (error) {
        if (activeOperation?.kind === "check" && activeOperation.sequence === sequence) {
          activeOperation = null;
          publish({
            state: "error",
            operation: "check",
            message: error instanceof Error ? error.message : String(error),
            retryable: true,
          });
        }
      } finally {
        if (checkPromise === operation) checkPromise = null;
        resolveOperation(status);
      }
    })();
    return operation;
  }

  function download(): Promise<UpdateStatus> {
    if (installPromise !== null) return installPromise;
    // A refreshed check owns the updater's candidate until it settles.
    if (checkPromise !== null) return checkPromise;
    if (downloadPromise !== null) return downloadPromise;
    if (candidate === null) return Promise.resolve(status);

    const sequence = ++operationSequence;
    let resolveOperation!: (result: UpdateStatus) => void;
    const operation = new Promise<UpdateStatus>((resolve) => { resolveOperation = resolve; });
    downloadPromise = operation;
    activeOperation = { kind: "download", sequence };
    publish({
      state: "downloading",
      candidate,
      progress: { percent: 0, transferred: 0, total: 0 },
    });
    void (async () => {
      try {
        await engine.downloadUpdate(sequence);
        if (activeOperation?.kind === "download" && activeOperation.sequence === sequence) {
          throw new Error("The download finished without a matching verified release. Check for updates and retry.");
        }
      } catch (error) {
        if (activeOperation?.kind === "download" && activeOperation.sequence === sequence) {
          activeOperation = null;
          publish({
            state: "error",
            operation: "download",
            message: error instanceof Error ? error.message : String(error),
            retryable: true,
            candidate,
          });
        }
      } finally {
        if (downloadPromise === operation) downloadPromise = null;
        resolveOperation(status);
      }
    })();
    return operation;
  }

  function requestInstall(): Promise<UpdateStatus> {
    if (installPromise !== null) return installPromise;
    if (dependencies.platform !== "win32" || engine.installationUnavailableReason !== null) {
      const blocked = Promise.resolve().then(() => publish({
        state: "error",
        operation: "install",
        message: dependencies.platform === "darwin"
          ? "macOS installation is unavailable: native verification cannot be completed safely before shutdown. Install a signed release manually."
          : engine.installationUnavailableReason ?? "Installation is unavailable on this platform because signature verification cannot be established.",
        retryable: true,
      }));
      installPromise = blocked;
      void blocked.then(() => { if (installPromise === blocked) installPromise = null; });
      return blocked;
    }
    const retryCandidate = status.state === "error" && status.operation === "install"
      ? status.candidate ?? null
      : null;
    if (
      (status.state !== "downloaded" && retryCandidate === null)
      || candidate === null
      || downloadedCandidate === null
      || (retryCandidate !== null && (
        !sameCandidate(retryCandidate, candidate)
        || !sameCandidate(retryCandidate, downloadedCandidate)
      ))
    ) return Promise.resolve(status);

    const cachedCandidate = candidate;
    const candidateToInstall = downloadedCandidate;
    const sequence = ++operationSequence;
    activeOperation = { kind: "install", sequence };
    const isCurrent = () => activeOperation?.kind === "install" && activeOperation.sequence === sequence;
    const installError = (message: string) => publish({
      state: "error",
      operation: "install",
      message,
      retryable: true,
      candidate: cachedCandidate,
    });
    const operation = Promise.resolve().then(async () => {
      if (!sameCandidate(cachedCandidate, candidateToInstall)) {
        activeOperation = null;
        return installError("The downloaded update no longer matches the selected release. Download it again.");
      }

      try {
        if (!await dependencies.verifyCandidate(candidateToInstall)) {
          activeOperation = null;
          return installError("The downloaded update could not be verified. Download it again.");
        }
      } catch (error) {
        activeOperation = null;
        return installError(`The downloaded update could not be verified: ${error instanceof Error ? error.message : String(error)}`);
      }
      if (!isCurrent()) return status;

      let liveSessionCount: number | null;
      try {
        const sessions = await dependencies.querySessions();
        liveSessionCount = sessions.filter(({ lifecycle }) => lifecycle === "alive").length;
      } catch {
        liveSessionCount = null;
      }
      if (!isCurrent()) return status;

      let confirmed: boolean;
      try {
        confirmed = await dependencies.confirmInstall({ candidate: cachedCandidate, liveSessionCount });
      } catch (error) {
        activeOperation = null;
        return installError(`Update confirmation failed: ${error instanceof Error ? error.message : String(error)}`);
      }
      if (!isCurrent()) return status;
      if (!confirmed) {
        activeOperation = null;
        return publish({ state: "downloaded", candidate: cachedCandidate });
      }

      publish({ state: "installing", candidate: cachedCandidate });
      if (!isCurrent()) return status;
      try {
        const install = async () => {
          if (!isCurrent()) throw new Error("Update installation was superseded");
          await engine.quitAndInstall();
        };
        if (dependencies.runInstall) {
          await dependencies.runInstall(install);
        } else {
          await dependencies.stopSidecar(10_000);
          await install();
        }
        return status;
      } catch (error) {
        let recovered = false;
        if (!dependencies.runInstall) {
          try {
            await dependencies.restartSidecar();
            recovered = true;
          } catch {
            // The error below tells the user how to recover manually.
          }
        }
        activeOperation = null;
        const detail = error instanceof Error ? error.message : String(error);
        return installError(recovered
          ? `Update installation failed: ${detail}. The backend was restarted; retry the installation.`
          : `Update installation failed: ${detail}. Restart OrkWorks to recover.`);
      }
    });
    installPromise = operation;
    void operation.then(
      () => {
        if (installPromise !== operation) return;
        installPromise = null;
        if (activeOperation?.kind === "install" && activeOperation.sequence === sequence) activeOperation = null;
      },
      () => {
        if (installPromise !== operation) return;
        installPromise = null;
        if (activeOperation?.kind === "install" && activeOperation.sequence === sequence) activeOperation = null;
      },
    );
    return operation;
  }

  return {
    getStatus: () => status,
    check,
    download,
    requestInstall,
    subscribe(listener) {
      let lastSequence = -1;
      const deliver = (nextStatus: UpdateStatus) => {
        if (nextStatus.sequence <= lastSequence) return;
        lastSequence = nextStatus.sequence;
        listener(nextStatus);
      };
      listeners.add(deliver);
      deliver(status);
      return () => listeners.delete(deliver);
    },
  };
}
