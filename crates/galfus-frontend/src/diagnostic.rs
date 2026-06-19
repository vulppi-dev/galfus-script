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
    ExpectedInitializer,
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
            Self::ExpectedInitializer => "P0007",
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
            Self::ExpectedInitializer => "expected initializer",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolverDiagnosticCode {
    DuplicateSymbol,
    UnresolvedName,
    UnresolvedType,
    InvalidFunctionAnchor,
}

impl DiagnosticCodeKind for ResolverDiagnosticCode {
    fn as_code(&self) -> &'static str {
        match self {
            Self::DuplicateSymbol => "R0001",
            Self::UnresolvedName => "R0002",
            Self::UnresolvedType => "R0003",
            Self::InvalidFunctionAnchor => "R0004",
        }
    }

    fn as_message(&self) -> &'static str {
        match self {
            Self::DuplicateSymbol => "duplicate symbol",
            Self::UnresolvedName => "unresolved name",
            Self::UnresolvedType => "unresolved type",
            Self::InvalidFunctionAnchor => "invalid function anchor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeDiagnosticCode {
    TypeMismatch,
}

impl DiagnosticCodeKind for TypeDiagnosticCode {
    fn as_code(&self) -> &'static str {
        match self {
            Self::TypeMismatch => "T0001",
        }
    }

    fn as_message(&self) -> &'static str {
        match self {
            Self::TypeMismatch => "type mismatch",
        }
    }
}
