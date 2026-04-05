use crate::config::Config;
use crate::file::MarkdownFile;
use super::{Diagnostic, FileRule, Severity};

pub struct HeadingIncrement;

impl FileRule for HeadingIncrement {
    fn name(&self) -> &str {
        "heading-increment"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, file: &MarkdownFile, _config: &Config) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut prev_level: Option<u8> = None;

        for heading in &file.headings {
            if let Some(prev) = prev_level {
                if heading.level > prev + 1 {
                    diagnostics.push(Diagnostic {
                        rule: self.name().to_string(),
                        severity: self.default_severity(),
                        message: format!(
                            "heading level skipped: h{} -> h{}",
                            prev, heading.level
                        ),
                        file: file.path.clone(),
                        line: heading.line,
                        col: heading.col,
                        source_code: file.content.clone(),
                        start_offset: crate::links::line_col_to_offset(
                            &file.content,
                            heading.line,
                            heading.col,
                        ),
                        len: heading.text.len() + heading.level as usize + 1,
                        help: Some(format!(
                            "expected h{} or lower, found h{}",
                            prev + 1,
                            heading.level
                        )),
                    });
                }
            }
            prev_level = Some(heading.level);
        }

        diagnostics
    }
}
