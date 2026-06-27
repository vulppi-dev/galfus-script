use serde::{Deserialize, Serialize};

// =========================================================================
// Operand Indices (Newtype Wrappers)
// =========================================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Reg(pub u16);

impl Reg {
    pub const fn raw(&self) -> u16 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConstIdx(pub u16);

impl ConstIdx {
    pub const fn raw(&self) -> u16 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TypeIdx(pub u16);

impl TypeIdx {
    pub const fn raw(&self) -> u16 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FuncIdx(pub u16);

impl FuncIdx {
    pub const fn raw(&self) -> u16 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalIdx(pub u16);

impl GlobalIdx {
    pub const fn raw(&self) -> u16 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FieldIdx(pub u16);

impl FieldIdx {
    pub const fn raw(&self) -> u16 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StructLayoutIdx(pub u16);

impl StructLayoutIdx {
    pub const fn raw(&self) -> u16 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChoiceLayoutIdx(pub u16);

impl ChoiceLayoutIdx {
    pub const fn raw(&self) -> u16 {
        self.0
    }
}

// =========================================================================
// Opcode Instruction Set
// =========================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Instruction {
    // Category A: Data Movement & Constants
    LoadConst {
        dest: Reg,
        const_idx: ConstIdx,
    },
    Move {
        dest: Reg,
        src: Reg,
    },
    LoadGlobal {
        dest: Reg,
        global_idx: GlobalIdx,
    },
    StoreGlobal {
        global_idx: GlobalIdx,
        src: Reg,
    },
    LoadNull {
        dest: Reg,
    },

    // Category B: Unary & Binary Operations
    Add {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Sub {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Mul {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Div {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Rem {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Pow {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Neg {
        dest: Reg,
        src: Reg,
    },
    Not {
        dest: Reg,
        src: Reg,
    },
    BitNot {
        dest: Reg,
        src: Reg,
    },
    Shl {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Shr {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    And {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Or {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Xor {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Eq {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Ne {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Lt {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Le {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Gt {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Ge {
        dest: Reg,
        lhs: Reg,
        rhs: Reg,
    },
    Fallback {
        dest: Reg,
        src: Reg,
        fallback: Reg,
    },

    // Category C: Control Flow & Subroutines
    Jump {
        offset: i32,
    },
    JumpTrue {
        cond: Reg,
        offset: i32,
    },
    JumpFalse {
        cond: Reg,
        offset: i32,
    },
    JumpNull {
        val: Reg,
        offset: i32,
    },
    Call {
        dest: Reg,
        func: FuncIdx,
        args_start: Reg,
        arg_count: u8,
    },
    Ret {
        src: Reg,
    },
    RetNull,
    Panic {
        const_idx: ConstIdx,
    },

    // Category D: Heaps, Structs & Collections
    AllocLocal {
        dest: Reg,
        type_idx: TypeIdx,
    },
    AllocShared {
        dest: Reg,
        type_idx: TypeIdx,
    },
    LoadField {
        dest: Reg,
        obj: Reg,
        field: FieldIdx,
    },
    StoreField {
        obj: Reg,
        field: FieldIdx,
        val: Reg,
    },
    NewArray {
        dest: Reg,
        type_idx: TypeIdx,
        len_reg: Reg,
    },
    LoadIndex {
        dest: Reg,
        arr: Reg,
        idx: Reg,
    },
    StoreIndex {
        arr: Reg,
        idx: Reg,
        val: Reg,
    },
    NewTuple {
        dest: Reg,
        type_idx: TypeIdx,
        start: Reg,
        count: u8,
    },
    NewChoice {
        dest: Reg,
        type_idx: TypeIdx,
        variant_idx: u16,
        payload: Reg,
    },
    Cast {
        dest: Reg,
        src: Reg,
        type_idx: TypeIdx,
    },
    Instanceof {
        dest: Reg,
        src: Reg,
        type_idx: TypeIdx,
    },

    // Category E: Memory Ownership
    Drop {
        reg: Reg,
    },

    // Category F: Transactional Shared Memory
    TxStart {
        key_reg: Reg,
    },
    TxLoad {
        dest: Reg,
        obj: Reg,
        field: FieldIdx,
    },
    TxStore {
        obj: Reg,
        field: FieldIdx,
        val: Reg,
    },
    TxCommit {
        dest_reg: Reg,
    },
    TxRollback,
}
