# Hoshi Paint

A Rust raster editor inspired by the everyday drawing workflow of Paint.NET.
Targets: Windows, macOS, Ubuntu Linux, and eventually iOS/iPadOS.

**Status:** an experimental desktop canvas shell now sits above the UI-independent
document engine and input controller. It supports rectangle preview/adjust/commit,
undo/redo, Shift-constrained scaling, canvas navigation and RGB/HSV/hex/alpha
color selection and history in floating/docked panels, rich color swatches,
Paint.NET-style canvas navigation and a native Color Picker. This is not a completed v0.0.0 release.

Run `cargo run --package hoshi-canvas-spike --locked`. See the
[spike guide](spikes/desktop-canvas/README.md) for platform prerequisites,
controls, validation, and remaining evaluation gates.

## Document management

The desktop spike now supports multiple document tabs, native Open/Save dialogs,
Save As/Save All, recent files, and unsaved-change prompts. Use `.hoshi` to retain
layers, or PNG/JPEG for flat images. See [document management](docs/DOCUMENT_MANAGEMENT.md)
for shortcuts, formats, and remaining Paint.NET parity work.

Canvases support up to **262,144 pixels per dimension** with sparse tiles, viewport
rendering, scanline resize and streaming native/PNG files. Actual dense content is
bounded by available memory; see [large-image support](docs/LARGE_IMAGES.md).

## Development

Install Rust using rustup. The repository pins its toolchain in
`rust-toolchain.toml`; rustup installs the required components automatically.

```sh
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

No services or secrets are required for the application. The graphical spike
uses Slint and platform runtime libraries; the core/controller stay UI-independent.
Use the existing checkout; cloud tasks are already isolated and do not need a
separate worktree.

## Plan

- [Release roadmap](docs/ROADMAP.md)
- [v0.0.0 specification](docs/V0_0_0.md)
- [Architecture decisions](docs/ARCHITECTURE.md)
- [UI framework evaluation and next spike](docs/UI_EVALUATION.md)
- [Issue-ready backlog](docs/BACKLOG.md)
- [Apple-host validation checklist](docs/IOS_SPIKE_CHECKLIST.md)

Native shapes follow **draw → adjust → commit to pixels**. They remain editable
only while active; committed shapes are raster pixels, with one undo entry.

## License

Copyright 2026 Hoshi Paint contributors. Licensed under GNU GPL version 3 only
(`GPL-3.0-only`); see [LICENSE](LICENSE). Third-party code and assets retain their
own terms; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Earlier MIT
distributions retain their existing grants.

Development binary artifacts include the license and a corresponding source
archive with the locked Rust dependencies and build instructions.
