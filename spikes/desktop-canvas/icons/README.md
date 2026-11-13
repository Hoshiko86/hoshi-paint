# Icon assets

Lucide 1.71.0 is pinned to commit `3fef7aca71a50acc54bd2f24f0cc4747c22aa6d2`.
The standard Lucide filenames identify each upstream source in its `icons/`
directory. The complete ISC/Feather MIT notices are in
`licenses/Lucide-1.71.0.txt` at the repository root.

Button icons use Lucide's 24×24 viewBox, 2px stroke, rounded caps and joins.
Only the stroke color is adapted, to `#e2e8f0` for the application's dark theme.
Use the same settings when adding icons; keep buttons' accessible labels and
hover tooltips independent of asset filenames. No font or runtime download is needed.
Each action gets its own icon: `crosshair.svg` is Reset View, `pipette.svg` is the
canvas Color Picker tool and `scan-eye.svg` is the Screen Picker action.

`eye-dropper-cursor.svg` adapts Lucide's `pipette.svg`: its (2,22) tip becomes
(3,29) after scaling by 1.2 and translating by (.6,2.6). A white 4.5px halo
under the dark 2px outline keeps it visible over artwork. The Windows CUR is
rendered from this SVG at 32×32, with bottom-up 32-bit BGRA pixels, a 1-bit AND
transparency mask and hotspot (3,29). Keep both cursor assets synchronized;
`cursor_smoke` checks native cursor dimensions/hotspot at 1× and 2× on Linux.

`swap-colors.svg`, `reset-colors.svg`, and `rotate-shape-cursor.svg` are original
Hoshi Paint artwork under GPL-3.0-only. Their existing outline style is retained.
