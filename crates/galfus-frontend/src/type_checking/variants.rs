use galfus_core::{NodeId, SymbolId, TypeId};

use crate::{PathReferenceKind, SymbolKind, SyntaxNodeKind, TypeKind};

use super::DeclarationTypeChecker;

#[derive(Debug, Clone)]
struct VariantPayload {
    variant_name: String,
    owner_type: TypeId,
    payload_types: Vec<TypeId>,
}

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn infer_path_variant_expression_type(&mut self, node: NodeId) -> Option<TypeId> {
        let resolution = self.graph.resolution()?;
        println!(
            "PATH_EXPR: node={:?}, path_reference_kind={:?}",
            node,
            resolution.path_reference_kind(node)
        );
        let Some(kind) = resolution.path_reference_kind(node) else {
            return self.infer_value_anchor_path_type(node);
        };

        match kind {
            PathReferenceKind::EnumVariant => self.infer_enum_variant_path_type(node),
            PathReferenceKind::ChoiceVariant => self.infer_choice_variant_path_type(node),
            PathReferenceKind::AnchorFunction => self.infer_anchor_function_path_type(node),
            PathReferenceKind::ConstraintMember => self.infer_constraint_member_path_type(node),
            PathReferenceKind::LocalMember => {
                let target = self.graph.syntax().child(node, 0)?;
                let member = self.graph.syntax().child(node, 1)?;
                println!(
                    "LOCAL_MEMBER: target={:?} kind={:?}",
                    target,
                    self.graph.syntax().node(target).map(|n| n.kind())
                );
                let target_type = self.infer_expression_type(target);
                println!("LOCAL_MEMBER: target_type={:?}", target_type);
                let target_type = target_type?;
                let member_name = self.node_text(member);
                let res = self.member_type_for_target_type(target_type, member_name.as_str());
                println!("LOCAL_MEMBER: member_name={:?}, res={:?}", member_name, res);
                res
            }
        }
    }

    pub(super) fn infer_choice_variant_call_type(&mut self, call: NodeId) -> Option<TypeId> {
        let target = self.graph.syntax().child(call, 0)?;
        let arguments = self.graph.syntax().child(call, 1)?;

        let Some(payload) = self.choice_variant_payload(target) else {
            return None;
        };

        if payload.payload_types.is_empty() {
            self.report_choice_payload_not_allowed(target, payload.variant_name.as_str());

            let error = self.layer.table_mut().error();
            self.layer.bind_node_type(call, error);

            return Some(error);
        }

        let argument_nodes = self.call_argument_nodes(arguments);

        self.check_variant_argument_count(call, payload.payload_types.len(), argument_nodes.len());

        for (index, argument) in argument_nodes.iter().copied().enumerate() {
            let Some(expected) = payload.payload_types.get(index).copied() else {
                continue;
            };

            let Some(actual) = self.infer_expression_type(argument) else {
                continue;
            };

            if self.is_assignable(expected, actual) {
                continue;
            }

            self.report_type_mismatch(argument, expected, actual);
        }

        self.layer.bind_node_type(call, payload.owner_type);
        Some(payload.owner_type)
    }

    pub(super) fn is_choice_variant_call_target(&self, target: NodeId) -> bool {
        let Some(resolution) = self.graph.resolution() else {
            return false;
        };

        matches!(
            resolution.path_reference_kind(target),
            Some(PathReferenceKind::ChoiceVariant)
        )
    }

    fn infer_enum_variant_path_type(&mut self, node: NodeId) -> Option<TypeId> {
        let resolution = self.graph.resolution()?;
        let variant_symbol = resolution.path_reference_symbol(node)?;
        let enum_symbol = self.owner_symbol_for_member(variant_symbol, SymbolKind::Enum)?;

        let ty = self
            .layer
            .symbol_type(enum_symbol)
            .unwrap_or_else(|| self.layer.table_mut().intern_named(enum_symbol));

        self.layer.bind_node_type(node, ty);
        Some(ty)
    }

    fn infer_choice_variant_path_type(&mut self, node: NodeId) -> Option<TypeId> {
        let payload = self.choice_variant_payload(node)?;

        if !payload.payload_types.is_empty() {
            self.report_choice_payload_required(node, payload.variant_name.as_str());

            let error = self.layer.table_mut().error();
            self.layer.bind_node_type(node, error);

            return Some(error);
        }

        self.layer.bind_node_type(node, payload.owner_type);
        Some(payload.owner_type)
    }

    fn infer_anchor_function_path_type(&mut self, node: NodeId) -> Option<TypeId> {
        let resolution = self.graph.resolution()?;
        let function_symbol = resolution.path_reference_symbol(node)?;
        let ty = self.layer.symbol_type(function_symbol)?;

        self.layer.bind_node_type(node, ty);
        Some(ty)
    }

    fn infer_constraint_member_path_type(&mut self, node: NodeId) -> Option<TypeId> {
        let resolution = self.graph.resolution()?;
        let member_symbol = resolution.path_reference_symbol(node)?;
        let ty = self.layer.symbol_type(member_symbol)?;

        self.layer.bind_node_type(node, ty);
        Some(ty)
    }

    fn infer_value_anchor_path_type(&mut self, node: NodeId) -> Option<TypeId> {
        let target = self.graph.syntax().child(node, 0)?;
        let member = self.graph.syntax().child(node, 1)?;
        let target_type = self.infer_expression_type(target)?;
        let member_name = self.node_text(member);
        let member_type =
            self.constraint_function_type_for_value_anchor(target_type, member_name.as_str())?;

        self.layer.bind_node_type(node, member_type);
        Some(member_type)
    }

    fn constraint_function_type_for_value_anchor(
        &self,
        target_type: TypeId,
        member_name: &str,
    ) -> Option<TypeId> {
        let target_type = self.resolve_alias_type(target_type);
        let TypeKind::Named { symbol } = self.layer.table().kind(target_type)? else {
            return None;
        };

        let resolution = self.graph.resolution()?;
        let symbol_data = resolution.symbol(*symbol)?;

        if symbol_data.kind() != SymbolKind::Constraint {
            return None;
        }

        let member_scope = resolution.member_scope(*symbol)?;
        let member_symbol = resolution
            .scope(member_scope)
            .and_then(|scope| scope.symbol(member_name))?;
        let member_symbol_data = resolution.symbol(member_symbol)?;

        if member_symbol_data.kind() != SymbolKind::ConstraintFunction {
            return None;
        }

        self.layer.symbol_type(member_symbol)
    }

    fn choice_variant_payload(&mut self, node: NodeId) -> Option<VariantPayload> {
        let resolution = self.graph.resolution()?;

        if resolution.path_reference_kind(node) != Some(PathReferenceKind::ChoiceVariant) {
            return None;
        }

        let variant_symbol = resolution.path_reference_symbol(node)?;
        let owner_symbol = self.owner_symbol_for_member(variant_symbol, SymbolKind::Choice)?;

        let mut owner_type = self
            .layer
            .symbol_type(owner_symbol)
            .unwrap_or_else(|| self.layer.table_mut().intern_named(owner_symbol));

        let variant_name = resolution.symbol(variant_symbol)?.name().to_string();

        let mut payload_types = self.choice_variant_payload_types(owner_symbol, variant_symbol);

        if let Some(target) = self.graph.syntax().child(node, 0) {
            if let Some(target_type) = self.infer_expression_type(target) {
                let resolved = self.resolve_alias_type(target_type);
                if let Some(TypeKind::GenericInstance { arguments, .. }) =
                    self.layer.table().kind(resolved)
                {
                    owner_type = resolved;
                    let choice_type = self.layer.symbol_type(owner_symbol).unwrap_or(owner_type);
                    let parameters = self.generic_expression_parameter_symbols(target, choice_type);
                    let substitution = parameters
                        .into_iter()
                        .zip(arguments.clone())
                        .collect::<std::collections::HashMap<SymbolId, TypeId>>();
                    for payload_type in &mut payload_types {
                        *payload_type =
                            self.substitute_generic_expression_type(*payload_type, &substitution);
                    }
                }
            }
        }

        Some(VariantPayload {
            variant_name,
            owner_type,
            payload_types,
        })
    }

    pub(super) fn choice_variant_payload_types(
        &self,
        owner_symbol: SymbolId,
        variant_symbol: SymbolId,
    ) -> Vec<TypeId> {
        let Some(resolution) = self.graph.resolution() else {
            return Vec::new();
        };

        let Some(owner_data) = resolution.symbol(owner_symbol) else {
            return Vec::new();
        };

        let Some(variant_data) = resolution.symbol(variant_symbol) else {
            return Vec::new();
        };

        let Some(variant_node) =
            self.choice_variant_node_by_name(owner_data.name(), variant_data.name())
        else {
            return Vec::new();
        };

        let Some(payload) =
            self.find_descendant_of_kind(variant_node, SyntaxNodeKind::ChoicePayload)
        else {
            return Vec::new();
        };

        let Some(payload_node) = self.graph.syntax().node(payload) else {
            return Vec::new();
        };

        payload_node
            .children()
            .iter()
            .filter_map(|child| {
                let type_node = self.first_type_child(*child).unwrap_or(*child);
                self.layer.node_type(type_node)
            })
            .collect()
    }

    fn owner_symbol_for_member(
        &self,
        member_symbol: SymbolId,
        owner_kind: SymbolKind,
    ) -> Option<SymbolId> {
        let resolution = self.graph.resolution()?;

        for symbol in resolution.symbols() {
            if symbol.kind() != owner_kind {
                continue;
            }

            let Some(member_scope) = resolution.member_scope(symbol.id()) else {
                continue;
            };

            let Some(scope) = resolution.scope(member_scope) else {
                continue;
            };

            if scope
                .symbol(symbol.name())
                .is_some_and(|candidate| candidate == member_symbol)
            {
                return Some(symbol.id());
            }

            if scope
                .symbols()
                .iter()
                .any(|(_, candidate)| *candidate == member_symbol)
            {
                return Some(symbol.id());
            }
        }

        None
    }

    fn check_variant_argument_count(&mut self, call: NodeId, expected: usize, actual: usize) {
        if expected == actual {
            return;
        }

        self.report_argument_count_mismatch(call, expected, actual);
    }

    fn find_descendant_of_kind(&self, node: NodeId, kind: SyntaxNodeKind) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        for child in syntax_node.children() {
            let child_node = self.graph.syntax().node(*child)?;

            if child_node.kind() == kind {
                return Some(*child);
            }

            if let Some(found) = self.find_descendant_of_kind(*child, kind) {
                return Some(found);
            }
        }

        None
    }

    fn choice_variant_node_by_name(&self, choice_name: &str, variant_name: &str) -> Option<NodeId> {
        let root = self.graph.syntax().root()?;
        let choice_item = self.choice_item_node_by_name(root, choice_name)?;

        self.find_choice_variant_node_by_name(choice_item, variant_name)
    }

    fn choice_item_node_by_name(&self, node: NodeId, choice_name: &str) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        if syntax_node.kind() == SyntaxNodeKind::ChoiceItem {
            let identifier = self
                .graph
                .syntax()
                .first_child_of_kind(node, SyntaxNodeKind::Identifier)?;

            if self.node_text(identifier) == choice_name {
                return Some(node);
            }
        }

        for child in syntax_node.children() {
            if let Some(found) = self.choice_item_node_by_name(*child, choice_name) {
                return Some(found);
            }
        }

        None
    }

    fn find_choice_variant_node_by_name(&self, node: NodeId, variant_name: &str) -> Option<NodeId> {
        let syntax_node = self.graph.syntax().node(node)?;

        if syntax_node.kind() == SyntaxNodeKind::ChoiceVariant {
            let identifier = self
                .graph
                .syntax()
                .first_child_of_kind(node, SyntaxNodeKind::Identifier)?;

            if self.node_text(identifier) == variant_name {
                return Some(node);
            }
        }

        for child in syntax_node.children() {
            if let Some(found) = self.find_choice_variant_node_by_name(*child, variant_name) {
                return Some(found);
            }
        }

        None
    }
}
