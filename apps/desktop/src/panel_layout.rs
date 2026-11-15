//! Dock geometry in workspace-local logical pixels, independent of Slint.
//!
//! The tool rail is outside the available panel/canvas area. Horizontal docks
//! occupy the top/bottom first; vertical docks use the remaining middle band.
//! A window may briefly report zero size during creation, so rectangles never
//! receive negative dimensions. `min_window_width` lets the host request enough
//! width for fixed-width side panels plus a useful drawing viewport.

const GAP: f64 = 14.0;
const MIN_CANVAS_WIDTH: f64 = 240.0;
const MIN_CANVAS_HEIGHT: f64 = 200.0;
const MAX_ROW_HEIGHT: f64 = 264.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PanelId {
    Colors,
    History,
    ShapeProperties,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PanelSpec {
    pub id: PanelId,
    pub visible: bool,
    /// 0 floating, 1 left, 2 right, 3 top, 4 bottom.
    pub placement: i32,
    /// Preferred width when placed on a vertical edge.
    pub width: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PanelBounds {
    pub id: PanelId,
    pub rect: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DockLayout {
    pub canvas: Rect,
    pub panels: Vec<PanelBounds>,
    /// Workspace width, excluding the host window's outer padding/borders.
    pub min_window_width: f64,
}

impl DockLayout {
    pub fn calculate(width: f64, height: f64, rail_offset: f64, panels: &[PanelSpec]) -> Self {
        let width = nonnegative(width);
        let height = nonnegative(height);
        let rail_offset = nonnegative(rail_offset);
        let rail_x = rail_offset.min(width);
        let mut active: Vec<_> = panels
            .iter()
            .copied()
            .filter(|panel| panel.visible && (1..=4).contains(&panel.placement))
            .collect();
        // Neither caller ordering nor reopening a panel changes edge ordering.
        active.sort_by_key(|panel| panel.id);
        active.dedup_by_key(|panel| panel.id);

        let reserve = |edge| {
            active
                .iter()
                .filter(|panel| panel.placement == edge)
                .map(|panel| nonnegative(panel.width) + GAP)
                .sum::<f64>()
        };
        let left = reserve(1);
        let right = reserve(2);
        let has_top = active.iter().any(|panel| panel.placement == 3);
        let has_bottom = active.iter().any(|panel| panel.placement == 4);
        let row_count = usize::from(has_top) + usize::from(has_bottom);
        let row_height = if row_count == 0 {
            0.0
        } else {
            ((height - MIN_CANVAS_HEIGHT).max(0.0) / row_count as f64).min(MAX_ROW_HEIGHT)
        };
        let top = if has_top { row_height } else { 0.0 };
        let bottom = if has_bottom { row_height } else { 0.0 };
        let middle_height = (height - top - bottom).max(0.0);
        let mut result = Self {
            canvas: Rect {
                x: (rail_x + left).min(width),
                y: top,
                width: (width - rail_x - left - right).max(0.0),
                height: middle_height,
            },
            panels: Vec::with_capacity(active.len()),
            min_window_width: rail_offset + left + right + MIN_CANVAS_WIDTH,
        };

        // The first panel on the right is closest to the outer edge, matching
        // the stable Colors/History/Shape Properties order on that side.
        for edge in [1, 2] {
            let mut cursor = if edge == 1 { rail_x } else { width };
            for panel in active.iter().filter(|panel| panel.placement == edge) {
                let panel_width = nonnegative(panel.width);
                let x = if edge == 1 {
                    let x = cursor;
                    cursor += panel_width + GAP;
                    x
                } else {
                    cursor -= panel_width;
                    let x = cursor;
                    cursor -= GAP;
                    x
                };
                result.panels.push(PanelBounds {
                    id: panel.id,
                    rect: Rect {
                        x,
                        y: top,
                        width: panel_width,
                        height: middle_height,
                    },
                });
            }
        }

        for edge in [3, 4] {
            let row: Vec<_> = active
                .iter()
                .filter(|panel| panel.placement == edge)
                .collect();
            if row.is_empty() {
                continue;
            }
            let available_width = (width - rail_x).max(0.0);
            let gap_count = row.len() - 1;
            // Keep initial zero/small windows finite, even before constraints
            // have been applied by the native window manager.
            let gap = if gap_count == 0 {
                0.0
            } else {
                GAP.min(available_width / gap_count as f64)
            };
            let panel_width =
                (available_width - gap * gap_count as f64).max(0.0) / row.len() as f64;
            let panel_height = (row_height - GAP).max(0.0);
            let y = if edge == 3 {
                0.0
            } else {
                height - panel_height
            };
            for (index, panel) in row.into_iter().enumerate() {
                result.panels.push(PanelBounds {
                    id: panel.id,
                    rect: Rect {
                        x: rail_x + index as f64 * (panel_width + gap),
                        y,
                        width: panel_width,
                        height: panel_height,
                    },
                });
            }
        }
        result.panels.sort_by_key(|panel| panel.id);
        result
    }

    pub fn panel(&self, id: PanelId) -> Option<Rect> {
        self.panels
            .iter()
            .find(|panel| panel.id == id)
            .map(|panel| panel.rect)
    }
}

fn nonnegative(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn specs(placements: [i32; 3], visible: [bool; 3]) -> [PanelSpec; 3] {
        [
            PanelSpec {
                id: PanelId::Colors,
                visible: visible[0],
                placement: placements[0],
                width: 420.0,
            },
            PanelSpec {
                id: PanelId::History,
                visible: visible[1],
                placement: placements[1],
                width: 300.0,
            },
            PanelSpec {
                id: PanelId::ShapeProperties,
                visible: visible[2],
                placement: placements[2],
                width: 340.0,
            },
        ]
    }

    fn separated(a: Rect, b: Rect) -> bool {
        a.x + a.width <= b.x + 0.0001
            || b.x + b.width <= a.x + 0.0001
            || a.y + a.height <= b.y + 0.0001
            || b.y + b.height <= a.y + 0.0001
    }

    #[test]
    fn every_three_panel_placement_and_visibility_combination_keeps_hosts_separate() {
        for colors in 0..=4 {
            for history in 0..=4 {
                for shapes in 0..=4 {
                    for mask in 0..8 {
                        let panels = specs(
                            [colors, history, shapes],
                            [mask & 1 != 0, mask & 2 != 0, mask & 4 != 0],
                        );
                        let layout = DockLayout::calculate(2000.0, 1000.0, 78.0, &panels);
                        assert!(layout.canvas.width >= MIN_CANVAS_WIDTH);
                        assert!(layout.canvas.height >= MIN_CANVAS_HEIGHT);
                        assert!(layout.min_window_width <= 2000.0);
                        for (index, host) in layout.panels.iter().enumerate() {
                            let rect = host.rect;
                            assert!(rect.x >= 78.0 && rect.y >= 0.0);
                            assert!(rect.x + rect.width <= 2000.0001);
                            assert!(rect.y + rect.height <= 1000.0001);
                            assert!(separated(layout.canvas, rect), "canvas covered: {layout:?}");
                            for other in &layout.panels[index + 1..] {
                                assert!(separated(rect, other.rect), "panels overlap: {layout:?}");
                            }
                        }
                        for panel in panels {
                            assert_eq!(
                                layout.panel(panel.id).is_some(),
                                panel.visible && panel.placement != 0
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn ordering_is_stable_and_minimum_width_accounts_for_every_side_panel_and_gap() {
        let panels = specs([1, 1, 1], [true; 3]);
        let left = DockLayout::calculate(1500.0, 760.0, 78.0, &panels);
        assert_eq!(left.panel(PanelId::Colors).unwrap().x, 78.0);
        assert_eq!(left.panel(PanelId::History).unwrap().x, 512.0);
        assert_eq!(left.panel(PanelId::ShapeProperties).unwrap().x, 826.0);
        assert_eq!(left.min_window_width, 1420.0);
        let mut reversed = panels;
        reversed.reverse();
        assert_eq!(left, DockLayout::calculate(1500.0, 760.0, 78.0, &reversed));
        let right = DockLayout::calculate(1500.0, 760.0, 78.0, &specs([2, 2, 2], [true; 3]));
        assert_eq!(right.panel(PanelId::Colors).unwrap().x, 1080.0);
        assert_eq!(right.panel(PanelId::History).unwrap().x, 766.0);
        assert_eq!(right.panel(PanelId::ShapeProperties).unwrap().x, 412.0);
        assert_eq!(right.min_window_width, 1420.0);
    }

    #[test]
    fn opposite_bands_adapt_to_height_and_multiple_panels_share_equal_row_width() {
        let layout = DockLayout::calculate(1120.0, 546.0, 78.0, &specs([3, 4, 3], [true; 3]));
        assert_eq!(layout.canvas.y, 173.0);
        assert_eq!(layout.canvas.height, 200.0);
        let colors = layout.panel(PanelId::Colors).unwrap();
        let shapes = layout.panel(PanelId::ShapeProperties).unwrap();
        assert_eq!(colors.height, 159.0);
        assert_eq!(colors.width, 514.0);
        assert_eq!(shapes.width, colors.width);
        assert_eq!(shapes.x, colors.x + colors.width + GAP);
        assert_eq!(layout.panel(PanelId::History).unwrap().y, 387.0);
    }

    #[test]
    fn hidden_floating_and_invalid_placements_do_not_reserve_canvas_space() {
        let panels = specs([0, 2, 9], [true, false, true]);
        let layout = DockLayout::calculate(1120.0, 546.0, 78.0, &panels);
        assert_eq!(
            layout.canvas,
            Rect {
                x: 78.0,
                y: 0.0,
                width: 1042.0,
                height: 546.0
            }
        );
        assert!(layout.panels.is_empty());
        assert_eq!(layout.min_window_width, 318.0);
    }

    #[test]
    fn undersized_initial_windows_keep_finite_nonnegative_dimensions() {
        for width in [0.0, 1.0, 20.0, 78.0, 300.0, f64::NAN] {
            for height in [0.0, 1.0, 55.0, 199.0, 200.0, 201.0, 546.0, f64::INFINITY] {
                let layout =
                    DockLayout::calculate(width, height, 78.0, &specs([3, 4, 1], [true; 3]));
                for rect in
                    std::iter::once(layout.canvas).chain(layout.panels.iter().map(|host| host.rect))
                {
                    assert!(rect.x.is_finite() && rect.y.is_finite());
                    assert!(rect.width.is_finite() && rect.width >= 0.0);
                    assert!(rect.height.is_finite() && rect.height >= 0.0);
                }
            }
        }
    }
}
