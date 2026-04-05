use crate::config::Config;
use crate::resolve::{self, ResolveError};
use crate::workspace::Workspace;
use super::{Diagnostic, Severity, WorkspaceRule};

pub struct BrokenLinks;

impl WorkspaceRule for BrokenLinks {
    fn name(&self) -> &str {
        "broken-links"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, workspace: &Workspace, config: &Config) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for file in &workspace.files {
            for link in &file.links {
                if link.should_skip() {
                    continue;
                }
                if link.is_external() && !config.links.check_external {
                    continue;
                }

                match resolve::resolve_link(link, &file.path, workspace, config) {
                    Ok(_) => {}
                    Err(ResolveError::FileNotFound { target }) => {
                        diagnostics.push(Diagnostic {
                            rule: self.name().to_string(),
                            severity: Severity::Error,
                            message: format!("link target not found: {target}"),
                            file: file.path.clone(),
                            line: link.line,
                            col: link.col,
                            source_code: file.content.clone(),
                            start_offset: link.start_offset,
                            len: link.len,
                            help: Some(format!(
                                "file `{target}` does not exist relative to `{}`",
                                file.relative_path.parent().unwrap_or(&file.relative_path).display()
                            )),
                        });
                    }
                    Err(ResolveError::AnchorNotFound { file: target_file, anchor }) => {
                        let relative_target = target_file
                            .strip_prefix(&workspace.root)
                            .unwrap_or(&target_file);
                        diagnostics.push(Diagnostic {
                            rule: self.name().to_string(),
                            severity: Severity::Error,
                            message: format!("heading anchor not found: #{anchor}"),
                            file: file.path.clone(),
                            line: link.line,
                            col: link.col,
                            source_code: file.content.clone(),
                            start_offset: link.start_offset,
                            len: link.len,
                            help: Some(format!(
                                "no heading matching `#{anchor}` in `{}`",
                                relative_target.display()
                            )),
                        });
                    }
                    Err(ResolveError::AmbiguousWikilink { target, candidates }) => {
                        let candidate_list: Vec<String> = candidates
                            .iter()
                            .map(|c| {
                                c.strip_prefix(&workspace.root)
                                    .unwrap_or(c)
                                    .display()
                                    .to_string()
                            })
                            .collect();
                        diagnostics.push(Diagnostic {
                            rule: self.name().to_string(),
                            severity: Severity::Warning,
                            message: format!("ambiguous wikilink: [[{target}]]"),
                            file: file.path.clone(),
                            line: link.line,
                            col: link.col,
                            source_code: file.content.clone(),
                            start_offset: link.start_offset,
                            len: link.len,
                            help: Some(format!(
                                "multiple files match: {}",
                                candidate_list.join(", ")
                            )),
                        });
                    }
                    Err(ResolveError::CaseMismatch { target, actual }) => {
                        let actual_name = actual
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("?");
                        diagnostics.push(Diagnostic {
                            rule: self.name().to_string(),
                            severity: Severity::Warning,
                            message: format!("case mismatch: `{target}` vs `{actual_name}`"),
                            file: file.path.clone(),
                            line: link.line,
                            col: link.col,
                            source_code: file.content.clone(),
                            start_offset: link.start_offset,
                            len: link.len,
                            help: Some(format!(
                                "link uses `{target}` but file is `{actual_name}` (may break on case-sensitive filesystems)"
                            )),
                        });
                    }
                }
            }
        }

        diagnostics
    }
}
