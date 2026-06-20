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
