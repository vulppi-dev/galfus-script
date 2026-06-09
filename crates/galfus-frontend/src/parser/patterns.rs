use galfus_core::{Diagnostic, NodeId, Span};

use crate::{ParserDiagnosticCode, SyntaxNodeKind, TokenKind};

use super::Parser;

impl Parser {
    pub(super) fn parse_pattern(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::Integer)
            || self.at(&TokenKind::Float)
            || self.at(&TokenKind::String)
            || self.at(&TokenKind::True)
            || self.at(&TokenKind::False)
            || self.at(&TokenKind::Null)
        {
            return self.parse_literal_pattern();
        }

        if self.at(&TokenKind::Identifier) {
            if self.peek_after_newlines(1).kind() == &TokenKind::ColonColon {
                return self.parse_variant_pattern();
            }

            return self.parse_binding_pattern();
        }

        let found = self.bump();

        self.graph.push_diagnostic(Diagnostic::error_with_message(
            ParserDiagnosticCode::UnexpectedToken,
            format!("expected pattern, found `{:?}`", found.kind()),
            found.span(),
        ));

        None
    }

    pub(super) fn parse_literal_pattern(&mut self) -> Option<NodeId> {
        let literal = if self.at(&TokenKind::Integer) {
            self.parse_integer_literal()?
        } else if self.at(&TokenKind::Float) {
            self.parse_float_literal()?
        } else if self.at(&TokenKind::String) {
            self.parse_string_literal()?
        } else if self.at(&TokenKind::True) || self.at(&TokenKind::False) {
            self.parse_bool_literal()?
        } else if self.at(&TokenKind::Null) {
            self.parse_null_literal()?
        } else {
            let found = self.bump();

            self.graph.push_diagnostic(Diagnostic::error_with_message(
                ParserDiagnosticCode::UnexpectedToken,
                format!("expected literal pattern, found `{:?}`", found.kind()),
                found.span(),
            ));

            return None;
        };

        let span = self.node_span(literal);

        Some(self.add_node(SyntaxNodeKind::LiteralPattern, span, vec![literal]))
    }

    pub(super) fn parse_binding_pattern(&mut self) -> Option<NodeId> {
        let name = self.parse_identifier()?;
        let span = self.node_span(name);

        Some(self.add_node(SyntaxNodeKind::BindingPattern, span, vec![name]))
    }

    pub(super) fn parse_variant_pattern_payload(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftParen)?;

        let mut patterns = Vec::new();

        self.skip_newlines();

        while !self.is_eof() && !self.at(&TokenKind::RightParen) {
            let start_position = self.position;

            if let Some(pattern) = self.parse_pattern() {
                patterns.push(pattern);
            }

            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            if self.at(&TokenKind::Comma) {
                self.bump();
                self.skip_newlines();

                if self.at(&TokenKind::RightParen) {
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

        let right = self.expect(TokenKind::RightParen)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::VariantPatternPayload, span, patterns))
    }

    pub(super) fn parse_variant_pattern(&mut self) -> Option<NodeId> {
        let target = self.parse_identifier()?;

        self.skip_newlines();

        self.expect(TokenKind::ColonColon)?;

        self.skip_newlines();

        let variant = self.parse_identifier()?;

        let mut children = vec![target, variant];
        let mut end_span = self.node_span(variant);

        if self.at(&TokenKind::LeftParen) {
            let payload = self.parse_variant_pattern_payload()?;
            end_span = self.node_span(payload);
            children.push(payload);
        }

        let span =
            Span::cover(self.node_span(target), end_span).unwrap_or_else(|| self.node_span(target));

        Some(self.add_node(SyntaxNodeKind::VariantPattern, span, children))
    }

    pub(super) fn parse_match_arm(&mut self) -> Option<NodeId> {
        let pattern = self.parse_pattern()?;

        self.skip_newlines();

        let body = self.parse_block()?;

        let span = Span::cover(self.node_span(pattern), self.node_span(body))
            .unwrap_or_else(|| self.node_span(pattern));

        Some(self.add_node(SyntaxNodeKind::MatchArm, span, vec![pattern, body]))
    }
}
