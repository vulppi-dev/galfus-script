#[cfg(test)]
mod tests;

use crate::{ModuleGraph, ParserDiagnosticCode, SyntaxNodeKind, Token, TokenKind, lex};
use galfus_core::{Diagnostic, DiagnosticBag, NodeId, SourceFile, Span};

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

    fn source_file_span(&self) -> Span {
        let end = self
            .tokens
            .last()
            .map(|token| token.span().start())
            .unwrap_or(0);

        Span::new(self.graph.source_id(), 0, end)
    }

    fn parse_source_file(&mut self) {
        let span = self.source_file_span();

        let root = self.add_node(SyntaxNodeKind::SourceFile, span, Vec::new());

        self.graph.syntax_mut().set_root(root);

        while !self.is_eof() {
            if self.at(&TokenKind::Fn) {
                if let Some(item) = self.parse_function_item() {
                    self.graph.syntax_mut().push_child(root, item);
                }

                continue;
            }

            if self.at(&TokenKind::Type) {
                if let Some(item) = self.parse_type_alias_item() {
                    self.graph.syntax_mut().push_child(root, item);
                }

                continue;
            }

            let found = self.bump();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedItem,
                format!("expected item, found `{:?}`", found.kind()),
                found.span(),
            ));
        }
    }

    fn parse_function_item(&mut self) -> Option<NodeId> {
        let fn_token = self.expect(TokenKind::Fn)?;

        let name = self.parse_identifier()?;
        let parameters = self.parse_parameter_list()?;

        self.expect(TokenKind::Colon)?;

        let return_type = self.parse_type()?;
        let body = self.parse_block()?;

        let span = Span::cover(fn_token.span(), self.node_span(body)).unwrap_or(fn_token.span());

        Some(self.add_node(
            SyntaxNodeKind::FunctionItem,
            span,
            vec![name, parameters, return_type, body],
        ))
    }

    fn parse_type_alias_item(&mut self) -> Option<NodeId> {
        let type_token = self.expect(TokenKind::Type)?;

        let name = self.parse_identifier()?;

        self.expect(TokenKind::Equal)?;

        let aliased_type = self.parse_type()?;

        let span = Span::cover(type_token.span(), self.node_span(aliased_type))
            .unwrap_or(type_token.span());

        Some(self.add_node(
            SyntaxNodeKind::TypeAliasItem,
            span,
            vec![name, aliased_type],
        ))
    }

    fn parse_identifier(&mut self) -> Option<NodeId> {
        let token = self.expect(TokenKind::Identifier)?;

        Some(self.add_node(SyntaxNodeKind::Identifier, token.span(), Vec::new()))
    }

    fn parse_parameter_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftParen)?;

        let mut parameters = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightParen) {
            if let Some(parameter) = self.parse_parameter() {
                parameters.push(parameter);
            }

            if !self.at(&TokenKind::Comma) {
                break;
            }

            self.bump();

            if self.at(&TokenKind::RightParen) {
                break;
            }
        }

        let right = self.expect(TokenKind::RightParen)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::ParameterList, span, parameters))
    }

    fn parse_type(&mut self) -> Option<NodeId> {
        let first = self.parse_primary_type()?;

        if !self.at(&TokenKind::Pipe) {
            return Some(first);
        }

        let mut types = vec![first];
        let start_span = self.node_span(first);

        while self.at(&TokenKind::Pipe) {
            self.bump();

            let next = self.parse_primary_type()?;
            types.push(next);
        }

        let last = *types
            .last()
            .expect("union type must have at least one type");
        let span = Span::cover(start_span, self.node_span(last)).unwrap_or(start_span);

        Some(self.add_node(SyntaxNodeKind::UnionType, span, types))
    }

    fn parse_primary_type(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::Null) {
            let token = self.bump();

            return Some(self.add_node(SyntaxNodeKind::TypeNull, token.span(), Vec::new()));
        }

        if self.at(&TokenKind::LeftBracket) {
            return self.parse_array_type();
        }

        if self.at(&TokenKind::Identifier) {
            let identifier = self.parse_identifier()?;
            let span = self.node_span(identifier);

            return Some(self.add_node(SyntaxNodeKind::TypeName, span, vec![identifier]));
        }

        let found = self.bump();

        self.graph.push_diagnostic(Diagnostic::error_with_message(
            ParserDiagnosticCode::ExpectedType,
            format!("expected type, found `{:?}`", found.kind()),
            found.span(),
        ));

        None
    }

    fn parse_block(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut statements = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            if let Some(statement) = self.parse_statement() {
                statements.push(statement);
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::Block, span, statements))
    }

    fn parse_statement(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::Return) {
            return self.parse_return_statement();
        }

        let found = self.bump();

        self.graph.push_diagnostic(Diagnostic::error_with_message(
            ParserDiagnosticCode::ExpectedStatement,
            format!("expected statement, found `{:?}`", found.kind()),
            found.span(),
        ));

        None
    }

    fn parse_return_statement(&mut self) -> Option<NodeId> {
        let token = self.expect(TokenKind::Return)?;

        Some(self.add_node(SyntaxNodeKind::ReturnStatement, token.span(), Vec::new()))
    }

    fn parse_parameter(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;

        self.expect(TokenKind::Colon)?;

        let parameter_type = self.parse_type()?;

        let span = Span::cover(self.node_span(name), self.node_span(parameter_type))
            .unwrap_or_else(|| self.node_span(name));

        Some(self.add_node(SyntaxNodeKind::Parameter, span, vec![name, parameter_type]))
    }

    fn parse_integer_literal(&mut self) -> Option<NodeId> {
        let token = self.expect(TokenKind::Integer)?;

        Some(self.add_node(SyntaxNodeKind::IntegerLiteral, token.span(), Vec::new()))
    }

    fn parse_array_type(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBracket)?;

        let element_type = self.parse_type()?;

        if self.at(&TokenKind::Semicolon) {
            self.bump();

            let size = self.parse_array_size()?;
            let right = self.expect(TokenKind::RightBracket)?;

            let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

            return Some(self.add_node(
                SyntaxNodeKind::FixedArrayType,
                span,
                vec![element_type, size],
            ));
        }

        let right = self.expect(TokenKind::RightBracket)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::ArrayType, span, vec![element_type]))
    }

    fn parse_array_size(&mut self) -> Option<NodeId> {
        if !self.at(&TokenKind::Integer) {
            let found = self.bump();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!(
                    "expected array size integer literal, found `{:?}`",
                    found.kind()
                ),
                found.span(),
            ));

            return None;
        }

        let value = self.parse_integer_literal()?;
        let span = self.node_span(value);

        Some(self.add_node(SyntaxNodeKind::ArraySize, span, vec![value]))
    }
}

pub fn parse(source: &SourceFile) -> ParseResult {
    let lex_result = lex(source);
    let (tokens, diagnostics) = lex_result.into_parts();
    let mut parser = Parser::new(source, tokens, diagnostics);

    parser.parse_source_file();
    parser.finish()
}
