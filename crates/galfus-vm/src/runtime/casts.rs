use super::*;

impl VirtualMachine {
    pub(super) fn cast_value(&self, val: &Value, target_ty: TypeIdx) -> Result<Value, VmError> {
        let ty = self
            .image
            .types
            .get(target_ty.raw() as usize)
            .ok_or(VmError::TypeOutOfBounds { index: target_ty })?;

        if self.check_value_type(val, target_ty) {
            return Ok(val.clone());
        }

        // Numeric casting / conversions
        let casted = match val {
            Value::Int8(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Int16(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Int32(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Int64(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Uint8(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Uint16(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Uint32(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Uint64(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Float32(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x)),
                ImageType::Float64 => Some(Value::Float64(*x as f64)),
                _ => None,
            },
            Value::Float64(x) => match ty {
                ImageType::Int8 => Some(Value::Int8(*x as i8)),
                ImageType::Int16 => Some(Value::Int16(*x as i16)),
                ImageType::Int32 => Some(Value::Int32(*x as i32)),
                ImageType::Int64 => Some(Value::Int64(*x as i64)),
                ImageType::Uint8 => Some(Value::Uint8(*x as u8)),
                ImageType::Uint16 => Some(Value::Uint16(*x as u16)),
                ImageType::Uint32 => Some(Value::Uint32(*x as u32)),
                ImageType::Uint64 => Some(Value::Uint64(*x as u64)),
                ImageType::Float32 => Some(Value::Float32(*x as f32)),
                ImageType::Float64 => Some(Value::Float64(*x)),
                _ => None,
            },
            _ => None,
        };

        casted.ok_or_else(|| VmError::TypeMismatch {
            expected: format!("{:?}", ty),
            found: format!("{:?}", val),
        })
    }
}
