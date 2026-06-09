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

    fn can_start_expression(&self) -> bool {
        matches!(
            self.current().kind(),
            TokenKind::Integer
                | TokenKind::Float
                | TokenKind::String
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::Identifier
        )
    }

    fn peek_after_newlines(&self, start_offset: usize) -> &Token {
        let mut offset = start_offset;

        while self.peek(offset).kind() == &TokenKind::Newline {
            offset += 1;
        }

        self.peek(offset)
    }

    fn can_continue_expression_after_newline(kind: &TokenKind) -> bool {
        matches!(kind, TokenKind::Dot | TokenKind::ColonColon)
    }

    // MARK: Start

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
            self.skip_newlines();

            if self.is_eof() {
                break;
            }

            let start_position = self.position;

            if let Some(item) = self.parse_item() {
                self.graph.syntax_mut().push_child(root, item);
            }

            if self.position == start_position {
                self.bump();
            }
        }
    }

    fn parse_item(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::Import) {
            return self.parse_import_item();
        }

        if self.at(&TokenKind::Fn) {
            return self.parse_function_item();
        }

        if self.at(&TokenKind::Type) {
            return self.parse_type_alias_item();
        }

        if self.at(&TokenKind::Struct) {
            return self.parse_struct_item();
        }

        if self.at(&TokenKind::Enum) {
            return self.parse_enum_item();
        }

        if self.at(&TokenKind::Choice) {
            return self.parse_choice_item();
        }

        if self.at(&TokenKind::Export) {
            return self.parse_export_item();
        }

        let found = self.bump();

        self.graph.push_diagnostic(Diagnostic::error_with_message(
            ParserDiagnosticCode::ExpectedItem,
            format!("expected item, found `{:?}`", found.kind()),
            found.span(),
        ));

        None
    }

    // MARK: Items

    fn parse_export_item(&mut self) -> Option<NodeId> {
        let export_token = self.expect(TokenKind::Export)?;

        let item = if self.at(&TokenKind::Fn) {
            self.parse_function_item()?
        } else if self.at(&TokenKind::Type) {
            self.parse_type_alias_item()?
        } else if self.at(&TokenKind::Struct) {
            self.parse_struct_item()?
        } else if self.at(&TokenKind::Enum) {
            self.parse_enum_item()?
        } else if self.at(&TokenKind::Choice) {
            self.parse_choice_item()?
        } else {
            let found = self.bump();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedItem,
                format!("expected exportable item, found `{:?}`", found.kind()),
                found.span(),
            ));

            return None;
        };

        let span =
            Span::cover(export_token.span(), self.node_span(item)).unwrap_or(export_token.span());

        Some(self.add_node(SyntaxNodeKind::ExportItem, span, vec![item]))
    }

    fn parse_import_item(&mut self) -> Option<NodeId> {
        let import_token = self.expect(TokenKind::Import)?;

        let clause = self.parse_import_clause()?;

        self.expect(TokenKind::From)?;

        let source = self.parse_import_source()?;

        let span =
            Span::cover(import_token.span(), self.node_span(source)).unwrap_or(import_token.span());

        Some(self.add_node(SyntaxNodeKind::ImportItem, span, vec![clause, source]))
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

    fn parse_struct_item(&mut self) -> Option<NodeId> {
        let struct_token = self.expect(TokenKind::Struct)?;

        let name = self.parse_identifier()?;
        let fields = self.parse_struct_field_list()?;

        let span =
            Span::cover(struct_token.span(), self.node_span(fields)).unwrap_or(struct_token.span());

        Some(self.add_node(SyntaxNodeKind::StructItem, span, vec![name, fields]))
    }

    fn parse_enum_item(&mut self) -> Option<NodeId> {
        let enum_token = self.expect(TokenKind::Enum)?;

        let name = self.parse_identifier()?;
        let variants = self.parse_enum_variant_list()?;

        let span =
            Span::cover(enum_token.span(), self.node_span(variants)).unwrap_or(enum_token.span());

        Some(self.add_node(SyntaxNodeKind::EnumItem, span, vec![name, variants]))
    }

    fn parse_choice_item(&mut self) -> Option<NodeId> {
        let choice_token = self.expect(TokenKind::Choice)?;

        let name = self.parse_identifier()?;
        let variants = self.parse_choice_variant_list()?;

        let span = Span::cover(choice_token.span(), self.node_span(variants))
            .unwrap_or(choice_token.span());

        Some(self.add_node(SyntaxNodeKind::ChoiceItem, span, vec![name, variants]))
    }

    // MARK: Statements

    fn parse_statement(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::Return) {
            return self.parse_return_statement();
        }

        if self.at(&TokenKind::Var) {
            return self.parse_var_statement();
        }

        if self.at(&TokenKind::Const) {
            return self.parse_const_statement();
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
        let return_token = self.expect(TokenKind::Return)?;

        let mut children = Vec::new();
        let mut end_span = return_token.span();

        if self.can_start_expression() {
            let expression = self.parse_expression()?;
            end_span = self.node_span(expression);
            children.push(expression);
        }

        self.expect_statement_end();

        let span = Span::cover(return_token.span(), end_span).unwrap_or(return_token.span());

        Some(self.add_node(SyntaxNodeKind::ReturnStatement, span, children))
    }

    fn parse_var_statement(&mut self) -> Option<NodeId> {
        let var_token = self.expect(TokenKind::Var)?;
        let name = self.parse_identifier()?;

        let mut children = vec![name];
        let mut end_span = self.node_span(name);

        if self.at(&TokenKind::Colon) {
            let annotation = self.parse_type_annotation()?;
            end_span = self.node_span(annotation);
            children.push(annotation);
        }

        if self.at(&TokenKind::Equal) {
            let initializer = self.parse_initializer()?;
            end_span = self.node_span(initializer);
            children.push(initializer);
        }

        self.expect_statement_end();

        let span = Span::cover(var_token.span(), end_span).unwrap_or(var_token.span());

        Some(self.add_node(SyntaxNodeKind::VarStatement, span, children))
    }

    fn parse_const_statement(&mut self) -> Option<NodeId> {
        let const_token = self.expect(TokenKind::Const)?;
        let name = self.parse_identifier()?;

        let mut children = vec![name];

        if self.at(&TokenKind::Colon) {
            let annotation = self.parse_type_annotation()?;
            children.push(annotation);
        }

        let initializer = self.parse_initializer()?;
        let end_span = self.node_span(initializer);
        children.push(initializer);

        self.expect_statement_end();

        let span = Span::cover(const_token.span(), end_span).unwrap_or(const_token.span());

        Some(self.add_node(SyntaxNodeKind::ConstStatement, span, children))
    }

    fn expect_statement_end(&mut self) {
        if self.at(&TokenKind::Newline) {
            self.skip_newlines();
            return;
        }

        if self.at(&TokenKind::Semicolon) {
            self.bump();
            self.skip_newlines();
            return;
        }

        if self.at(&TokenKind::RightBrace) || self.is_eof() {
            return;
        }

        let found = self.current().clone();

        self.graph.push_diagnostic(Diagnostic::error_with_message(
            ParserDiagnosticCode::ExpectedToken,
            format!("expected statement terminator, found `{:?}`", found.kind()),
            found.span(),
        ));
    }

    // MARK: Literals

    fn parse_integer_literal(&mut self) -> Option<NodeId> {
        let token = self.expect(TokenKind::Integer)?;

        Some(self.add_node(SyntaxNodeKind::IntegerLiteral, token.span(), Vec::new()))
    }

    fn parse_float_literal(&mut self) -> Option<NodeId> {
        let token = self.expect(TokenKind::Float)?;

        Some(self.add_node(SyntaxNodeKind::FloatLiteral, token.span(), Vec::new()))
    }

    fn parse_bool_literal(&mut self) -> Option<NodeId> {
        let token = if self.at(&TokenKind::True) {
            self.bump()
        } else {
            self.expect(TokenKind::False)?
        };

        Some(self.add_node(SyntaxNodeKind::BoolLiteral, token.span(), Vec::new()))
    }

    fn parse_null_literal(&mut self) -> Option<NodeId> {
        let token = self.expect(TokenKind::Null)?;

        Some(self.add_node(SyntaxNodeKind::NullLiteral, token.span(), Vec::new()))
    }

    fn parse_string_literal(&mut self) -> Option<NodeId> {
        let token = self.expect(TokenKind::String)?;

        Some(self.add_node(SyntaxNodeKind::StringLiteral, token.span(), Vec::new()))
    }

    // MARK: Lists

    fn parse_parameter_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftParen)?;

        let mut parameters = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightParen) {
            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            let start_position = self.position;

            if let Some(parameter) = self.parse_parameter() {
                parameters.push(parameter);
            }

            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();
                continue;
            }

            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!("expected `Comma`, found `{:?}`", found.kind()),
                found.span(),
            ));

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightParen)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::ParameterList, span, parameters))
    }

    fn parse_named_import_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut imports = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            if let Some(import) = self.parse_named_import() {
                imports.push(import);
            }

            if !self.at(&TokenKind::Comma) {
                break;
            }

            self.bump();
            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::NamedImportList, span, imports))
    }

    fn parse_struct_field_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut fields = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            let start_position = self.position;

            if let Some(field) = self.parse_struct_field() {
                fields.push(field);
            }

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();
                self.skip_newlines();

                if self.at(&TokenKind::RightBrace) {
                    break;
                }

                continue;
            }

            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!("expected `Comma`, found `{:?}`", found.kind()),
                found.span(),
            ));

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::StructFieldList, span, fields))
    }

    fn parse_enum_variant_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut variants = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            let start_position = self.position;

            if let Some(variant) = self.parse_enum_variant() {
                variants.push(variant);
            }

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();
                self.skip_newlines();

                if self.at(&TokenKind::RightBrace) {
                    break;
                }

                continue;
            }

            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!("expected `Comma`, found `{:?}`", found.kind()),
                found.span(),
            ));

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::EnumVariantList, span, variants))
    }

    fn parse_choice_variant_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftBrace)?;

        let mut variants = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightBrace) {
            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            let start_position = self.position;

            if let Some(variant) = self.parse_choice_variant() {
                variants.push(variant);
            }

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();
                self.skip_newlines();

                if self.at(&TokenKind::RightBrace) {
                    break;
                }

                continue;
            }

            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!("expected `Comma`, found `{:?}`", found.kind()),
                found.span(),
            ));

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::ChoiceVariantList, span, variants))
    }

    fn parse_argument_list(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftParen)?;

        let mut arguments = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightParen) {
            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            let start_position = self.position;

            if let Some(argument) = self.parse_argument() {
                arguments.push(argument);
            }

            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();
                continue;
            }

            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!("expected `Comma`, found `{:?}`", found.kind()),
                found.span(),
            ));

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightParen)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::ArgumentList, span, arguments))
    }

    // MARK: Others

    fn parse_identifier(&mut self) -> Option<NodeId> {
        let token = self.expect(TokenKind::Identifier)?;

        Some(self.add_node(SyntaxNodeKind::Identifier, token.span(), Vec::new()))
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
            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            if let Some(statement) = self.parse_statement() {
                statements.push(statement);
            }
        }

        let right = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::Block, span, statements))
    }

    fn parse_parameter(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;

        self.expect(TokenKind::Colon)?;

        let parameter_type = self.parse_type()?;

        let span = Span::cover(self.node_span(name), self.node_span(parameter_type))
            .unwrap_or_else(|| self.node_span(name));

        Some(self.add_node(SyntaxNodeKind::Parameter, span, vec![name, parameter_type]))
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

    fn parse_import_clause(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::LeftBrace) {
            return self.parse_named_import_list();
        }

        self.parse_namespace_import()
    }

    fn parse_namespace_import(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;
        let span = self.node_span(name);

        Some(self.add_node(SyntaxNodeKind::NamespaceImport, span, vec![name]))
    }

    fn parse_named_import(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;

        let mut children = vec![name];
        let mut end_span = self.node_span(name);

        if self.at(&TokenKind::As) {
            let alias = self.parse_import_alias()?;
            end_span = self.node_span(alias);
            children.push(alias);
        }

        let span =
            Span::cover(self.node_span(name), end_span).unwrap_or_else(|| self.node_span(name));

        Some(self.add_node(SyntaxNodeKind::NamedImport, span, children))
    }

    fn parse_import_source(&mut self) -> Option<NodeId> {
        let literal = self.parse_string_literal()?;
        let span = self.node_span(literal);

        Some(self.add_node(SyntaxNodeKind::ImportSource, span, vec![literal]))
    }

    fn parse_import_alias(&mut self) -> Option<NodeId> {
        let as_token = self.expect(TokenKind::As)?;
        let name = self.parse_identifier()?;

        let span = Span::cover(as_token.span(), self.node_span(name)).unwrap_or(as_token.span());

        Some(self.add_node(SyntaxNodeKind::ImportAlias, span, vec![name]))
    }

    fn parse_struct_field(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;

        self.expect(TokenKind::Colon)?;

        let field_type = self.parse_type()?;

        let span = Span::cover(self.node_span(name), self.node_span(field_type))
            .unwrap_or_else(|| self.node_span(name));

        Some(self.add_node(SyntaxNodeKind::StructField, span, vec![name, field_type]))
    }

    fn parse_enum_variant(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;
        let span = self.node_span(name);

        Some(self.add_node(SyntaxNodeKind::EnumVariant, span, vec![name]))
    }

    fn parse_choice_payload(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftParen)?;

        let mut payload_types = Vec::new();

        while !self.is_eof() && !self.at(&TokenKind::RightParen) {
            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            let start_position = self.position;

            if let Some(payload_type) = self.parse_type() {
                payload_types.push(payload_type);
            }

            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();
                continue;
            }

            let found = self.current().clone();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedToken,
                format!("expected `Comma`, found `{:?}`", found.kind()),
                found.span(),
            ));

            if self.position == start_position {
                self.bump();
            }
        }

        let right = self.expect(TokenKind::RightParen)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::ChoicePayload, span, payload_types))
    }

    fn parse_choice_variant(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;

        let mut children = vec![name];
        let mut end_span = self.node_span(name);

        if self.at(&TokenKind::LeftParen) {
            let payload = self.parse_choice_payload()?;
            end_span = self.node_span(payload);
            children.push(payload);
        }

        let span =
            Span::cover(self.node_span(name), end_span).unwrap_or_else(|| self.node_span(name));

        Some(self.add_node(SyntaxNodeKind::ChoiceVariant, span, children))
    }

    fn parse_type_annotation(&mut self) -> Option<NodeId> {
        let colon = self.expect(TokenKind::Colon)?;
        let type_node = self.parse_type()?;

        let span = Span::cover(colon.span(), self.node_span(type_node)).unwrap_or(colon.span());

        Some(self.add_node(SyntaxNodeKind::TypeAnnotation, span, vec![type_node]))
    }

    fn parse_name_expression(&mut self) -> Option<NodeId> {
        let identifier = self.parse_identifier()?;
        let span = self.node_span(identifier);

        Some(self.add_node(SyntaxNodeKind::NameExpression, span, vec![identifier]))
    }

    fn parse_primary_expression(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::Integer) {
            return self.parse_integer_literal();
        }

        if self.at(&TokenKind::Float) {
            return self.parse_float_literal();
        }

        if self.at(&TokenKind::String) {
            return self.parse_string_literal();
        }

        if self.at(&TokenKind::True) || self.at(&TokenKind::False) {
            return self.parse_bool_literal();
        }

        if self.at(&TokenKind::Null) {
            return self.parse_null_literal();
        }

        if self.at(&TokenKind::Identifier) {
            return self.parse_name_expression();
        }

        let found = self.bump();

        self.graph.push_diagnostic(Diagnostic::error_with_message(
            ParserDiagnosticCode::UnexpectedToken,
            format!("expected expression, found `{:?}`", found.kind()),
            found.span(),
        ));

        None
    }

    fn parse_expression(&mut self) -> Option<NodeId> {
        let mut expression = self.parse_primary_expression()?;

        loop {
            self.skip_soft_newlines_before_expression_continuation();

            if self.at(&TokenKind::Dot) {
                expression = self.parse_member_expression(expression)?;
                continue;
            }

            if self.at(&TokenKind::ColonColon) {
                expression = self.parse_anchor_expression(expression)?;
                continue;
            }

            if self.at(&TokenKind::LeftParen) {
                expression = self.parse_call_expression(expression)?;
                continue;
            }

            break;
        }

        Some(expression)
    }

    fn parse_initializer(&mut self) -> Option<NodeId> {
        let equal = self.expect(TokenKind::Equal)?;
        let expression = self.parse_expression()?;

        let span = Span::cover(equal.span(), self.node_span(expression)).unwrap_or(equal.span());

        Some(self.add_node(SyntaxNodeKind::Initializer, span, vec![expression]))
    }

    fn parse_argument(&mut self) -> Option<NodeId> {
        let expression = self.parse_expression()?;
        let span = self.node_span(expression);

        Some(self.add_node(SyntaxNodeKind::Argument, span, vec![expression]))
    }

    fn parse_call_expression(&mut self, target: NodeId) -> Option<NodeId> {
        let arguments = self.parse_argument_list()?;

        let span = Span::cover(self.node_span(target), self.node_span(arguments))
            .unwrap_or_else(|| self.node_span(target));

        Some(self.add_node(
            SyntaxNodeKind::CallExpression,
            span,
            vec![target, arguments],
        ))
    }

    fn parse_member_expression(&mut self, target: NodeId) -> Option<NodeId> {
        self.expect(TokenKind::Dot)?;

        self.skip_newlines();

        let member = self.parse_identifier()?;

        let span = Span::cover(self.node_span(target), self.node_span(member))
            .unwrap_or_else(|| self.node_span(target));

        Some(self.add_node(SyntaxNodeKind::MemberExpression, span, vec![target, member]))
    }

    fn parse_anchor_expression(&mut self, target: NodeId) -> Option<NodeId> {
        self.expect(TokenKind::ColonColon)?;

        self.skip_newlines();

        let anchor = self.parse_identifier()?;

        let span = Span::cover(self.node_span(target), self.node_span(anchor))
            .unwrap_or_else(|| self.node_span(target));

        Some(self.add_node(SyntaxNodeKind::AnchorExpression, span, vec![target, anchor]))
    }
}

pub fn parse(source: &SourceFile) -> ParseResult {
    let lex_result = lex(source);
    let (tokens, diagnostics) = lex_result.into_parts();
    let mut parser = Parser::new(source, tokens, diagnostics);

    parser.parse_source_file();
    parser.finish()
}
