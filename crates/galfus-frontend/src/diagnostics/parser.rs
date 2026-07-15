use galfus_core::DiagnosticCodeKind;

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
