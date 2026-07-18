use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ModuleId(u32);

impl ModuleId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SourceId(u32);

impl SourceId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(u32);

impl NodeId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SymbolId(u32);

impl SymbolId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ScopeId(u32);

impl ScopeId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TypeId(u32);

impl TypeId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FunctionId(u32);

impl FunctionId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StructId(u32);

impl StructId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EnumId(u32);

impl EnumId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChoiceId(u32);

impl ChoiceId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ConstraintId(u32);

impl ConstraintId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ExportId(u32);

impl ExportId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ImportId(u32);

impl ImportId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn raw(&self) -> u32 {
        self.0
    }
}

#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct Revision(pub u64);

impl Revision {
    pub const fn new(rev: u64) -> Self {
        Self(rev)
    }

    pub fn next(&mut self) {
        self.0 += 1;
    }
}

#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct SemanticRevision(pub u64);

impl SemanticRevision {
    pub const fn new(rev: u64) -> Self {
        Self(rev)
    }

    pub fn next(&mut self) {
        self.0 += 1;
    }
}
