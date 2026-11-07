# Hoshi project format, version 1

The `.hoshi` extension stores bounded, uncompressed straight-alpha RGBA8 layers.
All integers are unsigned little-endian. No external paths, executable content,
compressed payloads or undo history are stored.

| Field | Encoding |
| --- | --- |
| Magic | 8 bytes: `HOSHIPNT` |
| Version | u32, currently 1 |
| Width, height | u32 each; validated against the document pixel limit |
| Resolution | u32, pixels/inch multiplied by 1000 |
| Selected layer index | u32, zero-based |
| Layer count | u32, 1–32 |
| Each layer name | u32 byte count followed by UTF-8; 1–1024 bytes, nonblank |
| Layer properties | u8 visible (0 or 1), u8 opacity (0–255) |
| Layer pixels | width × height × 4 bytes, row-major R,G,B,A |
| Checksum | u32 CRC-32 of every preceding byte, using crc32fast's IEEE CRC-32 |

Layers are stored bottom to top. RGB values beneath transparent pixels are retained.
The checksum, version, dimensions, density, layer count, selection, names, visibility,
full pixel payload and exact EOF are validated before accepting a file. Maximum file
size is 128 MiB + 64 KiB, covering the current 32-layer, one-megapixel bounds. Unknown
versions fail explicitly. CRC detects accidental corruption, not malicious tampering.

The implementation lives in `crates/hoshi-core/src/project.rs`; filesystem and
raster codecs live in the desktop adapter. This is Hoshi's own format and does not
claim compatibility with Paint.NET's `.pdn` format.
