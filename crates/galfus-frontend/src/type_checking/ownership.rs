use std::collections::{HashMap, HashSet};

use galfus_core::{NodeId, SymbolId, TypeId};

use crate::{PrimitiveType, SymbolKind, SyntaxNodeKind, TypeDiagnosticCode, TypeKind};

use super::{
    AnchorKind, AnchorMetadata, CaptureMetadata, DeclarationTypeChecker, EdgeMetadata,
    OwnershipCycleMetadata, ReleaseEligibilityKind, ReleaseEligibilityMetadata, TemporaryMetadata,
    WeakFieldMetadata, WeakObserverMetadata,
};

impl<'a> DeclarationTypeChecker<'a> {
    pub(super) fn check_ownership_metadata(&mut self, root: NodeId) {
        let mut seen_anchors = HashSet::new();
        let mut seen_edges = HashSet::new();
        let mut seen_captures = HashSet::new();
        let mut seen_temporaries = HashSet::new();

        self.collect_ownership_metadata(
            root,
            None,
            &mut seen_anchors,
            &mut seen_edges,
            &mut seen_captures,
            &mut seen_temporaries,
        );
        self.validate_ownership_cycles();
        self.validate_release_eligibility();
    }

    fn collect_ownership_metadata(
        &mut self,
        node: NodeId,
        owner_struct: Option<SymbolId>,
        seen_anchors: &mut HashSet<(AnchorKind, NodeId, Option<SymbolId>)>,
        seen_edges: &mut HashSet<(SymbolId, SymbolId)>,
        seen_captures: &mut HashSet<(NodeId, SymbolId)>,
        seen_temporaries: &mut HashSet<NodeId>,
    ) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        let owner_struct = if syntax_node.kind() == SyntaxNodeKind::StructItem {
            self.direct_identifier_symbol(node, SymbolKind::Struct)
                .or(owner_struct)
        } else {
            owner_struct
        };

        self.collect_anchor_metadata(node, seen_anchors);
        self.collect_temporary_metadata(node, seen_anchors, seen_temporaries);

        match syntax_node.kind() {
            SyntaxNodeKind::StructField => {
                self.collect_struct_edge_metadata(node, owner_struct, seen_edges);
            }

            SyntaxNodeKind::WeakStructField => {
                self.check_weak_struct_field(node, owner_struct, seen_edges);
            }

            SyntaxNodeKind::ArrowFunctionExpression => {
                self.collect_closure_capture_metadata(node, seen_captures);
            }

            SyntaxNodeKind::FunctionItem => {
                self.collect_function_anchor_metadata(node, seen_anchors);
            }

            _ => {}
        }

        let children = syntax_node.children().to_vec();

        for child in children {
            self.collect_ownership_metadata(
                child,
                owner_struct,
                seen_anchors,
                seen_edges,
                seen_captures,
                seen_temporaries,
            );
        }
    }

    fn collect_anchor_metadata(
        &mut self,
        node: NodeId,
        seen_anchors: &mut HashSet<(AnchorKind, NodeId, Option<SymbolId>)>,
    ) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        let (kind, symbol_kinds) = match syntax_node.kind() {
            SyntaxNodeKind::VarItem | SyntaxNodeKind::ConstItem => (
                AnchorKind::ModuleState,
                &[SymbolKind::Var, SymbolKind::Const][..],
            ),

            SyntaxNodeKind::VarStatement | SyntaxNodeKind::ConstStatement => (
                AnchorKind::BlockLocal,
                &[SymbolKind::Var, SymbolKind::Const][..],
            ),

            SyntaxNodeKind::Parameter | SyntaxNodeKind::RestParameter => (
                AnchorKind::FunctionParameter,
                &[SymbolKind::Parameter, SymbolKind::RestParameter][..],
            ),

            SyntaxNodeKind::ForBinding => (AnchorKind::BlockLocal, &[SymbolKind::ForBinding][..]),

            SyntaxNodeKind::ArrowFunctionExpression => (AnchorKind::Closure, &[][..]),

            _ => return,
        };

        if symbol_kinds.is_empty() {
            self.push_anchor_metadata(kind, node, None, self.layer.node_type(node), seen_anchors);
            return;
        }

        for symbol in self.declaration_symbols_in_node(node, symbol_kinds) {
            let ty = self.layer.symbol_type(symbol);
            self.push_anchor_metadata(kind, node, Some(symbol), ty, seen_anchors);
        }
    }

    fn collect_function_anchor_metadata(
        &mut self,
        function: NodeId,
        seen_anchors: &mut HashSet<(AnchorKind, NodeId, Option<SymbolId>)>,
    ) {
        let Some(anchor) = self
            .graph
            .syntax()
            .first_child_of_kind(function, SyntaxNodeKind::FunctionAnchor)
        else {
            return;
        };

        let function_symbol = self.direct_identifier_symbol(function, SymbolKind::Function);
        let anchor_type = self
            .graph
            .syntax()
            .first_child(anchor)
            .and_then(|ty| self.layer.node_type(ty));

        self.push_anchor_metadata(
            AnchorKind::FunctionAnchor,
            anchor,
            function_symbol,
            anchor_type,
            seen_anchors,
        );
    }

    fn push_anchor_metadata(
        &mut self,
        kind: AnchorKind,
        node: NodeId,
        symbol: Option<SymbolId>,
        ty: Option<TypeId>,
        seen_anchors: &mut HashSet<(AnchorKind, NodeId, Option<SymbolId>)>,
    ) {
        if !seen_anchors.insert((kind, node, symbol)) {
            return;
        }

        self.ownership_metadata
            .anchors
            .push(AnchorMetadata::new(kind, node, symbol, ty));
    }

    fn collect_struct_edge_metadata(
        &mut self,
        field: NodeId,
        owner_struct: Option<SymbolId>,
        seen_edges: &mut HashSet<(SymbolId, SymbolId)>,
    ) {
        let Some(owner_struct) = owner_struct else {
            return;
        };

        let Some(field_symbol) = self.direct_identifier_symbol(field, SymbolKind::StructField)
        else {
            return;
        };

        let Some(field_type) = self.layer.symbol_type(field_symbol) else {
            return;
        };

        if !self.is_owned_edge_type(field_type) {
            return;
        }

        if !seen_edges.insert((owner_struct, field_symbol)) {
            return;
        }

        self.ownership_metadata.edges.push(EdgeMetadata::new(
            owner_struct,
            field_symbol,
            field,
            field_type,
        ));
    }

    fn check_weak_struct_field(
        &mut self,
        field: NodeId,
        owner_struct: Option<SymbolId>,
        seen_edges: &mut HashSet<(SymbolId, SymbolId)>,
    ) {
        let Some(owner_struct) = owner_struct else {
            return;
        };

        let Some(field_symbol) = self.direct_identifier_symbol(field, SymbolKind::StructField)
        else {
            return;
        };

        let Some(field_type) = self.layer.symbol_type(field_symbol) else {
            return;
        };

        if !self.is_weak_field_nullable_type(field_type) {
            self.report_invalid_weak_field_type(field, field_type);
        }

        self.ownership_metadata
            .weak_fields
            .push(WeakFieldMetadata::new(
                owner_struct,
                field_symbol,
                field,
                field_type,
            ));

        if !seen_edges.insert((owner_struct, field_symbol)) {
            return;
        }

        self.ownership_metadata
            .weak_observers
            .push(WeakObserverMetadata::new(
                owner_struct,
                field_symbol,
                field,
                field_type,
            ));
    }

    fn collect_closure_capture_metadata(
        &mut self,
        closure: NodeId,
        seen_captures: &mut HashSet<(NodeId, SymbolId)>,
    ) {
        let local_symbols = self.local_symbols_in_closure(closure);
        let mut references = Vec::new();
        self.collect_closure_references(closure, true, &mut references);

        for (reference, symbol) in references {
            if local_symbols.contains(&symbol) {
                continue;
            }

            let Some(symbol_data) = self.graph.resolution().and_then(|resolution| {
                resolution
                    .symbol(symbol)
                    .map(|symbol_data| (symbol_data.kind(), symbol_data.declaration()))
            }) else {
                continue;
            };

            if !matches!(
                symbol_data.0,
                SymbolKind::Var
                    | SymbolKind::Const
                    | SymbolKind::Parameter
                    | SymbolKind::RestParameter
                    | SymbolKind::ForBinding
                    | SymbolKind::PatternBinding
                    | SymbolKind::TypePatternBinding
            ) {
                continue;
            }

            if self.node_contains_child(closure, symbol_data.1) {
                continue;
            }

            let Some(ty) = self.layer.symbol_type(symbol) else {
                continue;
            };

            if !seen_captures.insert((closure, symbol)) {
                continue;
            }

            self.ownership_metadata
                .captures
                .push(CaptureMetadata::new(closure, reference, symbol, ty));
        }
    }

    fn local_symbols_in_closure(&self, closure: NodeId) -> HashSet<SymbolId> {
        let mut symbols = HashSet::new();

        self.collect_local_symbols_in_closure(closure, true, &mut symbols);

        symbols
    }

    fn collect_local_symbols_in_closure(
        &self,
        node: NodeId,
        is_root: bool,
        symbols: &mut HashSet<SymbolId>,
    ) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        if !is_root && syntax_node.kind() == SyntaxNodeKind::ArrowFunctionExpression {
            return;
        }

        for symbol in self.declaration_symbols_in_node(
            node,
            &[
                SymbolKind::Var,
                SymbolKind::Const,
                SymbolKind::Parameter,
                SymbolKind::RestParameter,
                SymbolKind::ForBinding,
                SymbolKind::PatternBinding,
                SymbolKind::TypePatternBinding,
            ],
        ) {
            symbols.insert(symbol);
        }

        for child in syntax_node.children() {
            self.collect_local_symbols_in_closure(*child, false, symbols);
        }
    }

    fn collect_closure_references(
        &self,
        node: NodeId,
        is_root: bool,
        references: &mut Vec<(NodeId, SymbolId)>,
    ) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        if !is_root && syntax_node.kind() == SyntaxNodeKind::ArrowFunctionExpression {
            return;
        }

        if syntax_node.kind() == SyntaxNodeKind::NameExpression
            && let Some(symbol) = self
                .graph
                .resolution()
                .and_then(|resolution| resolution.reference_symbol(node))
        {
            references.push((node, symbol));
        }

        for child in syntax_node.children() {
            self.collect_closure_references(*child, false, references);
        }
    }

    fn collect_temporary_metadata(
        &mut self,
        node: NodeId,
        seen_anchors: &mut HashSet<(AnchorKind, NodeId, Option<SymbolId>)>,
        seen_temporaries: &mut HashSet<NodeId>,
    ) {
        let Some(syntax_node) = self.graph.syntax().node(node) else {
            return;
        };

        if !matches!(
            syntax_node.kind(),
            SyntaxNodeKind::ArrayLiteral
                | SyntaxNodeKind::StructLiteral
                | SyntaxNodeKind::InferredStructLiteral
                | SyntaxNodeKind::TupleExpression
                | SyntaxNodeKind::CopyExpression
                | SyntaxNodeKind::CallExpression
        ) {
            return;
        }

        let Some(ty) = self.layer.node_type(node) else {
            return;
        };

        if !self.is_owned_edge_type(ty) {
            return;
        }

        if !seen_temporaries.insert(node) {
            return;
        }

        self.ownership_metadata
            .temporaries
            .push(TemporaryMetadata::new(node, ty));
        self.push_anchor_metadata(AnchorKind::Temporary, node, None, Some(ty), seen_anchors);
    }

    fn is_owned_edge_type(&self, ty: TypeId) -> bool {
        let ty = self.resolve_alias_type(ty);

        match self.layer.table().kind(ty) {
            Some(TypeKind::Named { symbol }) => self
                .graph
                .resolution()
                .and_then(|resolution| resolution.symbol(*symbol))
                .is_some_and(|symbol| {
                    matches!(symbol.kind(), SymbolKind::Struct | SymbolKind::Choice)
                }),

            Some(TypeKind::Array { .. })
            | Some(TypeKind::FixedArray { .. })
            | Some(TypeKind::Tuple { .. }) => true,

            Some(TypeKind::Union { members }) => members
                .iter()
                .copied()
                .any(|member| self.is_owned_edge_type(member)),

            _ => false,
        }
    }

    fn validate_ownership_cycles(&mut self) {
        let graph = self.strong_edge_graph();
        let mut seen_cycles = HashSet::new();

        for start in graph.keys().copied() {
            let mut path = Vec::new();
            let mut visiting = HashSet::new();

            self.collect_ownership_cycles_from(
                start,
                start,
                &graph,
                &mut path,
                &mut visiting,
                &mut seen_cycles,
            );
        }
    }

    fn strong_edge_graph(&self) -> HashMap<SymbolId, Vec<SymbolId>> {
        let mut graph = HashMap::new();

        for edge in &self.ownership_metadata.edges {
            let mut targets = Vec::new();
            self.collect_owned_struct_symbols(edge.field_type(), &mut targets);

            if targets.is_empty() {
                continue;
            }

            let entry = graph.entry(edge.owner_symbol()).or_insert_with(Vec::new);

            for target in targets {
                if !entry.contains(&target) {
                    entry.push(target);
                }
            }
        }

        graph
    }

    fn collect_ownership_cycles_from(
        &mut self,
        start: SymbolId,
        current: SymbolId,
        graph: &HashMap<SymbolId, Vec<SymbolId>>,
        path: &mut Vec<SymbolId>,
        visiting: &mut HashSet<SymbolId>,
        seen_cycles: &mut HashSet<Vec<u32>>,
    ) {
        if !visiting.insert(current) {
            return;
        }

        path.push(current);

        if let Some(targets) = graph.get(&current) {
            for target in targets {
                if *target == start {
                    let mut cycle = path.clone();
                    cycle.push(start);

                    if self.record_ownership_cycle(cycle, seen_cycles) {
                        continue;
                    }
                }

                if visiting.contains(target) {
                    continue;
                }

                self.collect_ownership_cycles_from(
                    start,
                    *target,
                    graph,
                    path,
                    visiting,
                    seen_cycles,
                );
            }
        }

        path.pop();
        visiting.remove(&current);
    }

    fn record_ownership_cycle(
        &mut self,
        cycle: Vec<SymbolId>,
        seen_cycles: &mut HashSet<Vec<u32>>,
    ) -> bool {
        let key = self.ownership_cycle_key(cycle.as_slice());

        if !seen_cycles.insert(key) {
            return false;
        }

        self.ownership_metadata
            .cycles
            .push(OwnershipCycleMetadata::new(cycle));

        true
    }

    fn ownership_cycle_key(&self, cycle: &[SymbolId]) -> Vec<u32> {
        let mut raw_cycle = cycle
            .iter()
            .take(cycle.len().saturating_sub(1))
            .map(|symbol| symbol.raw())
            .collect::<Vec<_>>();

        raw_cycle.sort_unstable();
        raw_cycle.dedup();
        raw_cycle
    }

    fn collect_owned_struct_symbols(&self, ty: TypeId, symbols: &mut Vec<SymbolId>) {
        let ty = self.resolve_alias_type(ty);

        match self.layer.table().kind(ty) {
            Some(TypeKind::Named { symbol }) => {
                if self
                    .graph
                    .resolution()
                    .and_then(|resolution| resolution.symbol(*symbol))
                    .is_some_and(|symbol| symbol.kind() == SymbolKind::Struct)
                    && !symbols.contains(symbol)
                {
                    symbols.push(*symbol);
                }
            }

            Some(TypeKind::Array { element })
            | Some(TypeKind::FixedArray { element, .. })
            | Some(TypeKind::Range { element }) => {
                self.collect_owned_struct_symbols(*element, symbols);
            }

            Some(TypeKind::Tuple { elements }) | Some(TypeKind::Union { members: elements }) => {
                for element in elements {
                    self.collect_owned_struct_symbols(*element, symbols);
                }
            }

            Some(TypeKind::GenericInstance { base, arguments }) => {
                self.collect_owned_struct_symbols(*base, symbols);

                for argument in arguments {
                    self.collect_owned_struct_symbols(*argument, symbols);
                }
            }

            _ => {}
        }
    }

    fn validate_release_eligibility(&mut self) {
        let mut seen = HashSet::new();

        for anchor in self.ownership_metadata.anchors.clone() {
            let Some(ty) = anchor.ty() else {
                continue;
            };

            self.push_release_eligibility(
                ReleaseEligibilityKind::Anchor,
                anchor.node(),
                anchor.symbol(),
                ty,
                &mut seen,
            );
        }

        for capture in self.ownership_metadata.captures.clone() {
            self.push_release_eligibility(
                ReleaseEligibilityKind::Capture,
                capture.reference(),
                Some(capture.symbol()),
                capture.ty(),
                &mut seen,
            );
        }

        for temporary in self.ownership_metadata.temporaries.clone() {
            self.push_release_eligibility(
                ReleaseEligibilityKind::Temporary,
                temporary.expression(),
                None,
                temporary.ty(),
                &mut seen,
            );
        }
    }

    fn push_release_eligibility(
        &mut self,
        kind: ReleaseEligibilityKind,
        node: NodeId,
        symbol: Option<SymbolId>,
        ty: TypeId,
        seen: &mut HashSet<(ReleaseEligibilityKind, NodeId, Option<SymbolId>)>,
    ) {
        if !self.is_owned_edge_type(ty) {
            return;
        }

        if !seen.insert((kind, node, symbol)) {
            return;
        }

        self.ownership_metadata
            .release_eligibilities
            .push(ReleaseEligibilityMetadata::new(kind, node, symbol, ty));
    }

    fn node_contains_child(&self, parent: NodeId, child: NodeId) -> bool {
        if parent == child {
            return true;
        }

        let Some(parent_node) = self.graph.syntax().node(parent) else {
            return false;
        };

        parent_node
            .children()
            .iter()
            .any(|nested| self.node_contains_child(*nested, child))
    }

    fn is_weak_field_nullable_type(&self, ty: TypeId) -> bool {
        let ty = self.resolve_alias_type(ty);
        let null_type = self.layer.table().primitive(PrimitiveType::Null);

        match self.layer.table().kind(ty) {
            Some(TypeKind::Union { members }) => members.contains(&null_type),
            Some(TypeKind::Primitive(PrimitiveType::Null)) => true,
            _ => false,
        }
    }

    fn report_invalid_weak_field_type(&mut self, field: NodeId, field_type: TypeId) {
        let span = self
            .graph
            .syntax()
            .node(field)
            .map(|node| node.span())
            .unwrap_or_else(|| self.source.span());

        let field_type = self.describe_type_for_diagnostic(field_type);

        self.diagnostics
            .push(galfus_core::Diagnostic::error_with_message(
                TypeDiagnosticCode::InvalidWeakFieldType,
                format!("weak field type must be nullable, got `{field_type}`"),
                span,
            ));
    }
}
