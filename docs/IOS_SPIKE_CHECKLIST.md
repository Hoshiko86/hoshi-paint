# Apple-host feasibility gate for the canvas spike

Status: **pending, not executed**. This task runs on Linux. A desktop software
renderer build or screenshot does not validate iOS compilation, launch, touch,
Pencil, signing, accessibility, or lifecycle behavior.

Reference version: Slint **1.18.1**, Rust **1.99.1**. The official Slint iOS path
uses the **winit backend and Skia renderer**. The current desktop spike enables
the software renderer; its manifest must be adapted before the commands below
can serve as an iOS build procedure.

Official references:

- [Released iOS guide](https://github.com/slint-ui/slint/blob/v1.18.1/docs/astro/src/content/docs/guide/platforms/mobile/ios.mdx)
- [Released Xcode Cargo build helper](https://github.com/slint-ui/slint/blob/v1.18.1/scripts/build_for_ios_with_cargo.bash)
- [Released manifest and feature definitions](https://github.com/slint-ui/slint/blob/v1.18.1/api/rs/slint/Cargo.toml)

## Required preparation

Use an Apple Silicon Mac with supported macOS/Xcode and the iOS simulator runtime
installed. Install XcodeGen using its official instructions. Signing/device
installation additionally needs an available development team and iPad. Do not
put signing certificates, credentials, or provisioning profiles in this repository.

Before the first iOS build, split the spike's Slint dependency into target-specific
tables: retain `renderer-software` on desktop, and enable `renderer-skia` on
`cfg(target_os = "ios")`. Keep `std`, `backend-winit`, `accessibility`,
`compat-1-18` and the exact version `=1.18.1` on the appropriate targets. Do not
use `renderer-skia-opengl`: the official guide excludes it for iOS. Regenerate
the lockfile and rerun desktop/core checks after this intentional manifest change.
This dependency adaptation has **not** been implemented by this checklist.

In a checkout of Hoshi Paint on the Mac:

```sh
xcodebuild -version
xcrun --sdk iphonesimulator --show-sdk-path
xcrun --sdk iphoneos --show-sdk-path
rustup toolchain install 1.99.1 --profile minimal --component rustfmt --component clippy
rustup target add --toolchain 1.99.1 aarch64-apple-ios-sim aarch64-apple-ios
cargo +1.99.1 test -p hoshi-core -p hoshi-controller --locked
cargo +1.99.1 build -p hoshi-canvas-spike --target aarch64-apple-ios-sim --locked
cargo +1.99.1 build -p hoshi-canvas-spike --target aarch64-apple-ios --locked
```

Record the exact Mac, macOS/Xcode versions, installed SDK/runtime versions,
Cargo output and commit SHA. A successful binary build alone is not an installed
or runnable application.

## Simulator app packaging and launch

In a dedicated evaluation directory inside the checkout, prepare `project.yml`
and the Cargo build helper from the released official guide. Keep its copyright
and license notices. Adapt the helper invocation to the workspace package and
binary; the guide's `slint-rust-template` example name must not remain unchanged.
Use a distinct evaluation bundle identifier, for example
`dev.hoshipaint.HoshiCanvasSpike`, and an Xcode target/scheme `HoshiCanvasSpike`.

Required XcodeGen settings follow the released guide: application target,
platform iOS, supported deployment target, `ENABLE_USER_SCRIPT_SANDBOXING: NO`,
generated Info.plist, `CADisableMinimumFrameDurationOnPhone: true`, an empty
launch screen, and the scene manifest using `SlintWindowSceneDelegate`. The
official guide notes that the scene lifecycle entry is required on iOS 27.

The post-compile script should run from the checkout root and invoke the
released helper with these arguments:

```sh
bash path/to/build_for_ios_with_cargo.bash hoshi-canvas-spike --package hoshi-canvas-spike --locked
```

The exact `path/to` depends on the evaluation directory chosen above. The helper
uses Xcode's build variables to choose simulator/device target, places the Cargo
binary at the application executable path, and generates the release dSYM.
Verify the resulting app's executable and Info.plist before launch.

Once those reviewed packaging files exist:

```sh
xcodegen generate --spec project.yml
xcodebuild -project HoshiCanvasSpike.xcodeproj -scheme HoshiCanvasSpike \
  -configuration Debug -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath build CODE_SIGNING_ALLOWED=NO build
xcrun simctl list devices available
```

Choose an actual available simulator UDID from that output:

```sh
SIMULATOR_UDID='replace-with-an-available-simulator-UDID'
xcrun simctl boot "$SIMULATOR_UDID"
xcrun simctl bootstatus "$SIMULATOR_UDID" -b
xcrun simctl install "$SIMULATOR_UDID" build/Build/Products/Debug-iphonesimulator/HoshiCanvasSpike.app
xcrun simctl launch "$SIMULATOR_UDID" dev.hoshipaint.HoshiCanvasSpike
xcrun simctl io "$SIMULATOR_UDID" screenshot simulator-canvas.png
```

If the simulator is already booted, skip `simctl boot`. Save build/launch output
and a screenshot. The desktop layout currently has a large minimum window size;
record whether controls fit on iPad rather than silently declaring the interface
usable. A responsive mobile layout is a later deliverable.

## Physical iPad

Open the generated Xcode project, choose an authorized development team in
Signing & Capabilities, choose a connected iPad, and build/run using Xcode.
For reproducible command-line evidence after that configuration:

```sh
xcrun devicectl list devices
IPAD_UDID='replace-with-the-connected-iPad-UDID'
xcodebuild -project HoshiCanvasSpike.xcodeproj -scheme HoshiCanvasSpike \
  -configuration Debug -destination "id=$IPAD_UDID" -derivedDataPath build build
xcrun devicectl device install app --device "$IPAD_UDID" \
  build/Build/Products/Debug-iphoneos/HoshiCanvasSpike.app
xcrun devicectl device process launch --device "$IPAD_UDID" \
  dev.hoshipaint.HoshiCanvasSpike
```

Availability and command options depend on the installed Xcode; retain
`xcrun devicectl help` output if the supported syntax differs. Do not use signing
bypass techniques to turn an unsuccessful device install into a passing result.

## Required recorded outcomes

- [ ] Simulator builds, installs and launches with the intended Skia renderer.
- [ ] Device builds, signs, installs and launches on the named iPad/iOS version.
- [ ] Touch coordinates map correctly through canvas zoom/pan and safe areas.
- [ ] Rectangle drag, editable release, commit/cancel, undo/redo match core pixels.
- [ ] Gesture cancellation and background/resume never leave a stuck drag.
- [ ] Pencil position, pressure and cancellation are observed on real hardware;
      simulator mouse input is not evidence of Pencil support.
- [ ] Hardware keyboard and text composition do not leak edit shortcuts into text.
- [ ] VoiceOver exposes tools/settings/layer controls and a useful canvas label.
- [ ] Record renderer/backend, memory and latency measurements on named hardware.
- [ ] Separate unsupported features, failed checks and unavailable equipment from
      passed checks. Keep ARCH-1 open until required evidence or an explicit scope
      decision is recorded.
