# Document management

The desktop spike follows Paint.NET's File workflow: New and Open add document tabs,
Save updates the current file, Save As changes its name/type, Save All saves modified
tabs, and Close/Close All/Exit offer Save, Don't Save, and Cancel for unsaved work.
The reference is the [current Paint.NET File menu documentation](https://www.getpaint.net/doc/latest/FileMenu.html).
This is a working subset, not complete file-format parity.

Each tab has its own pixels, layers, resolution, view, undo/redo history, filename,
and save point. Asterisk markers disappear when undo returns to the saved content.
Primary/Secondary colors and drawing settings remain shared. Switching tabs commits
an active shape; saving commits it too. Cancelling a file chooser or current-tab close prompt
retains the active preview. Close All/Exit visit tabs in order, so switching during
that process follows the same shape-commit rule. New starts a clean history without replacing the old tab.
Closing the last tab leaves an empty workspace with New and Open available.

| Command | Shortcut |
| --- | --- |
| New | Ctrl/Cmd+N |
| Open, including multiple selections | Ctrl/Cmd+O |
| Save | Ctrl/Cmd+S |
| Save As | Ctrl/Cmd+Shift+S |
| Save All | Ctrl/Cmd+Alt+S |
| Close current tab | Ctrl/Cmd+W |
| Close all tabs | Ctrl/Cmd+Shift+W |
| Next/previous tab | Ctrl/Cmd+Tab / Ctrl/Cmd+Shift+Tab |

File shortcuts also work from floating Colors and History windows. The tab strip
shows image thumbnails, filenames, modified markers, close buttons, and full-path
tooltips. Opening an already open path focuses its tab and preserves unsaved edits.
Recent files retain the last ten opened/saved paths across restarts, with a Clear
command. The configuration is `hoshi-paint/recent-files.json` in the OS application
configuration directory; `HOSHI_CONFIG_DIR` overrides the directory for development.
Command-line image paths open as separate tabs at startup.

## Formats and safeguards

- **.hoshi** is the native, versioned layered project format; see [format details](FILE_FORMAT.md).
  It preserves every layer's pixels/name/visibility/opacity, selection, dimensions,
  and resolution. Undo history is session-local and is not written to files.
- **PNG** saves lossless RGBA8 with transparency and resolution. Hoshi stores an
  additional resolution text field to preserve its exact 0.001-DPI resolution;
  standard PNG density is also included for other applications.
- **JPEG** offers quality 1–100. Confirmation explains lossy compression, flattening,
  a white background for transparency, and whole-DPI density rounding. JPEG density
  above 65535 DPI is rejected; PNG and .hoshi support the document's full range.
- Open supports PNG, JPEG, BMP, GIF, TIFF, TGA, WebP and ICO through the Rust codecs.
  Animated/multipage files load the first image only. Saving formats other than the
  three above opens Save As with a .hoshi suggestion. JPEG orientation is applied;
  JFIF density is read. Other metadata, including EXIF, is not preserved on saving.
- Embedded ICC profiles and channels above 8 bits are rejected before opening,
  rather than silently changing their colors or precision. Convert to untagged
  sRGB RGBA8 in another application for now. Full color-managed import is future work.
- **.pdn is not supported**. Export a raster image from Paint.NET to open it here.
  DDS, JXL, JXR, HEIC and AVIF, format plug-ins, printing, scanner acquisition,
  clipboard-driven New defaults, OS file associations and drag/drop remain future work.

Saving a layered document to PNG/JPEG requires explicit flattening confirmation.
Flattening is undoable. After accepting, a failed write keeps the image open and
modified, including its undo history. Encoded bytes are written to a sibling temporary
file, flushed and synced, then atomically replace the destination. A failed encode
leaves an existing destination untouched. Read-only destinations report an error.
Save As refuses a destination already owned by another open tab. Native file pickers
provide overwrite confirmation. The filename extension determines the encoder; use
.hoshi, .png or .jpg explicitly when switching formats in a picker.

Prototype bounds remain 1,048,576 pixels per image, 32 layers, 16 undo entries, and
32 open tabs. Input is bounded before pixel allocation; tab thumbnails are cached.
These bounds require a larger-image memory/tile design before production use.
Linux file dialogs use the desktop's XDG portal; a portal provider is needed in
minimal/headless Linux sessions. Windows and macOS use native OS file dialogs.

## Validation

Tests cover layered and PNG round trips (including RGB beneath zero alpha), density,
corrupt/truncated/unsupported projects, independent tab histories, save-point undo,
duplicate Open, recent-file persistence, failed atomic writes, format confirmation,
Save All, chooser cancellation, and close cancellation/discard. The native winit
`documents_smoke` example verifies the rendered tab strip and prompts using real
filesystem files and deterministic dialog selections; screenshots are included in
Linux CI artifacts. Native OS chooser interaction is a manual platform check.
