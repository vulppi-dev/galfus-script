use super::*;

impl Parser {
    pub(super) fn parse_statement(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::Return) {
            return self.parse_return_statement();
        }

        if self.at(&TokenKind::Var) {
            return self.parse_var_statement();
        }

        if self.at(&TokenKind::Const) {
            return self.parse_const_statement();
        }

        if self.at(&TokenKind::If) {
            return self.parse_if_statement();
        }

        if self.can_start_expression() {
            return self.parse_expression_statement();
        }

        let found = self.bump();

        self.graph.push_diagnostic(Diagnostic::error_with_message(
            ParserDiagnosticCode::ExpectedStatement,
            format!("expected statement, found `{:?}`", found.kind()),
            found.span(),
        ));

        None
    }

    pub(super) fn parse_return_statement(&mut self) -> Option<NodeId> {
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

    pub(super) fn parse_var_statement(&mut self) -> Option<NodeId> {
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

    pub(super) fn parse_const_statement(&mut self) -> Option<NodeId> {
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

    pub(super) fn expect_statement_end(&mut self) {
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

    pub(super) fn parse_expression_statement(&mut self) -> Option<NodeId> {
        let expression = self.parse_expression()?;
        let span = self.node_span(expression);

        if !self.expression_can_be_statement(expression) {
            let expression_kind = self
                .graph
                .syntax()
                .node(expression)
                .expect("expression node must exist")
                .kind();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::ExpectedStatement,
                format!(
                    "expected call expression statement, found `{:?}`",
                    expression_kind
                ),
                span,
            ));
        }

        self.expect_statement_end();

        Some(self.add_node(SyntaxNodeKind::ExpressionStatement, span, vec![expression]))
    }

    pub(super) fn parse_if_statement(&mut self) -> Option<NodeId> {
        let if_token = self.expect(TokenKind::If)?;

        self.skip_newlines();

        let condition = self.parse_expression()?;

        self.skip_newlines();

        let then_block = self.parse_block()?;

        let mut children = vec![condition, then_block];
        let mut end_span = self.node_span(then_block);

        self.skip_newlines();

        if self.at(&TokenKind::Else) {
            let else_clause = self.parse_else_clause()?;
            end_span = self.node_span(else_clause);
            children.push(else_clause);
        }

        let span = Span::cover(if_token.span(), end_span).unwrap_or(if_token.span());

        Some(self.add_node(SyntaxNodeKind::IfStatement, span, children))
    }

    pub(super) fn parse_else_clause(&mut self) -> Option<NodeId> {
        let else_token = self.expect(TokenKind::Else)?;

        self.skip_newlines();

        let child = if self.at(&TokenKind::If) {
            self.parse_if_statement()?
        } else {
            self.parse_block()?
        };

        let span =
            Span::cover(else_token.span(), self.node_span(child)).unwrap_or(else_token.span());

        Some(self.add_node(SyntaxNodeKind::ElseClause, span, vec![child]))
    }
}
