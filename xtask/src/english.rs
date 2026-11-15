//! Guard for the rule "no user-facing text in Rust source" (#92, #119).
//!
//! User copy lives in `apps/desktop/i18n/*.json`; Rust carries kinds
//! and data. This test scans non-test Rust source of the app crates for string
//! literals that read like English prose, a capitalized word followed later
//! by a lowercase word, and fails on any not in `ALLOWED`.
//!
//! Not flagged: comments, `#[error(..)]` developer text, `eprintln!` logs,
//! `expect`/`panic!`/`debug_assert` messages, and `#[cfg(test)]` modules (a
//! file is scanned up to its first test module). Write developer-only text in
//! lowercase so it never looks like UI copy.
//!
//! A second test applies a stricter rule to the Slint layouts in
//! `SLINT_UI`: any literal with two consecutive ASCII letters is copy, unless
//! it is an import path, an `@image-url(..)` or listed in `SLINT_ALLOWED`.
//! Layouts read text from the `Translations` global instead.

/// Crates whose source the guard scans.
#[cfg(test)]
const ROOTS: &[&str] = &[
    "crates/hoshi-core/src",
    "crates/hoshi-controller/src",
    "apps/desktop/src",
];

/// `(file, literal prefix)` pairs that are allowed, each with a reason.
#[cfg(test)]
/// Empty since the About box moved to `ui/main-window.slint` (`SLINT_ALLOWED`).
const ALLOWED: &[(&str, &str)] = &[];

/// Slint layouts the stricter `.slint` scan covers.
#[cfg(test)]
const SLINT_UI: &str = "apps/desktop/ui";

/// `(file, literal)` pairs allowed in `.slint` layouts, each with a reason.
#[cfg(test)]
const SLINT_ALLOWED: &[(&str, &str)] = &[
    // Window titles end with the product name, which is not translated.
    ("colors.slint", " · Hoshi Paint"),
    ("history.slint", " · Hoshi Paint"),
    ("shape-properties.slint", " · Hoshi Paint"),
    // The hexadecimal channel symbol, like R/G/B and H/S/V.
    ("colors.slint", "Hex"),
    // Main window: product name (default title and About heading) and font.
    ("main-window.slint", "Hoshi Paint"),
    ("main-window.slint", "DejaVu Sans"),
    // About box: copyright and license notices stay as written.
    (
        "main-window.slint",
        r"Copyright 2026 Hoshi Paint contributors\nGNU GPL version 3 only · GPL-3.0-only",
    ),
    (
        "main-window.slint",
        r"Icons: Lucide 1.71.0 · lucide.dev\nISC / MIT · Lucide and Feather contributors\nCursor adapted with contrast outline. See THIRD_PARTY_NOTICES.md.",
    ),
];

/// Literals on a `.slint` line that read as copy: two consecutive ASCII
/// letters once `\{..}` interpolations, imports and image paths are removed.
#[cfg(test)]
fn slint_copy(line: &str) -> Vec<String> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("export {")
    {
        return Vec::new();
    }
    let mut code = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find("@image-url(") {
        code.push_str(&rest[..start]);
        rest = rest[start..]
            .find(')')
            .map_or("", |end| &rest[start + end + 1..]);
    }
    code.push_str(rest);
    literals(&code)
        .into_iter()
        .filter(|text| {
            let mut plain = String::with_capacity(text.len());
            let mut depth = 0usize;
            let mut chars = text.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '\\' && chars.peek() == Some(&'{') {
                    chars.next();
                    depth += 1;
                } else if depth > 0 {
                    if c == '}' {
                        depth -= 1;
                    }
                } else {
                    plain.push(c);
                }
            }
            plain
                .as_bytes()
                .windows(2)
                .any(|pair| pair[0].is_ascii_alphabetic() && pair[1].is_ascii_alphabetic())
        })
        .collect()
}

/// Whether a line's literals are exempt: comments and developer-only text.
#[cfg(test)]
fn exempt(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//")
        || trimmed.starts_with("#[error")
        || trimmed.starts_with('*')
        || [
            "eprintln!",
            "expect(",
            "debug_assert",
            "panic!",
            "unreachable!",
        ]
        .iter()
        .any(|marker| line.contains(marker))
}

/// The contents of each double-quoted string literal on `line`.
#[cfg(test)]
fn literals(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut text = String::new();
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    if let Some(escaped) = chars.next() {
                        text.push('\\');
                        text.push(escaped);
                    }
                }
                '"' => break,
                c => text.push(c),
            }
        }
        found.push(text);
    }
    found
}

/// A capitalized word (an uppercase letter then lowercase letters), followed
/// later by a space and a lowercase letter: "Choose a file", not "PNG" or
/// "hoshi_core".
#[cfg(test)]
fn reads_like_prose(text: &str) -> bool {
    let mut chars = text.chars();
    let starts_capitalized = chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && chars.next().is_some_and(|c| c.is_ascii_lowercase());
    starts_capitalized
        && text
            .as_bytes()
            .windows(2)
            .any(|pair| pair[0] == b' ' && pair[1].is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_files(&path, files);
            } else if path.extension().is_some_and(|e| e == "rs") {
                files.push(path);
            }
        }
    }

    #[test]
    fn no_user_facing_english_in_rust_source() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut found = Vec::new();
        for dir in ROOTS {
            let mut files = Vec::new();
            rust_files(&root.join(dir), &mut files);
            files.sort();
            for file in files {
                let relative = file
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let source = std::fs::read_to_string(&file).unwrap();
                let lines: Vec<&str> = source.lines().collect();
                for (index, line) in lines.iter().enumerate() {
                    let test_module = line.trim() == "#[cfg(test)]"
                        && lines.get(index + 1).is_some_and(|next| {
                            next.trim_start().starts_with("mod ")
                                || next.trim_start().starts_with("pub mod ")
                        });
                    if test_module {
                        break;
                    }
                    if exempt(line) {
                        continue;
                    }
                    for text in literals(line) {
                        let allowed = ALLOWED
                            .iter()
                            .any(|(path, prefix)| *path == relative && text.starts_with(prefix));
                        if reads_like_prose(&text) && !allowed {
                            found.push(format!("{relative}:{}: \"{text}\"", index + 1));
                        }
                    }
                }
            }
        }
        assert!(
            found.is_empty(),
            "user-facing English in Rust source; move it to the i18n catalogs \
             (or write developer-only text in lowercase):\n{}",
            found.join("\n")
        );
    }

    #[test]
    fn no_english_literals_in_slint_layouts() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(SLINT_UI);
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|e| e == "slint"))
            .collect();
        files.sort();
        assert!(!files.is_empty(), "no .slint files under {SLINT_UI}");
        let mut found = Vec::new();
        for file in files {
            let name = file.file_name().unwrap().to_string_lossy().into_owned();
            let source = std::fs::read_to_string(&file).unwrap();
            for (index, line) in source.lines().enumerate() {
                for text in slint_copy(line) {
                    if !SLINT_ALLOWED.contains(&(name.as_str(), text.as_str())) {
                        found.push(format!("{SLINT_UI}/{name}:{}: \"{text}\"", index + 1));
                    }
                }
            }
        }
        assert!(
            found.is_empty(),
            "English in .slint layouts; add a Translations property and catalog keys:\n{}",
            found.join("\n")
        );
    }

    #[test]
    fn slint_scan_skips_imports_images_interpolation_and_symbols() {
        assert_eq!(slint_copy(r#"    Text { text: "Units"; }"#), ["Units"]);
        assert_eq!(
            slint_copy(r#"  a: "Fill pattern: " + name;"#),
            ["Fill pattern: "]
        );
        assert!(slint_copy(r#"import { Palette } from "std-widgets.slint";"#).is_empty());
        assert!(slint_copy(r#"  icon: @image-url("../icons/x.svg"); t: "R";"#).is_empty());
        assert!(slint_copy(r#"  text: @markdown("\{name}"); sep: " · ";"#).is_empty());
        assert!(slint_copy(r#"  // "Old copy""#).is_empty());
    }

    #[test]
    fn prose_detection_matches_copy_but_not_identifiers() {
        for text in [
            "Choose a file.",
            "Untitled image",
            "Palette line {line} is bad",
        ] {
            assert!(reads_like_prose(text), "{text}");
        }
        for text in [
            "PNG",
            "hoshi_core",
            "Hoshi Paint",
            "image/png",
            "error_file_io",
            "# Hoshi",
        ] {
            assert!(!reads_like_prose(text), "{text}");
        }
    }

    #[test]
    fn literal_extraction_handles_escapes_and_several_literals() {
        assert_eq!(
            literals(r#"f("a \"b\" c", "d")"#),
            [r#"a \"b\" c"#.to_string(), "d".to_string()]
        );
        assert!(exempt("    // \"Not code\""));
        assert!(exempt(r#"    #[error("cannot read {path}")]"#));
    }
}
