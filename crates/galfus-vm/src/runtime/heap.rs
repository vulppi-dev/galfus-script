use super::*;

impl VirtualMachine {
    pub(super) fn get_object(&self, obj_ref: ObjectRef) -> Result<&HeapObject, VmError> {
        self.heap
            .get(obj_ref.raw())
            .and_then(|opt| opt.as_ref())
            .ok_or_else(|| VmError::TypeMismatch {
                expected: "valid object reference".to_string(),
                found: format!("{:?}", obj_ref),
            })
    }

    pub(super) fn get_object_mut(
        &mut self,
        obj_ref: ObjectRef,
    ) -> Result<&mut HeapObject, VmError> {
        self.heap
            .get_mut(obj_ref.raw())
            .and_then(|opt| opt.as_mut())
            .ok_or_else(|| VmError::TypeMismatch {
                expected: "valid object reference".to_string(),
                found: format!("{:?}", obj_ref),
            })
    }

    pub(super) fn to_shift_amount(&self, val: Value) -> Result<u32, VmError> {
        match val {
            Value::Int8(x) => Ok(x as u32),
            Value::Int16(x) => Ok(x as u32),
            Value::Int32(x) => Ok(x as u32),
            Value::Int64(x) => Ok(x as u32),
            Value::Uint8(x) => Ok(x as u32),
            Value::Uint16(x) => Ok(x as u32),
            Value::Uint32(x) => Ok(x),
            Value::Uint64(x) => Ok(x as u32),
            x => Err(VmError::TypeMismatch {
                expected: "integer shift amount".to_string(),
                found: format!("{:?}", x),
            }),
        }
    }

    pub(super) fn to_raw_array_index(&self, val: Value) -> Result<i128, VmError> {
        match val {
            Value::Int8(x) => Ok(i128::from(x)),
            Value::Int16(x) => Ok(i128::from(x)),
            Value::Int32(x) => Ok(i128::from(x)),
            Value::Int64(x) => Ok(i128::from(x)),
            Value::Uint8(x) => Ok(i128::from(x)),
            Value::Uint16(x) => Ok(i128::from(x)),
            Value::Uint32(x) => Ok(i128::from(x)),
            Value::Uint64(x) => Ok(i128::from(x)),
            x => Err(VmError::TypeMismatch {
                expected: "integer array index".to_string(),
                found: format!("{:?}", x),
            }),
        }
    }

    pub(super) fn resolve_raw_array_index(&self, raw_index: i128, len: usize) -> Option<usize> {
        let len = len as i128;
        let resolved = if raw_index < 0 {
            len + raw_index
        } else {
            raw_index
        };

        if resolved < 0 || resolved >= len {
            return None;
        }

        Some(resolved as usize)
    }

    pub(super) fn pow_values(&self, lhs: Value, rhs: Value) -> Result<Value, VmError> {
        let res = match (lhs, rhs) {
            (Value::Int8(l), Value::Int8(r)) => Value::Int8(l.wrapping_pow(r as u32)),
            (Value::Int16(l), Value::Int16(r)) => Value::Int16(l.wrapping_pow(r as u32)),
            (Value::Int32(l), Value::Int32(r)) => Value::Int32(l.wrapping_pow(r as u32)),
            (Value::Int64(l), Value::Int64(r)) => Value::Int64(l.wrapping_pow(r as u32)),
            (Value::Uint8(l), Value::Uint8(r)) => Value::Uint8(l.wrapping_pow(r as u32)),
            (Value::Uint16(l), Value::Uint16(r)) => Value::Uint16(l.wrapping_pow(r as u32)),
            (Value::Uint32(l), Value::Uint32(r)) => Value::Uint32(l.wrapping_pow(r)),
            (Value::Uint64(l), Value::Uint64(r)) => Value::Uint64(l.wrapping_pow(r as u32)),
            (Value::Float32(l), Value::Float32(r)) => Value::Float32(l.powf(r)),
            (Value::Float64(l), Value::Float64(r)) => Value::Float64(l.powf(r)),
            (l, r) => {
                return Err(VmError::TypeMismatch {
                    expected: "matching numeric types".to_string(),
                    found: format!("{:?} and {:?}", l, r),
                });
            }
        };
        Ok(res)
    }

    pub(super) fn compare_values(
        &self,
        lhs: &Value,
        rhs: &Value,
    ) -> Result<Option<std::cmp::Ordering>, VmError> {
        let ord = match (lhs, rhs) {
            (Value::Bool(l), Value::Bool(r)) => Some(l.cmp(r)),
            (Value::Int8(l), Value::Int8(r)) => Some(l.cmp(r)),
            (Value::Int16(l), Value::Int16(r)) => Some(l.cmp(r)),
            (Value::Int32(l), Value::Int32(r)) => Some(l.cmp(r)),
            (Value::Int64(l), Value::Int64(r)) => Some(l.cmp(r)),
            (Value::Uint8(l), Value::Uint8(r)) => Some(l.cmp(r)),
            (Value::Uint16(l), Value::Uint16(r)) => Some(l.cmp(r)),
            (Value::Uint32(l), Value::Uint32(r)) => Some(l.cmp(r)),
            (Value::Uint64(l), Value::Uint64(r)) => Some(l.cmp(r)),
            (Value::Float32(l), Value::Float32(r)) => l.partial_cmp(r),
            (Value::Float64(l), Value::Float64(r)) => l.partial_cmp(r),
            (l, r) => {
                return Err(VmError::TypeMismatch {
                    expected: "matching comparable types".to_string(),
                    found: format!("{:?} and {:?}", l, r),
                });
            }
        };
        Ok(ord)
    }

    pub(super) fn check_value_type(&self, val: &Value, expected_ty: TypeIdx) -> bool {
        let ty = match self.image.types.get(expected_ty.raw() as usize) {
            Some(t) => t,
            None => return false,
        };
        match (val, ty) {
            (Value::Null, ImageType::Null) => true,
            (Value::Bool(_), ImageType::Bool) => true,
            (Value::Int8(_), ImageType::Int8) => true,
            (Value::Int16(_), ImageType::Int16) => true,
            (Value::Int32(_), ImageType::Int32) => true,
            (Value::Int64(_), ImageType::Int64) => true,
            (Value::Uint8(_), ImageType::Uint8) => true,
            (Value::Uint16(_), ImageType::Uint16) => true,
            (Value::Uint32(_), ImageType::Uint32) => true,
            (Value::Uint64(_), ImageType::Uint64) => true,
            (Value::Float32(_), ImageType::Float32) => true,
            (Value::Float64(_), ImageType::Float64) => true,
            (Value::Object(obj_ref), ImageType::Struct(expected_layout_idx)) => {
                if let Ok(HeapObject::Struct { layout_idx, .. }) = self.get_object(*obj_ref) {
                    layout_idx == expected_layout_idx
                } else {
                    false
                }
            }
            (Value::Object(obj_ref), ImageType::Array(expected_el_ty)) => {
                if let Ok(HeapObject::Array { element_ty, .. }) = self.get_object(*obj_ref) {
                    self.type_idx_matches(*element_ty, *expected_el_ty)
                } else {
                    false
                }
            }
            (Value::Object(obj_ref), ImageType::FixedArray(expected_el_ty, expected_len)) => {
                if let Ok(HeapObject::Array {
                    element_ty,
                    elements,
                }) = self.get_object(*obj_ref)
                {
                    self.type_idx_matches(*element_ty, *expected_el_ty)
                        && elements.len() == *expected_len
                } else {
                    false
                }
            }
            (Value::Object(obj_ref), ImageType::Tuple(expected_tys)) => {
                if let Ok(HeapObject::Tuple { elements }) = self.get_object(*obj_ref) {
                    if elements.len() == expected_tys.len() {
                        elements
                            .iter()
                            .zip(expected_tys.iter())
                            .all(|(v, &ty)| self.check_value_type(v, ty))
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            (Value::Object(obj_ref), ImageType::Choice(expected_layout_idx)) => {
                if let Ok(HeapObject::Choice { layout_idx, .. }) = self.get_object(*obj_ref) {
                    layout_idx == expected_layout_idx
                } else {
                    false
                }
            }
            (
                Value::Object(obj_ref),
                ImageType::ChoiceVariant(expected_choice_idx, expected_variant_idx),
            ) => {
                if let Ok(HeapObject::Choice {
                    layout_idx,
                    variant_idx,
                    ..
                }) = self.get_object(*obj_ref)
                {
                    if layout_idx == expected_choice_idx && *variant_idx == *expected_variant_idx {
                        return true;
                    }

                    let Some(actual_layout) = self.image.choice_layouts.get(layout_idx.raw() as usize) else {
                        return false;
                    };
                    let Some(expected_layout) = self
                        .image
                        .choice_layouts
                        .get(expected_choice_idx.raw() as usize)
                    else {
                        return false;
                    };

                    actual_layout.name == expected_layout.name
                        && actual_layout
                            .variants
                            .get(*variant_idx as usize)
                            .map(|variant| variant.name.as_str())
                            == expected_layout
                                .variants
                                .get(*expected_variant_idx as usize)
                                .map(|variant| variant.name.as_str())
                } else {
                    false
                }
            }
            (Value::Object(obj_ref), ImageType::Constraint(expected_constraint)) => {
                if let Ok(HeapObject::Struct { layout_idx, .. }) = self.get_object(*obj_ref) {
                    self.image
                        .struct_layouts
                        .get(layout_idx.raw() as usize)
                        .is_some_and(|layout| layout.constraints.contains(expected_constraint))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn type_idx_matches(&self, actual: TypeIdx, expected: TypeIdx) -> bool {
        self.type_idx_matches_inner(actual, expected, &mut std::collections::HashSet::new())
    }

    fn type_idx_matches_inner(
        &self,
        actual: TypeIdx,
        expected: TypeIdx,
        seen: &mut std::collections::HashSet<(u16, u16)>,
    ) -> bool {
        if actual == expected {
            return true;
        }
        if !seen.insert((actual.raw(), expected.raw())) {
            return true;
        }

        let Some(actual_ty) = self.image.types.get(actual.raw() as usize) else {
            return false;
        };
        let Some(expected_ty) = self.image.types.get(expected.raw() as usize) else {
            return false;
        };

        match (actual_ty, expected_ty) {
            (ImageType::Array(actual_el), ImageType::Array(expected_el)) => {
                self.type_idx_matches_inner(*actual_el, *expected_el, seen)
            }
            (
                ImageType::FixedArray(actual_el, actual_len),
                ImageType::FixedArray(expected_el, expected_len),
            ) => {
                actual_len == expected_len
                    && self.type_idx_matches_inner(*actual_el, *expected_el, seen)
            }
            (ImageType::Tuple(actual_elements), ImageType::Tuple(expected_elements)) => {
                actual_elements.len() == expected_elements.len()
                    && actual_elements.iter().zip(expected_elements.iter()).all(
                        |(&actual_el, &expected_el)| {
                            self.type_idx_matches_inner(actual_el, expected_el, seen)
                        },
                    )
            }
            _ => actual_ty == expected_ty,
        }
    }
}
