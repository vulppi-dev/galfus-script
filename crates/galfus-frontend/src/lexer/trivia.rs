use super::*;

impl Lexer<'_> {
    pub(super) fn skip_trivia(&mut self) {
        loop {
            let start = self.offset;

            self.skip_whitespace();

            if self.starts_with("//") {
                self.skip_line_comment();
            } else if self.starts_with("/*") {
                self.skip_block_comment();
            }

            if self.offset == start {
                break;
            }
        }
    }

    pub(super) fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            match ch {
                ' ' | '\t' | '\u{000C}' => {
                    self.bump();
                }
                _ => break,
            }
        }
    }

    pub(super) fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' || ch == '\r' {
                break;
            }

            self.bump();
        }
    }

    pub(super) fn skip_block_comment(&mut self) {
        if !self.starts_with("/*") {
            return;
        }

        let start = self.offset;

        self.bump(); // /
        self.bump(); // *

        while !self.is_eof() {
            if self.starts_with("*/") {
                self.bump(); // *
                self.bump(); // /
                return;
            }

            self.bump();
        }

        let span = Span::new(self.source.id(), start, self.offset);

        self.diagnostics.push(Diagnostic::error(
            LexicalDiagnosticCode::UnterminatedBlockComment,
            span,
        ));
    }
}
