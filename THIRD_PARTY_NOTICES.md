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

The swap-colors and reset-colors SVGs are original Hoshi Paint artwork covered
by GPL-3.0-only.
