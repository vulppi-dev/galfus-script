#[cfg(test)]
mod tests;

mod cursor;
mod identifier;
mod numbers;
mod strings;
mod tokenize;
mod trivia;

use crate::{LexicalDiagnosticCode, Token, TokenKind};
use galfus_core::{Diagnostic, DiagnosticBag, SourceFile, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumberBase {
    Decimal,
    Hex,
    Binary,
    Octal,
}

#[derive(Debug, Clone)]
pub struct LexResult {
    tokens: Vec<Token>,
    diagnostics: DiagnosticBag,
}

impl LexResult {
    pub fn new(tokens: Vec<Token>, diagnostics: DiagnosticBag) -> Self {
        Self {
            tokens,
            diagnostics,
        }
    }

    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    pub fn diagnostics(&self) -> &DiagnosticBag {
        &self.diagnostics
    }

    pub fn into_parts(self) -> (Vec<Token>, DiagnosticBag) {
        (self.tokens, self.diagnostics)
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }
}

pub struct Lexer<'a> {
    source: &'a SourceFile,
    text: &'a str,
    offset: u32,
    diagnostics: DiagnosticBag,
    previous_significant_kind: Option<TokenKind>,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a SourceFile) -> Self {
        Self {
            source,
            text: source.text(),
            offset: 0,
            diagnostics: DiagnosticBag::new(),
            previous_significant_kind: None,
        }
    }

    pub fn diagnostics(&self) -> &DiagnosticBag {
        &self.diagnostics
    }

    pub fn into_diagnostics(self) -> DiagnosticBag {
        self.diagnostics
    }

    fn make_token(&mut self, kind: TokenKind, span: Span) -> Token {
        if Self::is_significant_for_regex(&kind) {
            self.previous_significant_kind = Some(kind.clone());
        }

        Token::new(kind, span)
    }

    fn is_significant_for_regex(kind: &TokenKind) -> bool {
        !matches!(
            kind,
            TokenKind::Newline | TokenKind::Unknown | TokenKind::Eof
        )
    }

}

pub fn lex(source: &SourceFile) -> LexResult {
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();

    loop {
        let token = lexer.next_token();
        let is_eof = token.kind() == &TokenKind::Eof;

        tokens.push(token);

        if is_eof {
            break;
        }
    }

    LexResult::new(tokens, lexer.into_diagnostics())
}
