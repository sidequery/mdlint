use crate::rules::{Diagnostic, Severity};
use miette::{Diagnostic as MietteDiag, GraphicalReportHandler, GraphicalTheme, NamedSource, SourceSpan};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Pretty,
    Json,
    Short,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Pretty => write!(f, "pretty"),
            OutputFormat::Json => write!(f, "json"),
            OutputFormat::Short => write!(f, "short"),
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pretty" => Ok(OutputFormat::Pretty),
            "json" => Ok(OutputFormat::Json),
            "short" => Ok(OutputFormat::Short),
            _ => Err(format!("unknown format: {s} (expected pretty, json, or short)")),
        }
    }
}

pub fn print_diagnostics(diagnostics: &[Diagnostic], format: OutputFormat) {
    match format {
        OutputFormat::Pretty => print_pretty(diagnostics),
        OutputFormat::Json => print_json(diagnostics),
        OutputFormat::Short => print_short(diagnostics),
    }
}

fn print_pretty(diagnostics: &[Diagnostic]) {
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());

    for diag in diagnostics {
        let report = DiagnosticReport::from(diag);
        let mut buf = String::new();
        if handler.render_report(&mut buf, &report).is_ok() {
            eprint!("{buf}");
        }
    }

    let error_count = diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
    let warning_count = diagnostics.iter().filter(|d| d.severity == Severity::Warning).count();

    if diagnostics.is_empty() {
        eprintln!("No issues found.");
    } else {
        eprintln!(
            "\nFound {} error(s), {} warning(s) across {} file(s).",
            error_count,
            warning_count,
            {
                let mut files: Vec<_> = diagnostics.iter().map(|d| &d.file).collect();
                files.dedup();
                files.len()
            }
        );
    }
}

fn print_short(diagnostics: &[Diagnostic]) {
    for diag in diagnostics {
        let severity = match diag.severity {
            Severity::Error => "error",
            Severity::Warning => "warn",
            Severity::Info => "info",
        };
        eprintln!(
            "{}:{}:{}: {}[{}] {}",
            diag.file.display(),
            diag.line,
            diag.col,
            severity,
            diag.rule,
            diag.message,
        );
    }
}

fn print_json(diagnostics: &[Diagnostic]) {
    #[derive(serde::Serialize)]
    struct JsonDiag<'a> {
        rule: &'a str,
        severity: &'a str,
        message: &'a str,
        file: String,
        line: usize,
        col: usize,
        help: Option<&'a str>,
    }

    let items: Vec<JsonDiag> = diagnostics
        .iter()
        .map(|d| JsonDiag {
            rule: &d.rule,
            severity: match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            },
            message: &d.message,
            file: d.file.display().to_string(),
            line: d.line,
            col: d.col,
            help: d.help.as_deref(),
        })
        .collect();

    // Print to stdout for JSON (machine-readable)
    if let Ok(json) = serde_json::to_string_pretty(&items) {
        println!("{json}");
    }
}

// Adapter to make our Diagnostic work with miette's rendering
#[derive(Debug)]
struct DiagnosticReport {
    severity_prefix: String,
    message: String,
    rule: String,
    source: NamedSource<String>,
    span: SourceSpan,
    help: Option<String>,
}

impl From<&Diagnostic> for DiagnosticReport {
    fn from(d: &Diagnostic) -> Self {
        let severity_prefix = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        };
        Self {
            severity_prefix: severity_prefix.to_string(),
            message: d.message.clone(),
            rule: d.rule.clone(),
            source: NamedSource::new(d.file.display().to_string(), d.source_code.clone()),
            span: SourceSpan::from((d.start_offset, d.len)),
            help: d.help.clone(),
        }
    }
}

impl fmt::Display for DiagnosticReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DiagnosticReport {}

impl MietteDiag for DiagnosticReport {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new(&self.rule))
    }

    fn severity(&self) -> Option<miette::Severity> {
        Some(match self.severity_prefix.as_str() {
            "error" => miette::Severity::Error,
            "warning" => miette::Severity::Warning,
            _ => miette::Severity::Advice,
        })
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.help.as_ref().map(|h| Box::new(h) as Box<dyn fmt::Display>)
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.source)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        Some(Box::new(std::iter::once(
            miette::LabeledSpan::new_with_span(None, self.span),
        )))
    }
}
