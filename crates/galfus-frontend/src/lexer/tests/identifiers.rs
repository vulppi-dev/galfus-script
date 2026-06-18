use super::*;

#[test]
fn lexer_reads_identifiers() {
    assert_eq!(
        kinds("main user_name User _private a1"),
        vec![
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_reads_unicode_identifiers() {
    assert_eq!(
        kinds("ação usuário 名前 変数 привет δelta _私有"),
        vec![
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_reads_keywords() {
    assert_eq!(
        kinds(
            "import from export as var const fn return struct enum choice type constraint \
                 satisfies match instanceof if else for in while loop break continue weak null true false copy"
        ),
        vec![
            TokenKind::Import,
            TokenKind::From,
            TokenKind::Export,
            TokenKind::As,
            TokenKind::Var,
            TokenKind::Const,
            TokenKind::Fn,
            TokenKind::Return,
            TokenKind::Struct,
            TokenKind::Enum,
            TokenKind::Choice,
            TokenKind::Type,
            TokenKind::Constraint,
            TokenKind::Satisfies,
            TokenKind::Match,
            TokenKind::Instanceof,
            TokenKind::If,
            TokenKind::Else,
            TokenKind::For,
            TokenKind::In,
            TokenKind::While,
            TokenKind::Loop,
            TokenKind::Break,
            TokenKind::Continue,
            TokenKind::Weak,
            TokenKind::Null,
            TokenKind::True,
            TokenKind::False,
            TokenKind::Copy,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_does_not_split_keyword_prefixes() {
    assert_eq!(
        kinds("function returnValue nullable aspect"),
        vec![
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Identifier,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lex_stamp_keyword() {
    let tokens = kinds("stamp fn max() {}");

    assert_eq!(
        tokens,
        vec![
            TokenKind::Stamp,
            TokenKind::Fn,
            TokenKind::Identifier,
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lex_string_type_name_as_identifier() {
    let tokens = kinds("String [uint8]");

    assert_eq!(
        tokens,
        vec![
            TokenKind::Identifier,
            TokenKind::LeftBracket,
            TokenKind::Identifier,
            TokenKind::RightBracket,
            TokenKind::Eof,
        ]
    );
}
