import { createHash } from "node:crypto";
import {
  lstatSync,
  readdirSync,
  readFileSync,
  realpathSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";
import yaml from "js-yaml";

const CHECKSUM_MANIFEST = "SHA256SUMS.txt";

function isDistributableFile(name, expectedVersion) {
  if (expectedVersion) {
    return name.startsWith(`OrkWorks-${expectedVersion}-`) || /^latest.*\.yml$/.test(name);
  }
  return name !== CHECKSUM_MANIFEST
    && (name.startsWith("OrkWorks-")
      || /^latest.*\.yml$/.test(name)
      || name.endsWith(".blockmap"));
}

export function writeChecksumManifest({ releaseDir, outputPath, expectedVersion }) {
  const releaseRoot = resolve(releaseDir);
  const realReleaseRoot = realpathSync(releaseRoot);
  const validatedOutputPath = resolveChecksumOutputPath(
    releaseRoot,
    realReleaseRoot,
    outputPath,
  );
  const entries = readdirSync(releaseRoot)
    .filter((name) => isDistributableFile(name, expectedVersion))
    .map((name) => ({
      name,
      path: resolveContainedRealPath(
        realReleaseRoot,
        join(releaseRoot, name),
        "checksum input",
      ),
    }))
    .filter(({ path }) => statSync(path).isFile())
    .sort((left, right) => left.name < right.name ? -1 : left.name > right.name ? 1 : 0)
    .map(({ name, path }) => {
      const digest = createHash("sha256")
        .update(readFileSync(path))
        .digest("hex");
      return `${digest}  ${name}`;
    });

  writeFileSync(validatedOutputPath, `${entries.join("\n")}\n`);
  return outputPath;
}

export function runChecksumCli({
  appRoot = resolve(import.meta.dirname, ".."),
  output = console.log,
} = {}) {
  const packageJson = JSON.parse(readFileSync(join(appRoot, "package.json"), "utf8"));
  const version = packageJson.version;
  if (
    typeof version !== "string"
    || version.length === 0
    || version !== version.trim()
    || /[\\/\0]/.test(version)
  ) {
    throw new Error("package version is invalid for release checksum generation");
  }
  const releaseDir = join(appRoot, "release");
  const outputPath = join(releaseDir, CHECKSUM_MANIFEST);
  const result = writeChecksumManifest({ releaseDir, outputPath, expectedVersion: version });
  output(result);
  return result;
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

function resolveContainedRealPath(realReleaseRoot, path, description, allowRoot = false) {
  let realPath;
  try {
    realPath = realpathSync(path);
  } catch (error) {
    throw new Error(`${description}: ${path}`, { cause: error });
  }

  const relativePath = relative(realReleaseRoot, realPath);
  if (
    (relativePath.length === 0 && !allowRoot)
    || relativePath === ".."
    || relativePath.startsWith(`..${sep}`)
    || isAbsolute(relativePath)
  ) {
    throw new Error(`${description} escapes release directory: ${path}`);
  }
  return realPath;
}

function resolveChecksumOutputPath(releaseRoot, realReleaseRoot, outputPath) {
  if (typeof outputPath !== "string" || outputPath.length === 0 || outputPath.includes("\0")) {
    throw new Error("checksum output path is invalid");
  }

  const resolvedOutputPath = resolve(outputPath);
  const relativePath = relative(releaseRoot, resolvedOutputPath);
  if (
    relativePath.length === 0
    || relativePath === ".."
    || relativePath.startsWith(`..${sep}`)
    || isAbsolute(relativePath)
  ) {
    throw new Error(`checksum output path escapes release directory: ${outputPath}`);
  }

  try {
    lstatSync(resolvedOutputPath);
    return resolveContainedRealPath(realReleaseRoot, resolvedOutputPath, "checksum output path");
  } catch (error) {
    if (error?.code !== "ENOENT") {
      throw error;
    }
    const realParentPath = resolveContainedRealPath(
      realReleaseRoot,
      dirname(resolvedOutputPath),
      "checksum output path",
      true,
    );
    return join(realParentPath, basename(resolvedOutputPath));
  }
}

export function verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion }) {
  if (typeof expectedVersion !== "string" || expectedVersion.length === 0) {
    throw new Error("expected release version is invalid");
  }

  const releaseRoot = resolve(releaseDir);
  const realReleaseRoot = realpathSync(releaseRoot);
  const realMetadataPath = resolveContainedRealPath(
    realReleaseRoot,
    metadataPath,
    "metadata path",
  );
  requireFile(realMetadataPath, "metadata path");
  const metadata = yaml.load(readFileSync(realMetadataPath, "utf8"));
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
    runChecksumCli();
  }
}
