use super::*;

impl Parser {
    pub(super) fn can_start_expression(&self) -> bool {
        matches!(
            self.current().kind(),
            TokenKind::Minus
                | TokenKind::Bang
                | TokenKind::Tilde
                | TokenKind::LeftParen
                | TokenKind::LeftBracket
                | TokenKind::Integer
                | TokenKind::Float
                | TokenKind::String
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::Identifier
        )
    }

    pub(super) fn can_continue_expression_after_newline(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Dot
                | TokenKind::ColonColon
                | TokenKind::LeftBracket
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

    pub(super) fn binary_operator_info(kind: &TokenKind) -> Option<(u8, BinaryAssociativity)> {
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

    pub(super) fn is_unary_operator(kind: &TokenKind) -> bool {
        matches!(kind, TokenKind::Minus | TokenKind::Bang | TokenKind::Tilde)
    }

    pub(super) fn is_assignment_operator(kind: &TokenKind) -> bool {
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

    pub(super) fn expression_can_be_assignment_target(&self, expression: NodeId) -> bool {
        let Some(node) = self.graph.syntax().node(expression) else {
            return false;
        };

        matches!(
            node.kind(),
            SyntaxNodeKind::NameExpression
                | SyntaxNodeKind::MemberExpression
                | SyntaxNodeKind::IndexExpression
        )
    }

    pub(super) fn expression_can_be_statement(&self, expression: NodeId) -> bool {
        let Some(node) = self.graph.syntax().node(expression) else {
            return false;
        };

        match node.kind() {
            SyntaxNodeKind::CallExpression => true,

            SyntaxNodeKind::GroupedExpression => node
                .children()
                .first()
                .copied()
                .map(|child| self.expression_can_be_statement(child))
                .unwrap_or(false),

            _ => false,
        }
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

    pub(super) fn parameter_has_default(&self, parameter: NodeId) -> bool {
        let Some(parameter_node) = self.graph.syntax().node(parameter) else {
            return false;
        };

        parameter_node
            .children()
            .iter()
            .any(|child| match self.graph.syntax().node(*child) {
                Some(node) => node.kind() == SyntaxNodeKind::ParameterDefault,
                None => false,
            })
    }
}
