use serde::{Deserialize, Serialize};

pub mod gfb;
pub mod instruction;
pub mod validation;

pub use gfb::*;
pub use instruction::*;
pub use validation::*;

// =========================================================================
// VM Runtime Value Model
// =========================================================================

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObjectRef(pub usize);

impl ObjectRef {
    pub const fn raw(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Uint8(u8),
    Uint16(u16),
    Uint32(u32),
    Uint64(u64),
    Float32(f32),
    Float64(f64),
    Object(ObjectRef),
}

// =========================================================================
// Constant Pool
// =========================================================================

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Constant {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConstantPool {
    pub constants: Vec<Constant>,
}

// =========================================================================
// Types & Layout Table
// =========================================================================

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageType {
    Null,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Float32,
    Float64,
    Struct(StructLayoutIdx),
    Array(TypeIdx),
    FixedArray(TypeIdx, usize),
    Tuple(Vec<TypeIdx>),
    Choice(ChoiceLayoutIdx),
    Function { params: Vec<TypeIdx>, ret: TypeIdx },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OwnershipKind {
    Strong,
    Weak,
    Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldLayout {
    pub name: String,
    pub ty: TypeIdx,
    pub offset: usize,
    pub ownership: OwnershipKind,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructLayout {
    pub name: String,
    pub fields: Vec<FieldLayout>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChoiceVariantLayout {
    pub name: String,
    pub payload_ty: Option<TypeIdx>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChoiceLayout {
    pub name: String,
    pub variants: Vec<ChoiceVariantLayout>,
}

// =========================================================================
// Imports & Exports
// =========================================================================

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSlot {
    pub module_name: String,
    pub symbol_name: String,
    pub ty: TypeIdx,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportSlot {
    pub symbol_name: String,
    pub func_idx: FuncIdx,
}

// =========================================================================
// Image Function & Module Image Container
// =========================================================================

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageFunction {
    pub name: String,
    pub param_count: u8,
    pub local_count: u16,
    pub temp_count: u16,
    pub return_ty: TypeIdx,
    pub instructions: Vec<Instruction>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModuleImage {
    pub name: String,
    pub constants: ConstantPool,
    pub functions: Vec<ImageFunction>,
    pub types: Vec<ImageType>,
    pub struct_layouts: Vec<StructLayout>,
    pub choice_layouts: Vec<ChoiceLayout>,
    pub imports: Vec<ImportSlot>,
    pub exports: Vec<ExportSlot>,
    pub init_func_idx: Option<FuncIdx>,
}
