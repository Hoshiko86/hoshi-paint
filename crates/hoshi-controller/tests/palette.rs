use hoshi_controller::{
    Color,
    palette::{PALETTE_SIZE, Palette, PaletteError, parse_paint_net},
};

#[test]
fn default_swatches_are_editable_and_roundtrip_all_channels() {
    let mut palette = Palette::default();
    assert_eq!(palette.colors().len(), PALETTE_SIZE);
    assert_eq!(palette.colors()[0], Color::rgba(0, 0, 0, 255));
    assert_eq!(palette.colors()[1], Color::rgba(255, 255, 255, 255));
    for index in 0..PALETTE_SIZE {
        palette
            .replace(
                index,
                Color::rgba(index as u8, 255 - index as u8, 71, index as u8),
            )
            .unwrap();
    }
    let text = palette.to_paint_net();
    assert!(!text.lines().any(|line| line == "00FF4700"));
    assert_eq!(parse_paint_net(&text).unwrap(), palette);
    assert_eq!(Palette::parse_paint_net(&text).unwrap(), palette);
    assert!(text.lines().any(|line| line == "0000FF47"));
}

#[test]
fn import_accepts_bom_comments_blanks_case_and_pads_white() {
    let palette = parse_paint_net(
        "\u{feff}; header\r\n\r\n  8012aBcD ; translucent\r\n; skip\r\n00112233\r\n",
    )
    .unwrap();
    assert_eq!(palette.colors()[0], Color::rgba(0x12, 0xab, 0xcd, 0x80));
    assert_eq!(palette.colors()[1], Color::rgba(0x11, 0x22, 0x33, 0));
    assert!(
        palette.colors()[2..]
            .iter()
            .all(|c| *c == Color::rgba(255, 255, 255, 255))
    );
    assert!(
        parse_paint_net("")
            .unwrap()
            .colors()
            .iter()
            .all(|c| *c == Color::rgba(255, 255, 255, 255))
    );
}

#[test]
fn import_ignores_entries_beyond_capacity_even_when_invalid() {
    let mut text = "FF010203\n".repeat(PALETTE_SIZE);
    text.push_str("not a color\n00112233\n");
    let palette = parse_paint_net(&text).unwrap();
    assert!(
        palette
            .colors()
            .iter()
            .all(|c| *c == Color::rgba(1, 2, 3, 255))
    );
}

#[test]
fn invalid_retained_line_reports_its_original_source_line() {
    for invalid in [
        "#FF112233",
        "FF11223",
        "FF11223344",
        "FF1122GG",
        "ＦＦ112233",
        "FF11 233",
    ] {
        let input = format!("; comment\n\nFF123456\n{invalid}\n");
        assert_eq!(
            parse_paint_net(&input),
            Err(PaletteError::InvalidColor { line: 4 })
        );
    }
}

#[test]
fn replacement_bounds_preserve_palette_on_error() {
    let mut palette = Palette::default();
    let color = Color::rgba(12, 34, 56, 78);
    palette.replace(PALETTE_SIZE - 1, color).unwrap();
    assert_eq!(palette.colors()[PALETTE_SIZE - 1], color);
    let before = palette.clone();
    for index in [PALETTE_SIZE, usize::MAX] {
        assert_eq!(
            palette.replace(index, color),
            Err(PaletteError::InvalidIndex { index })
        );
        assert_eq!(palette, before);
    }
}
