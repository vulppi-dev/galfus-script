use super::*;

impl Parser {
    pub(super) fn parse_integer_literal(&mut self) -> Option<NodeId> {
        let token = self.expect(TokenKind::Integer)?;

        Some(self.add_node(SyntaxNodeKind::IntegerLiteral, token.span(), Vec::new()))
    }

    pub(super) fn parse_float_literal(&mut self) -> Option<NodeId> {
        let token = self.expect(TokenKind::Float)?;

        Some(self.add_node(SyntaxNodeKind::FloatLiteral, token.span(), Vec::new()))
    }

    pub(super) fn parse_bool_literal(&mut self) -> Option<NodeId> {
        let token = if self.at(&TokenKind::True) {
            self.bump()
        } else {
            self.expect(TokenKind::False)?
        };

        Some(self.add_node(SyntaxNodeKind::BoolLiteral, token.span(), Vec::new()))
    }

    pub(super) fn parse_null_literal(&mut self) -> Option<NodeId> {
        let token = self.expect(TokenKind::Null)?;

        Some(self.add_node(SyntaxNodeKind::NullLiteral, token.span(), Vec::new()))
    }

    pub(super) fn parse_string_literal(&mut self) -> Option<NodeId> {
        let token = self.expect(TokenKind::String)?;

        Some(self.add_node(SyntaxNodeKind::StringLiteral, token.span(), Vec::new()))
    }
}
