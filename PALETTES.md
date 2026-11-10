# Colors and palettes

Type a CSS/SVG color name in Colors to filter suggestions. Each result includes a
preview swatch. Select a result or press Enter to apply it; typing alone does not
change the drawing color. There are 148 opaque named colors plus Transparent.
Spaces, capitalization and hyphens do not affect matching.

The palette contains 96 RGBA colors. The compact view shows the first 32; expand
to see all 96. Click a swatch to set the selected workspace color. Right-click
(or Shift+Enter) sets the other workspace color without changing the selected slot.
To replace a swatch, click Add Color, then its destination slot. This stores the
current selected color, including transparency, and exits insertion mode.

Save As writes the current palette and adds it to the saved palette list. Import
loads a palette; Export writes a copy without changing its saved baseline.
Reset restores Hoshi Paint's default colors. Open Palettes Folder lets you manage
saved files with your operating system. Reopen Colors to refresh the file list.
Working palette edits persist separately from image documents and undo history.
Editing a saved palette does not overwrite its file until you explicitly save it.

Paint.NET palette `.txt` interchange uses one eight-digit **AARRGGBB** value per
line, with semicolon comments. Missing entries become opaque white; entries after
96 are ignored. This differs from the Colors hex field's **RRGGBB[AA]** order.
Hoshi Paint supplies its own default swatch values; the file format and palette
workflow follow Paint.NET, while the default RGB values differ.

References: [Paint.NET palettes](https://www.getpaint.net/doc/latest/WorkingWithPalettes.html)
and [Colors window](https://www.getpaint.net/doc/latest/ColorsWindow.html).
