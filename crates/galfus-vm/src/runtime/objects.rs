use super::*;
use std::collections::HashMap;

impl VirtualMachine {
    pub(super) fn execute_object_instruction(
        &mut self,
        instr: Instruction,
    ) -> Result<ExecutionStep, VmError> {
        match instr {
            // Category D: Heaps, Structs & Collections
            Instruction::AllocLocal { dest, type_idx }
            | Instruction::AllocShared { dest, type_idx } => {
                let ty = self
                    .image
                    .types
                    .get(type_idx.raw() as usize)
                    .ok_or(VmError::TypeOutOfBounds { index: type_idx })?;
                if let ImageType::Struct(layout_idx) = ty {
                    let layout = self
                        .image
                        .struct_layouts
                        .get(layout_idx.raw() as usize)
                        .ok_or(VmError::TypeOutOfBounds { index: type_idx })?;
                    let fields = vec![Value::Null; layout.fields.len()];
                    let obj_ref = self.alloc(HeapObject::Struct {
                        layout_idx: *layout_idx,
                        fields,
                    });
                    self.write_reg(dest, Value::Object(obj_ref))?;
                } else {
                    return Err(VmError::TypeMismatch {
                        expected: "Struct type".to_string(),
                        found: format!("{:?}", ty),
                    });
                }
            }
            Instruction::LoadField { dest, obj, field } => {
                let obj_val = self.read_reg(obj)?;
                if let Value::Object(obj_ref) = obj_val {
                    let heap_obj = self.get_object(obj_ref)?;
                    if let HeapObject::Struct { fields, .. } = heap_obj {
                        let val = fields
                            .get(field.raw() as usize)
                            .cloned()
                            .ok_or(VmError::FieldOutOfBounds { index: field })?;
                        self.write_reg(dest, val)?;
                    } else if let HeapObject::Choice { payload, .. } = heap_obj {
                        self.write_reg(dest, payload.clone())?;
                    } else {
                        return Err(VmError::TypeMismatch {
                            expected: "Struct or Choice object".to_string(),
                            found: format!("{:?}", heap_obj),
                        });
                    }
                } else {
                    return Err(VmError::TypeMismatch {
                        expected: "Object reference".to_string(),
                        found: format!("{:?}", obj_val),
                    });
                }
            }
            Instruction::StoreField { obj, field, val } => {
                let obj_val = self.read_reg(obj)?;
                let val_to_store = self.read_reg(val)?;
                if let Value::Object(obj_ref) = obj_val {
                    let heap_obj = self.get_object_mut(obj_ref)?;
                    if let HeapObject::Struct { fields, .. } = heap_obj {
                        if (field.raw() as usize) < fields.len() {
                            fields[field.raw() as usize] = val_to_store;
                        } else {
                            return Err(VmError::FieldOutOfBounds { index: field });
                        }
                    } else {
                        return Err(VmError::TypeMismatch {
                            expected: "Struct object".to_string(),
                            found: format!("{:?}", heap_obj),
                        });
                    }
                } else {
                    return Err(VmError::TypeMismatch {
                        expected: "Object reference".to_string(),
                        found: format!("{:?}", obj_val),
                    });
                }
            }
            Instruction::NewArray {
                dest,
                type_idx,
                len_reg,
            } => {
                let ty = self
                    .image
                    .types
                    .get(type_idx.raw() as usize)
                    .ok_or(VmError::TypeOutOfBounds { index: type_idx })?;
                let element_ty = match ty {
                    ImageType::Array(el_ty) => *el_ty,
                    ImageType::FixedArray(el_ty, _) => *el_ty,
                    _ => {
                        return Err(VmError::TypeMismatch {
                            expected: "Array or FixedArray type".to_string(),
                            found: format!("{:?}", ty),
                        });
                    }
                };
                let len_val = self.read_reg(len_reg)?;
                let len = match len_val {
                    Value::Int8(x) if x >= 0 => x as usize,
                    Value::Int16(x) if x >= 0 => x as usize,
                    Value::Int32(x) if x >= 0 => x as usize,
                    Value::Int64(x) if x >= 0 => x as usize,
                    Value::Uint8(x) => x as usize,
                    Value::Uint16(x) => x as usize,
                    Value::Uint32(x) => x as usize,
                    Value::Uint64(x) => x as usize,
                    x => {
                        return Err(VmError::TypeMismatch {
                            expected: "positive array length".to_string(),
                            found: format!("{:?}", x),
                        });
                    }
                };
                let zero = self.zero_value_for_type(element_ty)?;
                let elements = vec![zero; len];
                let obj_ref = self.alloc(HeapObject::Array {
                    element_ty,
                    elements,
                });
                self.write_reg(dest, Value::Object(obj_ref))?;
            }
            Instruction::LoadIndex { dest, arr, idx } => {
                let arr_val = self.read_reg(arr)?;
                let idx_val = self.read_reg(idx)?;
                let raw_index = self.to_raw_array_index(idx_val)?;

                if let Value::Object(obj_ref) = arr_val {
                    let heap_obj = self.get_object(obj_ref)?;
                    let val = match heap_obj {
                        HeapObject::Array { elements, .. } | HeapObject::Tuple { elements } => {
                            if let Some(index) = self.resolve_raw_array_index(raw_index, elements.len()) {
                                elements[index].clone()
                            } else {
                                Value::Null
                            }
                        }
                        _ => {
                            return Err(VmError::TypeMismatch {
                                expected: "Array or Tuple object".to_string(),
                                found: format!("{:?}", heap_obj),
                            });
                        }
                    };

                    self.write_reg(dest, val)?;
                } else {
                    return Err(VmError::TypeMismatch {
                        expected: "Object reference".to_string(),
                        found: format!("{:?}", arr_val),
                    });
                }
            }
            Instruction::StoreIndex { arr, idx, val } => {
                let arr_val = self.read_reg(arr)?;
                let idx_val = self.read_reg(idx)?;
                let raw_index = self.to_raw_array_index(idx_val)?;
                let val_to_store = self.read_reg(val)?;

                if let Value::Object(obj_ref) = arr_val {
                    let (index, val_to_store) = {
                        let heap_obj = self.get_object(obj_ref)?;

                        match heap_obj {
                            HeapObject::Array {
                                element_ty,
                                elements,
                            } => {
                                let index = self
                                    .resolve_raw_array_index(raw_index, elements.len())
                                    .ok_or(VmError::IndexOutOfBounds {
                                        index: raw_index,
                                        len: elements.len(),
                                    })?;

                                let val_to_store = self.cast_value(&val_to_store, *element_ty)?;
                                (index, val_to_store)
                            }
                            HeapObject::Tuple { elements } => {
                                let index = self
                                    .resolve_raw_array_index(raw_index, elements.len())
                                    .ok_or(VmError::IndexOutOfBounds {
                                        index: raw_index,
                                        len: elements.len(),
                                    })?;

                                (index, val_to_store)
                            }
                            heap_obj => {
                                return Err(VmError::TypeMismatch {
                                    expected: "Array or Tuple object".to_string(),
                                    found: format!("{:?}", heap_obj),
                                });
                            }
                        }
                    };

                    let heap_obj = self.get_object_mut(obj_ref)?;

                    match heap_obj {
                        HeapObject::Array { elements, .. } | HeapObject::Tuple { elements } => {
                            elements[index] = val_to_store;
                        }
                        heap_obj => {
                            return Err(VmError::TypeMismatch {
                                expected: "Array or Tuple object".to_string(),
                                found: format!("{:?}", heap_obj),
                            });
                        }
                    }
                } else {
                    return Err(VmError::TypeMismatch {
                        expected: "Object reference".to_string(),
                        found: format!("{:?}", arr_val),
                    });
                }
            }
            Instruction::NewTuple {
                dest,
                type_idx,
                start,
                count,
            } => {
                let ty = self
                    .image
                    .types
                    .get(type_idx.raw() as usize)
                    .ok_or(VmError::TypeOutOfBounds { index: type_idx })?;
                if let ImageType::Tuple(tys) = ty {
                    if count as usize != tys.len() {
                        return Err(VmError::TypeMismatch {
                            expected: format!("Tuple size {}", tys.len()),
                            found: format!("Tuple size {}", count),
                        });
                    }
                    let mut elements = Vec::new();
                    for i in 0..count as usize {
                        let src_reg = Reg(start.raw() + i as u16);
                        elements.push(self.read_reg(src_reg)?);
                    }
                    let obj_ref = self.alloc(HeapObject::Tuple { elements });
                    self.write_reg(dest, Value::Object(obj_ref))?;
                } else {
                    return Err(VmError::TypeMismatch {
                        expected: "Tuple type".to_string(),
                        found: format!("{:?}", ty),
                    });
                }
            }
            Instruction::NewChoice {
                dest,
                type_idx,
                variant_idx,
                payload,
            } => {
                let ty = self
                    .image
                    .types
                    .get(type_idx.raw() as usize)
                    .ok_or(VmError::TypeOutOfBounds { index: type_idx })?;
                if let ImageType::Choice(layout_idx) = ty {
                    let payload_val = self.read_reg(payload)?;
                    let obj_ref = self.alloc(HeapObject::Choice {
                        layout_idx: *layout_idx,
                        variant_idx,
                        payload: payload_val,
                    });
                    self.write_reg(dest, Value::Object(obj_ref))?;
                } else {
                    return Err(VmError::TypeMismatch {
                        expected: "Choice type".to_string(),
                        found: format!("{:?}", ty),
                    });
                }
            }
            Instruction::Cast {
                dest,
                src,
                type_idx,
            } => {
                let val = self.read_reg(src)?;
                let res = self.cast_value(&val, type_idx)?;
                self.write_reg(dest, res)?;
            }
            Instruction::Copy { dest, src } => {
                let val = self.read_reg(src)?;
                let copied = self.deep_copy_value(&val, &mut HashMap::new())?;
                self.write_reg(dest, copied)?;
            }
            Instruction::Instanceof {
                dest,
                src,
                type_idx,
            } => {
                let val = self.read_reg(src)?;
                let is_instance = self.check_value_type(&val, type_idx);
                self.write_reg(dest, Value::Bool(is_instance))?;
            }

            _ => unreachable!("instruction routed to the wrong runtime handler"),
        }

        Ok(ExecutionStep::Continue)
    }

    fn deep_copy_value(
        &mut self,
        value: &Value,
        copied: &mut HashMap<usize, ObjectRef>,
    ) -> Result<Value, VmError> {
        let Value::Object(obj_ref) = value else {
            return Ok(value.clone());
        };

        if let Some(copied_ref) = copied.get(&obj_ref.raw()) {
            return Ok(Value::Object(*copied_ref));
        }

        let object = self.get_object(*obj_ref)?.clone();

        match object {
            HeapObject::Struct { layout_idx, fields } => {
                let layout = self
                    .image
                    .struct_layouts
                    .get(layout_idx.raw() as usize)
                    .ok_or(VmError::TypeMismatch {
                        expected: "valid struct layout".to_string(),
                        found: format!("{:?}", layout_idx),
                    })?
                    .clone();

                if layout.fields.is_empty() {
                    return Err(VmError::TypeMismatch {
                        expected: "copyable struct with fields".to_string(),
                        found: format!("fieldless struct `{}`", layout.name),
                    });
                }

                let copied_ref = self.alloc(HeapObject::Struct {
                    layout_idx,
                    fields: vec![Value::Null; fields.len()],
                });
                copied.insert(obj_ref.raw(), copied_ref);

                let copied_fields = fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        let is_weak = layout
                            .fields
                            .get(index)
                            .is_some_and(|field| field.ownership == OwnershipKind::Weak);

                        if is_weak {
                            self.copy_weak_value(field, copied)
                        } else {
                            self.deep_copy_value(field, copied)
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                match self.get_object_mut(copied_ref)? {
                    HeapObject::Struct { fields, .. } => {
                        *fields = copied_fields;
                    }
                    other => {
                        return Err(VmError::TypeMismatch {
                            expected: "Struct object".to_string(),
                            found: format!("{:?}", other),
                        });
                    }
                }

                Ok(Value::Object(copied_ref))
            }
            HeapObject::Array {
                element_ty,
                elements,
            } => {
                let copied_ref = self.alloc(HeapObject::Array {
                    element_ty,
                    elements: Vec::new(),
                });
                copied.insert(obj_ref.raw(), copied_ref);

                let copied_elements = elements
                    .iter()
                    .map(|element| self.deep_copy_value(element, copied))
                    .collect::<Result<Vec<_>, _>>()?;

                match self.get_object_mut(copied_ref)? {
                    HeapObject::Array { elements, .. } => {
                        *elements = copied_elements;
                    }
                    other => {
                        return Err(VmError::TypeMismatch {
                            expected: "Array object".to_string(),
                            found: format!("{:?}", other),
                        });
                    }
                }

                Ok(Value::Object(copied_ref))
            }
            HeapObject::Tuple { elements } => {
                let copied_ref = self.alloc(HeapObject::Tuple {
                    elements: Vec::new(),
                });
                copied.insert(obj_ref.raw(), copied_ref);

                let copied_elements = elements
                    .iter()
                    .map(|element| self.deep_copy_value(element, copied))
                    .collect::<Result<Vec<_>, _>>()?;

                match self.get_object_mut(copied_ref)? {
                    HeapObject::Tuple { elements } => {
                        *elements = copied_elements;
                    }
                    other => {
                        return Err(VmError::TypeMismatch {
                            expected: "Tuple object".to_string(),
                            found: format!("{:?}", other),
                        });
                    }
                }

                Ok(Value::Object(copied_ref))
            }
            HeapObject::Choice {
                layout_idx,
                variant_idx,
                payload,
            } => {
                let copied_ref = self.alloc(HeapObject::Choice {
                    layout_idx,
                    variant_idx,
                    payload: Value::Null,
                });
                copied.insert(obj_ref.raw(), copied_ref);

                let copied_payload = self.deep_copy_value(&payload, copied)?;

                match self.get_object_mut(copied_ref)? {
                    HeapObject::Choice { payload, .. } => {
                        *payload = copied_payload;
                    }
                    other => {
                        return Err(VmError::TypeMismatch {
                            expected: "Choice object".to_string(),
                            found: format!("{:?}", other),
                        });
                    }
                }

                Ok(Value::Object(copied_ref))
            }
        }
    }

    fn copy_weak_value(
        &mut self,
        value: &Value,
        copied: &HashMap<usize, ObjectRef>,
    ) -> Result<Value, VmError> {
        let Value::Object(obj_ref) = value else {
            return Ok(value.clone());
        };

        if let Some(copied_ref) = copied.get(&obj_ref.raw()) {
            return Ok(Value::Object(*copied_ref));
        }

        if self
            .heap
            .get(obj_ref.raw())
            .is_some_and(|slot| slot.is_some())
        {
            Ok(Value::Object(*obj_ref))
        } else {
            Ok(Value::Null)
        }
    }

    /// Returns the default `Value` for element types that can be safely default-initialized.
    fn zero_value_for_type(&self, type_idx: TypeIdx) -> Result<Value, VmError> {
        let ty = self
            .image
            .types
            .get(type_idx.raw() as usize)
            .ok_or(VmError::TypeOutOfBounds { index: type_idx })?;

        Ok(match ty {
            ImageType::Bool => Value::Bool(false),
            ImageType::Int8 => Value::Int8(0),
            ImageType::Int16 => Value::Int16(0),
            ImageType::Int32 => Value::Int32(0),
            ImageType::Int64 => Value::Int64(0),
            ImageType::Uint8 => Value::Uint8(0),
            ImageType::Uint16 => Value::Uint16(0),
            ImageType::Uint32 => Value::Uint32(0),
            ImageType::Uint64 => Value::Uint64(0),
            ImageType::Float32 => Value::Float32(0.0),
            ImageType::Float64 => Value::Float64(0.0),
            ImageType::Null => Value::Null,
            _ => {
                return Err(VmError::TypeMismatch {
                    expected: "defaultable array element type".to_string(),
                    found: format!("{:?}", ty),
                });
            }
        })
    }
}
