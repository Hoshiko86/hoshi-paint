//! Standard CSS/SVG named colors for exact lookup and autocomplete.
//!
//! RGB values are the standardized color-name facts from CSS Color. The catalog
//! includes spelling aliases and RebeccaPurple; Transparent is RGBA zero.
use crate::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NamedColor {
    pub name: &'static str,
    pub color: Color,
}

const fn opaque(name: &'static str, rgb: u32) -> NamedColor {
    NamedColor {
        name,
        color: Color::rgba((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, 255),
    }
}

// Alphabetic display order also supplies deterministic order within each group
// of autocomplete results. These factual values do not require a GUI dependency.
static COLORS: &[NamedColor] = &[
    opaque("AliceBlue", 0xF0F8FF),
    opaque("AntiqueWhite", 0xFAEBD7),
    opaque("Aqua", 0x00FFFF),
    opaque("Aquamarine", 0x7FFFD4),
    opaque("Azure", 0xF0FFFF),
    opaque("Beige", 0xF5F5DC),
    opaque("Bisque", 0xFFE4C4),
    opaque("Black", 0x000000),
    opaque("BlanchedAlmond", 0xFFEBCD),
    opaque("Blue", 0x0000FF),
    opaque("BlueViolet", 0x8A2BE2),
    opaque("Brown", 0xA52A2A),
    opaque("BurlyWood", 0xDEB887),
    opaque("CadetBlue", 0x5F9EA0),
    opaque("Chartreuse", 0x7FFF00),
    opaque("Chocolate", 0xD2691E),
    opaque("Coral", 0xFF7F50),
    opaque("CornflowerBlue", 0x6495ED),
    opaque("Cornsilk", 0xFFF8DC),
    opaque("Crimson", 0xDC143C),
    opaque("Cyan", 0x00FFFF),
    opaque("DarkBlue", 0x00008B),
    opaque("DarkCyan", 0x008B8B),
    opaque("DarkGoldenrod", 0xB8860B),
    opaque("DarkGray", 0xA9A9A9),
    opaque("DarkGreen", 0x006400),
    opaque("DarkGrey", 0xA9A9A9),
    opaque("DarkKhaki", 0xBDB76B),
    opaque("DarkMagenta", 0x8B008B),
    opaque("DarkOliveGreen", 0x556B2F),
    opaque("DarkOrange", 0xFF8C00),
    opaque("DarkOrchid", 0x9932CC),
    opaque("DarkRed", 0x8B0000),
    opaque("DarkSalmon", 0xE9967A),
    opaque("DarkSeaGreen", 0x8FBC8F),
    opaque("DarkSlateBlue", 0x483D8B),
    opaque("DarkSlateGray", 0x2F4F4F),
    opaque("DarkSlateGrey", 0x2F4F4F),
    opaque("DarkTurquoise", 0x00CED1),
    opaque("DarkViolet", 0x9400D3),
    opaque("DeepPink", 0xFF1493),
    opaque("DeepSkyBlue", 0x00BFFF),
    opaque("DimGray", 0x696969),
    opaque("DimGrey", 0x696969),
    opaque("DodgerBlue", 0x1E90FF),
    opaque("FireBrick", 0xB22222),
    opaque("FloralWhite", 0xFFFAF0),
    opaque("ForestGreen", 0x228B22),
    opaque("Fuchsia", 0xFF00FF),
    opaque("Gainsboro", 0xDCDCDC),
    opaque("GhostWhite", 0xF8F8FF),
    opaque("Gold", 0xFFD700),
    opaque("Goldenrod", 0xDAA520),
    opaque("Gray", 0x808080),
    opaque("Green", 0x008000),
    opaque("GreenYellow", 0xADFF2F),
    opaque("Grey", 0x808080),
    opaque("Honeydew", 0xF0FFF0),
    opaque("HotPink", 0xFF69B4),
    opaque("IndianRed", 0xCD5C5C),
    opaque("Indigo", 0x4B0082),
    opaque("Ivory", 0xFFFFF0),
    opaque("Khaki", 0xF0E68C),
    opaque("Lavender", 0xE6E6FA),
    opaque("LavenderBlush", 0xFFF0F5),
    opaque("LawnGreen", 0x7CFC00),
    opaque("LemonChiffon", 0xFFFACD),
    opaque("LightBlue", 0xADD8E6),
    opaque("LightCoral", 0xF08080),
    opaque("LightCyan", 0xE0FFFF),
    opaque("LightGoldenrodYellow", 0xFAFAD2),
    opaque("LightGray", 0xD3D3D3),
    opaque("LightGreen", 0x90EE90),
    opaque("LightGrey", 0xD3D3D3),
    opaque("LightPink", 0xFFB6C1),
    opaque("LightSalmon", 0xFFA07A),
    opaque("LightSeaGreen", 0x20B2AA),
    opaque("LightSkyBlue", 0x87CEFA),
    opaque("LightSlateGray", 0x778899),
    opaque("LightSlateGrey", 0x778899),
    opaque("LightSteelBlue", 0xB0C4DE),
    opaque("LightYellow", 0xFFFFE0),
    opaque("Lime", 0x00FF00),
    opaque("LimeGreen", 0x32CD32),
    opaque("Linen", 0xFAF0E6),
    opaque("Magenta", 0xFF00FF),
    opaque("Maroon", 0x800000),
    opaque("MediumAquamarine", 0x66CDAA),
    opaque("MediumBlue", 0x0000CD),
    opaque("MediumOrchid", 0xBA55D3),
    opaque("MediumPurple", 0x9370DB),
    opaque("MediumSeaGreen", 0x3CB371),
    opaque("MediumSlateBlue", 0x7B68EE),
    opaque("MediumSpringGreen", 0x00FA9A),
    opaque("MediumTurquoise", 0x48D1CC),
    opaque("MediumVioletRed", 0xC71585),
    opaque("MidnightBlue", 0x191970),
    opaque("MintCream", 0xF5FFFA),
    opaque("MistyRose", 0xFFE4E1),
    opaque("Moccasin", 0xFFE4B5),
    opaque("NavajoWhite", 0xFFDEAD),
    opaque("Navy", 0x000080),
    opaque("OldLace", 0xFDF5E6),
    opaque("Olive", 0x808000),
    opaque("OliveDrab", 0x6B8E23),
    opaque("Orange", 0xFFA500),
    opaque("OrangeRed", 0xFF4500),
    opaque("Orchid", 0xDA70D6),
    opaque("PaleGoldenrod", 0xEEE8AA),
    opaque("PaleGreen", 0x98FB98),
    opaque("PaleTurquoise", 0xAFEEEE),
    opaque("PaleVioletRed", 0xDB7093),
    opaque("PapayaWhip", 0xFFEFD5),
    opaque("PeachPuff", 0xFFDAB9),
    opaque("Peru", 0xCD853F),
    opaque("Pink", 0xFFC0CB),
    opaque("Plum", 0xDDA0DD),
    opaque("PowderBlue", 0xB0E0E6),
    opaque("Purple", 0x800080),
    opaque("RebeccaPurple", 0x663399),
    opaque("Red", 0xFF0000),
    opaque("RosyBrown", 0xBC8F8F),
    opaque("RoyalBlue", 0x4169E1),
    opaque("SaddleBrown", 0x8B4513),
    opaque("Salmon", 0xFA8072),
    opaque("SandyBrown", 0xF4A460),
    opaque("SeaGreen", 0x2E8B57),
    opaque("SeaShell", 0xFFF5EE),
    opaque("Sienna", 0xA0522D),
    opaque("Silver", 0xC0C0C0),
    opaque("SkyBlue", 0x87CEEB),
    opaque("SlateBlue", 0x6A5ACD),
    opaque("SlateGray", 0x708090),
    opaque("SlateGrey", 0x708090),
    opaque("Snow", 0xFFFAFA),
    opaque("SpringGreen", 0x00FF7F),
    opaque("SteelBlue", 0x4682B4),
    opaque("Tan", 0xD2B48C),
    opaque("Teal", 0x008080),
    opaque("Thistle", 0xD8BFD8),
    opaque("Tomato", 0xFF6347),
    NamedColor {
        name: "Transparent",
        color: Color::TRANSPARENT,
    },
    opaque("Turquoise", 0x40E0D0),
    opaque("Violet", 0xEE82EE),
    opaque("Wheat", 0xF5DEB3),
    opaque("White", 0xFFFFFF),
    opaque("WhiteSmoke", 0xF5F5F5),
    opaque("Yellow", 0xFFFF00),
    opaque("YellowGreen", 0x9ACD32),
];

/// All supported names in alphabetic order, including aliases.
pub fn all() -> &'static [NamedColor] {
    COLORS
}

fn normalize(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_ascii_whitespace() && *c != '-')
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Exact name lookup, ignoring ASCII case, spaces, and hyphens.
/// Partial names never select an approximate color.
pub fn lookup(name: &str) -> Option<Color> {
    let name = normalize(name);
    COLORS
        .iter()
        .find(|entry| normalize(entry.name) == name)
        .map(|entry| entry.color)
}

/// Prefix matches first, followed by other substring matches. Each group is
/// alphabetic. An empty query lists the full catalog; callers may limit the list.
pub fn suggestions(query: &str) -> Vec<NamedColor> {
    let query = normalize(query);
    let mut prefixes = Vec::new();
    let mut substrings = Vec::new();
    for entry in COLORS {
        let name = normalize(entry.name);
        if name.starts_with(&query) {
            prefixes.push(*entry);
        } else if name.contains(&query) {
            substrings.push(*entry);
        }
    }
    prefixes.extend(substrings);
    prefixes
}

/// Canonical name for an exact RGBA match. Gray spellings, Cyan, and Magenta
/// are preferred over Grey, Aqua, and Fuchsia. No nearest-color match is made.
pub fn name_for(color: Color) -> Option<&'static str> {
    COLORS
        .iter()
        .find(|entry| entry.color == color && entry.name != "Aqua" && entry.name != "Fuchsia")
        .map(|entry| entry.name)
}
