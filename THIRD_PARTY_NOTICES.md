# Third-party notices for the canvas spike

Hoshi Paint's own source is licensed under **GNU GPL version 3 only**
(`GPL-3.0-only`), copyright 2026 Hoshi Paint contributors. See [LICENSE](LICENSE).
Dependency and asset licenses apply to their respective code and resources.

The experimental desktop shell selects the **GPL-3.0-only** option offered by
**Slint 1.18.1**. The Help → About Hoshi Paint screen retains `AboutSlint` for
attribution. Slint's alternative royalty-free and commercial terms are not the
selected basis for current distributions.

Earlier Hoshi Paint versions were distributed under MIT. Their existing grants
remain valid; the original notice is preserved in
[licenses/Hoshi-Paint-legacy-MIT.txt](licenses/Hoshi-Paint-legacy-MIT.txt).

Cargo.lock records transitive dependencies and versions. A complete distributable
license inventory remains a release gate; this spike is not a packaged release.
Fonts use the operating system's installed fonts; no font files are copied into
the repository. The framework's bundled resources retain their original notices.

## Lucide

UI outline icons are from **Lucide 1.71.0**, pinned to upstream commit
`3fef7aca71a50acc54bd2f24f0cc4747c22aa6d2`:
https://github.com/lucide-icons/lucide/tree/3fef7aca71a50acc54bd2f24f0cc4747c22aa6d2/icons .
Lucide uses ISC; icons derived from Feather retain MIT attribution. The complete
upstream license, including both notices, is bundled in
[licenses/Lucide-1.71.0.txt](licenses/Lucide-1.71.0.txt).

Button SVGs retain Lucide's 24px viewport, rounded joins/caps and 2px outline;
`currentColor` is replaced with Hoshi Paint's dark foreground for static SVG rendering.
The pipette cursor is adapted to a 32px viewport with a white contrast halo and
hotspot (3,29). The Windows `.cur` is a rasterized version of that SVG.
See [the icon asset guide](spikes/desktop-canvas/icons/README.md).

The swap-colors, reset-colors and rotate-shape-cursor SVGs are original Hoshi Paint
artwork covered by GPL-3.0-only.

## Image files and native dialogs

The desktop adapter and development examples use **image 0.25.10** for image codecs and screenshot encoding, selecting its
MIT license option (upstream offers MIT OR Apache-2.0). Its source and upstream
license texts are included in the matching source archive under `vendor/image-0.25.10/`.
The raster core implements resampling directly and does not depend on this library.

The desktop adapter uses **png 0.18.1** (MIT OR Apache-2.0; selecting MIT),
**rfd 0.17.2** (MIT) for native file dialogs, **tempfile 3.27.0**
(MIT OR Apache-2.0; selecting MIT), and **serde_json 1.0.154**
(MIT OR Apache-2.0; selecting MIT). Project checksums use **crc32fast 1.5.3**
(MIT OR Apache-2.0; selecting MIT). Their sources and license texts, together
with locked transitive dependencies, are included under `vendor/` in the matching
source archive.

**sysinfo 0.37.2** (MIT) supplies cross-platform available-memory information for
the desktop memory policy. Its source and license are in the matching vendored archive.

Disk-backed undo uses **lz4_flex 0.11.6** (MIT) to compress private history records.
The core also uses the existing **tempfile 3.27.0** dependency (selecting MIT) for
session scratch files. Their sources and license texts are included in the matching
vendored source archive.
