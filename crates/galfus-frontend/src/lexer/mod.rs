#[cfg(test)]
mod tests;

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
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a SourceFile) -> Self {
        Self {
            source,
            text: source.text(),
            offset: 0,
            diagnostics: DiagnosticBag::new(),
        }
    }

    pub fn diagnostics(&self) -> &DiagnosticBag {
        &self.diagnostics
    }

    pub fn into_diagnostics(self) -> DiagnosticBag {
        self.diagnostics
    }

    fn keyword_kind(text: &str) -> Option<TokenKind> {
        let kind = match text {
            "import" => TokenKind::Import,
            "from" => TokenKind::From,
            "export" => TokenKind::Export,
            "as" => TokenKind::As,
            "var" => TokenKind::Var,
            "const" => TokenKind::Const,
            "fn" => TokenKind::Fn,
            "return" => TokenKind::Return,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "choice" => TokenKind::Choice,
            "type" => TokenKind::Type,
            "constraint" => TokenKind::Constraint,
            "satisfies" => TokenKind::Satisfies,
            "match" => TokenKind::Match,
            "instanceof" => TokenKind::Instanceof,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "loop" => TokenKind::Loop,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "weak" => TokenKind::Weak,
            "null" => TokenKind::Null,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "copy" => TokenKind::Copy,
            _ => return None,
        };

        Some(kind)
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_trivia();

        let start = self.offset;

        if self.is_eof() {
            return Token::new(TokenKind::Eof, Span::empty(self.source.id(), start));
        }

        let ch = self.bump().expect("lexer offset should not be EOF");

        if Self::is_identifier_start(ch) {
            let kind = self.lex_identifier(start);
            return Token::new(kind, Span::new(self.source.id(), start, self.offset));
        }

        if Self::is_decimal_digit(ch) {
            let kind = self.lex_number(start);
            return Token::new(kind, Span::new(self.source.id(), start, self.offset));
        }

        if ch == '"' || ch == '\'' {
            let kind = self.lex_string(start, ch);
            return Token::new(kind, Span::new(self.source.id(), start, self.offset));
        }

        if ch == '`' {
            let kind = self.lex_multiline_string(start);
            return Token::new(kind, Span::new(self.source.id(), start, self.offset));
        }

        let kind = match ch {
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '{' => TokenKind::LeftBrace,
            '}' => TokenKind::RightBrace,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,

            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            '@' => TokenKind::At,

            '.' => {
                if self.match_char('.') {
                    if self.match_char('.') {
                        TokenKind::DotDotDot
                    } else {
                        TokenKind::DotDot
                    }
                } else {
                    TokenKind::Dot
                }
            }

            ':' => {
                if self.match_char(':') {
                    TokenKind::ColonColon
                } else {
                    TokenKind::Colon
                }
            }

            '?' => {
                if self.match_char('.') {
                    TokenKind::QuestionDot
                } else if self.match_char('?') {
                    TokenKind::QuestionQuestion
                } else {
                    TokenKind::Unknown
                }
            }

            '+' => {
                if self.match_char('+') {
                    TokenKind::PlusPlus
                } else if self.match_char('=') {
                    TokenKind::PlusEqual
                } else {
                    TokenKind::Plus
                }
            }

            '-' => {
                if self.match_char('-') {
                    TokenKind::MinusMinus
                } else if self.match_char('=') {
                    TokenKind::MinusEqual
                } else {
                    TokenKind::Minus
                }
            }

            '*' => {
                if self.match_char('*') {
                    if self.match_char('=') {
                        TokenKind::StarStarEqual
                    } else {
                        TokenKind::StarStar
                    }
                } else if self.match_char('=') {
                    TokenKind::StarEqual
                } else {
                    TokenKind::Star
                }
            }

            '/' => {
                if self.match_char('=') {
                    TokenKind::SlashEqual
                } else {
                    TokenKind::Slash
                }
            }

            '%' => {
                if self.match_char('=') {
                    TokenKind::PercentEqual
                } else {
                    TokenKind::Percent
                }
            }

            '!' => {
                if self.match_char('=') {
                    TokenKind::BangEqual
                } else {
                    TokenKind::Bang
                }
            }

            '=' => {
                if self.match_char('=') {
                    TokenKind::EqualEqual
                } else if self.match_char('>') {
                    TokenKind::Arrow
                } else {
                    TokenKind::Equal
                }
            }

            '<' => {
                if self.match_char('<') {
                    if self.match_char('=') {
                        TokenKind::ShiftLeftEqual
                    } else {
                        TokenKind::ShiftLeft
                    }
                } else if self.match_char('=') {
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                }
            }

            '>' => {
                if self.match_char('>') {
                    if self.match_char('=') {
                        TokenKind::ShiftRightEqual
                    } else {
                        TokenKind::ShiftRight
                    }
                } else if self.match_char('=') {
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }

            '&' => {
                if self.match_char('&') {
                    TokenKind::AmpAmp
                } else if self.match_char('=') {
                    TokenKind::AmpEqual
                } else {
                    TokenKind::Amp
                }
            }

            '|' => {
                if self.match_char('|') {
                    TokenKind::PipePipe
                } else if self.match_char('=') {
                    TokenKind::PipeEqual
                } else {
                    TokenKind::Pipe
                }
            }

            '^' => {
                if self.match_char('=') {
                    TokenKind::CaretEqual
                } else {
                    TokenKind::Caret
                }
            }

            '~' => TokenKind::Tilde,

            '\n' => TokenKind::Newline,
            '\r' => {
                if self.peek() == Some('\n') {
                    self.bump();
                }

                TokenKind::Newline
            }

            _ => {
                let span = Span::new(self.source.id(), start, self.offset);

                self.diagnostics.push(Diagnostic::error(
                    LexicalDiagnosticCode::UnknownCharacter,
                    span,
                ));

                TokenKind::Unknown
            }
        };

        Token::new(kind, Span::new(self.source.id(), start, self.offset))
    }

    fn is_eof(&self) -> bool {
        self.offset as usize >= self.text.len()
    }

    fn is_identifier_extra(ch: char) -> bool {
        matches!(ch, '_' | '#' | '$')
    }

    fn is_identifier_start(ch: char) -> bool {
        Self::is_identifier_extra(ch) || unicode_ident::is_xid_start(ch)
    }

    fn is_identifier_continue(ch: char) -> bool {
        Self::is_identifier_extra(ch) || unicode_ident::is_xid_continue(ch)
    }

    fn lex_identifier(&mut self, start: u32) -> TokenKind {
        while let Some(ch) = self.peek() {
            if !Self::is_identifier_continue(ch) {
                break;
            }

            self.bump();
        }

        let text = &self.text[start as usize..self.offset as usize];

        Self::keyword_kind(text).unwrap_or(TokenKind::Identifier)
    }

    fn is_decimal_digit(ch: char) -> bool {
        ch.is_ascii_digit()
    }

    fn is_hex_digit(ch: char) -> bool {
        ch.is_ascii_hexdigit()
    }

    fn is_binary_digit(ch: char) -> bool {
        ch == '0' || ch == '1'
    }

    fn is_octal_digit(ch: char) -> bool {
        matches!(ch, '0'..='7')
    }

    fn is_digit_for_base(ch: char, base: NumberBase) -> bool {
        match base {
            NumberBase::Decimal => Self::is_decimal_digit(ch),
            NumberBase::Hex => Self::is_hex_digit(ch),
            NumberBase::Binary => Self::is_binary_digit(ch),
            NumberBase::Octal => Self::is_octal_digit(ch),
        }
    }

    fn lex_number(&mut self, start: u32) -> TokenKind {
        let (base, digits_start) = self.consume_number_prefix(start);

        while let Some(ch) = self.peek() {
            if Self::is_digit_for_base(ch, base) || ch == '_' {
                self.bump();
            } else {
                break;
            }
        }

        let mut kind = TokenKind::Integer;

        if base == NumberBase::Decimal && self.starts_float_fraction() {
            self.bump(); // .

            while let Some(ch) = self.peek() {
                if Self::is_decimal_digit(ch) || ch == '_' {
                    self.bump();
                } else {
                    break;
                }
            }

            kind = TokenKind::Float;
        }

        self.validate_number_separators(digits_start, self.offset, base);

        kind
    }

    fn consume_number_prefix(&mut self, start: u32) -> (NumberBase, u32) {
        let after_first_digit = self.offset;

        if self.text.get(start as usize..after_first_digit as usize) != Some("0") {
            return (NumberBase::Decimal, start);
        }

        match self.peek() {
            Some('x') | Some('X') => {
                self.bump();
                (NumberBase::Hex, self.offset)
            }
            Some('b') | Some('B') => {
                self.bump();
                (NumberBase::Binary, self.offset)
            }
            Some('o') | Some('O') => {
                self.bump();
                (NumberBase::Octal, self.offset)
            }
            _ => (NumberBase::Decimal, start),
        }
    }

    fn starts_float_fraction(&self) -> bool {
        if self.peek() != Some('.') {
            return false;
        }

        if self.peek_next() == Some('.') {
            return false;
        }

        matches!(self.peek_next(), Some(ch) if Self::is_decimal_digit(ch))
    }

    fn current_text(&self) -> &str {
        &self.text[self.offset as usize..]
    }

    fn peek(&self) -> Option<char> {
        self.current_text().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.offset += ch.len_utf8() as u32;
        Some(ch)
    }

    fn peek_next(&self) -> Option<char> {
        let mut chars = self.current_text().chars();
        chars.next()?;
        chars.next()
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() != Some(expected) {
            return false;
        }

        self.bump();
        true
    }

    fn starts_with(&self, text: &str) -> bool {
        self.current_text().starts_with(text)
    }

    fn skip_trivia(&mut self) {
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

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            match ch {
                ' ' | '\t' | '\u{000C}' => {
                    self.bump();
                }
                _ => break,
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' || ch == '\r' {
                break;
            }

            self.bump();
        }
    }

    fn skip_block_comment(&mut self) {
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

    fn lex_string(&mut self, start: u32, quote: char) -> TokenKind {
        while let Some(ch) = self.peek() {
            if ch == quote {
                self.bump();
                return TokenKind::String;
            }

            if ch == '\n' || ch == '\r' {
                let span = Span::new(self.source.id(), start, self.offset);

                self.diagnostics.push(Diagnostic::error(
                    LexicalDiagnosticCode::UnterminatedStringLiteral,
                    span,
                ));

                return TokenKind::String;
            }

            self.bump();
        }

        let span = Span::new(self.source.id(), start, self.offset);

        self.diagnostics.push(Diagnostic::error(
            LexicalDiagnosticCode::UnterminatedStringLiteral,
            span,
        ));

        TokenKind::String
    }

    fn lex_multiline_string(&mut self, start: u32) -> TokenKind {
        while let Some(ch) = self.peek() {
            if ch == '`' {
                self.bump();
                return TokenKind::String;
            }

            self.bump();
        }

        let span = Span::new(self.source.id(), start, self.offset);

        self.diagnostics.push(Diagnostic::error(
            LexicalDiagnosticCode::UnterminatedMultilineStringLiteral,
            span,
        ));

        TokenKind::String
    }

    fn validate_number_separators(&mut self, start: u32, end: u32, base: NumberBase) {
        let text = &self.text[start as usize..end as usize];

        let mut previous_was_digit = false;
        let mut previous_was_separator = false;

        for (relative_offset, ch) in text.char_indices() {
            if ch == '_' {
                let absolute_offset = start + relative_offset as u32;

                if !previous_was_digit || previous_was_separator {
                    let span = Span::new(self.source.id(), absolute_offset, absolute_offset + 1);

                    self.diagnostics.push(Diagnostic::error(
                        LexicalDiagnosticCode::InvalidNumericSeparator,
                        span,
                    ));
                }

                previous_was_separator = true;
                previous_was_digit = false;
                continue;
            }

            if Self::is_digit_for_base(ch, base) {
                previous_was_digit = true;
                previous_was_separator = false;
                continue;
            }

            // Prefix chars like x/b/o are ignored here.
            previous_was_digit = false;
            previous_was_separator = false;
        }

        if previous_was_separator {
            let span = Span::new(self.source.id(), end - 1, end);

            self.diagnostics.push(Diagnostic::error(
                LexicalDiagnosticCode::InvalidNumericSeparator,
                span,
            ));
        }
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
