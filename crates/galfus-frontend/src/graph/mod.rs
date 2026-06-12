#[cfg(test)]
mod tests;

use crate::{Token, TokenKind};
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

    pub fn child(&self, parent: NodeId, index: usize) -> Option<NodeId> {
        self.node(parent)?.children().get(index).copied()
    }

    pub fn first_child(&self, parent: NodeId) -> Option<NodeId> {
        self.child(parent, 0)
    }

    pub fn last_child(&self, parent: NodeId) -> Option<NodeId> {
        self.node(parent)?.children().last().copied()
    }

    pub fn child_count(&self, parent: NodeId) -> Option<usize> {
        Some(self.node(parent)?.child_count())
    }

    pub fn child_node(&self, parent: NodeId, index: usize) -> Option<&SyntaxNode> {
        let child = self.child(parent, index)?;
        self.node(child)
    }

    pub fn first_child_of_kind(&self, parent: NodeId, kind: SyntaxNodeKind) -> Option<NodeId> {
        self.node(parent)?
            .children()
            .iter()
            .copied()
            .find(|child| self.node(*child).is_some_and(|node| node.kind() == kind))
    }

    pub fn children_of_kind(
        &self,
        parent: NodeId,
        kind: SyntaxNodeKind,
    ) -> impl Iterator<Item = NodeId> + '_ {
        self.node(parent)
            .into_iter()
            .flat_map(|node| node.children().iter().copied())
            .filter(move |child| self.node(*child).is_some_and(|node| node.kind() == kind))
    }

    pub fn add_operator_node(
        &mut self,
        kind: SyntaxNodeKind,
        span: Span,
        operator: OperatorKind,
    ) -> NodeId {
        let id = NodeId::new(self.nodes.len() as u32);
        self.nodes
            .push(SyntaxNode::new_operator(kind, span, operator));
        id
    }
}

impl Default for SyntaxLayer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOperatorKind {
    Negate,
    Not,
    BitwiseNot,
}

impl UnaryOperatorKind {
    pub fn from_token(kind: &TokenKind) -> Option<Self> {
        match kind {
            TokenKind::Minus => Some(Self::Negate),
            TokenKind::Bang => Some(Self::Not),
            TokenKind::Tilde => Some(Self::BitwiseNot),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOperatorKind {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Power,

    ShiftLeft,
    ShiftRight,

    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,

    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    LogicalAnd,
    LogicalOr,

    NullFallback,
}

impl BinaryOperatorKind {
    pub fn from_token(kind: &TokenKind) -> Option<Self> {
        match kind {
            TokenKind::Plus => Some(Self::Add),
            TokenKind::Minus => Some(Self::Subtract),
            TokenKind::Star => Some(Self::Multiply),
            TokenKind::Slash => Some(Self::Divide),
            TokenKind::Percent => Some(Self::Remainder),
            TokenKind::StarStar => Some(Self::Power),

            TokenKind::ShiftLeft => Some(Self::ShiftLeft),
            TokenKind::ShiftRight => Some(Self::ShiftRight),

            TokenKind::Amp => Some(Self::BitwiseAnd),
            TokenKind::Pipe => Some(Self::BitwiseOr),
            TokenKind::Caret => Some(Self::BitwiseXor),

            TokenKind::EqualEqual => Some(Self::Equal),
            TokenKind::BangEqual => Some(Self::NotEqual),
            TokenKind::Less => Some(Self::Less),
            TokenKind::LessEqual => Some(Self::LessEqual),
            TokenKind::Greater => Some(Self::Greater),
            TokenKind::GreaterEqual => Some(Self::GreaterEqual),

            TokenKind::AmpAmp => Some(Self::LogicalAnd),
            TokenKind::PipePipe => Some(Self::LogicalOr),

            TokenKind::QuestionQuestion => Some(Self::NullFallback),

            _ => None,
        }
    }

    pub fn precedence(self) -> u8 {
        match self {
            Self::Power => 80,

            Self::Multiply | Self::Divide | Self::Remainder => 70,

            Self::Add | Self::Subtract => 60,

            Self::ShiftLeft | Self::ShiftRight => 55,

            Self::BitwiseAnd => 54,
            Self::BitwiseXor => 53,
            Self::BitwiseOr => 52,

            Self::Less | Self::LessEqual | Self::Greater | Self::GreaterEqual => 50,

            Self::Equal | Self::NotEqual => 45,

            Self::LogicalAnd => 30,

            Self::LogicalOr => 20,

            Self::NullFallback => 10,
        }
    }

    pub fn associativity(self) -> BinaryAssociativity {
        match self {
            Self::Power | Self::NullFallback => BinaryAssociativity::Right,
            _ => BinaryAssociativity::Left,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignmentOperatorKind {
    Assign,

    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    RemainderAssign,
    PowerAssign,

    BitwiseAndAssign,
    BitwiseOrAssign,
    BitwiseXorAssign,
    ShiftLeftAssign,
    ShiftRightAssign,
}

impl AssignmentOperatorKind {
    pub fn from_token(kind: &TokenKind) -> Option<Self> {
        match kind {
            TokenKind::Equal => Some(Self::Assign),

            TokenKind::PlusEqual => Some(Self::AddAssign),
            TokenKind::MinusEqual => Some(Self::SubtractAssign),
            TokenKind::StarEqual => Some(Self::MultiplyAssign),
            TokenKind::SlashEqual => Some(Self::DivideAssign),
            TokenKind::PercentEqual => Some(Self::RemainderAssign),
            TokenKind::StarStarEqual => Some(Self::PowerAssign),

            TokenKind::AmpEqual => Some(Self::BitwiseAndAssign),
            TokenKind::PipeEqual => Some(Self::BitwiseOrAssign),
            TokenKind::CaretEqual => Some(Self::BitwiseXorAssign),
            TokenKind::ShiftLeftEqual => Some(Self::ShiftLeftAssign),
            TokenKind::ShiftRightEqual => Some(Self::ShiftRightAssign),

            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryAssociativity {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatorKind {
    Unary(UnaryOperatorKind),
    Binary(BinaryOperatorKind),
    Assignment(AssignmentOperatorKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxNode {
    kind: SyntaxNodeKind,
    span: Span,
    children: Vec<NodeId>,
    operator: Option<OperatorKind>,
}

impl SyntaxNode {
    pub fn new(kind: SyntaxNodeKind, span: Span, children: Vec<NodeId>) -> Self {
        Self {
            kind,
            span,
            children,
            operator: None,
        }
    }

    pub fn new_operator(kind: SyntaxNodeKind, span: Span, operator: OperatorKind) -> Self {
        Self {
            kind,
            span,
            children: Vec::new(),
            operator: Some(operator),
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

    pub fn child(&self, index: usize) -> Option<NodeId> {
        self.children.get(index).copied()
    }

    pub fn first_child(&self) -> Option<NodeId> {
        self.child(0)
    }

    pub fn last_child(&self) -> Option<NodeId> {
        self.children.last().copied()
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    pub fn is(&self, kind: SyntaxNodeKind) -> bool {
        self.kind == kind
    }

    pub fn operator(&self) -> Option<OperatorKind> {
        self.operator
    }

    pub fn unary_operator(&self) -> Option<UnaryOperatorKind> {
        match self.operator {
            Some(OperatorKind::Unary(operator)) => Some(operator),
            _ => None,
        }
    }

    pub fn binary_operator(&self) -> Option<BinaryOperatorKind> {
        match self.operator {
            Some(OperatorKind::Binary(operator)) => Some(operator),
            _ => None,
        }
    }

    pub fn assignment_operator(&self) -> Option<AssignmentOperatorKind> {
        match self.operator {
            Some(OperatorKind::Assignment(operator)) => Some(operator),
            _ => None,
        }
    }
}

/// Syntax node kinds used by the parsed syntax graph.
///
/// Child ordering is intentionally stable. Parser tests, syntax helpers,
/// and later resolver/lowering code may rely on child positions for compact
/// access, but higher-level code should prefer helper methods when available.
///
/// General conventions:
///
/// - `SourceFile` children are top-level items in source order.
/// - `ExportItem` has exactly one child: the exported item.
/// - List nodes contain only list elements, in source order.
/// - Wrapper nodes usually contain the wrapped syntax as their first child.
/// - `TypeAnnotation` contains exactly one type child.
/// - `Initializer` contains exactly one expression child.
/// - Operator nodes contain no children; their span points to the operator token.
/// - Literal nodes contain no children.
/// - `Identifier` contains no children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxNodeKind {
    // Root
    SourceFile,

    // Items
    ImportItem,
    ExportItem,
    FunctionItem,
    TypeAliasItem,
    StructItem,
    EnumItem,
    ChoiceItem,
    ConstraintItem,
    VarItem,
    ConstItem,

    // Imports
    NamespaceImport,
    NamedImportList,
    NamedImport,
    ImportAlias,
    ImportSource,

    // Paths / names
    Identifier,
    Path,

    // Declarations
    FunctionAnchor,
    StructFieldList,
    StructField,
    StructExpansion,
    WeakStructField,
    StructFieldDefault,
    EnumVariantList,
    EnumVariant,
    ChoiceVariantList,
    ChoiceVariant,
    ChoicePayload,

    // Parameters
    ParameterList,
    Parameter,
    RestParameter,
    ParameterDefault,

    // Generics / constraints
    GenericParameterList,
    GenericParameter,
    GenericParameterConstraint,
    BasicConstraint,
    ConstraintMemberList,
    ConstraintField,
    ConstraintFunctionSignature,
    SatisfiesClause,

    // Statements
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

    // Patterns
    BindingPattern,
    TypePattern,
    TypePatternBinding,
    VariantPattern,
    VariantPatternPayload,
    LiteralPattern,
    RegexPattern,
    StructBindingPattern,
    StructBindingField,
    TupleBindingPattern,
    ArrayBindingPattern,
    RestBindingPattern,

    // Binding helpers
    TypeAnnotation,
    Initializer,

    // Types
    TypeNull,
    NamedType,
    GenericType,
    TypeArgumentList,
    ArrayType,
    FixedArrayType,
    ArraySize,
    UnionType,
    FunctionType,
    FunctionTypeParameterList,
    GroupedType,

    // Arguments
    Argument,
    ArgumentList,
    SpreadArgument,

    // Expressions
    CallExpression,
    PathExpression,
    NameExpression,
    MemberExpression,
    IndexExpression,
    GroupedExpression,
    CopyExpression,
    UnaryExpression,
    BinaryExpression,
    ArrowFunctionExpression,
    GenericExpression,
    GenericArgumentList,
    EnumDiscriminant,

    // Operators
    UnaryOperator,
    BinaryOperator,
    AssignmentOperator,

    // Literals
    ArrayLiteral,
    ArrayElement,
    SpreadArrayElement,
    StructLiteral,
    StructLiteralFieldList,
    StructLiteralField,
    StructLiteralFieldShorthand,
    SpreadStructLiteralField,
    InferredStructLiteral,
    IntegerLiteral,
    FloatLiteral,
    BoolLiteral,
    NullLiteral,
    StringLiteral,
    RegexLiteral,
}

impl SyntaxNodeKind {
    pub fn is_item(self) -> bool {
        matches!(
            self,
            SyntaxNodeKind::ImportItem
                | SyntaxNodeKind::ExportItem
                | SyntaxNodeKind::FunctionItem
                | SyntaxNodeKind::TypeAliasItem
                | SyntaxNodeKind::StructItem
                | SyntaxNodeKind::EnumItem
                | SyntaxNodeKind::ChoiceItem
                | SyntaxNodeKind::ConstraintItem
                | SyntaxNodeKind::VarItem
                | SyntaxNodeKind::ConstItem
        )
    }

    pub fn is_statement(self) -> bool {
        matches!(
            self,
            SyntaxNodeKind::ReturnStatement
                | SyntaxNodeKind::BreakStatement
                | SyntaxNodeKind::ContinueStatement
                | SyntaxNodeKind::VarStatement
                | SyntaxNodeKind::ConstStatement
                | SyntaxNodeKind::ExpressionStatement
                | SyntaxNodeKind::IfStatement
                | SyntaxNodeKind::ForStatement
                | SyntaxNodeKind::WhileStatement
                | SyntaxNodeKind::LoopStatement
                | SyntaxNodeKind::AssignmentStatement
                | SyntaxNodeKind::MatchStatement
                | SyntaxNodeKind::InstanceofStatement
        )
    }

    pub fn is_expression(self) -> bool {
        matches!(
            self,
            SyntaxNodeKind::CallExpression
                | SyntaxNodeKind::PathExpression
                | SyntaxNodeKind::NameExpression
                | SyntaxNodeKind::MemberExpression
                | SyntaxNodeKind::IndexExpression
                | SyntaxNodeKind::GroupedExpression
                | SyntaxNodeKind::CopyExpression
                | SyntaxNodeKind::UnaryExpression
                | SyntaxNodeKind::BinaryExpression
                | SyntaxNodeKind::ArrowFunctionExpression
                | SyntaxNodeKind::GenericExpression
                | SyntaxNodeKind::ArrayLiteral
                | SyntaxNodeKind::StructLiteral
                | SyntaxNodeKind::InferredStructLiteral
                | SyntaxNodeKind::IntegerLiteral
                | SyntaxNodeKind::FloatLiteral
                | SyntaxNodeKind::BoolLiteral
                | SyntaxNodeKind::NullLiteral
                | SyntaxNodeKind::StringLiteral
                | SyntaxNodeKind::RegexLiteral
        )
    }

    pub fn is_type(self) -> bool {
        matches!(
            self,
            SyntaxNodeKind::TypeNull
                | SyntaxNodeKind::NamedType
                | SyntaxNodeKind::Path
                | SyntaxNodeKind::GenericType
                | SyntaxNodeKind::ArrayType
                | SyntaxNodeKind::FixedArrayType
                | SyntaxNodeKind::UnionType
                | SyntaxNodeKind::FunctionType
                | SyntaxNodeKind::GroupedType
        )
    }

    pub fn is_literal(self) -> bool {
        matches!(
            self,
            SyntaxNodeKind::IntegerLiteral
                | SyntaxNodeKind::FloatLiteral
                | SyntaxNodeKind::BoolLiteral
                | SyntaxNodeKind::NullLiteral
                | SyntaxNodeKind::StringLiteral
                | SyntaxNodeKind::RegexLiteral
        )
    }

    pub fn is_operator(self) -> bool {
        matches!(
            self,
            SyntaxNodeKind::UnaryOperator
                | SyntaxNodeKind::BinaryOperator
                | SyntaxNodeKind::AssignmentOperator
        )
    }

    pub fn is_list(self) -> bool {
        matches!(
            self,
            SyntaxNodeKind::NamedImportList
                | SyntaxNodeKind::StructFieldList
                | SyntaxNodeKind::EnumVariantList
                | SyntaxNodeKind::ChoiceVariantList
                | SyntaxNodeKind::ParameterList
                | SyntaxNodeKind::GenericParameterList
                | SyntaxNodeKind::ConstraintMemberList
                | SyntaxNodeKind::MatchArmList
                | SyntaxNodeKind::InstanceofArmList
                | SyntaxNodeKind::TypeArgumentList
                | SyntaxNodeKind::FunctionTypeParameterList
                | SyntaxNodeKind::ArgumentList
                | SyntaxNodeKind::GenericArgumentList
                | SyntaxNodeKind::StructLiteralFieldList
        )
    }

    pub fn is_pattern(self) -> bool {
        matches!(
            self,
            SyntaxNodeKind::BindingPattern
                | SyntaxNodeKind::StructBindingPattern
                | SyntaxNodeKind::StructBindingField
                | SyntaxNodeKind::TupleBindingPattern
                | SyntaxNodeKind::ArrayBindingPattern
                | SyntaxNodeKind::RestBindingPattern
                | SyntaxNodeKind::TypePattern
                | SyntaxNodeKind::TypePatternBinding
                | SyntaxNodeKind::VariantPattern
                | SyntaxNodeKind::VariantPatternPayload
                | SyntaxNodeKind::LiteralPattern
                | SyntaxNodeKind::RegexPattern
        )
    }
}
