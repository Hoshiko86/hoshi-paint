use hoshi_controller::{Color, named_colors};
use std::collections::HashSet;

#[test]
fn catalog_has_all_standard_names_and_deterministic_display_order() {
    let catalog = named_colors::all();
    assert_eq!(catalog.len(), 149, "148 CSS/SVG names plus Transparent");
    let names: Vec<_> = catalog
        .iter()
        .map(|entry| entry.name.to_ascii_lowercase())
        .collect();
    assert_eq!(names.iter().collect::<HashSet<_>>().len(), names.len());
    assert!(names.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        catalog.iter().filter(|entry| entry.color.a == 255).count(),
        148
    );
}

#[test]
fn lookup_normalizes_case_spacing_and_hyphens_without_guessing() {
    for (name, expected) in [
        ("Alice Blue", Color::rgba(240, 248, 255, 255)),
        ("  LiGhT-bLuE  ", Color::rgba(173, 216, 230, 255)),
        ("RebeccaPurple", Color::rgba(102, 51, 153, 255)),
        ("turquoise", Color::rgba(64, 224, 208, 255)),
        ("green", Color::rgba(0, 128, 0, 255)),
        ("lime", Color::rgba(0, 255, 0, 255)),
        ("transparent", Color::TRANSPARENT),
    ] {
        assert_eq!(named_colors::lookup(name), Some(expected), "{name}");
    }
    for unknown in ["", "blu", "ultra violet", "red!", "#FF0000"] {
        assert_eq!(named_colors::lookup(unknown), None, "{unknown}");
    }
}

#[test]
fn aliases_resolve_identically_and_reverse_names_are_canonical_and_exact() {
    for (canonical, alias) in [
        ("Gray", "Grey"),
        ("DarkGray", "DarkGrey"),
        ("LightGray", "LightGrey"),
        ("SlateGray", "SlateGrey"),
        ("DarkSlateGray", "DarkSlateGrey"),
        ("LightSlateGray", "LightSlateGrey"),
        ("DimGray", "DimGrey"),
        ("Cyan", "Aqua"),
        ("Magenta", "Fuchsia"),
    ] {
        let color = named_colors::lookup(canonical).unwrap();
        assert_eq!(named_colors::lookup(alias), Some(color));
        assert_eq!(named_colors::name_for(color), Some(canonical));
    }
    assert_eq!(
        named_colors::name_for(Color::rgba(255, 0, 0, 255)),
        Some("Red")
    );
    assert_eq!(named_colors::name_for(Color::rgba(255, 0, 0, 128)), None);
    assert_eq!(named_colors::name_for(Color::rgba(254, 0, 0, 255)), None);
    assert_eq!(
        named_colors::name_for(Color::TRANSPARENT),
        Some("Transparent")
    );
    assert_eq!(named_colors::name_for(Color::rgba(1, 0, 0, 0)), None);
}

#[test]
fn suggestions_prioritize_prefixes_then_return_alphabetic_substrings() {
    let matches = named_colors::suggestions("bLuE");
    let names: Vec<_> = matches.iter().map(|entry| entry.name).collect();
    assert_eq!(&names[..2], &["Blue", "BlueViolet"]);
    assert!(
        names[2..]
            .iter()
            .all(|name| name.to_ascii_lowercase().contains("blue"))
    );
    assert!(
        names[2..]
            .windows(2)
            .all(|pair| pair[0].to_ascii_lowercase() < pair[1].to_ascii_lowercase())
    );
    assert!(names.contains(&"AliceBlue"));
    assert_eq!(
        named_colors::suggestions("light-b"),
        named_colors::suggestions("Light B")
    );
    assert_eq!(named_colors::suggestions(""), named_colors::all());
    assert!(named_colors::suggestions("not-a-color").is_empty());
}
