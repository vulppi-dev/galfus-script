use super::LowerCtx;
use crate::mir::Constant as MirConstant;
use galfus_image::Constant;
use galfus_image::instruction::ConstIdx;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum HashableConstant {
    Bool(bool),
    Int32(i32),
    Int64(i64),
    FloatBits(u64),
    String(String),
    Function(u32),
}

impl HashableConstant {
    pub fn from_mir(constant: &MirConstant) -> Option<Self> {
        match constant {
            MirConstant::Null => None,
            MirConstant::Bool(b) => Some(Self::Bool(*b)),
            MirConstant::Int(i) => Some(
                i32::try_from(*i)
                    .map(Self::Int32)
                    .unwrap_or(Self::Int64(*i)),
            ),
            MirConstant::Float(f) => Some(Self::FloatBits(f.to_bits())),
            MirConstant::String(s) => Some(Self::String(s.clone())),
            MirConstant::Function(id) => Some(Self::Function(id.raw())),
        }
    }
}

pub fn get_or_create_constant(ctx: &mut LowerCtx, constant: &MirConstant) -> ConstIdx {
    let hashable = match HashableConstant::from_mir(constant) {
        Some(h) => h,
        None => return ConstIdx(0), // Placeholder for Null
    };

    if let Some(&idx) = ctx.constants_map.get(&hashable) {
        return idx;
    }

    let next_idx = ConstIdx(ctx.constant_pool.constants.len() as u16);
    ctx.constants_map.insert(hashable, next_idx);

    let c = match constant {
        MirConstant::Null => unreachable!(),
        MirConstant::Bool(b) => Constant::Bool(*b),
        MirConstant::Int(i) => i32::try_from(*i)
            .map(Constant::Int32)
            .unwrap_or(Constant::Int64(*i)),
        MirConstant::Float(f) => Constant::Float(*f),
        MirConstant::String(s) => Constant::String(s.clone()),
        MirConstant::Function(id) => {
            let func_idx = *ctx.function_map.get(id).unwrap_or_else(|| {
                panic!(
                    "FunctionId {:?} not found in function_map during constant lowering",
                    id
                )
            });
            Constant::Function(func_idx)
        }
    };

    ctx.constant_pool.constants.push(c);
    next_idx
}
