use crate::config::Config;
use crate::file::MarkdownFile;
use super::{Diagnostic, FileRule, Severity};

pub struct FirstHeading;

impl FileRule for FirstHeading {
    fn name(&self) -> &str {
        "first-heading"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, file: &MarkdownFile, config: &Config) -> Vec<Diagnostic> {
        let required_level = config
            .rule_config(self.name())
            .and_then(|rc| rc.option_u8("level"))
            .unwrap_or(1);

        let first = match file.headings.first() {
            Some(h) => h,
            None => return vec![], // no headings, nothing to check
        };

        if first.level != required_level {
            vec![Diagnostic {
                rule: self.name().to_string(),
                severity: self.default_severity(),
                message: format!(
                    "first heading should be h{}, found h{}",
                    required_level, first.level
                ),
                file: file.path.clone(),
                line: first.line,
                col: first.col,
                source_code: file.content.clone(),
                start_offset: crate::links::line_col_to_offset(
                    &file.content,
                    first.line,
                    first.col,
                ),
                len: first.text.len() + first.level as usize + 1,
                help: None,
            }]
        } else {
            vec![]
        }
    }
}
