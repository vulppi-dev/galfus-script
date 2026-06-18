use galfus_core::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    kind: TokenKind,
    span: Span,
}

impl Token {
    pub const fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub const fn kind(&self) -> &TokenKind {
        &self.kind
    }

    pub const fn span(&self) -> Span {
        self.span
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // Special
    Eof,
    Newline,
    Unknown,

    // Literals / identifiers
    Identifier,
    Integer,
    Float,
    String,

    // Keywords
    Import,
    From,
    Export,
    As,
    Var,
    Const,
    Fn,
    Return,
    Struct,
    Enum,
    Choice,
    Type,
    Constraint,
    Satisfies,
    Match,
    Instanceof,
    If,
    Else,
    For,
    In,
    While,
    Loop,
    Break,
    Continue,
    Weak,
    Null,
    True,
    False,
    Copy,
    Stamp,

    // Delimiters
    LeftParen,    // (
    RightParen,   // )
    LeftBrace,    // {
    RightBrace,   // }
    LeftBracket,  // [
    RightBracket, // ]

    // Punctuation
    Comma,     // ,
    Dot,       // .
    Colon,     // :
    Semicolon, // ;
    At,        // @

    // Operators
    Plus,     // +
    Minus,    // -
    Star,     // *
    Slash,    // /
    Percent,  // %
    StarStar, // **

    Bang,       // !
    Equal,      // =
    EqualEqual, // ==
    BangEqual,  // !=

    Less,         // <
    LessEqual,    // <=
    Greater,      // >
    GreaterEqual, // >=

    Amp,   // &
    Pipe,  // |
    Caret, // ^
    Tilde, // ~

    AmpAmp,   // &&
    PipePipe, // ||

    ShiftLeft,  // <<
    ShiftRight, // >>

    PlusEqual,     // +=
    MinusEqual,    // -=
    StarEqual,     // *=
    SlashEqual,    // /=
    PercentEqual,  // %=
    StarStarEqual, // **=

    AmpEqual,        // &=
    PipeEqual,       // |=
    CaretEqual,      // ^=
    ShiftLeftEqual,  // <<=
    ShiftRightEqual, // >>=

    PlusPlus,   // ++
    MinusMinus, // --

    QuestionDot,           // ?.
    QuestionQuestion,      // ??
    QuestionQuestionEqual, // ??=

    ColonColon, // ::
    DotDot,     // ..
    DotDotDot,  // ...
    Arrow,      // =>
}
