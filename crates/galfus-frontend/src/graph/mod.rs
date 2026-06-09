#[cfg(test)]
mod tests;

use crate::Token;
use galfus_core::{Diagnostic, DiagnosticBag, NodeId, SourceId, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphPhase {
    Parsed,
}

#[derive(Debug, Clone)]
pub struct ModuleGraph {
    source_id: SourceId,
    phase: GraphPhase,
    syntax: SyntaxLayer,
    diagnostics: DiagnosticBag,
}

impl ModuleGraph {
    pub fn new(source_id: SourceId) -> Self {
        Self {
            source_id,
            phase: GraphPhase::Parsed,
            syntax: SyntaxLayer::new(),
            diagnostics: DiagnosticBag::new(),
        }
    }

    pub fn source_id(&self) -> SourceId {
        self.source_id
    }

    pub fn phase(&self) -> GraphPhase {
        self.phase
    }

    pub fn syntax(&self) -> &SyntaxLayer {
        &self.syntax
    }

    pub fn syntax_mut(&mut self) -> &mut SyntaxLayer {
        &mut self.syntax
    }

    pub fn diagnostics(&self) -> &DiagnosticBag {
        &self.diagnostics
    }

    pub fn diagnostics_mut(&mut self) -> &mut DiagnosticBag {
        &mut self.diagnostics
    }

    pub fn push_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn extend_diagnostics(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }
}

#[derive(Debug, Clone)]
pub struct SyntaxLayer {
    root: Option<NodeId>,
    tokens: Vec<Token>,
    nodes: Vec<SyntaxNode>,
}

impl SyntaxLayer {
    pub fn new() -> Self {
        Self {
            root: None,
            tokens: Vec::new(),
            nodes: Vec::new(),
        }
    }

    pub fn root(&self) -> Option<NodeId> {
        self.root
    }

    pub fn set_root(&mut self, root: NodeId) {
        self.root = Some(root);
    }

    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    pub fn set_tokens(&mut self, tokens: Vec<Token>) {
        self.tokens = tokens;
    }

    pub fn nodes(&self) -> &[SyntaxNode] {
        &self.nodes
    }

    pub fn add_node(&mut self, kind: SyntaxNodeKind, span: Span, children: Vec<NodeId>) -> NodeId {
        let id = NodeId::new(self.nodes.len() as u32);

        self.nodes.push(SyntaxNode::new(kind, span, children));

        id
    }

    pub fn node(&self, id: NodeId) -> Option<&SyntaxNode> {
        self.nodes.get(id.raw() as usize)
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut SyntaxNode> {
        self.nodes.get_mut(id.raw() as usize)
    }

    pub fn push_child(&mut self, parent: NodeId, child: NodeId) {
        if let Some(node) = self.node_mut(parent) {
            node.children.push(child);
        }
    }
}

impl Default for SyntaxLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxNode {
    kind: SyntaxNodeKind,
    span: Span,
    children: Vec<NodeId>,
}

impl SyntaxNode {
    pub fn new(kind: SyntaxNodeKind, span: Span, children: Vec<NodeId>) -> Self {
        Self {
            kind,
            span,
            children,
        }
    }

    pub fn kind(&self) -> SyntaxNodeKind {
        self.kind
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn children(&self) -> &[NodeId] {
        &self.children
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxNodeKind {
    SourceFile,

    FunctionItem,
    TypeAliasItem,
    ExportItem,
    ImportItem,
    StructItem,
    EnumItem,
    ChoiceItem,

    NamespaceImport,
    NamedImportList,
    NamedImport,
    ImportAlias,

    StructFieldList,
    StructField,
    StructFieldDefault,

    EnumVariantList,
    EnumVariant,

    ChoiceVariantList,
    ChoiceVariant,
    ChoicePayload,

    ParameterList,
    Parameter,
    RestParameter,
    ParameterDefault,

    Block,
    ReturnStatement,
    BreakStatement,
    ContinueStatement,
    VarStatement,
    ConstStatement,
    ExpressionStatement,
    IfStatement,
    ElseClause,
    ForStatement,
    ForBinding,
    WhileStatement,
    LoopStatement,
    AssignmentStatement,
    MatchStatement,
    MatchArmList,
    MatchArm,
    InstanceofStatement,
    InstanceofArmList,
    InstanceofArm,
    TypePattern,
    TypePatternBinding,

    BindingPattern,
    VariantPattern,
    VariantPatternPayload,
    LiteralPattern,
    RegexPattern,

    TypeAnnotation,
    Initializer,
    TypeNull,
    TypeName,
    ArrayType,
    FixedArrayType,
    ArraySize,
    UnionType,

    ImportSource,

    Identifier,

    Argument,
    ArgumentList,

    CallExpression,
    NameExpression,
    MemberExpression,
    AnchorExpression,
    IndexExpression,
    GroupedExpression,
    CopyExpression,
    UnaryExpression,
    BinaryExpression,
    ArrowFunctionExpression,

    UnaryOperator,
    BinaryOperator,
    AssignmentOperator,

    ArrayLiteral,
    ArrayElement,
    StructLiteral,
    StructLiteralFieldList,
    StructLiteralField,
    StructLiteralFieldShorthand,
    InferredStructLiteral,
    SpreadArgument,
    SpreadArrayElement,

    IntegerLiteral,
    FloatLiteral,
    BoolLiteral,
    NullLiteral,
    StringLiteral,
    RegexLiteral,
}
