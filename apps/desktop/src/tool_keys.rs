//! Stable tool-key mappings for the desktop adapter.

use hoshi_controller::{Tool, ToolId};

/// Maps implemented tool IDs to their controller tools.
pub(crate) fn tool_for(id: ToolId) -> Option<Tool> {
    match id {
        ToolId::Shapes => Some(Tool::Rectangle),
        ToolId::Pan => Some(Tool::Pan),
        ToolId::ColorPicker => Some(Tool::Eyedropper),
        ToolId::Paintbrush => Some(Tool::Brush),
        ToolId::Eraser => Some(Tool::Eraser),
        _ => None,
    }
}

/// Returns the stable UI key, sharing Shapes with MoveShape.
pub(crate) fn tool_key(tool: Tool) -> &'static str {
    match tool {
        Tool::Rectangle | Tool::MoveShape => ToolId::Shapes,
        Tool::Pan => ToolId::Pan,
        Tool::Eyedropper => ToolId::ColorPicker,
        Tool::Brush => ToolId::Paintbrush,
        Tool::Eraser => ToolId::Eraser,
    }
    .key()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rail_tool_id_parses_and_maps_to_an_implemented_tool() {
        let rail = include_str!("../ui/drawing-tools.slint");
        let mut tools = Vec::new();
        for line in rail.lines() {
            if let Some(entry) = line.trim().strip_prefix("{ id: \"") {
                let key = entry.split('"').next().unwrap();
                let id = ToolId::from_key(key).expect("rail tool has a stable tool id");
                assert!(tool_for(id).is_some());
                tools.push(id);
            }
        }
        assert_eq!(
            tools,
            [ToolId::Paintbrush, ToolId::Eraser, ToolId::ColorPicker]
        );
        let main = include_str!("../ui/main-window.slint");
        assert!(main.contains("root.choose-tool(\"shapes\")"));
        assert_eq!(tool_for(ToolId::Shapes), Some(Tool::Rectangle));
    }

    #[test]
    fn supported_tools_round_trip_and_move_shape_uses_the_shapes_key() {
        for tool in [
            Tool::Rectangle,
            Tool::Pan,
            Tool::Eyedropper,
            Tool::Brush,
            Tool::Eraser,
        ] {
            assert_eq!(
                ToolId::from_key(tool_key(tool)).and_then(tool_for),
                Some(tool)
            );
        }
        assert_eq!(tool_key(Tool::MoveShape), ToolId::Shapes.key());
        for id in ToolId::ALL {
            if !matches!(
                id,
                ToolId::Shapes
                    | ToolId::Pan
                    | ToolId::ColorPicker
                    | ToolId::Paintbrush
                    | ToolId::Eraser
            ) {
                assert_eq!(tool_for(id), None);
            }
        }
    }
}
