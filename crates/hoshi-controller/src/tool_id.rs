/// Stable identity for each tool, in Paint.NET's Tools-panel order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolId {
    MovePixels,
    MoveSelection,
    RectangleSelect,
    LassoSelect,
    EllipseSelect,
    MagicWand,
    Zoom,
    Pan,
    PaintBucket,
    Gradient,
    Paintbrush,
    Eraser,
    Pencil,
    ColorPicker,
    CloneStamp,
    Recolor,
    Text,
    LineCurve,
    Shapes,
}

impl ToolId {
    /// Every tool exactly once, in Paint.NET's Tools-panel order.
    pub const ALL: [ToolId; 19] = [
        Self::MovePixels,
        Self::MoveSelection,
        Self::RectangleSelect,
        Self::LassoSelect,
        Self::EllipseSelect,
        Self::MagicWand,
        Self::Zoom,
        Self::Pan,
        Self::PaintBucket,
        Self::Gradient,
        Self::Paintbrush,
        Self::Eraser,
        Self::Pencil,
        Self::ColorPicker,
        Self::CloneStamp,
        Self::Recolor,
        Self::Text,
        Self::LineCurve,
        Self::Shapes,
    ];

    /// Stable wire-format key shared with the UI, localization and icons; never rename it.
    pub const fn key(self) -> &'static str {
        match self {
            Self::MovePixels => "move-pixels",
            Self::MoveSelection => "move-selection",
            Self::RectangleSelect => "rectangle-select",
            Self::LassoSelect => "lasso-select",
            Self::EllipseSelect => "ellipse-select",
            Self::MagicWand => "magic-wand",
            Self::Zoom => "zoom",
            Self::Pan => "pan",
            Self::PaintBucket => "paint-bucket",
            Self::Gradient => "gradient",
            Self::Paintbrush => "paintbrush",
            Self::Eraser => "eraser",
            Self::Pencil => "pencil",
            Self::ColorPicker => "color-picker",
            Self::CloneStamp => "clone-stamp",
            Self::Recolor => "recolor",
            Self::Text => "text",
            Self::LineCurve => "line-curve",
            Self::Shapes => "shapes",
        }
    }

    /// Resolve an exact stable key, returning `None` for unknown keys.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tool| tool.key() == key)
    }

    /// Lowercase shortcut letter; tools sharing a letter participate in shortcut cycling.
    pub const fn shortcut(self) -> char {
        match self {
            Self::MovePixels | Self::MoveSelection => 'm',
            Self::RectangleSelect | Self::LassoSelect | Self::EllipseSelect | Self::MagicWand => {
                's'
            }
            Self::Zoom => 'z',
            Self::Pan => 'h',
            Self::PaintBucket => 'f',
            Self::Gradient => 'g',
            Self::Paintbrush => 'b',
            Self::Eraser => 'e',
            Self::Pencil => 'p',
            Self::ColorPicker => 'k',
            Self::CloneStamp => 'l',
            Self::Recolor => 'r',
            Self::Text => 't',
            Self::LineCurve | Self::Shapes => 'o',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ToolId;
    use std::collections::HashSet;

    #[test]
    fn stable_keys_round_trip_and_have_unique_wire_format() {
        let mut keys = HashSet::new();
        for tool in ToolId::ALL {
            let key = tool.key();
            assert!(!key.is_empty());
            assert!(
                key.bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
            );
            assert!(keys.insert(key));
            assert_eq!(ToolId::from_key(key), Some(tool));
        }
        for unknown in ["nope", "", "Paintbrush", "paint_brush", " paintbrush"] {
            assert_eq!(ToolId::from_key(unknown), None);
        }
    }

    #[test]
    fn all_preserves_tools_panel_order_and_shortcuts() {
        let expected = [
            (ToolId::MovePixels, "move-pixels", 'm'),
            (ToolId::MoveSelection, "move-selection", 'm'),
            (ToolId::RectangleSelect, "rectangle-select", 's'),
            (ToolId::LassoSelect, "lasso-select", 's'),
            (ToolId::EllipseSelect, "ellipse-select", 's'),
            (ToolId::MagicWand, "magic-wand", 's'),
            (ToolId::Zoom, "zoom", 'z'),
            (ToolId::Pan, "pan", 'h'),
            (ToolId::PaintBucket, "paint-bucket", 'f'),
            (ToolId::Gradient, "gradient", 'g'),
            (ToolId::Paintbrush, "paintbrush", 'b'),
            (ToolId::Eraser, "eraser", 'e'),
            (ToolId::Pencil, "pencil", 'p'),
            (ToolId::ColorPicker, "color-picker", 'k'),
            (ToolId::CloneStamp, "clone-stamp", 'l'),
            (ToolId::Recolor, "recolor", 'r'),
            (ToolId::Text, "text", 't'),
            (ToolId::LineCurve, "line-curve", 'o'),
            (ToolId::Shapes, "shapes", 'o'),
        ];
        assert_eq!(ToolId::ALL.len(), 19);
        assert_eq!(ToolId::ALL.into_iter().collect::<HashSet<_>>().len(), 19);
        for (tool, (expected_tool, key, shortcut)) in ToolId::ALL.into_iter().zip(expected) {
            assert_eq!(tool, expected_tool);
            assert_eq!(tool.key(), key);
            assert_eq!(tool.shortcut(), shortcut);
        }
    }
}
