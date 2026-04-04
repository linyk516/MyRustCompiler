use std::path::{Path, PathBuf};
use crate::lexer::token::Span;

pub struct SourceFile {
    pub path: Option<PathBuf>,
    pub text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(text: impl Into<String>) -> Self {
        let mut source = SourceFile {
            path: None,
            text: text.into(),
            line_starts: Vec::new(),
        };
        source.calc_line_start();
        source
    }
    pub fn with_path(path: impl Into<PathBuf>, text: impl Into<String>) -> Self {
        let mut source = SourceFile {
            path: Some(path.into()),
            text: text.into(),
            line_starts: Vec::new(),
        };
        source.calc_line_start();
        source
    }
    pub fn from_path(path: impl Into<PathBuf>) -> std::io::Result<Self> {
        let path = path.into();
        let text = std::fs::read_to_string(&path)?;
        Ok(SourceFile::with_path(path, text))
    }

    fn calc_line_start(&mut self) {
        self.line_starts.push(0);
        for (i, ch) in self.text.char_indices() {
            if ch == '\n' {
                self.line_starts.push(i + 1);
            }
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_ref().map(AsRef::as_ref)
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn len_bytes(&self) -> usize {
        self.text.len()
    }
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn slice(&self, span: Span) -> Option<&str> {
        span.text(self.text())
    }
    pub fn line_col(&self, byte_pos: usize) -> Option<(usize, usize)> {
        if byte_pos > self.text.len() {
            return None;
        }

        let line = match self.line_starts.binary_search(&byte_pos) {
            Ok(line) => line,
            Err(0) => return None,
            Err(next_line) => next_line - 1,
        };

        let col = byte_pos - self.line_starts[line];
        Some((line, col))
    }
    pub fn line_text(&self, line: usize) -> Option<&str> {
        let line_start = self.line_starts.get(line)?;
        let line_end = self.line_starts.get(line + 1).cloned().unwrap_or_else(|| self.text.len());
        self.text.get(*line_start..line_end)
    }
}
