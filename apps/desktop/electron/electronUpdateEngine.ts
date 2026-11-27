import { createHash } from "crypto";
import type { AppUpdater, NsisUpdater, UpdateInfo } from "electron-updater";
import {
  channelForVersion,
  type UpdateCandidate,
  type UpdateEngine,
  type UpdateEngineEvent,
  type UpdateService,
} from "./updateService.ts";

export type ElectronUpdateInfo = UpdateInfo & {
  tag?: string;
  downloadedFile?: string;
};

export interface UpdateIpcSender {
  readonly id: number;
  send(channel: string, payload: unknown): void;
  once(event: "destroyed", listener: () => void): void;
}

export interface UpdateIpcRegistrar {
  handle(channel: string, listener: (...args: any[]) => unknown): void;
  on(channel: string, listener: (event: { sender: UpdateIpcSender }) => void): void;
}

export function updateCandidate(
  info: ElectronUpdateInfo,
  channel: "latest" | "nightly",
  metadataUrl: string | null,
  platform: NodeJS.Platform = process.platform,
): UpdateCandidate {
  const tag = info.tag;
  const metadataFile = platform === "darwin" ? `${channel}-mac.yml` : `${channel}.yml`;
  if (
    channelForVersion(info.version) !== channel
    || tag !== `v${info.version}`
    || metadataUrl !== `https://github.com/Rambolarsen/orkworks/releases/download/${encodeURIComponent(tag)}/${metadataFile}`
  ) {
    throw new Error("Update metadata does not match the requested channel, version, and release tag.");
  }
  const payload = info.files?.find(({ url }) => url.endsWith(platform === "darwin" ? ".zip" : ".exe"));
  if (!payload || !/^[A-Za-z0-9+/]{86}==$/.test(payload.sha512)) {
    throw new Error("Update metadata is missing a valid platform payload checksum.");
  }
  // UpdateDownloadedEvent extends UpdateInfo with a local completion path.
  // Hash the same release metadata at check, completion, and revalidation.
  const { downloadedFile: _downloadedFile, ...releaseMetadata } = info;
  const metadataDigest = createHash("sha256").update(JSON.stringify(releaseMetadata)).digest("hex");
  const releaseNotes = Array.isArray(info.releaseNotes)
    ? info.releaseNotes.map(({ note }) => note).filter((note): note is string => note !== null).join("\n\n") || null
    : info.releaseNotes ?? null;
  return {
    identity: {
      channel,
      version: info.version,
      tag,
      metadataUrl,
      metadataDigest: `sha256:${metadataDigest}`,
      payloadDigest: `sha512:${payload.sha512}`,
    },
    releaseNotes,
    publishedAt: info.releaseDate || null,
  };
}

export function sameUpdateCandidate(left: UpdateCandidate, right: UpdateCandidate): boolean {
  return left.identity.channel === right.identity.channel
    && left.identity.version === right.identity.version
    && left.identity.tag === right.identity.tag
    && left.identity.metadataUrl === right.identity.metadataUrl
    && left.identity.metadataDigest === right.identity.metadataDigest
    && left.identity.payloadDigest === right.identity.payloadDigest;
}

export function createElectronUpdateEngine(
  autoUpdater: AppUpdater,
  options: { platform?: NodeJS.Platform } = {},
): {
  engine: UpdateEngine;
  verifyCandidate(candidate: UpdateCandidate): Promise<boolean>;
} {
  const platform = options.platform ?? process.platform;
  const updaterSettings = {
    autoDownload: false,
    autoInstallOnAppQuit: false,
    allowDowngrade: false,
  };
  Object.assign(autoUpdater, updaterSettings);
  autoUpdater.setFeedURL({ provider: "github", owner: "Rambolarsen", repo: "orkworks" });
  autoUpdater.requestHeaders = { "Cache-Control": "no-cache" };
  autoUpdater.disableWebInstaller = true;

  const listeners = new Set<(event: UpdateEngineEvent) => void>();
  let channel: "latest" | "nightly" = "latest";
  let downloadedCandidate: UpdateCandidate | null = null;
  let metadataUrl: string | null = null;
  let signatureVerified = false;
  // Observe the public NSIS verifier without replacing or bypassing its verdict.
  // Cached downloads that skip it cannot establish verification in this process.
  const windowsUpdater = autoUpdater as AppUpdater & Partial<Pick<NsisUpdater, "verifyUpdateCodeSignature">>;
  if (platform === "win32" && typeof windowsUpdater.verifyUpdateCodeSignature === "function") {
    let verifyingPublishers: string[] | null = null;
    let verificationWarning = false;
    const logger = autoUpdater.logger;
    autoUpdater.logger = {
      info: (message) => logger?.info(message),
      error: (message) => logger?.error(message),
      debug: (message) => logger?.debug?.(message),
      warn: (message) => {
        // The release pipeline pins the certificate SimpleName (CN). This exact
        // pinned-verifier message means a successful match, not skipped checks.
        if (verifyingPublishers !== null && !verifyingPublishers.some((publisher) => message ===
          `Signature validated using only CN ${publisher}. Please add your full Distinguished Name (DN) to publisherNames configuration`)) {
          verificationWarning = true;
        }
        logger?.warn(message);
      },
    };
    const verify = windowsUpdater.verifyUpdateCodeSignature.bind(autoUpdater);
    windowsUpdater.verifyUpdateCodeSignature = async (publishers, file) => {
      signatureVerified = false;
      if (!publishers.length || publishers.some((publisher) => !publisher.trim())) {
        return "Update signature verification requires an expected publisher.";
      }
      verificationWarning = false;
      verifyingPublishers = publishers;
      try {
        const failure = await verify(publishers, file);
        // The pinned verifier can warn and return null after skipping validation
        // (e.g. unsupported PowerShell or missing path). Only the explicit
        // successful SimpleName message above is exempt from failing closed.
        signatureVerified = failure === null && !verificationWarning;
        return failure ?? (signatureVerified ? null : "Windows signature verification could not be established without warnings.");
      } finally {
        verifyingPublishers = null;
      }
    };
  }
  // GitHubProvider falls back to latest metadata on *any* prerelease metadata
  // error. Cancel that request in the updater's own public Electron session.
  autoUpdater.netSession.webRequest.onBeforeRequest({
    urls: ["https://github.com/Rambolarsen/orkworks/releases/download/*"],
  }, ({ url }, callback) => {
    const parsed = new URL(url);
    if (!parsed.pathname.endsWith(".yml")) return callback({});
    const expected = platform === "darwin" ? `${channel}-mac.yml` : `${channel}.yml`;
    const allowed = parsed.pathname.endsWith(`/${expected}`);
    if (allowed) metadataUrl = url;
    callback({ cancel: !allowed });
  });
  let operationQueue: Promise<void> = Promise.resolve();
  const emit = (event: UpdateEngineEvent): void => {
    for (const listener of listeners) listener(event);
  };
  const serialize = <T>(operation: () => Promise<T>): Promise<T> => {
    const result = operationQueue.then(operation, operation);
    operationQueue = result.then(() => undefined, () => undefined);
    return result;
  };

  const engine: UpdateEngine = {
    // NSIS schedules quit before async spawn failure is known and exposes no
    // supported latch reset/retry handshake. Block before any sidecar effects.
    installationUnavailableReason: platform === "win32"
      ? "Windows installation is unavailable: installer failure cannot be recovered safely through the public updater API. Install a signed release manually."
      : "Installation is unavailable: native verification cannot be completed safely before shutdown. Install a signed release manually.",
    get autoDownload() { return autoUpdater.autoDownload; },
    set autoDownload(value) { autoUpdater.autoDownload = value; },
    get autoInstallOnAppQuit() { return autoUpdater.autoInstallOnAppQuit; },
    set autoInstallOnAppQuit(value) { autoUpdater.autoInstallOnAppQuit = value; },
    get allowDowngrade() { return autoUpdater.allowDowngrade; },
    set allowDowngrade(value) { autoUpdater.allowDowngrade = value; },
    get allowPrerelease() { return autoUpdater.allowPrerelease; },
    set allowPrerelease(value) { autoUpdater.allowPrerelease = value; },
    get channel() { return channel; },
    set channel(value) {
      channel = value;
      autoUpdater.channel = value;
      autoUpdater.allowDowngrade = false;
    },
    onEvent(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    checkForUpdates(operationId) {
      return serialize(async () => {
        metadataUrl = null;
        const operation = { id: operationId, channel, finished: false };
        const cleanup = () => {
          autoUpdater.removeListener("update-available", onAvailable);
          autoUpdater.removeListener("update-not-available", onNotAvailable);
          autoUpdater.removeListener("error", onError);
        };
        const finish = (event: UpdateEngineEvent) => {
          if (operation.finished) return;
          operation.finished = true;
          cleanup();
          emit(event);
        };
        const onAvailable = (info: ElectronUpdateInfo) => {
          if (operation.finished) return;
          try {
            finish({
              type: "update-available",
              operationId: operation.id,
              candidate: updateCandidate(info, operation.channel, metadataUrl, platform),
            });
          } catch (error) {
            onError(error instanceof Error ? error : new Error(String(error)));
          }
        };
        const onNotAvailable = () => finish({ type: "update-not-available", operationId: operation.id });
        const onError = (error: Error) => finish({
          type: "error",
          operation: "check",
          operationId: operation.id,
          message: error.message,
        });
        autoUpdater.on("update-available", onAvailable);
        autoUpdater.on("update-not-available", onNotAvailable);
        autoUpdater.on("error", onError);
        try {
          await autoUpdater.checkForUpdates();
        } catch (error) {
          onError(error instanceof Error ? error : new Error(String(error)));
          throw error;
        } finally {
          operation.finished = true;
          cleanup();
        }
      });
    },
    downloadUpdate(operationId) {
      return serialize(async () => {
        downloadedCandidate = null;
        signatureVerified = false;
        let completedCandidate: UpdateCandidate | null = null;
        const operation = { id: operationId, channel, finished: false };
        const cleanup = () => {
          autoUpdater.removeListener("download-progress", onProgress);
          autoUpdater.removeListener("update-downloaded", onDownloaded);
          autoUpdater.removeListener("error", onError);
        };
        const finish = (event: UpdateEngineEvent) => {
          if (operation.finished) return;
          operation.finished = true;
          cleanup();
          emit(event);
        };
        const onProgress = (progress: { percent: number; transferred: number; total: number }) => {
          if (operation.finished) return;
          emit({
            type: "download-progress",
            operationId: operation.id,
            percent: progress.percent,
            transferred: progress.transferred,
            total: progress.total,
          });
        };
        const onDownloaded = (info: ElectronUpdateInfo) => {
          if (!operation.finished) {
            completedCandidate = updateCandidate(info, operation.channel, metadataUrl, platform);
          }
        };
        const onError = (error: Error) => finish({
          type: "error",
          operation: "download",
          operationId: operation.id,
          message: error.message,
        });
        autoUpdater.on("download-progress", onProgress);
        autoUpdater.on("update-downloaded", onDownloaded);
        autoUpdater.on("error", onError);
        try {
          await autoUpdater.downloadUpdate();
          if (!operation.finished && completedCandidate !== null) {
            downloadedCandidate = completedCandidate;
            finish({ type: "update-downloaded", operationId: operation.id, candidate: completedCandidate });
          }
        } catch (error) {
          onError(error instanceof Error ? error : new Error(String(error)));
          throw error;
        } finally {
          operation.finished = true;
          cleanup();
        }
      });
    },
    async quitAndInstall() {
      // Also fail closed if a caller bypasses the service's availability guard.
      throw new Error(engine.installationUnavailableReason!);
    },
  };

  return {
    engine,
    verifyCandidate: (candidate) => serialize(async () => {
      if (platform !== "win32" || !signatureVerified || downloadedCandidate === null
        || !sameUpdateCandidate(candidate, downloadedCandidate)) return false;
      metadataUrl = null;
      const current = await autoUpdater.checkForUpdates();
      return current !== null && sameUpdateCandidate(
        candidate,
        updateCandidate(current.updateInfo, channel, metadataUrl, platform),
      );
    }),
  };
}

export function registerUpdateIpc(service: UpdateService, ipc: UpdateIpcRegistrar): void {
  const updateSubscriptions = new Map<number, () => void>();
  ipc.handle("get-update-status", () => service.getStatus());
  ipc.handle("check-for-updates", () => service.check());
  ipc.handle("download-update", () => service.download());
  ipc.handle("request-update-install", () => service.requestInstall());
  ipc.on("subscribe-update-status", (event) => {
    const senderId = event.sender.id;
    updateSubscriptions.get(senderId)?.();
    const unsubscribe = service.subscribe((status) => event.sender.send("update-status", status));
    updateSubscriptions.set(senderId, unsubscribe);
    event.sender.once("destroyed", () => {
      if (updateSubscriptions.get(senderId) !== unsubscribe) return;
      updateSubscriptions.delete(senderId);
      unsubscribe();
    });
  });
}
