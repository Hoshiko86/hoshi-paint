# Hoshi Paint

A Rust raster editor inspired by the everyday drawing workflow of Paint.NET.
Targets: Windows, macOS, Ubuntu Linux, and eventually iOS/iPadOS.

**Status:** implementation has started; there is no graphical editor yet.
The first slice is a UI-independent document engine with layers, compositing,
shape preview/commit/cancel, and undo/redo. This is not a completed v0.0.0 release.

## Development

Install Rust using rustup. The repository pins its toolchain in
`rust-toolchain.toml`; rustup installs the required components automatically.

```sh
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

No services, secrets, or external Rust dependencies are required for this slice.
Use the existing checkout; cloud tasks are already isolated and do not need a
separate worktree.

## Plan

- [Release roadmap](docs/ROADMAP.md)
- [v0.0.0 specification](docs/V0_0_0.md)
- [Architecture decisions](docs/ARCHITECTURE.md)
- [Issue-ready backlog](docs/BACKLOG.md)

Native shapes follow **draw → adjust → commit to pixels**. They remain editable
only while active; committed shapes are raster pixels, with one undo entry.
