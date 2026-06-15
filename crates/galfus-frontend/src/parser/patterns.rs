use super::*;
use crate::{ParserDiagnosticCode, SyntaxNodeKind, TokenKind};
use galfus_core::{Diagnostic, NodeId, Span};

impl Parser {
    pub(super) fn parse_pattern(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::Regex) {
            return self.parse_regex_pattern();
        }

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
        self.skip_newlines();

        let inner = if self.at(&TokenKind::LeftBrace) {
            self.parse_struct_binding_pattern()?
        } else if self.at(&TokenKind::LeftParen) {
            self.parse_tuple_binding_pattern()?
        } else if self.at(&TokenKind::LeftBracket) {
            self.parse_array_binding_pattern()?
        } else {
            self.parse_identifier()?
        };

        let span = self.node_span(inner);

        Some(self.add_node(SyntaxNodeKind::BindingPattern, span, vec![inner]))
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

        self.expect(TokenKind::Arrow)?;

        self.skip_newlines();

        let body = self.parse_block()?;

        let span = Span::cover(self.node_span(pattern), self.node_span(body))
            .unwrap_or_else(|| self.node_span(pattern));

        Some(self.add_node(SyntaxNodeKind::MatchArm, span, vec![pattern, body]))
    }

    pub(super) fn parse_type_pattern_binding(&mut self) -> Option<NodeId> {
        let left = self.expect(TokenKind::LeftParen)?;

        self.skip_newlines();

        let name = self.parse_identifier()?;

        self.skip_newlines();

        let right = self.expect(TokenKind::RightParen)?;

        let span = Span::cover(left.span(), right.span()).unwrap_or(left.span());

        Some(self.add_node(SyntaxNodeKind::TypePatternBinding, span, vec![name]))
    }

    pub(super) fn parse_type_pattern(&mut self) -> Option<NodeId> {
        let pattern_type = self.parse_type()?;

        let mut children = vec![pattern_type];
        let mut end_span = self.node_span(pattern_type);

        self.skip_newlines();

        if self.at(&TokenKind::LeftParen) {
            let binding = self.parse_type_pattern_binding()?;
            end_span = self.node_span(binding);
            children.push(binding);
        }

        let span = Span::cover(self.node_span(pattern_type), end_span)
            .unwrap_or_else(|| self.node_span(pattern_type));

        Some(self.add_node(SyntaxNodeKind::TypePattern, span, children))
    }

    pub(super) fn parse_instanceof_pattern(&mut self) -> Option<NodeId> {
        if self.at(&TokenKind::Identifier)
            && self.peek_after_newlines(1).kind() != &TokenKind::LeftParen
        {
            return self.parse_binding_pattern();
        }

        self.parse_type_pattern()
    }

    pub(super) fn parse_regex_pattern(&mut self) -> Option<NodeId> {
        let regex = self.parse_regex_literal()?;
        let span = self.node_span(regex);

        Some(self.add_node(SyntaxNodeKind::RegexPattern, span, vec![regex]))
    }

    pub(super) fn parse_struct_binding_pattern(&mut self) -> Option<NodeId> {
        let open = self.expect(TokenKind::LeftBrace)?;

        self.skip_newlines();

        let mut fields = Vec::new();

        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            self.skip_newlines();

            if self.at(&TokenKind::RightBrace) {
                break;
            }

            let field = self.parse_struct_binding_field()?;
            fields.push(field);

            self.skip_newlines();

            if self.at(&TokenKind::Comma) {
                self.bump();
                self.skip_newlines();
                continue;
            }

            break;
        }

        let close = self.expect(TokenKind::RightBrace)?;

        let span = Span::cover(open.span(), close.span()).unwrap_or(open.span());

        Some(self.add_node(SyntaxNodeKind::StructBindingPattern, span, fields))
    }

    pub(super) fn parse_struct_binding_field(&mut self) -> Option<NodeId> {
        self.skip_newlines();

        let name = self.parse_identifier()?;
        let mut children = vec![name];
        let mut end_span = self.node_span(name);

        self.skip_newlines();

        if self.at(&TokenKind::Colon) {
            self.bump();
            self.skip_newlines();

            let alias = self.parse_binding_pattern()?;
            end_span = self.node_span(alias);
            children.push(alias);
        }

        let span =
            Span::cover(self.node_span(name), end_span).unwrap_or_else(|| self.node_span(name));

        Some(self.add_node(SyntaxNodeKind::StructBindingField, span, children))
    }

    pub(super) fn parse_tuple_binding_pattern(&mut self) -> Option<NodeId> {
        let open = self.expect(TokenKind::LeftParen)?;

        self.skip_newlines();

        let mut elements = Vec::new();

        while !self.at(&TokenKind::RightParen) && !self.at(&TokenKind::Eof) {
            self.skip_newlines();

            if self.at(&TokenKind::RightParen) {
                break;
            }

            let element = self.parse_binding_pattern()?;
            elements.push(element);

            self.skip_newlines();

            if self.at(&TokenKind::Comma) {
                self.bump();
                self.skip_newlines();
                continue;
            }

            break;
        }

        let close = self.expect(TokenKind::RightParen)?;

        let span = Span::cover(open.span(), close.span()).unwrap_or(open.span());

        Some(self.add_node(SyntaxNodeKind::TupleBindingPattern, span, elements))
    }

    pub(super) fn parse_array_binding_pattern(&mut self) -> Option<NodeId> {
        let open = self.expect(TokenKind::LeftBracket)?;

        self.skip_newlines();

        let mut elements = Vec::new();

        while !self.at(&TokenKind::RightBracket) && !self.at(&TokenKind::Eof) {
            self.skip_newlines();

            if self.at(&TokenKind::RightBracket) {
                break;
            }

            let element = if self.at(&TokenKind::DotDotDot) {
                self.parse_rest_binding_pattern()?
            } else {
                self.parse_binding_pattern()?
            };

            elements.push(element);

            self.skip_newlines();

            if self.at(&TokenKind::Comma) {
                self.bump();
                self.skip_newlines();
                continue;
            }

            break;
        }

        let close = self.expect(TokenKind::RightBracket)?;

        let span = Span::cover(open.span(), close.span()).unwrap_or(open.span());

        Some(self.add_node(SyntaxNodeKind::ArrayBindingPattern, span, elements))
    }

    pub(super) fn parse_rest_binding_pattern(&mut self) -> Option<NodeId> {
        let spread = self.expect(TokenKind::DotDotDot)?;

        self.skip_newlines();

        let pattern = self.parse_binding_pattern()?;

        let span = Span::cover(spread.span(), self.node_span(pattern)).unwrap_or(spread.span());

        Some(self.add_node(SyntaxNodeKind::RestBindingPattern, span, vec![pattern]))
    }
}
