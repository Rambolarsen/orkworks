# Native 1.0.90 static inspection packet

This packet contains source/schema selections and identities, not runtime
events or a permission fixture. No Copilot process is started by the inspector.
It does not install, extract to the package cache, import JavaScript, load an
addon or read session logs/settings/credentials. Output is a new disposable
directory chosen by the observer.

## Reproduce the local static comparison

From the repository root, set `ORK740_NATIVE` to the installed native
executable, `ORK740_CACHE` to its existing 1.0.90 distribution cache, and
`ORK740_OUTPUT` to a fresh observer-owned disposable directory. These variables
denote filesystem paths, not credential values. Then run:

```bash
rtk proxy python3 docs/validation/fixtures/copilot-1.0.90-native/inspect-runtime.py \
  --native "$ORK740_NATIVE" \
  --cache "$ORK740_CACHE" \
  --output "$ORK740_OUTPUT"
```

Host-specific input/cwd/output paths in the manifest and package comparison are
replaced by declared observer placeholders. They are normalized provenance,
not a raw argv recording or runtime event. File sizes/digests and source
selections are unchanged. The inspector
refuses a native executable whose digest differs from this packet. All six
`cacheMatches` values must be true to reproduce this selected-file identity.
Absent/mismatched cache files invalidate that comparison; do not silently
replace them, install/update Copilot or infer identity from its version string.
Compare the five generated JSON files to the retained files byte-for-byte.

The parser discovers the SEA Mach-O section, verifies its magic/known flags,
reads length-prefixed loader/assets/exec arguments, and requires no unparsed
bytes. The embedded package is read in memory. It selects seven event roots and
recursively includes every referenced definition; the API selection contains
two tool methods and their complete local definition closure. All references
must resolve within those selections. The JSON selections are reserialized,
with their full source digests recorded in `runtime-identity.json`. The source
excerpts retain UTF-8 byte offsets into the hashed loader/app; these are static
code excerpts rather than synthetic runtime observations.

## Recheck the registry comparison

`package-comparison.json` records the exact public version metadata URLs,
tarball URLs, SHA-512 SRI values, archive SHA-256/size and every outer member's
SHA-256/size. Fetch only those public versions into a disposable directory,
verify each archive's SHA-512 against the recorded SRI **and** its SHA-256/size,
then read members without executing/installing them. Compare the native
`package/copilot` bytes with the installed executable and recorded SHA-256.
No login or inference authentication is needed. The downloaded wrapper is a
registry reference, not the installed 1.0.28 outer wrapper.

Disposable archives/native files exceed the retained-evidence ceiling and are
not committed. Every retained artifact stays below 128 KiB and the packet below
2 MiB. Nothing here certifies a full dependency closure, selected runtime
during a future launch, permission enforcement, content emission or model-loop
continuation. Recheck identities and effective configuration at that launch.
