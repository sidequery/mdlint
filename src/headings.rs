use comrak::nodes::{AstNode, NodeValue};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub slug: String,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SlugMode {
    #[default]
    Gfm,
    Obsidian,
}

/// Extract all headings from a comrak AST.
pub fn extract_headings<'a>(root: &'a AstNode<'a>, mode: SlugMode) -> Vec<Heading> {
    let mut headings = Vec::new();
    collect_headings(root, mode, &mut headings);
    deduplicate_slugs(&mut headings);
    headings
}

fn collect_headings<'a>(node: &'a AstNode<'a>, mode: SlugMode, out: &mut Vec<Heading>) {
    let data = node.data.borrow();
    if let NodeValue::Heading(ref h) = data.value {
        let text = collect_text(node);
        let slug = match mode {
            SlugMode::Gfm => gfm_slug(&text),
            SlugMode::Obsidian => text.clone(), // obsidian matches exact text, case-insensitive
        };
        out.push(Heading {
            level: h.level,
            text,
            slug,
            line: data.sourcepos.start.line,
            col: data.sourcepos.start.column,
        });
    }
    drop(data);
    for child in node.children() {
        collect_headings(child, mode, out);
    }
}

/// Collect all plain text from a node and its descendants.
fn collect_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();
    collect_text_inner(node, &mut text);
    text
}

fn collect_text_inner<'a>(node: &'a AstNode<'a>, buf: &mut String) {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Text(t) | NodeValue::Code(comrak::nodes::NodeCode { literal: t, .. }) => {
            buf.push_str(t);
        }
        NodeValue::SoftBreak | NodeValue::LineBreak => {
            buf.push(' ');
        }
        _ => {}
    }
    drop(data);
    for child in node.children() {
        collect_text_inner(child, buf);
    }
}

/// GitHub Flavored Markdown slug generation.
/// 1. NFC normalize unicode
/// 2. Lowercase
/// 3. Remove anything not alphanumeric, space, or hyphen (keep unicode letters/numbers)
/// 4. Spaces to hyphens
fn gfm_slug(text: &str) -> String {
    let normalized: String = text.nfc().collect();
    let lowered = normalized.to_lowercase();
    let mut slug = String::with_capacity(lowered.len());
    for ch in lowered.chars() {
        if ch.is_alphanumeric() || ch == '-' || ch == '_' {
            slug.push(ch);
        } else if ch == ' ' {
            slug.push('-');
        }
        // everything else is dropped
    }
    slug
}

/// Append -1, -2, etc. for duplicate slugs (GFM behavior).
fn deduplicate_slugs(headings: &mut [Heading]) {
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for h in headings.iter_mut() {
        let count = seen.entry(h.slug.clone()).or_insert(0);
        if *count > 0 {
            h.slug = format!("{}-{}", h.slug, count);
        }
        *count += 1;
    }
}

/// Check if a slug matches a heading, respecting the slug mode.
pub fn slug_matches(heading: &Heading, anchor: &str, mode: SlugMode) -> bool {
    match mode {
        SlugMode::Gfm => heading.slug == anchor,
        SlugMode::Obsidian => heading.text.eq_ignore_ascii_case(anchor),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gfm_slug_basic() {
        assert_eq!(gfm_slug("Hello World"), "hello-world");
        assert_eq!(gfm_slug("Hello, World!"), "hello-world");
        assert_eq!(gfm_slug("C++ Programming"), "c-programming");
        assert_eq!(gfm_slug("What's New?"), "whats-new");
    }

    #[test]
    fn test_gfm_slug_unicode() {
        assert_eq!(gfm_slug("Hllo Wrld"), "hllo-wrld");
    }

    #[test]
    fn test_gfm_slug_numbers() {
        assert_eq!(gfm_slug("2024 Updates"), "2024-updates");
    }

    #[test]
    fn test_deduplicate_slugs() {
        let mut headings = vec![
            Heading { level: 2, text: "Foo".into(), slug: "foo".into(), line: 1, col: 1 },
            Heading { level: 2, text: "Foo".into(), slug: "foo".into(), line: 5, col: 1 },
            Heading { level: 2, text: "Foo".into(), slug: "foo".into(), line: 9, col: 1 },
        ];
        deduplicate_slugs(&mut headings);
        assert_eq!(headings[0].slug, "foo");
        assert_eq!(headings[1].slug, "foo-1");
        assert_eq!(headings[2].slug, "foo-2");
    }
}
