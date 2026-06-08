use galfus_core::DiagnosticCodeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LexicalDiagnosticCode {
    UnterminatedBlockComment,
    UnterminatedStringLiteral,
    UnterminatedMultilineStringLiteral,
    UnknownCharacter,
    InvalidNumericSeparator,
}

impl DiagnosticCodeKind for LexicalDiagnosticCode {
    fn as_code(&self) -> &'static str {
        match self {
            Self::UnterminatedBlockComment => "L0001",
            Self::UnterminatedStringLiteral => "L0002",
            Self::UnterminatedMultilineStringLiteral => "L0003",
            Self::UnknownCharacter => "L0004",
            Self::InvalidNumericSeparator => "L0005",
        }
    }

    fn as_message(&self) -> &'static str {
        match self {
            Self::UnterminatedBlockComment => "unterminated block comment",
            Self::UnterminatedStringLiteral => "unterminated string literal",
            Self::UnterminatedMultilineStringLiteral => "unterminated multiline string literal",
            Self::UnknownCharacter => "unknown character",
            Self::InvalidNumericSeparator => "invalid numeric separator",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParserDiagnosticCode {
    ExpectedToken,
    ExpectedItem,
    ExpectedIdentifier,
    ExpectedType,
    ExpectedStatement,
    UnexpectedToken,
}

impl DiagnosticCodeKind for ParserDiagnosticCode {
    fn as_code(&self) -> &'static str {
        match self {
            Self::ExpectedToken => "P0001",
            Self::ExpectedItem => "P0002",
            Self::ExpectedIdentifier => "P0003",
            Self::ExpectedType => "P0004",
            Self::ExpectedStatement => "P0005",
            Self::UnexpectedToken => "P0006",
        }
    }

    fn as_message(&self) -> &'static str {
        match self {
            Self::ExpectedToken => "expected token",
            Self::ExpectedItem => "expected item",
            Self::ExpectedIdentifier => "expected identifier",
            Self::ExpectedType => "expected type",
            Self::ExpectedStatement => "expected statement",
            Self::UnexpectedToken => "unexpected token",
        }
    }
}
