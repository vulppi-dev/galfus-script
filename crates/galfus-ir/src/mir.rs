use galfus_core::{FunctionId, StorageMetadata, SymbolId, TypeId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LocalId(u32);

impl LocalId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlockId(u32);

impl BlockId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobalDecl {
    pub name: String,
    pub ty: TypeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirModule {
    pub functions: Vec<MirFunction>,
    pub globals: Vec<GlobalDecl>,
    #[serde(default)]
    pub constant_pool: Vec<Constant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDecl {
    pub id: LocalId,
    pub ty: TypeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirFunction {
    pub id: FunctionId,
    pub name: String,
    pub return_type: TypeId,
    pub parameter_types: Vec<TypeId>,
    pub locals: Vec<LocalDecl>,
    pub blocks: Vec<BasicBlock>,
    #[serde(default)]
    pub type_substitutions: HashMap<SymbolId, TypeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicBlock {
    pub id: BlockId,
    pub parameters: Vec<LocalDecl>,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    Assign(LocalId, RValue),
    Drop(LocalId),
    StoreGlobal(String, Operand),

    /// Store a value into an indexed aggregate:
    ///
    /// `array[index] = value`
    ///
    /// This is a statement-level side effect, not an RValue, because it does
    /// not produce a value. It lowers directly to bytecode `StoreIndex`.
    StoreIndex {
        arr: Operand,
        idx: Operand,
        val: Operand,
    },
    StoreField {
        obj: Operand,
        field_name: String,
        val: Operand,
    },
    TransactionStart {
        targets: Vec<Operand>,
    },
    TransactionCommit {
        destination: LocalId,
    },
    TransactionRollback,
    Call {
        func: FunctionId,
        args: Vec<Operand>,
        destination: LocalId,
    },
    IndirectCall {
        func: Operand,
        args: Vec<Operand>,
        destination: LocalId,
    },
    ConstraintCall {
        method_name: String,
        obj: Operand,
        args: Vec<Operand>,
        destination: LocalId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MirUnaryOp {
    Negate,
    Not,
    BitwiseNot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MirBinaryOp {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RValue {
    Use(Operand),
    UnaryOp(MirUnaryOp, Operand),
    BinaryOp(MirBinaryOp, Operand, Operand),
    Cast(Operand, TypeId),
    Copy(Operand),
    NewStruct {
        struct_type: TypeId,
        fields: Vec<Operand>,
        storage_meta: StorageMetadata,
    },
    NewArray(TypeId, Vec<Operand>),
    NewArrayDynamic(TypeId, Vec<ArrayLiteralElement>),
    /// Zero-initialised allocation with compile-time length.
    /// `array_type` is the full `Array` TypeId.
    /// `element_type` is the element TypeId (for zero-value generation).
    /// `size` is the known compile-time length.
    NewArrayZeroed {
        array_type: TypeId,
        element_type: TypeId,
        size: usize,
        storage: StorageMetadata,
    },
    /// Zero-initialised allocation with runtime length.
    NewArrayZeroedDynamic {
        array_type: TypeId,
        element_type: TypeId,
        length: Operand,
        storage: StorageMetadata,
    },
    NewTuple(TypeId, Vec<Operand>),
    MemberAccess(Operand, String),
    ArrayIndex(Operand, Operand),
    Choice(TypeId, String, Option<Operand>),
    ChoiceVariantIs(Operand, SymbolId),
    Instanceof(Operand, TypeId),
    LoadGlobal(String),
    Len(Operand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArrayLiteralElement {
    Single(Operand),
    Spread(Operand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Terminator {
    Return(Option<Operand>),
    Jump {
        target: BlockId,
        args: Vec<Operand>,
    },
    Branch {
        cond: Operand,
        true_block: BlockId,
        true_args: Vec<Operand>,
        false_block: BlockId,
        false_args: Vec<Operand>,
    },
    Panic(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operand {
    Constant(Constant),
    ConstRef(usize),
    Local(LocalId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constant {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Function(FunctionId),
}
