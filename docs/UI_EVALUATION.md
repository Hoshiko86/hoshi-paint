# UI architecture evaluation — ARCH-1

Status: research complete; **production framework decision remains open**.
The subsequent desktop experiment and its actual validation are recorded in
[SPIKE_RESULTS.md](SPIKE_RESULTS.md); statements below describe the research phase.
Evidence reviewed on 2026-10-31. This document recommends the next bounded
evaluation, not an already validated UI stack. No Windows/macOS/iOS build,
GUI input test, accessibility test, or performance measurement was run here.

## Recommendation

Evaluate **Slint first**, with the existing Rust core behind a narrow controller
interface. Its documented Rust iOS route and declarative UI make it a plausible
fit for a desktop editor that later gets a different touch layout. It also has
an official wgpu texture-import example. Those are useful starting points, not
proof that pointer capture, Pencil, accessibility, or high-performance painting
will meet Hoshi Paint's requirements.

Keep **egui + egui-wgpu** as the desktop fallback if Slint's custom canvas or
licensing terms are unsuitable. Do not assume that eframe's desktop portability
automatically supplies an iOS application: its official platform list includes
desktop, web and Android, but does not list iOS. A custom iOS integration would
need separate ownership and evidence.

Do not build four native widget shells for v0.0.0. They offer the strongest access
to platform conventions, but multiply UI implementation and interaction testing
before the editing workflow is established. The core remains reusable if a
native iPad shell becomes justified later.

## Comparison

| Criterion | Slint | egui / eframe | Native shells or lower-level windowing |
| --- | --- | --- | --- |
| Application model | Declarative retained UI, Rust callbacks/models; layout can be distinct from document logic | Immediate-mode Rust UI; convenient tool panels and custom drawing; stable widget identities need care | SwiftUI/AppKit/UIKit, WinUI and GTK require separate UI layers; winit supplies windows/events rather than widgets |
| Desktop targets | Documented Windows/macOS/Linux support; select and test backend/renderer explicitly | Official eframe targets include Windows/macOS/Linux | Native frameworks cover their own OS; winit supports Windows/macOS/X11/Wayland |
| iOS path | Official Rust instructions use macOS, Xcode, Rust targets and XcodeGen; simulator/device app work still required | Not an officially listed eframe target in the reviewed README; winit support alone is insufficient | UIKit/SwiftUI gives direct iOS integration; winit lists iOS, but needs a UI/rendering integration |
| Custom canvas / wgpu | Official texture-import example uses rendering notifications; released example enables `unstable-wgpu-30`, so version coupling is a real maintenance concern | Official egui-wgpu integration and custom-paint callbacks; good fit for GPU canvas composition | Direct wgpu surface gives control; native view embedding, resizing and lifecycle become application work |
| Mouse / keyboard / pen | Must verify pointer capture outside canvas/window, modifiers, focus loss and coordinates; TouchArea capability is not proof of pressure/Pencil delivery | Input and touch integration exist; must verify retained drag behavior and pen/pressure routing through selected backend | Native events offer platform detail but require normalization; winit abstracts common events rather than every platform capability |
| Text / IME | Existing text widgets reduce application work; validate composition, focus and shortcuts on each OS | Text editing is provided; IME and global shortcut interaction still require platform tests | Native text fields offer system integration, at the cost of separate implementations |
| Accessibility | Built-in accessibility machinery exists; custom canvas and tool semantics still need explicit labels/actions and screen-reader testing | Optional AccessKit integration is enabled by default in eframe; a custom canvas is not automatically accessible | Native widgets provide a platform starting point; custom drawing still needs semantic accessibility nodes/actions |
| Menus / dialogs | Confirm native menu integration and file-dialog solution against chosen release/backend | Separate platform integration is needed for native menus/dialogs; UI menus need not be native | Best direct access in native shells; winit itself explicitly does not create native menus |
| High DPI | Verify logical-to-document coordinate conversion during zoom, monitor changes and scaling | Same requirement; do not use screen pixels as document pixels | Same requirement across every shell; native APIs do not eliminate conversion bugs |
| Licensing | Framework is GPLv3 OR royalty-free custom license OR commercial; examples being MIT does not make the framework MIT | MIT OR Apache-2.0; bundled fonts/assets need their own notices | Mixed by chosen components; winit, platform bindings and each third-party dependency need a license inventory |
| Maintenance / build cost | Additional UI language/compiler; renderer choice can add Skia or other backend dependencies | All-Rust UI surface, but breaking library upgrades and integration dependencies remain | Highest application-owned integration work; more OS-specific CI and release expertise |

Tauri is a fourth, web-shell alternative: its official architecture uses Rust
with system WebViews (WebView2, WKWebView, WebKitGTK) across desktop and mobile.
It could provide accessible HTML controls, but canvas input, GPU interoperability,
WebView variation and the JavaScript/Rust boundary need a separate spike. Do not
stream complete image buffers or individual pointer events through serialized
IPC as an assumed high-performance architecture. This is not the first choice
for the requested Rust-centered editor; it remains an option if web UI expertise
or distribution constraints change the priorities.

## Version and licensing gates

The repository at the research baseline pinned Rust **1.85.1**. The released workspace manifests
reviewed here declare:

| Candidate | Reviewed release | Declared Rust minimum |
| --- | --- | --- |
| Slint | 1.18.1 | 1.92 |
| egui / eframe | 0.37.0 | 1.95 |

Neither can simply be added to the current toolchain. The spike must explicitly
choose a supported released version and compatible pinned Rust, regenerate the
lockfile, and run the entire core suite before adopting the change. Investigate
transitive dependency minimums too. Do not silently use an older toolkit just
to preserve the initial pin, or follow a development branch in production.

Slint's reviewed license summary offers GPLv3, a royalty-free license with
conditions including disclosure, or a commercial license. The application
currently carries an MIT license; an MIT application license does not erase
dependency obligations. Before adopting Slint, record which framework license
will be used, its disclosure/distribution requirements, and the planned release
notices. This research is not a legal conclusion or a license selection.

## Next deliverable: bounded canvas/integration spike

Keep this separate from the final product UI. Use the existing core through a
controller, and make the evaluation easy to remove or replace. Deliver a source
directory, exact dependency/toolchain pins, build/run commands, recorded results,
and a short accept/reject decision against these criteria:

1. **Canvas boundary:** display a 512×512 core composite with checkerboard; zoom
   around the pointer and pan without modifying the document. Separate document
   pixels, logical window coordinates and device pixels.
2. **Shape lifecycle:** drag a rectangle preview; release, adjust, Enter to commit,
   Escape to cancel; undo/redo through the controller. Pixel output must match
   the core reference. Check dragging outside the canvas and releasing outside
   the window, focus loss, and switching tools. Record actual behavior.
3. **Settings and text:** keyboard-operable tool settings and layer-name editing;
   IME composition must not accidentally trigger global edit shortcuts. A modal
   dialog must not leak drawing input to the canvas.
4. **Rendering integration:** demonstrate the supported texture-upload route first;
   separately test the proposed wgpu texture path. Record copies/uploads per frame,
   renderer/backend, supported feature names, resize/device-loss behavior, and
   upgrade coupling. No claim of zero-copy or acceleration without measurement.
5. **Desktop matrix:** compile and run on Windows, macOS and Ubuntu, including
   Ubuntu X11/Wayland where available. Record OS/hardware and scaling; test mixed
   DPI monitor transitions and pointer-anchored zoom. Linux-only success is not
   sufficient evidence for all desktop targets.
6. **Accessibility:** keyboard traversal, visible focus, labeled tool/settings/layer
   controls, and an accessible canvas description. Test NVDA on Windows,
   VoiceOver on macOS, and Orca on Ubuntu; document unsupported behavior.
7. **iOS gate on an Apple host:** follow the chosen release's Xcode instructions,
   build a simulator app and install/run it. Then build/sign/run on an iPad when
   hardware and signing are available; inspect touch coordinates, Pencil pressure
   and event delivery, cancellation, safe areas, and background/resume. Simulator
   launch does not establish Pencil support. Credentials belong in secure platform
   settings; no signing material goes in the repository.
8. **Performance:** measure release-build input-to-preview latency and idle CPU on
   named hardware using the small supported core document first. The current
   core cap prevents the planned 2048×2048 workload; do not bypass that cap to
   claim the final v0.0.0 performance gate has passed.

If Apple tooling is unavailable, deliver the desktop findings and a precise
pending Apple-host validation task. ARCH-1 remains incomplete until the required
iOS evidence or an explicit scope decision is recorded. If Slint fails a must-have
desktop criterion, run the equivalent bounded egui evaluation; if it fails iOS
requirements, compare a shared engine with a native iPad shell before choosing.

## Evidence and official references

These are documentation/source observations, not executed application checks.
Released tags are used for version-sensitive evidence; other source revisions
below identify the reviewed documentation snapshot.

- [Slint 1.18.1 workspace manifest](https://github.com/slint-ui/slint/blob/v1.18.1/Cargo.toml)
  — release version and Rust minimum.
- [Slint 1.18.1 license summary](https://github.com/slint-ui/slint/blob/v1.18.1/LICENSE.md)
  — framework alternatives and disclosure summary; follow its links for full terms.
- [Slint 1.18.1 iOS platform guide](https://github.com/slint-ui/slint/blob/v1.18.1/docs/astro/src/content/docs/guide/platforms/mobile/ios.mdx)
  — macOS/Xcode/Rust/XcodeGen workflow. The same guide was read at revision
  `8d7072cbfb7b8ed5f38913c440dad9249384cb1d`; recheck the release guide when implementing.
- [Slint 1.18.1 wgpu example manifest](https://github.com/slint-ui/slint/blob/v1.18.1/examples/wgpu_texture/Cargo.toml)
  and [example overview](https://github.com/slint-ui/slint/blob/v1.18.1/examples/wgpu_texture/README.md)
  — texture import, rendering notifier and unstable wgpu feature.
- [egui README](https://github.com/emilk/egui/blob/fc9d66dda742d722eedaa8684d23f86369cf8e5f/README.md)
  — supported eframe platforms, integration model, accessibility and licensing;
  native appearance is explicitly a non-goal.
- [egui 0.37.0 workspace manifest](https://github.com/emilk/egui/blob/0.37.0/Cargo.toml)
  — Rust minimum and dependency versions.
- [eframe manifest](https://github.com/emilk/egui/blob/fc9d66dda742d722eedaa8684d23f86369cf8e5f/crates/eframe/Cargo.toml)
  — AccessKit and renderer features.
- [winit scope](https://github.com/rust-windowing/winit/blob/db098642637c7e073327c298831678f46a1bb5ed/FEATURES.md)
  — platform list and explicit exclusion of drawing/native menus.
- [Tauri README](https://github.com/tauri-apps/tauri/blob/31f29e197e7ac18797dbb9270fe658c1ca1010c8/README.md)
  — system WebView architecture and desktop/mobile platforms.

No framework was added, no toolchain pin was changed, and no production UI
decision is implied by this document.
