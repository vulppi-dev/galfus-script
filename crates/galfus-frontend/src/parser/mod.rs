#[cfg(test)]
mod tests;

mod expressions;
mod items;
mod lists;
mod literals;
mod start;
mod statements;
mod syntax;

use crate::{ModuleGraph, ParserDiagnosticCode, SyntaxNodeKind, Token, TokenKind, lex};
use galfus_core::{Diagnostic, DiagnosticBag, NodeId, SourceFile, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryAssociativity {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct ParseResult {
    graph: ModuleGraph,
}

impl ParseResult {
    pub fn new(graph: ModuleGraph) -> Self {
        Self { graph }
    }

    pub fn graph(&self) -> &ModuleGraph {
        &self.graph
    }

    pub fn into_graph(self) -> ModuleGraph {
        self.graph
    }

    pub fn diagnostics(&self) -> &DiagnosticBag {
        self.graph.diagnostics()
    }

    pub fn has_errors(&self) -> bool {
        self.graph.has_errors()
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    graph: ModuleGraph,
}

impl Parser {
    pub fn new(source: &SourceFile, tokens: Vec<Token>, diagnostics: DiagnosticBag) -> Self {
        let mut graph = ModuleGraph::new(source.id());

        graph.extend_diagnostics(diagnostics.into_vec());

        Self {
            tokens,
            position: 0,
            graph,
        }
    }

    pub fn finish(mut self) -> ParseResult {
        self.graph.syntax_mut().set_tokens(self.tokens);

        ParseResult::new(self.graph)
    }

    fn peek(&self, offset: usize) -> &Token {
        let index = self.position.saturating_add(offset);

        self.tokens
            .get(index)
            .unwrap_or_else(|| self.tokens.last().expect("lexer must emit EOF token"))
    }

    fn current(&self) -> &Token {
        self.peek(0)
    }

    fn is_eof(&self) -> bool {
        self.current().kind() == &TokenKind::Eof
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.current().kind() == kind
    }

    fn bump(&mut self) -> Token {
        let token = self.current().clone();

        if !self.is_eof() {
            self.position += 1;
        }

        token
    }

    fn expect(&mut self, expected: TokenKind) -> Option<Token> {
        if self.at(&expected) {
            return Some(self.bump());
        }

        let found = self.current();

        let message = format!("expected `{:?}`, found `{:?}`", expected, found.kind(),);

        self.graph.push_diagnostic(Diagnostic::error_with_message(
            ParserDiagnosticCode::ExpectedToken,
            message,
            found.span(),
        ));

        None
    }

    fn skip_newlines(&mut self) {
        while self.at(&TokenKind::Newline) {
            self.bump();
        }
    }

    fn skip_soft_newlines_before_expression_continuation(&mut self) -> bool {
        if !self.at(&TokenKind::Newline) {
            return false;
        }

        let next = self.peek_after_newlines(0);

        if !Self::can_continue_expression_after_newline(next.kind()) {
            return false;
        }

        self.skip_newlines();

        true
    }

    fn add_node(&mut self, kind: SyntaxNodeKind, span: Span, children: Vec<NodeId>) -> NodeId {
        self.graph.syntax_mut().add_node(kind, span, children)
    }

    fn node_span(&self, id: NodeId) -> Span {
        self.graph
            .syntax()
            .node(id)
            .expect("node id must exist")
            .span()
    }

    fn peek_after_newlines(&self, start_offset: usize) -> &Token {
        let mut offset = start_offset;

        while self.peek(offset).kind() == &TokenKind::Newline {
            offset += 1;
        }

        self.peek(offset)
    }

    fn can_start_expression(&self) -> bool {
        matches!(
            self.current().kind(),
            TokenKind::Minus
                | TokenKind::Bang
                | TokenKind::Tilde
                | TokenKind::LeftParen
                | TokenKind::Integer
                | TokenKind::Float
                | TokenKind::String
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::Identifier
        )
    }

    fn can_continue_expression_after_newline(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Dot
                | TokenKind::ColonColon
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Percent
                | TokenKind::StarStar
                | TokenKind::Less
                | TokenKind::LessEqual
                | TokenKind::Greater
                | TokenKind::GreaterEqual
                | TokenKind::EqualEqual
                | TokenKind::BangEqual
                | TokenKind::AmpAmp
                | TokenKind::PipePipe
                | TokenKind::QuestionQuestion
        )
    }

    fn binary_operator_info(kind: &TokenKind) -> Option<(u8, BinaryAssociativity)> {
        match kind {
            TokenKind::StarStar => Some((80, BinaryAssociativity::Right)),

            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => {
                Some((70, BinaryAssociativity::Left))
            }

            TokenKind::Plus | TokenKind::Minus => Some((60, BinaryAssociativity::Left)),

            TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual => Some((50, BinaryAssociativity::Left)),

            TokenKind::EqualEqual | TokenKind::BangEqual => Some((45, BinaryAssociativity::Left)),

            TokenKind::AmpAmp => Some((30, BinaryAssociativity::Left)),

            TokenKind::PipePipe => Some((20, BinaryAssociativity::Left)),

            TokenKind::QuestionQuestion => Some((10, BinaryAssociativity::Right)),

            _ => None,
        }
    }

    fn is_unary_operator(kind: &TokenKind) -> bool {
        matches!(kind, TokenKind::Minus | TokenKind::Bang | TokenKind::Tilde)
    }

    fn is_assignment_operator(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Equal
                | TokenKind::PlusEqual
                | TokenKind::MinusEqual
                | TokenKind::StarEqual
                | TokenKind::SlashEqual
                | TokenKind::PercentEqual
                | TokenKind::StarStarEqual
                | TokenKind::AmpEqual
                | TokenKind::PipeEqual
                | TokenKind::CaretEqual
                | TokenKind::ShiftLeftEqual
                | TokenKind::ShiftRightEqual
        )
    }

    fn expression_can_be_assignment_target(&self, expression: NodeId) -> bool {
        let Some(node) = self.graph.syntax().node(expression) else {
            return false;
        };

        matches!(
            node.kind(),
            SyntaxNodeKind::NameExpression | SyntaxNodeKind::MemberExpression
        )
    }
}

pub fn parse(source: &SourceFile) -> ParseResult {
    let lex_result = lex(source);
    let (tokens, diagnostics) = lex_result.into_parts();
    let mut parser = Parser::new(source, tokens, diagnostics);

    parser.parse_source_file();
    parser.finish()
}
