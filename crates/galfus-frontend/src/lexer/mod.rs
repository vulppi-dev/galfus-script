use crate::{LexicalDiagnosticCode, Token, TokenKind};
use galfus_core::{Diagnostic, Span};

#[cfg(test)]
mod tests;

mod cursor;
mod identifier;
mod numbers;
mod result;
mod state;
mod strings;
mod tokenize;
mod trivia;

pub use result::LexResult;
pub use state::{Lexer, lex};
