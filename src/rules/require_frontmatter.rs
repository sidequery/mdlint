use crate::config::Config;
use crate::file::MarkdownFile;
use super::{Diagnostic, FileRule, Severity};

pub struct RequireFrontmatter;

impl FileRule for RequireFrontmatter {
    fn name(&self) -> &str {
        "require-frontmatter"
    }

    fn default_severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, file: &MarkdownFile, config: &Config) -> Vec<Diagnostic> {
        let required_fields = match config.rule_config(self.name()) {
            Some(rc) => rc.option_strs("fields"),
            None => return vec![], // no fields configured, nothing to check
        };

        if required_fields.is_empty() {
            return vec![];
        }

        let mut diagnostics = Vec::new();

        match &file.frontmatter {
            None => {
                diagnostics.push(Diagnostic {
                    rule: self.name().to_string(),
                    severity: self.default_severity(),
                    message: "missing frontmatter".to_string(),
                    file: file.path.clone(),
                    line: 1,
                    col: 1,
                    source_code: file.content.clone(),
                    start_offset: 0,
                    len: file.content.find('\n').unwrap_or(file.content.len()).min(40),
                    help: Some(format!(
                        "required fields: {}",
                        required_fields.join(", ")
                    )),
                });
            }
            Some(fm) => {
                for field in &required_fields {
                    if !fm.has_field(field) {
                        diagnostics.push(Diagnostic {
                            rule: self.name().to_string(),
                            severity: self.default_severity(),
                            message: format!("missing required frontmatter field: {field}"),
                            file: file.path.clone(),
                            line: 1,
                            col: 1,
                            source_code: file.content.clone(),
                            start_offset: 0,
                            len: 3, // length of "---"
                            help: None,
                        });
                    }
                }
            }
        }

        diagnostics
    }
}
