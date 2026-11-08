# Hoshi project format, version 2

The `.hoshi` extension stores sparse straight-alpha RGBA8 layers. Integers are
unsigned little-endian. No external paths, executable content, compressed payloads,
or undo history are stored. Version 2 replaces whole-image payloads with a default
color plus allocated 128 × 128 pixel tiles. A blank 262,144 × 262,144 image therefore
requires only its header and layer metadata, rather than a 256 GiB pixel array.

| Field | Encoding |
| --- | --- |
| Magic | 8 bytes: `HOSHIPNT` |
| Version | u32, currently 2 |
| Width, height | u32 each; 1–262,144 per axis |
| Resolution | u32, pixels/inch multiplied by 1000 |
| Selected layer index | u32, zero-based |
| Layer count | u32, 1–32 |
| Each layer name | u32 byte count followed by UTF-8; 1–1024 bytes, nonblank |
| Layer properties | u8 visible (0 or 1), u8 opacity (0–255) |
| Layer default | 4 bytes R,G,B,A; applies wherever no tile is stored |
| Tile count | u32, bounded by the canvas tile grid |
| Each tile position | u32 tile-column index, u32 tile-row index |
| Tile pixels | Row-major R,G,B,A; 128 × 128 pixels, cropped at right/bottom document edges |
| Checksum | u32 IEEE CRC-32 of every preceding byte, using crc32fast |

Layers are stored bottom to top. Tile record order is unspecified. Coordinates
must be in bounds and unique within each layer. RGB beneath transparent pixels is
retained. The version, dimensions, density, layers, selection, names, properties,
tile coordinates/payload, checksum and exact EOF are validated before accepting a
file. Unknown versions fail explicitly. CRC detects accidental corruption, not
malicious tampering.

The reader and writer stream through `std::io` traits. Encoded file length has no
fixed 128 MiB ceiling; allocated tiles and working buffers must fit the shared
image-memory budget. A failed read leaves the existing document untouched. Save
writes to a sibling temporary file and replaces the destination only after encoding,
flush and sync succeed. In-memory `encode_project`/`decode_project` convenience
APIs remain available for small callers; desktop files use the streaming APIs.

## Version 1 compatibility

The reader still accepts version 1 files produced by earlier Hoshi Paint builds.
Their global header, names and properties match the layout above, but each layer
contains width × height × 4 raw RGBA bytes directly after its properties, with no
default color or tile records. Legacy pixels are streamed into tiled storage.
Saving an imported V1 project writes V2. Earlier Hoshi Paint builds cannot open V2.

The implementation lives in `crates/hoshi-core/src/project.rs`; filesystem and
raster codecs live in the desktop adapter. This is Hoshi's own format and does not
claim compatibility with Paint.NET's `.pdn` format.
