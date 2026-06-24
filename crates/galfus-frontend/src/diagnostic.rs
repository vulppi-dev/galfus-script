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
    NotCallable,
    ArgumentCountMismatch,
    UnsupportedOperator,
    AssignmentToImmutable,
    UnknownMember,
    InvalidIndexTarget,
    InvalidIndexType,
    InvalidSpreadTarget,
    CannotInferType,
    EmptyArrayLiteral,
    DynamicSpreadInArrayLiteral,
    UnknownStructField,
    DuplicateStructField,
    MissingStructField,
    InvalidStructLiteralTarget,
    ChoicePayloadRequired,
    ChoicePayloadNotAllowed,
    InvalidConditionType,
    BreakOutsideLoop,
    ContinueOutsideLoop,
    InvalidIterableType,
    InvalidMatchPatternType,
    IncompatibleMatchArmType,
    InvalidInstanceofPatternType,
    IncompatibleInstanceofArmType,
    InvalidSatisfiesTarget,
    MissingConstraintField,
    ConstraintFieldTypeMismatch,
    MissingConstraintFunction,
    ConstraintFunctionTypeMismatch,
    ConstraintGenericArgumentCountMismatch,
    InvalidRangeOperandType,
    GenericArgumentCountMismatch,
    NonExhaustiveMatch,
    InvalidDecoratorUsage,
    RecursiveFunctionStamp,
    InvalidWeakFieldType,
    InvalidEnumBaseType,
}

impl DiagnosticCodeKind for TypeDiagnosticCode {
    fn as_code(&self) -> &'static str {
        match self {
            Self::TypeMismatch => "T0001",
            Self::NotCallable => "T0002",
            Self::ArgumentCountMismatch => "T0003",
            Self::UnsupportedOperator => "T0004",
            Self::AssignmentToImmutable => "T0005",
            Self::UnknownMember => "T0006",
            Self::InvalidIndexTarget => "T0007",
            Self::InvalidIndexType => "T0008",
            Self::InvalidSpreadTarget => "T0009",
            Self::CannotInferType => "T0010",
            Self::EmptyArrayLiteral => "T0011",
            Self::DynamicSpreadInArrayLiteral => "T0012",
            Self::UnknownStructField => "T0013",
            Self::DuplicateStructField => "T0014",
            Self::MissingStructField => "T0015",
            Self::InvalidStructLiteralTarget => "T0016",
            Self::ChoicePayloadRequired => "T0017",
            Self::ChoicePayloadNotAllowed => "T0018",
            Self::InvalidConditionType => "T0019",
            Self::BreakOutsideLoop => "T0020",
            Self::ContinueOutsideLoop => "T0021",
            Self::InvalidIterableType => "T0022",
            Self::InvalidMatchPatternType => "T0023",
            Self::IncompatibleMatchArmType => "T0024",
            Self::InvalidInstanceofPatternType => "T0025",
            Self::IncompatibleInstanceofArmType => "T0026",
            Self::InvalidSatisfiesTarget => "T0027",
            Self::MissingConstraintField => "T0028",
            Self::ConstraintFieldTypeMismatch => "T0029",
            Self::MissingConstraintFunction => "T0030",
            Self::ConstraintFunctionTypeMismatch => "T0031",
            Self::ConstraintGenericArgumentCountMismatch => "T0032",
            Self::InvalidRangeOperandType => "T0033",
            Self::GenericArgumentCountMismatch => "T0034",
            Self::NonExhaustiveMatch => "T0035",
            Self::InvalidDecoratorUsage => "T0036",
            Self::RecursiveFunctionStamp => "T0037",
            Self::InvalidWeakFieldType => "T0038",
            Self::InvalidEnumBaseType => "T0039",
        }
    }

    fn as_message(&self) -> &'static str {
        match self {
            Self::TypeMismatch => "type mismatch",
            Self::NotCallable => "not callable",
            Self::ArgumentCountMismatch => "argument count mismatch",
            Self::UnsupportedOperator => "unsupported operator",
            Self::AssignmentToImmutable => "assignment to immutable binding",
            Self::UnknownMember => "unknown member",
            Self::InvalidIndexTarget => "invalid index target",
            Self::InvalidIndexType => "invalid index type",
            Self::InvalidSpreadTarget => "invalid spread target",
            Self::CannotInferType => "cannot infer type",
            Self::EmptyArrayLiteral => "empty array literal is not allowed",
            Self::DynamicSpreadInArrayLiteral => "dynamic spread in array literal is not allowed",
            Self::UnknownStructField => "unknown struct field",
            Self::DuplicateStructField => "duplicate struct field",
            Self::MissingStructField => "missing struct field",
            Self::InvalidStructLiteralTarget => "invalid struct literal target",
            Self::ChoicePayloadRequired => "choice payload required",
            Self::ChoicePayloadNotAllowed => "choice payload not allowed",
            Self::InvalidConditionType => "invalid condition type",
            Self::BreakOutsideLoop => "break outside loop",
            Self::ContinueOutsideLoop => "continue outside loop",
            Self::InvalidIterableType => "invalid iterable type",
            Self::InvalidMatchPatternType => "invalid match pattern type",
            Self::IncompatibleMatchArmType => "incompatible match arm type",
            Self::InvalidInstanceofPatternType => "invalid instanceof pattern type",
            Self::IncompatibleInstanceofArmType => "incompatible instanceof arm type",
            Self::InvalidSatisfiesTarget => "invalid satisfies target",
            Self::MissingConstraintField => "missing constraint field",
            Self::ConstraintFieldTypeMismatch => "constraint field type mismatch",
            Self::MissingConstraintFunction => "missing constraint function",
            Self::ConstraintFunctionTypeMismatch => "constraint function type mismatch",
            Self::ConstraintGenericArgumentCountMismatch => {
                "constraint generic argument count mismatch"
            }
            Self::InvalidRangeOperandType => "invalid range operand type",
            Self::GenericArgumentCountMismatch => "generic argument count mismatch",
            Self::NonExhaustiveMatch => "non-exhaustive match",
            Self::InvalidDecoratorUsage => "invalid decorator usage",
            Self::RecursiveFunctionStamp => "recursive function stamp",
            Self::InvalidWeakFieldType => "invalid weak field type",
            Self::InvalidEnumBaseType => "invalid enum base type",
        }
    }
}
