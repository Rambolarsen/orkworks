const repository = "https://github.com/Rambolarsen/orkworks";
export async function fetchPublicRelease(fetcher = fetch) {
  const response = await fetcher("https://api.github.com/repos/Rambolarsen/orkworks/releases/latest", {
    headers: { Accept: "application/vnd.github+json" },
    signal: AbortSignal.timeout(15000),
  });
  if (response.status === 404) return null;
  if (!response.ok) throw new Error(`Public release lookup failed: HTTP ${response.status}`);
  return publicRelease([await response.json()]);
}
export function codingTools(registry) {
  return registry.builtins
    .filter((tool) => !tool.retired && tool.launch.kind === "command-template")
    .map(({ id, name }) => ({ id, name }));
}
export function publicRelease(releases) {
  const release = releases
    .filter(
      (item) =>
        !item.draft &&
        !item.prerelease &&
        item.published_at &&
        item.html_url?.startsWith(`${repository}/releases/tag/`),
    )
    .sort((a, b) => Date.parse(b.published_at) - Date.parse(a.published_at))[0];
  if (!release) return null;
  const platforms = [
    [/-mac-arm64\.dmg$/, "macOS · Apple Silicon"],
    [/-win-x64\.exe$/, "Windows · x64"],
  ];
  return {
    version: release.tag_name,
    url: release.html_url,
    downloads: release.assets.flatMap((asset) => {
      const platform = platforms.find(([pattern]) => pattern.test(asset.name));
      return platform &&
        asset.browser_download_url.startsWith(
          `${repository}/releases/download/`,
        )
        ? [{ label: platform[1], url: asset.browser_download_url }]
        : [];
    }),
  };
}
