# 0007. Desktop UI theme: semantic tokens, built-in Light/Dark, theme files

- Status: Accepted (owner, 2026-11-19)
- Date: 2026-11-19
- Deciders: owner (approve), Claude (architect)
- Related: ADR 0006 (Slint), #236 (contrast bug, slice 1)

## Context

The desktop UI is dark, with a `DesktopTheme` global in
`apps/desktop/ui/theme.slint` whose header says components should use its
roles "instead of repeating literal colors". Nothing enforces it:

- About 60 color literals in 9 `.slint` files bypass it (count on 2026-11-19).
- Two shape components were built for a light theme inside dark windows. The
  Bounds labels are at 1.8:1 contrast and the field text at about 1.1:1
  (owner report). The selected-tool ring is at 2.0:1.
- Std widgets (`LineEdit`, `ComboBox`, …) follow Slint's own `Palette` global
  and color scheme, not `DesktopTheme`, so custom parts and std widgets can
  disagree.

Owner decisions (2026-11-19):
- A real theme system, not hardcoded colors.
- Light and Dark from the start.
- Community themes must be possible without rebuilding the app.

Slint has no utility-class system like Tailwind: styling is component
properties. The idiomatic theme mechanism is a `global` of tokens. A global's
`in-out` properties have compiled defaults and can be set from Rust at runtime.
Each window (`AppWindow`, `HistoryWindow`, `ColorPickerWindow`,
`ShapePropertiesWindow`) has its own instance of every global, which is why
`localization.rs` applies catalogs once per window.

## Decision

1. **One theme global, semantic tokens only.** `DesktopTheme` holds
   role-named color tokens (`text`, `muted-text`, `panel-background`,
   `field-background`, `field-border`, `focus-ring`, `highlight-background`, …)
   as `in-out property <color>`. Their compiled defaults are the built-in Dark
   values, so previews, tests and the first frame render correctly before Rust
   applies a theme.
   - Components use only `DesktopTheme` tokens for color, never literals.
     On-canvas overlays (shape handles) and the transparency checkerboard also
     get tokens.
   - Lengths (spacing, radii, button sizes, font sizes) stay compiled `out`
     tokens in the same global. They are not themeable for now, so layouts
     stay predictable.

2. **Themes are JSON files, read at runtime.** The format is version 1:
   ```json
   {
     "format": 1,
     "name": "Dark",
     "scheme": "dark",
     "base": "dark",
     "palette": { "gray-900": "#202226", "blue-400": "#72a7f8" },
     "tokens":  { "window-background": "gray-900", "focus-ring": "blue-400",
                  "text": "#e8edf5" }
   }
   ```
   - `scheme` (`dark` or `light`) also sets Slint's `Palette.color-scheme`, so
     std widgets match.
   - `palette` holds the raw colors (Tailwind-style scales such as
     `gray-50…950` and `blue-…`). It is for authoring only: tokens refer to a
     palette name or give `#rrggbb` / `#rrggbbaa`.
   - `base` (`dark` or `light`) is optional and only for user themes. Tokens a
     user theme leaves out come from that built-in, so a community theme can
     override just a few tokens.
   - Validation: an unknown token, an unknown palette name or a malformed color
     is an error. A user theme that fails to load leaves the current theme in
     place and reports a localized error. Errors are a `thiserror` enum; the
     text comes from the catalogs.

3. **Built-in Light and Dark are embedded.** `apps/desktop/themes/dark.json`
   and `light.json` are compiled in with `include_str!`, so they always work.
   - Tests require both to define every token and to meet WCAG 2.1 contrast on
     the listed pairs: text on its background at least 4.5:1, borders and focus
     rings at least 3:1.
   - Dark matches today's look.

4. **One apply path.** A desktop `theme` module resolves a theme to concrete
   colors once, then sets every token on all four windows' `DesktopTheme`, and
   `Palette.color-scheme` on each. Rust code outside that module never reads
   or sets theme colors.

5. **Selection.**
   - The default is **System**: Dark or Light, following the OS color scheme.
     Where the OS reports none, it is Dark.
   - The user can pick System, Dark, Light, or any `*.json` in
     `<config>/hoshi-paint/themes/`. `<config>` is the same root
     `documents.rs` already uses: `APPDATA`, `~/Library/Application Support`
     or `XDG_CONFIG_HOME`.
   - The choice is saved next to the existing workspace settings. Changing it
     applies immediately, with no restart.

6. **Enforced in CI.**
   - `cargo xtask test lint` fails on any color literal (`#…` or a named color)
     in `apps/desktop/ui/*.slint` outside `theme.slint`, like the English-text
     guards.
   - A unit test checks that the token list is the same in `theme.slint`,
     `dark.json`, `light.json` and the Rust apply code, like the catalog key
     parity test.

## Consequences

- Every color change is one token edit, and a new theme is one JSON file.
  Community themes need no build.
- Four windows × N tokens are set at startup and on every theme switch. That is
  a few hundred property writes, negligible next to a canvas redraw.
- The token list is now an API for theme authors. Renaming or removing a token
  breaks user themes, so it needs a `format` bump or an alias. Adding a token is
  safe, because user themes inherit it from `base`.
- Std-widget styling is limited to what Slint's `Palette` and style expose. We
  do not restyle std widgets beyond the color scheme.

## Slices (each a Codex task; later ones are filed when their prerequisite merges)

1. **#236.** `DesktopTheme` colors become `in-out` semantic tokens (adding the
   missing ones, defaults = Dark). Bounds and the stroke/fill pickers move onto
   them, which fixes the contrast bug.
2. **Theme engine.** The `theme` module, embedded `dark.json`/`light.json`,
   apply to all windows plus `Palette.color-scheme`, the System default, and the
   parity and contrast tests.
3. **Migration plus guard.** Move every remaining `.slint` file onto tokens and
   turn on the lint guard.
4. **User themes.** Load `<config>/hoshi-paint/themes/*.json`, add the theme
   picker UI, catalog keys and localized load errors, and document the format
   in `docs/THEMES.md`.
