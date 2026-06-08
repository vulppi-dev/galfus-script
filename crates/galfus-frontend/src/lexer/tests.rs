use super::*;
use galfus_core::{SourceId, Span};

fn source(text: &str) -> SourceFile {
    SourceFile::new(SourceId::new(0), "main.gfs".to_string(), text.to_string())
}

fn kinds(text: &str) -> Vec<TokenKind> {
    let source = source(text);
    let mut lexer = Lexer::new(&source);

    let mut kinds = Vec::new();

    loop {
        let token = lexer.next_token();
        let kind = token.kind().clone();

        kinds.push(kind.clone());

        if kind == TokenKind::Eof {
            break;
        }
    }

    kinds
}

#[test]
fn lexer_returns_eof_for_empty_source() {
    assert_eq!(kinds(""), vec![TokenKind::Eof]);
}

#[test]
fn lexer_skips_horizontal_whitespace() {
    assert_eq!(kinds("   \t  "), vec![TokenKind::Eof]);
}

#[test]
fn lexer_reads_newline_tokens() {
    assert_eq!(
        kinds("\n\r\n\r"),
        vec![
            TokenKind::Newline,
            TokenKind::Newline,
            TokenKind::Newline,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_tracks_newline_span() {
    let source = source("a\r\nb");
    let mut lexer = Lexer::new(&source);

    let first = lexer.next_token();
    let newline = lexer.next_token();
    let second = lexer.next_token();

    assert_eq!(first.kind(), &TokenKind::Identifier);
    assert_eq!(source.slice(first.span()), Some("a"));

    assert_eq!(newline.kind(), &TokenKind::Newline);
    assert_eq!(source.slice(newline.span()), Some("\r\n"));

    assert_eq!(second.kind(), &TokenKind::Identifier);
    assert_eq!(source.slice(second.span()), Some("b"));
}

#[test]
fn lexer_reads_single_char_delimiters() {
    assert_eq!(
        kinds("( ) { } [ ]"),
        vec![
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
            TokenKind::LeftBracket,
            TokenKind::RightBracket,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_reads_single_char_punctuation() {
    assert_eq!(
        kinds(", . : ; @"),
        vec![
            TokenKind::Comma,
            TokenKind::Dot,
            TokenKind::Colon,
            TokenKind::Semicolon,
            TokenKind::At,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_reads_single_char_operators() {
    assert_eq!(
        kinds("+ - * / % ! = < > & | ^ ~"),
        vec![
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
            TokenKind::Bang,
            TokenKind::Equal,
            TokenKind::Less,
            TokenKind::Greater,
            TokenKind::Amp,
            TokenKind::Pipe,
            TokenKind::Caret,
            TokenKind::Tilde,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_tracks_token_spans() {
    let source = source("  +  -");
    let mut lexer = Lexer::new(&source);

    let plus = lexer.next_token();
    let minus = lexer.next_token();
    let eof = lexer.next_token();

    assert_eq!(plus.kind(), &TokenKind::Plus);
    assert_eq!(plus.span().start(), 2);
    assert_eq!(plus.span().end(), 3);

    assert_eq!(minus.kind(), &TokenKind::Minus);
    assert_eq!(minus.span().start(), 5);
    assert_eq!(minus.span().end(), 6);

    assert_eq!(eof.kind(), &TokenKind::Eof);
    assert_eq!(eof.span().start(), 6);
    assert_eq!(eof.span().end(), 6);
}

#[test]
fn lexer_returns_unknown_for_unrecognized_character() {
    assert_eq!(kinds("¬"), vec![TokenKind::Unknown, TokenKind::Eof]);
    assert_eq!(kinds("´"), vec![TokenKind::Unknown, TokenKind::Eof]);
}

#[test]
fn lexer_reports_unknown_character() {
    let source = source("¬");
    let mut lexer = Lexer::new(&source);

    let token = lexer.next_token();

    assert_eq!(token.kind(), &TokenKind::Unknown);
    assert_eq!(
        token.span(),
        Span::new(SourceId::new(0), 0, "¬".len() as u32)
    );

    let diagnostics = lexer.diagnostics();

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_errors());

    let diagnostic = diagnostics.iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "L0004");
    assert_eq!(diagnostic.message(), "unknown character");
    assert_eq!(
        diagnostic.span(),
        Span::new(SourceId::new(0), 0, "¬".len() as u32)
    );
}

#[test]
fn lexer_reads_two_char_operators() {
    assert_eq!(
        kinds("== != <= >= && || :: .. => += -= *= /= %= &= |= ^= << >> ++ -- ?. ?? ** ..."),
        vec![
            TokenKind::EqualEqual,
            TokenKind::BangEqual,
            TokenKind::LessEqual,
            TokenKind::GreaterEqual,
            TokenKind::AmpAmp,
            TokenKind::PipePipe,
            TokenKind::ColonColon,
            TokenKind::DotDot,
            TokenKind::Arrow,
            TokenKind::PlusEqual,
            TokenKind::MinusEqual,
            TokenKind::StarEqual,
            TokenKind::SlashEqual,
            TokenKind::PercentEqual,
            TokenKind::AmpEqual,
            TokenKind::PipeEqual,
            TokenKind::CaretEqual,
            TokenKind::ShiftLeft,
            TokenKind::ShiftRight,
            TokenKind::PlusPlus,
            TokenKind::MinusMinus,
            TokenKind::QuestionDot,
            TokenKind::QuestionQuestion,
            TokenKind::StarStar,
            TokenKind::DotDotDot,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_reads_three_char_operators() {
    assert_eq!(
        kinds("**= <<= >>= ..."),
        vec![
            TokenKind::StarStarEqual,
            TokenKind::ShiftLeftEqual,
            TokenKind::ShiftRightEqual,
            TokenKind::DotDotDot,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_prefers_longest_operator_match() {
    assert_eq!(
        kinds("... .. . **= ** * <<= << <="),
        vec![
            TokenKind::DotDotDot,
            TokenKind::DotDot,
            TokenKind::Dot,
            TokenKind::StarStarEqual,
            TokenKind::StarStar,
            TokenKind::Star,
            TokenKind::ShiftLeftEqual,
            TokenKind::ShiftLeft,
            TokenKind::LessEqual,
            TokenKind::Eof,
        ]
    );
}

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
                 satisfies match instanceof if else for in loop break continue weak null true false copy"
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
fn lexer_reads_integer_literals() {
    assert_eq!(
        kinds("0 10 123 1_000 999_999"),
        vec![
            TokenKind::Integer,
            TokenKind::Integer,
            TokenKind::Integer,
            TokenKind::Integer,
            TokenKind::Integer,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_tracks_integer_span() {
    let source = source("  12345");
    let mut lexer = Lexer::new(&source);

    let token = lexer.next_token();

    assert_eq!(token.kind(), &TokenKind::Integer);
    assert_eq!(token.span().start(), 2);
    assert_eq!(token.span().end(), 7);
    assert_eq!(source.slice(token.span()), Some("12345"));
}

#[test]
fn lexer_stops_integer_before_identifier() {
    assert_eq!(
        kinds("123abc"),
        vec![TokenKind::Integer, TokenKind::Identifier, TokenKind::Eof]
    );
}

#[test]
fn lexer_reads_hex_integer_literals() {
    assert_eq!(
        kinds("0xFF 0xff 0x10 0XAB"),
        vec![
            TokenKind::Integer,
            TokenKind::Integer,
            TokenKind::Integer,
            TokenKind::Integer,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_reads_binary_integer_literals() {
    assert_eq!(
        kinds("0b0 0b1010 0B1111_0000"),
        vec![
            TokenKind::Integer,
            TokenKind::Integer,
            TokenKind::Integer,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_reads_octal_integer_literals() {
    assert_eq!(
        kinds("0o0 0o755 0O123"),
        vec![
            TokenKind::Integer,
            TokenKind::Integer,
            TokenKind::Integer,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_tracks_prefixed_integer_span() {
    let source = source("  0xFF");
    let mut lexer = Lexer::new(&source);

    let token = lexer.next_token();

    assert_eq!(token.kind(), &TokenKind::Integer);
    assert_eq!(source.slice(token.span()), Some("0xFF"));
}

#[test]
fn lexer_reads_float_literals() {
    assert_eq!(
        kinds("1.0 0.5 10.25 1_000.50"),
        vec![
            TokenKind::Float,
            TokenKind::Float,
            TokenKind::Float,
            TokenKind::Float,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_tracks_float_span() {
    let source = source("  10.25");
    let mut lexer = Lexer::new(&source);

    let token = lexer.next_token();

    assert_eq!(token.kind(), &TokenKind::Float);
    assert_eq!(source.slice(token.span()), Some("10.25"));
}

#[test]
fn lexer_does_not_parse_range_as_float() {
    assert_eq!(
        kinds("1..9"),
        vec![
            TokenKind::Integer,
            TokenKind::DotDot,
            TokenKind::Integer,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexer_does_not_parse_trailing_dot_as_float() {
    assert_eq!(
        kinds("1."),
        vec![TokenKind::Integer, TokenKind::Dot, TokenKind::Eof]
    );
}

#[test]
fn lexer_accepts_valid_numeric_separators() {
    let source = source("1_000 0xff_ff 0b1010_0101 0o755_123 1_000.50");
    let result = lex(&source);

    assert!(!result.has_errors());
}

#[test]
fn lexer_reports_trailing_numeric_separator() {
    let source = source("100_");
    let result = lex(&source);

    assert!(result.has_errors());
    assert_eq!(result.diagnostics().len(), 1);

    let diagnostic = result.diagnostics().iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "L0005");
    assert_eq!(diagnostic.message(), "invalid numeric separator");
    assert_eq!(diagnostic.span(), Span::new(SourceId::new(0), 3, 4));
}

#[test]
fn lexer_reports_repeated_numeric_separator() {
    let source = source("1__000");
    let result = lex(&source);

    assert!(result.has_errors());
    assert_eq!(result.diagnostics().len(), 1);

    let diagnostic = result.diagnostics().iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "L0005");
    assert_eq!(diagnostic.message(), "invalid numeric separator");
    assert_eq!(diagnostic.span(), Span::new(SourceId::new(0), 2, 3));
}

#[test]
fn lexer_reports_separator_after_numeric_prefix() {
    let source = source("0x_FF");
    let result = lex(&source);

    assert!(result.has_errors());
    assert_eq!(result.diagnostics().len(), 1);

    let diagnostic = result.diagnostics().iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "L0005");
    assert_eq!(diagnostic.message(), "invalid numeric separator");
    assert_eq!(diagnostic.span(), Span::new(SourceId::new(0), 2, 3));
}

#[test]
fn lexer_reports_invalid_separator_in_float_fraction() {
    let source = source("10.5_");
    let result = lex(&source);

    assert!(result.has_errors());
    assert_eq!(result.diagnostics().len(), 1);

    let diagnostic = result.diagnostics().iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "L0005");
    assert_eq!(diagnostic.message(), "invalid numeric separator");
    assert_eq!(diagnostic.span(), Span::new(SourceId::new(0), 4, 5));
}

#[test]
fn lexer_skips_line_comments_but_preserves_newline() {
    assert_eq!(
        kinds("// hello\nfn"),
        vec![TokenKind::Newline, TokenKind::Fn, TokenKind::Eof]
    );
}

#[test]
fn lexer_skips_line_comment_until_eof() {
    assert_eq!(kinds("// hello"), vec![TokenKind::Eof]);
}

#[test]
fn lexer_skips_block_comments() {
    assert_eq!(kinds("/* hello */ fn"), vec![TokenKind::Fn, TokenKind::Eof]);
}

#[test]
fn lexer_skips_block_comment_with_newlines_inside() {
    assert_eq!(
        kinds("/* hello\nworld */ fn"),
        vec![TokenKind::Fn, TokenKind::Eof]
    );
}

#[test]
fn lexer_skips_mixed_trivia_but_preserves_line_newline() {
    assert_eq!(
        kinds("  // line\n  /* block */  fn"),
        vec![TokenKind::Newline, TokenKind::Fn, TokenKind::Eof]
    );
}

#[test]
fn lexer_reports_unterminated_block_comment() {
    let source = source("/* hello");
    let mut lexer = Lexer::new(&source);

    let token = lexer.next_token();

    assert_eq!(token.kind(), &TokenKind::Eof);

    let diagnostics = lexer.diagnostics();

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_errors());

    let diagnostic = diagnostics.iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "L0001");
    assert_eq!(diagnostic.message(), "unterminated block comment");
    assert_eq!(diagnostic.span(), Span::new(SourceId::new(0), 0, 8));
}

#[test]
fn lexer_reads_double_quoted_string() {
    assert_eq!(kinds("\"hello\""), vec![TokenKind::String, TokenKind::Eof]);
}

#[test]
fn lexer_reads_single_quoted_string() {
    assert_eq!(kinds("'hello'"), vec![TokenKind::String, TokenKind::Eof]);
}

#[test]
fn lexer_tracks_string_span() {
    let source = source("  \"hello\"");
    let mut lexer = Lexer::new(&source);

    let token = lexer.next_token();

    assert_eq!(token.kind(), &TokenKind::String);
    assert_eq!(token.span().start(), 2);
    assert_eq!(token.span().end(), 9);
    assert_eq!(source.slice(token.span()), Some("\"hello\""));
}

#[test]
fn lexer_reports_unterminated_double_quoted_string() {
    let source = source("\"hello");
    let mut lexer = Lexer::new(&source);

    let token = lexer.next_token();

    assert_eq!(token.kind(), &TokenKind::String);

    let diagnostics = lexer.diagnostics();

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_errors());

    let diagnostic = diagnostics.iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "L0002");
    assert_eq!(diagnostic.message(), "unterminated string literal");
    assert_eq!(diagnostic.span(), Span::new(SourceId::new(0), 0, 6));
}

#[test]
fn lexer_reports_string_interrupted_by_newline() {
    let source = source("\"hello\nworld\"");
    let mut lexer = Lexer::new(&source);

    let first = lexer.next_token();
    let newline = lexer.next_token();
    let second = lexer.next_token();
    let third = lexer.next_token();

    assert_eq!(first.kind(), &TokenKind::String);
    assert_eq!(source.slice(first.span()), Some("\"hello"));

    assert_eq!(newline.kind(), &TokenKind::Newline);
    assert_eq!(source.slice(newline.span()), Some("\n"));

    assert_eq!(second.kind(), &TokenKind::Identifier);
    assert_eq!(source.slice(second.span()), Some("world"));

    assert_eq!(third.kind(), &TokenKind::String);
    assert_eq!(source.slice(third.span()), Some("\""));

    let diagnostics = lexer.diagnostics();

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.has_errors());

    let mut diagnostics = diagnostics.iter();

    let first_diagnostic = diagnostics.next().unwrap();
    let second_diagnostic = diagnostics.next().unwrap();

    assert_eq!(first_diagnostic.code().as_str(), "L0002");
    assert_eq!(first_diagnostic.message(), "unterminated string literal");
    assert_eq!(first_diagnostic.span(), Span::new(SourceId::new(0), 0, 6));

    assert_eq!(second_diagnostic.code().as_str(), "L0002");
    assert_eq!(second_diagnostic.message(), "unterminated string literal");
    assert_eq!(
        second_diagnostic.span(),
        Span::new(SourceId::new(0), 12, 13)
    );
}

#[test]
fn lexer_reads_multiline_string() {
    assert_eq!(
        kinds("`line 1\nline 2`"),
        vec![TokenKind::String, TokenKind::Eof]
    );
}

#[test]
fn lexer_tracks_multiline_string_span() {
    let source = source("  `line 1\nline 2`");
    let mut lexer = Lexer::new(&source);

    let token = lexer.next_token();

    assert_eq!(token.kind(), &TokenKind::String);
    assert_eq!(source.slice(token.span()), Some("`line 1\nline 2`"));
}

#[test]
fn lexer_reports_unterminated_multiline_string() {
    let source = source("`line 1\nline 2");
    let mut lexer = Lexer::new(&source);

    let token = lexer.next_token();

    assert_eq!(token.kind(), &TokenKind::String);
    assert_eq!(source.slice(token.span()), Some("`line 1\nline 2"));

    let diagnostics = lexer.diagnostics();

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics.has_errors());

    let diagnostic = diagnostics.iter().next().unwrap();

    assert_eq!(diagnostic.code().as_str(), "L0003");
    assert_eq!(
        diagnostic.message(),
        "unterminated multiline string literal"
    );
    assert_eq!(
        diagnostic.span(),
        Span::new(SourceId::new(0), 0, "`line 1\nline 2".len() as u32)
    );
}

#[test]
fn lex_returns_tokens_and_diagnostics() {
    let source = source("fn main(): null {}");

    let result = lex(&source);

    let kinds: Vec<TokenKind> = result
        .tokens()
        .iter()
        .map(|token| token.kind().clone())
        .collect();

    assert_eq!(
        kinds,
        vec![
            TokenKind::Fn,
            TokenKind::Identifier,
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::Colon,
            TokenKind::Null,
            TokenKind::LeftBrace,
            TokenKind::RightBrace,
            TokenKind::Eof,
        ]
    );

    assert!(!result.has_errors());
    assert!(result.diagnostics().is_empty());
}

#[test]
fn lex_preserves_newline_tokens() {
    let source = source("fn main(): null {\n  return\n}");

    let result = lex(&source);

    let kinds: Vec<TokenKind> = result
        .tokens()
        .iter()
        .map(|token| token.kind().clone())
        .collect();

    assert_eq!(
        kinds,
        vec![
            TokenKind::Fn,
            TokenKind::Identifier,
            TokenKind::LeftParen,
            TokenKind::RightParen,
            TokenKind::Colon,
            TokenKind::Null,
            TokenKind::LeftBrace,
            TokenKind::Newline,
            TokenKind::Return,
            TokenKind::Newline,
            TokenKind::RightBrace,
            TokenKind::Eof,
        ]
    );
}
