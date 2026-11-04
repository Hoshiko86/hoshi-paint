# Third-party notices for the canvas spike

Hoshi Paint's own source remains MIT licensed. Dependency licenses apply to
their respective code; this file does not relicense any dependency.

The experimental desktop shell uses **Slint 1.18.1** under the Slint Royalty-free
Desktop, Mobile, and Web Applications License, version 2.0. The unmodified license
is included at [licenses/Slint-Royalty-free-2.0.md](licenses/Slint-Royalty-free-2.0.md).
The top-level Help → About Hoshi Paint screen displays the toolkit's `AboutSlint`
widget for attribution. Retain it when distributing the spike. The framework's
other license alternatives have not been selected for this experiment.

Cargo.lock records transitive dependencies and versions. A complete distributable
license inventory remains a release gate; this spike is not a packaged release.
Fonts use the operating system's installed fonts; no font files are copied into
the repository. The framework's bundled resources retain their original notices.

## Font Awesome Free

The eye-dropper SVG is from **Font Awesome Free 6.7.2**, copyright 2024 Fonticons,
Inc., https://fontawesome.com. SVG icons are licensed **CC BY 4.0**:
https://creativecommons.org/licenses/by/4.0/ . The upstream notice is included at
[licenses/Font-Awesome-Free-6.7.2.txt](licenses/Font-Awesome-Free-6.7.2.txt).
Original source: https://github.com/FortAwesome/Font-Awesome/tree/6.7.2/svgs/solid/eye-dropper.svg .

The cursor derivative adds a 32px viewport and a dark fill/white outline for
contrast; path geometry is unchanged. Attribution comments remain in both SVGs. The Windows `.cur` is a rasterized
32×32 derivative with the same tip hotspot (3,29).
Only Free SVGs are bundled, not Pro assets or font files. Hoshi Paint source
remains MIT licensed; the icon retains its own CC BY 4.0 terms.
