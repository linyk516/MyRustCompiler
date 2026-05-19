use crate::compiler::Compiler;
use crate::compiler::diagnostic::{Diagnostic, DiagnosticLabel, Severity};
use crate::compiler::output::{CompileOutcome, CompileOutput};
use crate::compiler::source::SourceFile;
use crate::lexer::token::{Token, TokenKind};
use crate::parser::CstSpanDisplayMode;
use std::fmt::Write;

#[derive(Clone, Copy)]
enum AnsiStyle {
    Bold,
    Red,
    RedBold,
    Yellow,
    YellowBold,
    Cyan,
    CyanBold,
    BlueBold,
    GreenBold,
}

impl AnsiStyle {
    fn code(self) -> &'static str {
        match self {
            AnsiStyle::Bold => "\x1b[1m",
            AnsiStyle::Red => "\x1b[31m",
            AnsiStyle::RedBold => "\x1b[31;1m",
            AnsiStyle::Yellow => "\x1b[33m",
            AnsiStyle::YellowBold => "\x1b[33;1m",
            AnsiStyle::Cyan => "\x1b[36m",
            AnsiStyle::CyanBold => "\x1b[36;1m",
            AnsiStyle::BlueBold => "\x1b[34;1m",
            AnsiStyle::GreenBold => "\x1b[32;1m",
        }
    }
}

pub struct RenderConfig {
    pub verbose: bool,
    pub show_notes: bool,
    pub show_help: bool,
    pub show_tokens: bool,
    pub color: bool,
}

impl RenderConfig {
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose,
            show_notes: true,
            show_help: true,
            show_tokens: false,
            color: false,
        }
    }

    pub fn with_show_tokens(mut self, show_tokens: bool) -> Self {
        self.show_tokens = show_tokens;
        self
    }

    pub fn with_color(mut self, color: bool) -> Self {
        self.color = color;
        self
    }
}

pub struct RenderedOutput {
    pub stdout: String,
    pub stderr: String,
}

pub struct CliRenderer {
    config: RenderConfig,
}

impl CliRenderer {
    pub fn new(config: RenderConfig) -> Self {
        Self { config }
    }

    pub fn render_outcome(&self, compiler: &Compiler, outcome: &CompileOutcome) -> RenderedOutput {
        let mut stdout = String::new();
        let mut stderr = String::new();

        for diagnostic in &outcome.diagnostics {
            self.render_diagnostic(&mut stderr, diagnostic, &outcome.source);
        }

        if let Some(output) = &outcome.output {
            self.render_summary(&mut stdout, outcome, output);
            if self.config.show_tokens {
                self.render_tokens(&mut stdout, output, &outcome.source);
            }
            if self.config.verbose {
                let cst = compiler.display_cst_with_mode(
                    output,
                    &outcome.source,
                    CstSpanDisplayMode::Range,
                );
                let _ = writeln!(stdout);
                let _ = writeln!(stdout, "CST");
                let _ = writeln!(stdout, "{}", cst);
            }
        } else if outcome.has_errors() {
            let _ = writeln!(
                stderr,
                "{}",
                self.paint("Compile failed.", AnsiStyle::RedBold)
            );
        }

        RenderedOutput { stdout, stderr }
    }

    fn render_summary(&self, out: &mut String, outcome: &CompileOutcome, output: &CompileOutput) {
        let title = if outcome.has_errors() {
            "Compile finished with diagnostics"
        } else {
            "Compile succeeded"
        };
        let _ = writeln!(out, "{}", title);
        let _ = writeln!(out);
        let _ = writeln!(out, "{:<12}{}", "File", self.source_name(&outcome.source));
        let _ = writeln!(out, "{:<12}{}", "Stage", "lexer + parser");
        let _ = writeln!(out, "{:<12}{}", "Tokens", output.tokens().len());
        let _ = writeln!(out, "{:<12}{}", "CST nodes", output.cst().nodes.len());
        let _ = writeln!(out, "{:<12}{}", "Diagnostics", outcome.diagnostics.len());
    }

    fn render_tokens(&self, out: &mut String, output: &CompileOutput, source: &SourceFile) {
        let _ = writeln!(out);
        let _ = writeln!(out, "Tokens");
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "{:<6}{:<24}{:<24}{:<14}{}",
            "#", "Kind", "Text", "Span", "Line:Col"
        );

        for (index, token) in output.tokens().iter().enumerate() {
            let _ = writeln!(
                out,
                "{:<6}{:<24}{:<24}{:<14}{}",
                index,
                self.truncate_text(&format!("{:?}", token.kind), 23),
                self.truncate_text(&self.token_text(token, source), 23),
                format!("{}..{}", token.span.start, token.span.end),
                self.token_position(token, source)
            );
        }
    }

    fn render_diagnostic(&self, out: &mut String, diagnostic: &Diagnostic, source: &SourceFile) {
        let _ = writeln!(
            out,
            "{}{}: {}",
            self.paint(
                self.severity_name(&diagnostic.severity),
                self.severity_style(&diagnostic.severity)
            ),
            self.paint(format!("[{}]", diagnostic.code), AnsiStyle::Bold),
            diagnostic.message
        );

        if let Some(label) = self.primary_label(diagnostic) {
            self.render_source_location(out, diagnostic, label, source);
            self.render_source_snippet(out, label, source, &diagnostic.severity);
        }

        for label in diagnostic.labels.iter().filter(|label| !label.primary) {
            let _ = writeln!(out, "  = {}", label.message);
        }

        if self.config.show_notes {
            for note in &diagnostic.notes {
                let _ = writeln!(
                    out,
                    "  = {}: {}",
                    self.paint("note", AnsiStyle::CyanBold),
                    note
                );
            }
        }

        if self.config.show_help {
            if let Some(help) = &diagnostic.help {
                let _ = writeln!(
                    out,
                    "  = {}: {}",
                    self.paint("help", AnsiStyle::GreenBold),
                    help
                );
            }
        }

        let _ = writeln!(out);
    }

    fn render_source_location(
        &self,
        out: &mut String,
        diagnostic: &Diagnostic,
        label: &DiagnosticLabel,
        source: &SourceFile,
    ) {
        let path = diagnostic
            .source_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| self.source_name(source));
        if let Some(range) = &label.range {
            let _ = writeln!(
                out,
                " {} {}:{}:{}",
                self.paint("-->", AnsiStyle::BlueBold),
                path,
                range.start.line,
                range.start.column
            );
        } else {
            let _ = writeln!(out, " {} {}", self.paint("-->", AnsiStyle::BlueBold), path);
        }
    }

    fn render_source_snippet(
        &self,
        out: &mut String,
        label: &DiagnosticLabel,
        source: &SourceFile,
        severity: &Severity,
    ) {
        let Some(range) = &label.range else {
            let _ = writeln!(out, "  = {}", label.message);
            return;
        };

        let line_index = range.start.line.saturating_sub(1);
        let Some(line_text) = source.line_text(line_index) else {
            let _ = writeln!(out, "  = {}", label.message);
            return;
        };

        let line_text = line_text.trim_end_matches(['\r', '\n']);
        let line_no = range.start.line;
        let gutter_width = line_no.to_string().len();
        let caret_start = range.start.column.saturating_sub(1);
        let caret_len = if range.start.line == range.end.line {
            range.end.column.saturating_sub(range.start.column).max(1)
        } else {
            line_text.chars().count().saturating_sub(caret_start).max(1)
        };

        let gutter = self.paint("|", AnsiStyle::BlueBold);
        let _ = writeln!(out, "{:>width$} {}", "", gutter, width = gutter_width);
        let _ = writeln!(
            out,
            "{:>width$} {} {}",
            line_no,
            gutter,
            line_text,
            width = gutter_width
        );
        let marker = format!("{}{}", " ".repeat(caret_start), "^".repeat(caret_len));
        let _ = writeln!(
            out,
            "{:>width$} {} {} {}",
            "",
            gutter,
            self.paint(marker, self.marker_style(severity)),
            self.paint(&label.message, self.marker_style(severity)),
            width = gutter_width
        );
    }

    fn primary_label<'a>(&self, diagnostic: &'a Diagnostic) -> Option<&'a DiagnosticLabel> {
        diagnostic
            .labels
            .iter()
            .find(|label| label.primary)
            .or_else(|| diagnostic.labels.first())
    }

    fn severity_name(&self, severity: &Severity) -> &'static str {
        match severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    fn source_name(&self, source: &SourceFile) -> String {
        source
            .path()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<memory>".to_string())
    }

    fn severity_style(&self, severity: &Severity) -> AnsiStyle {
        match severity {
            Severity::Error => AnsiStyle::RedBold,
            Severity::Warning => AnsiStyle::YellowBold,
            Severity::Info => AnsiStyle::CyanBold,
        }
    }

    fn marker_style(&self, severity: &Severity) -> AnsiStyle {
        match severity {
            Severity::Error => AnsiStyle::Red,
            Severity::Warning => AnsiStyle::Yellow,
            Severity::Info => AnsiStyle::Cyan,
        }
    }

    fn paint(&self, text: impl AsRef<str>, style: AnsiStyle) -> String {
        if self.config.color {
            format!("{}{}\x1b[0m", style.code(), text.as_ref())
        } else {
            text.as_ref().to_string()
        }
    }

    fn token_text(&self, token: &Token, source: &SourceFile) -> String {
        if matches!(&token.kind, TokenKind::Eof) {
            return "<eof>".to_string();
        }

        match token.span.text(source.text()) {
            Some(text) => text
                .chars()
                .flat_map(|ch| ch.escape_default())
                .collect::<String>(),
            None => "<invalid span>".to_string(),
        }
    }

    fn token_position(&self, token: &Token, source: &SourceFile) -> String {
        source
            .line_utf8_col(token.span.start)
            .map(|(line, column)| format!("{}:{}", line + 1, column + 1))
            .unwrap_or_else(|| "-".to_string())
    }

    fn truncate_text(&self, text: &str, max_chars: usize) -> String {
        let char_count = text.chars().count();
        if char_count <= max_chars {
            return text.to_string();
        }

        let keep_chars = max_chars.saturating_sub(3);
        let mut truncated = text.chars().take(keep_chars).collect::<String>();
        truncated.push_str("...");
        truncated
    }
}
