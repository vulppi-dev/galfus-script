use super::*;

impl Lexer<'_> {
    pub(super) fn keyword_kind(text: &str) -> Option<TokenKind> {
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
            "while" => TokenKind::While,
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

    pub(super) fn is_identifier_extra(ch: char) -> bool {
        matches!(ch, '_' | '#' | '$')
    }

    pub(super) fn is_identifier_start(ch: char) -> bool {
        Self::is_identifier_extra(ch) || unicode_ident::is_xid_start(ch)
    }

    pub(super) fn is_identifier_continue(ch: char) -> bool {
        Self::is_identifier_extra(ch) || unicode_ident::is_xid_continue(ch)
    }

    pub(super) fn lex_identifier(&mut self, start: u32) -> TokenKind {
        while let Some(ch) = self.peek() {
            if !Self::is_identifier_continue(ch) {
                break;
            }

            self.bump();
        }

        let text = &self.text[start as usize..self.offset as usize];

        Self::keyword_kind(text).unwrap_or(TokenKind::Identifier)
    }
}
