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

## Font Awesome Free

The eye-dropper, rotate-left (Undo), and rotate-right (Redo) SVGs are from **Font Awesome Free 6.7.2**, copyright 2024 Fonticons,
Inc., https://fontawesome.com. SVG icons are licensed **CC BY 4.0**:
https://creativecommons.org/licenses/by/4.0/ . The upstream notice is included at
[licenses/Font-Awesome-Free-6.7.2.txt](licenses/Font-Awesome-Free-6.7.2.txt).
Original sources: https://github.com/FortAwesome/Font-Awesome/tree/6.7.2/svgs/solid/ (eye-dropper.svg, rotate-left.svg, rotate-right.svg). The latter two are bundled unchanged as undo.svg and redo.svg.

The cursor derivative adds a 32px viewport and a dark fill/white outline for
contrast; path geometry is unchanged. Attribution comments remain in both SVGs. The Windows `.cur` is a rasterized
32×32 derivative with the same tip hotspot (3,29).
Only Free SVGs are bundled, not Pro assets or font files. Hoshi Paint source
is GPL-3.0-only; the icon retains its own CC BY 4.0 terms.

The swap-colors, reset-colors and rotate-shape-cursor SVGs are original Hoshi Paint artwork covered
by GPL-3.0-only.

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
