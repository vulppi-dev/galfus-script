use galfus_core::{FunctionId, StorageMetadata, TypeId};
use serde::{Deserialize, Serialize};

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
    pub body: MirBody,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirBody {
    BasicBlock(BasicBlock),
    Block {
        locals: Vec<LocalDecl>,
        statements: Vec<MirBody>,
    },
    If {
        cond: Operand,
        then_branch: Box<MirBody>,
        else_branch: Option<Box<MirBody>>,
    },
    Loop {
        body: Box<MirBody>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    Assign(LocalId, RValue),
    Drop(LocalId),
    StoreGlobal(String, Operand),
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
    NewStruct {
        struct_type: TypeId,
        fields: Vec<Operand>,
        storage_meta: StorageMetadata,
    },
    NewArray(TypeId, Vec<Operand>),
    NewArrayDynamic(TypeId, Vec<ArrayLiteralElement>),
    NewTuple(TypeId, Vec<Operand>),
    MemberAccess(Operand, String),
    ArrayIndex(Operand, Operand),
    Choice(TypeId, String, Option<Operand>),
    Instanceof(Operand, TypeId),
    LoadGlobal(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArrayLiteralElement {
    Single(Operand),
    Spread(Operand),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Terminator {
    Return(Option<Operand>),
    Break,
    Continue,
    Call {
        func: FunctionId,
        args: Vec<Operand>,
        destination: LocalId,
    },
    Panic(String),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operand {
    Constant(Constant),
    Local(LocalId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constant {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}
