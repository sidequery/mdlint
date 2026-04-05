pub mod broken_links;
pub mod first_heading;
pub mod heading_increment;
pub mod orphan_pages;
pub mod require_frontmatter;

use crate::config::{Config, RuleLevel};
use crate::file::MarkdownFile;
use crate::workspace::Workspace;
use rayon::prelude::*;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl From<RuleLevel> for Severity {
    fn from(level: RuleLevel) -> Self {
        match level {
            RuleLevel::Error => Severity::Error,
            RuleLevel::Warning => Severity::Warning,
            RuleLevel::Info => Severity::Info,
            RuleLevel::Off => Severity::Info, // shouldn't happen, but safe default
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    pub file: PathBuf,
    pub line: usize,
    pub col: usize,
    pub source_code: String,
    pub start_offset: usize,
    pub len: usize,
    pub help: Option<String>,
}

pub trait FileRule: Send + Sync {
    fn name(&self) -> &str;
    fn default_severity(&self) -> Severity;
    fn check(&self, file: &MarkdownFile, config: &Config) -> Vec<Diagnostic>;
}

pub trait WorkspaceRule: Send + Sync {
    fn name(&self) -> &str;
    fn default_severity(&self) -> Severity;
    fn check(&self, workspace: &Workspace, config: &Config) -> Vec<Diagnostic>;
}

fn severity_for_rule(name: &str, default: Severity, config: &Config) -> Option<Severity> {
    match config.rule_config(name) {
        Some(rc) => {
            if rc.is_off() {
                None
            } else {
                Some(rc.level().into())
            }
        }
        None => Some(default),
    }
}

/// Run all lint rules and collect diagnostics.
pub fn run_all(workspace: &Workspace, config: &Config) -> Vec<Diagnostic> {
    let file_rules: Vec<Box<dyn FileRule>> = vec![
        Box::new(heading_increment::HeadingIncrement),
        Box::new(require_frontmatter::RequireFrontmatter),
        Box::new(first_heading::FirstHeading),
    ];

    let workspace_rules: Vec<Box<dyn WorkspaceRule>> = vec![
        Box::new(broken_links::BrokenLinks),
        Box::new(orphan_pages::OrphanPages),
    ];

    // Phase 1: file-level rules in parallel
    let mut diagnostics: Vec<Diagnostic> = workspace
        .files
        .par_iter()
        .flat_map(|file| {
            let mut file_diags = Vec::new();
            for rule in &file_rules {
                if let Some(severity) = severity_for_rule(rule.name(), rule.default_severity(), config) {
                    let mut diags = rule.check(file, config);
                    for d in &mut diags {
                        d.severity = severity.clone();
                    }
                    file_diags.extend(diags);
                }
            }
            file_diags
        })
        .collect();

    // Phase 2: workspace-level rules
    for rule in &workspace_rules {
        if let Some(severity) = severity_for_rule(rule.name(), rule.default_severity(), config) {
            let mut diags = rule.check(workspace, config);
            for d in &mut diags {
                d.severity = severity.clone();
            }
            diagnostics.extend(diags);
        }
    }

    // Sort by file, then line
    diagnostics.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.col.cmp(&b.col))
    });

    diagnostics
}
