import { createHash } from "node:crypto";
import {
  readdirSync,
  readFileSync,
  realpathSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { isAbsolute, join, relative, resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";
import yaml from "js-yaml";

const CHECKSUM_MANIFEST = "SHA256SUMS.txt";

function isDistributableFile(name) {
  return name !== CHECKSUM_MANIFEST
    && (name.startsWith("OrkWorks-")
      || /^latest.*\.yml$/.test(name)
      || name.endsWith(".blockmap"));
}

export function writeChecksumManifest({ releaseDir, outputPath }) {
  const entries = readdirSync(releaseDir)
    .filter(isDistributableFile)
    .filter((name) => statSync(join(releaseDir, name)).isFile())
    .sort()
    .map((name) => {
      const digest = createHash("sha256")
        .update(readFileSync(join(releaseDir, name)))
        .digest("hex");
      return `${digest}  ${name}`;
    });

  writeFileSync(outputPath, `${entries.join("\n")}\n`);
  return outputPath;
}

function resolvePayloadPath(releaseDir, url) {
  if (typeof url !== "string" || url.length === 0 || url.includes("\0")) {
    throw new Error("release metadata contains an invalid payload URL");
  }

  const releaseRoot = resolve(releaseDir);
  if (isAbsolute(url)) {
    throw new Error(`release metadata payload URL escapes release directory: ${url}`);
  }

  const payloadPath = resolve(releaseRoot, url);
  const relativePath = relative(releaseRoot, payloadPath);
  if (
    relativePath.length === 0
    || relativePath === ".."
    || relativePath.startsWith(`..${sep}`)
    || isAbsolute(relativePath)
  ) {
    throw new Error(`release metadata payload URL escapes release directory: ${url}`);
  }
  return payloadPath;
}

function requireFile(path, description) {
  let stats;
  try {
    stats = statSync(path);
  } catch (error) {
    throw new Error(`${description}: ${path}`, { cause: error });
  }
  if (!stats.isFile()) {
    throw new Error(`${description} is not a file: ${path}`);
  }
  return stats;
}

function resolveContainedRealPath(realReleaseRoot, path, description) {
  let realPath;
  try {
    realPath = realpathSync(path);
  } catch (error) {
    throw new Error(`${description}: ${path}`, { cause: error });
  }

  const relativePath = relative(realReleaseRoot, realPath);
  if (
    relativePath.length === 0
    || relativePath === ".."
    || relativePath.startsWith(`..${sep}`)
    || isAbsolute(relativePath)
  ) {
    throw new Error(`${description} escapes release directory: ${path}`);
  }
  return realPath;
}

export function verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion }) {
  if (typeof expectedVersion !== "string" || expectedVersion.length === 0) {
    throw new Error("expected release version is invalid");
  }

  const metadata = yaml.load(readFileSync(metadataPath, "utf8"));
  if (!metadata || typeof metadata !== "object" || Array.isArray(metadata)) {
    throw new Error(`release metadata is not an object: ${metadataPath}`);
  }
  if (metadata.version !== expectedVersion) {
    throw new Error(
      `release metadata version mismatch: expected ${expectedVersion}, got ${metadata.version}`,
    );
  }
  if (!Array.isArray(metadata.files) || metadata.files.length === 0) {
    throw new Error(`release metadata has no files: ${metadataPath}`);
  }

  const releaseRoot = resolve(releaseDir);
  const realReleaseRoot = realpathSync(releaseRoot);
  const files = metadata.files.map((entry) => {
    if (!entry || typeof entry !== "object" || Array.isArray(entry)) {
      throw new Error("release metadata contains an invalid files entry");
    }
    if (typeof entry.sha512 !== "string" || entry.sha512.length === 0) {
      throw new Error(`release metadata has an invalid SHA-512 for ${entry.url}`);
    }
    if (!Number.isInteger(entry.size) || entry.size < 0) {
      throw new Error(`release metadata has an invalid size for ${entry.url}`);
    }

    const payloadPath = resolvePayloadPath(releaseDir, entry.url);
    const realPayloadPath = resolveContainedRealPath(
      realReleaseRoot,
      payloadPath,
      "missing payload",
    );
    const payloadStats = requireFile(realPayloadPath, "missing payload");
    if (payloadStats.size !== entry.size) {
      throw new Error(
        `release metadata size mismatch for ${entry.url}: expected ${entry.size}, got ${payloadStats.size}`,
      );
    }

    const digest = createHash("sha512")
      .update(readFileSync(realPayloadPath))
      .digest("base64");
    if (digest !== entry.sha512) {
      throw new Error(`release metadata SHA-512 digest mismatch for ${entry.url}`);
    }

    const blockmapPath = resolveContainedRealPath(
      realReleaseRoot,
      `${payloadPath}.blockmap`,
      "missing blockmap",
    );
    requireFile(blockmapPath, "missing blockmap");
    return entry;
  });

  return { version: metadata.version, files };
}

if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  if (process.argv[2] === "--checksums") {
    const appRoot = resolve(import.meta.dirname, "..");
    const releaseDir = join(appRoot, "release");
    const outputPath = join(releaseDir, CHECKSUM_MANIFEST);
    console.log(writeChecksumManifest({ releaseDir, outputPath }));
  }
}
