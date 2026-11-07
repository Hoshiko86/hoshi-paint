# Resize resampling compatibility

Reference: the official [Paint.NET download page](https://www.getpaint.net/download.html)
listed **5.1.12** when checked for this change. Its current
[Image menu documentation](https://www.getpaint.net/doc/latest/ImageMenu.html)
(revision 15 December 2024) describes eight resampling modes. The linked
[resampling menu image](https://www.getpaint.net/doc/latest/images/menus/image/ResizeSamplingModes.png)
confirms their labels and order. The
[Resize dialog image](https://www.getpaint.net/doc/latest/images/menus/image/resize.png)
shows Bicubic selected and Use gamma correction checked.

The Hoshi Resize dialog now exposes those eight choices in the same order,
defaults to Bicubic with gamma correction enabled, and has an icon to reset the
resampling choice to Bicubic without changing gamma correction.

| Mode | Hoshi implementation |
| --- | --- |
| Bicubic | Catmull–Rom cubic, B=0 and C=0.5; support widens for antialiased reduction |
| Bicubic (Smooth) | Cubic B-spline, B=1 and C=0; support widens for reduction |
| Bilinear | Triangle/tent kernel, widened when shrinking to reduce aliasing |
| Bilinear (Low Quality) | Fixed two samples per axis, at most four source pixels per destination pixel; same as Bilinear when enlarging |
| Adaptive (Sharp) | Independent per-axis policy: antialiased Lanczos 3 for shrinking, Catmull–Rom for enlarging |
| Lanczos | Three-lobe sinc reconstruction, widened for reduction |
| Fant | Exact fractional pixel-area coverage for shrinking; triangle interpolation for enlarging, independently per axis |
| Nearest Neighbor | Pixel-center nearest lookup, preserving stored RGBA bytes exactly |

## Color and transparency

All interpolating filters operate on premultiplied RGBA. With gamma correction
on, RGB is first decoded from sRGB to linear light. RGB is premultiplied by alpha,
filtered, unpremultiplied, encoded to sRGB and rounded to RGBA8. Alpha is linear
coverage and is never gamma encoded. With gamma off, filtering uses encoded RGB
while still premultiplying by alpha. Fully transparent interpolated output is
canonical zero RGBA; Nearest preserves even hidden RGB bytes.

A black/white 50% average is approximately 188 with gamma correction, versus 128
without. Transparent blue beside opaque red contributes no blue fringe.

Pixel centers determine sample locations. Kernel support is clipped at image
edges and normalized to preserve constant colors. Unchanged axes use an identity
pass. Accumulation uses f64; clamping and quantization happen at the output.
The smaller separable intermediate is selected for mixed shrinking/enlarging
axes to keep the temporary pixel count within the endpoint pixel budget.

The core has no image-library dependency for resampling. Implementation is
original Rust under the project's GPLv3 license; the image crate remains a
screenshot helper in development examples.

## Compatibility boundary

This closes the **mode-list and documented-behavior gaps**, not a verified claim
of bit-identical Paint.NET output. Paint.NET's documentation does not specify
Adaptive (Sharp)'s exact switching thresholds, border rules, intermediate
precision or tie rounding. Our adaptive policy above is explicit and independently
implemented. Paint.NET-generated golden images are still needed to compare all
modes at borders, fractional scales, mixed-axis ratios, translucent edges and
high-contrast ringing. The documentation's eight names alone are not sufficient
to establish exact pixel parity.

The GUI applies explicit `ResizeOptions`. Existing Rust callers passing a bare
`Resampling` value retain encoded-RGB filtering; `ResizeOptions::default()` uses
Bicubic and gamma correction, matching the GUI defaults. Resize options do not
change validation, per-layer metadata, resolution history or undo/redo behavior.
The existing document size limit is unchanged.

## Validation

Hand-computed fixtures check fractional area weights in one and two dimensions,
the four-pixel low-quality bilinear footprint, sRGB luminance averaging, alpha
coverage, B-spline interpolation, nearest RGBA preservation, constant colors,
single-pixel axes and the documented adaptive policy. UI tests execute every
mode with gamma on/off and exercise the actual dropdown, gamma checkbox and
reset button. The native image-settings smoke renders the updated Resize dialog.
