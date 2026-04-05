use crate::config::Config;
use crate::links::LinkKind;
use crate::resolve;
use crate::workspace::Workspace;
use std::collections::HashSet;
use super::{Diagnostic, Severity, WorkspaceRule};

pub struct OrphanPages;

impl WorkspaceRule for OrphanPages {
    fn name(&self) -> &str {
        "orphan-pages"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn opt_in(&self) -> bool { true }

    fn check(&self, workspace: &Workspace, config: &Config) -> Vec<Diagnostic> {
        let exclude_patterns: Vec<String> = config
            .rule_config(self.name())
            .map(|rc| rc.option_strs("exclude"))
            .unwrap_or_else(|| vec!["index.md".to_string(), "README.md".to_string()]);

        // Build set of files that are referenced by at least one link
        let mut referenced: HashSet<&std::path::Path> = HashSet::new();

        for file in &workspace.files {
            for link in &file.links {
                if link.is_external() {
                    continue;
                }
                // Only count file-targeting links (not pure anchor links)
                if link.file_target.is_none() && link.kind != LinkKind::WikiLink {
                    continue;
                }

                if let Ok(Some(resolved)) = resolve::resolve_link(link, &file.path, workspace, config) {
                    // Find the workspace file matching this resolved path
                    for target_file in &workspace.files {
                        if target_file.path == resolved {
                            referenced.insert(&target_file.path);
                            break;
                        }
                    }
                }
            }
        }

        // Flag files with no incoming links
        let mut diagnostics = Vec::new();
        for file in &workspace.files {
            if referenced.contains(file.path.as_path()) {
                continue;
            }

            // Check exclude patterns
            let rel = &file.relative_path;
            let rel_str = rel.to_string_lossy();
            let file_name = rel
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            let excluded = exclude_patterns.iter().any(|pattern| {
                file_name == pattern
                    || rel_str == *pattern
                    || (pattern.contains('*') && glob_match(pattern, &rel_str))
            });

            if excluded {
                continue;
            }

            diagnostics.push(Diagnostic {
                rule: self.name().to_string(),
                severity: self.default_severity(),
                message: format!("page has no incoming links"),
                file: file.path.clone(),
                line: 1,
                col: 1,
                source_code: file.content.clone(),
                start_offset: 0,
                len: file.content.find('\n').unwrap_or(file.content.len()).min(40),
                help: Some(format!(
                    "no other file links to `{}`",
                    file.relative_path.display()
                )),
            });
        }

        diagnostics
    }
}

/// Simple glob matching supporting only `*` as a wildcard for any sequence of non-`/` chars.
fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }

    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        match text[pos..].find(part) {
            Some(found) => {
                if i == 0 && found != 0 {
                    return false; // pattern doesn't start with *, so must match from beginning
                }
                pos += found + part.len();
            }
            None => return false,
        }
    }

    // If pattern doesn't end with *, text must end at pos
    if !pattern.ends_with('*') {
        return pos == text.len();
    }

    true
}
