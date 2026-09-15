export function createReleaseBuildPlan(platform, arch) {
  switch (platform) {
    case "darwin":
      if (arch === "x64") {
        return [
          {
            builderTarget: "mac",
            electronArch: "x64",
            rustTarget: "x86_64-apple-darwin",
            sidecarBinaryName: "orkworksd",
          },
        ];
      }
      if (arch === "arm64") {
        return [
          {
            builderTarget: "mac",
            electronArch: "arm64",
            rustTarget: "aarch64-apple-darwin",
            sidecarBinaryName: "orkworksd",
          },
        ];
      }
      break;
    case "win32":
      return [
        {
          builderTarget: "win",
          electronArch: "x64",
          rustTarget: "x86_64-pc-windows-msvc",
          sidecarBinaryName: "orkworksd.exe",
        },
      ];
    case "linux":
      return [
        {
          builderTarget: "linux",
          electronArch: "x64",
          rustTarget: "x86_64-unknown-linux-gnu",
          sidecarBinaryName: "orkworksd",
        },
      ];
  }

  throw new Error(`Unsupported release platform/arch: ${platform}/${arch}`);
}

export function electronBuilderInvocation(
  nodeExecutable,
  cliPath,
  { builderTarget, electronArch },
  {
    channel = "latest",
    buildVersion,
    macBundleVersion,
  } = {},
) {
  if (channel !== "latest" && channel !== "nightly") {
    throw new Error("release channel must be latest or nightly");
  }
  const args = [cliPath, `--${builderTarget}`, `--${electronArch}`, "--publish", "never"];
  if (channel === "nightly") {
    if (typeof buildVersion !== "string" || buildVersion.length === 0) {
      throw new Error("nightly build version is required");
    }
    if (builderTarget === "mac" && (typeof macBundleVersion !== "string" || macBundleVersion.length === 0)) {
      throw new Error("nightly macOS bundle version is required");
    }
    args.push(
      "--config.publish.channel=nightly",
      `--config.${builderTarget}.publish.channel=nightly`,
      "--config.generateUpdatesFilesForAllChannels=false",
      `--config.buildVersion=${buildVersion}`,
    );
    if (builderTarget === "mac") {
      args.push(`--config.mac.bundleVersion=${macBundleVersion}`);
    }
  }
  return {
    command: nodeExecutable,
    args,
  };
}
