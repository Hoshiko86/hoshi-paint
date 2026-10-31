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
