use comrak::nodes::{AstNode, NodeValue};
use percent_encoding::percent_decode_str;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkKind {
    /// `[text](path.md)` or `[text](path.md#anchor)`
    Standard,
    /// `[[page]]` or `[[page|alias]]` or `[[page#heading]]`
    WikiLink,
    /// `![alt](image.png)`
    Image,
}

#[derive(Debug, Clone)]
pub struct Link {
    pub kind: LinkKind,
    pub raw_target: String,
    pub file_target: Option<String>,
    pub anchor: Option<String>,
    pub display_text: String,
    pub line: usize,
    pub col: usize,
    /// Byte offset in source for diagnostic spans
    pub start_offset: usize,
    pub len: usize,
}

impl Link {
    /// Whether this is an external URL (http, https, mailto, etc.)
    pub fn is_external(&self) -> bool {
        let t = &self.raw_target;
        t.starts_with("http://")
            || t.starts_with("https://")
            || t.starts_with("mailto:")
            || t.starts_with("tel:")
    }

    /// Decoded file target path (percent-decoded).
    pub fn decoded_file_target(&self) -> Option<String> {
        self.file_target.as_ref().map(|t| {
            percent_decode_str(t)
                .decode_utf8_lossy()
                .into_owned()
        })
    }
}

/// Extract all links from a comrak AST.
pub fn extract_links<'a>(root: &'a AstNode<'a>, source: &str) -> Vec<Link> {
    let mut links = Vec::new();
    collect_links(root, source, &mut links);
    links
}

fn collect_links<'a>(node: &'a AstNode<'a>, source: &str, out: &mut Vec<Link>) {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Link(link) => {
            let url = &link.url;
            let display = collect_text_children(node);
            let (file_target, anchor) = split_url_anchor(url);
            let sp = &data.sourcepos;
            let start_offset = line_col_to_offset(source, sp.start.line, sp.start.column);
            let end_offset = line_col_to_offset(source, sp.end.line, sp.end.column);
            out.push(Link {
                kind: LinkKind::Standard,
                raw_target: url.clone(),
                file_target,
                anchor,
                display_text: display,
                line: sp.start.line,
                col: sp.start.column,
                start_offset,
                len: end_offset.saturating_sub(start_offset).max(1),
            });
        }
        NodeValue::Image(link) => {
            let url = &link.url;
            let display = collect_text_children(node);
            let (file_target, _anchor) = split_url_anchor(url);
            let sp = &data.sourcepos;
            let start_offset = line_col_to_offset(source, sp.start.line, sp.start.column);
            let end_offset = line_col_to_offset(source, sp.end.line, sp.end.column);
            out.push(Link {
                kind: LinkKind::Image,
                raw_target: url.clone(),
                file_target,
                anchor: None,
                display_text: display,
                line: sp.start.line,
                col: sp.start.column,
                start_offset,
                len: end_offset.saturating_sub(start_offset).max(1),
            });
        }
        NodeValue::WikiLink(wl) => {
            let url = &wl.url;
            let (file_target, anchor) = split_wikilink(url);
            let sp = &data.sourcepos;
            let start_offset = line_col_to_offset(source, sp.start.line, sp.start.column);
            let end_offset = line_col_to_offset(source, sp.end.line, sp.end.column);
            out.push(Link {
                kind: LinkKind::WikiLink,
                raw_target: url.clone(),
                file_target: Some(file_target),
                anchor,
                display_text: collect_text_children(node),
                line: sp.start.line,
                col: sp.start.column,
                start_offset,
                len: end_offset.saturating_sub(start_offset).max(1),
            });
        }
        _ => {}
    }
    drop(data);
    for child in node.children() {
        collect_links(child, source, out);
    }
}

/// Collect text from immediate children of a node.
fn collect_text_children<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();
    for child in node.children() {
        collect_text_recursive(child, &mut text);
    }
    text
}

fn collect_text_recursive<'a>(node: &'a AstNode<'a>, buf: &mut String) {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Text(t) | NodeValue::Code(comrak::nodes::NodeCode { literal: t, .. }) => {
            buf.push_str(t);
        }
        NodeValue::SoftBreak | NodeValue::LineBreak => buf.push(' '),
        _ => {}
    }
    drop(data);
    for child in node.children() {
        collect_text_recursive(child, buf);
    }
}

/// Split a standard markdown URL into file path and anchor.
/// `path.md#heading` -> (Some("path.md"), Some("heading"))
/// `#heading` -> (None, Some("heading"))
/// `path.md` -> (Some("path.md"), None)
fn split_url_anchor(url: &str) -> (Option<String>, Option<String>) {
    // Skip external URLs
    if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("mailto:")
        || url.starts_with("tel:")
    {
        return (Some(url.to_string()), None);
    }

    if let Some(hash_pos) = url.find('#') {
        let path = &url[..hash_pos];
        let anchor = &url[hash_pos + 1..];
        let file_target = if path.is_empty() {
            None
        } else {
            Some(path.to_string())
        };
        let anchor = if anchor.is_empty() {
            None
        } else {
            Some(anchor.to_string())
        };
        (file_target, anchor)
    } else if url.is_empty() {
        (None, None)
    } else {
        (Some(url.to_string()), None)
    }
}

/// Split a wikilink target: `page#heading` -> ("page", Some("heading"))
fn split_wikilink(url: &str) -> (String, Option<String>) {
    if let Some(hash_pos) = url.find('#') {
        let page = url[..hash_pos].to_string();
        let anchor = url[hash_pos + 1..].to_string();
        let anchor = if anchor.is_empty() { None } else { Some(anchor) };
        (page, anchor)
    } else {
        (url.to_string(), None)
    }
}

/// Convert a 1-based line and column to a byte offset in source.
pub fn line_col_to_offset(source: &str, line: usize, col: usize) -> usize {
    let mut current_line = 1;
    let mut offset = 0;
    for (i, ch) in source.char_indices() {
        if current_line == line {
            // col is 1-based
            let target_offset = i + col.saturating_sub(1);
            return target_offset.min(source.len());
        }
        if ch == '\n' {
            current_line += 1;
        }
        offset = i + ch.len_utf8();
    }
    offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_url_anchor() {
        assert_eq!(
            split_url_anchor("path.md#heading"),
            (Some("path.md".into()), Some("heading".into()))
        );
        assert_eq!(
            split_url_anchor("#heading"),
            (None, Some("heading".into()))
        );
        assert_eq!(
            split_url_anchor("path.md"),
            (Some("path.md".into()), None)
        );
        assert_eq!(
            split_url_anchor("https://example.com"),
            (Some("https://example.com".into()), None)
        );
    }

    #[test]
    fn test_split_wikilink() {
        assert_eq!(
            split_wikilink("page#heading"),
            ("page".into(), Some("heading".into()))
        );
        assert_eq!(split_wikilink("page"), ("page".into(), None));
    }

    #[test]
    fn test_line_col_to_offset() {
        let source = "line1\nline2\nline3";
        assert_eq!(line_col_to_offset(source, 1, 1), 0);
        assert_eq!(line_col_to_offset(source, 2, 1), 6);
        assert_eq!(line_col_to_offset(source, 3, 1), 12);
        assert_eq!(line_col_to_offset(source, 2, 3), 8);
    }
}
