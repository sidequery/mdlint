use std::collections::HashMap;

/// Parsed YAML frontmatter.
#[derive(Debug, Clone, Default)]
pub struct Frontmatter {
    pub fields: HashMap<String, serde_yml::Value>,
}

impl Frontmatter {
    pub fn has_field(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    /// Get string aliases from frontmatter (aliases field).
    pub fn aliases(&self) -> Vec<String> {
        match self.fields.get("aliases") {
            Some(serde_yml::Value::Sequence(seq)) => seq
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            Some(serde_yml::Value::String(s)) => vec![s.clone()],
            _ => vec![],
        }
    }

    pub fn title(&self) -> Option<&str> {
        self.fields
            .get("title")
            .and_then(|v| v.as_str())
    }
}

/// Parse YAML frontmatter from the raw frontmatter string extracted by comrak.
/// comrak includes the `---` delimiters in the raw string, so we strip them.
pub fn parse_frontmatter(raw: &str) -> Option<Frontmatter> {
    let content = strip_delimiters(raw);
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Some(Frontmatter::default());
    }

    match serde_yml::from_str::<HashMap<String, serde_yml::Value>>(trimmed) {
        Ok(fields) => Some(Frontmatter { fields }),
        Err(_) => None, // invalid YAML, we'll let a rule report this
    }
}

fn strip_delimiters(raw: &str) -> &str {
    let s = raw.trim();
    let s = s.strip_prefix("---").unwrap_or(s);
    let s = s.strip_suffix("---").unwrap_or(s);
    s.trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let raw = "title: Hello\ntags:\n  - rust\n  - markdown";
        let fm = parse_frontmatter(raw).unwrap();
        assert_eq!(fm.title(), Some("Hello"));
        assert!(fm.has_field("tags"));
    }

    #[test]
    fn test_aliases() {
        let raw = "aliases:\n  - foo\n  - bar";
        let fm = parse_frontmatter(raw).unwrap();
        assert_eq!(fm.aliases(), vec!["foo", "bar"]);
    }

    #[test]
    fn test_empty_frontmatter() {
        let fm = parse_frontmatter("").unwrap();
        assert!(fm.fields.is_empty());
    }

    #[test]
    fn test_comrak_frontmatter_with_delimiters() {
        // comrak may include the --- delimiters in the raw string
        let raw = "---\ntitle: Hello\ntags:\n  - rust\n---\n";
        let fm = parse_frontmatter(raw).unwrap();
        assert_eq!(fm.title(), Some("Hello"));
    }
}
