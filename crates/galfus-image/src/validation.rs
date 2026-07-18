use crate::instruction::{ConstIdx, FieldIdx, FuncIdx, Reg, TypeIdx};
use crate::*;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageValidationError {
    InvalidConstantIndex {
        func_name: String,
        instr_idx: usize,
        index: ConstIdx,
    },
    InvalidTypeIndex {
        func_name: String,
        instr_idx: usize,
        index: TypeIdx,
    },
    InvalidFunctionIndex {
        func_name: String,
        instr_idx: usize,
        index: FuncIdx,
    },
    InvalidRegister {
        func_name: String,
        instr_idx: usize,
        reg: Reg,
        max_allowed: u16,
    },
    InvalidJumpOffset {
        func_name: String,
        instr_idx: usize,
        target_idx: i32,
        instr_count: usize,
    },
    TypeMismatchAlloc {
        func_name: String,
        instr_idx: usize,
        expected: &'static str,
        found: TypeIdx,
    },
    LayoutOutOfBounds {
        func_name: String,
        instr_idx: usize,
        expected: &'static str,
        layout_idx: u16,
    },
    ChoiceVariantOutOfBounds {
        func_name: String,
        instr_idx: usize,
        variant_idx: u16,
        variant_count: usize,
    },
    TupleCountMismatch {
        func_name: String,
        instr_idx: usize,
        expected_count: usize,
        found_count: usize,
    },
    FieldOutOfBounds {
        func_name: String,
        instr_idx: usize,
        field_idx: FieldIdx,
    },
    ExportFunctionOutOfBounds {
        symbol_name: String,
        func_idx: FuncIdx,
    },
    InitFunctionOutOfBounds {
        func_idx: FuncIdx,
    },
}

pub fn validate_module_image(image: &ModuleImage) -> Result<(), Vec<ImageValidationError>> {
    let mut errors = Vec::new();

    // 1. Validate init function
    if let Some(init_idx) = image.init_func_idx
        && init_idx.raw() as usize >= image.functions.len()
    {
        errors.push(ImageValidationError::InitFunctionOutOfBounds { func_idx: init_idx });
    }

    // 2. Validate exports
    for export in &image.exports {
        if export.func_idx.raw() as usize >= image.functions.len() {
            errors.push(ImageValidationError::ExportFunctionOutOfBounds {
                symbol_name: export.symbol_name.clone(),
                func_idx: export.func_idx,
            });
        }
    }

    // 3. Helper to determine max fields in any struct or choice variant
    let max_struct_fields = image
        .struct_layouts
        .iter()
        .map(|l| l.fields.len())
        .max()
        .unwrap_or(0);
    let max_choice_payloads = image
        .choice_layouts
        .iter()
        .map(|l| {
            l.variants
                .iter()
                .map(|v| if v.payload_ty.is_some() { 1 } else { 0 })
                .max()
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0);
    let max_fields = max_struct_fields.max(max_choice_payloads);

    // 4. Validate instructions of each function
    for func in &image.functions {
        let max_regs = func.param_count as u16 + func.local_count + func.temp_count;
        let func_name = &func.name;

        for (instr_idx, &instr) in func.instructions.iter().enumerate() {
            let check_reg = |reg: Reg, errors: &mut Vec<ImageValidationError>| {
                if reg.raw() >= max_regs {
                    errors.push(ImageValidationError::InvalidRegister {
                        func_name: func_name.clone(),
                        instr_idx,
                        reg,
                        max_allowed: max_regs,
                    });
                }
            };

            let check_const = |idx: ConstIdx, errors: &mut Vec<ImageValidationError>| {
                if idx.raw() as usize >= image.constants.constants.len() {
                    errors.push(ImageValidationError::InvalidConstantIndex {
                        func_name: func_name.clone(),
                        instr_idx,
                        index: idx,
                    });
                }
            };

            let check_type = |idx: TypeIdx, errors: &mut Vec<ImageValidationError>| {
                if idx.raw() as usize >= image.types.len() {
                    errors.push(ImageValidationError::InvalidTypeIndex {
                        func_name: func_name.clone(),
                        instr_idx,
                        index: idx,
                    });
                }
            };

            let check_func = |idx: FuncIdx, errors: &mut Vec<ImageValidationError>| {
                let limit = image.functions.len() + image.imports.len();
                if idx.raw() as usize >= limit {
                    errors.push(ImageValidationError::InvalidFunctionIndex {
                        func_name: func_name.clone(),
                        instr_idx,
                        index: idx,
                    });
                }
            };

            let check_jump = |offset: i32, errors: &mut Vec<ImageValidationError>| {
                let target = instr_idx as i32 + 1 + offset;
                if target < 0 || target >= func.instructions.len() as i32 {
                    errors.push(ImageValidationError::InvalidJumpOffset {
                        func_name: func_name.clone(),
                        instr_idx,
                        target_idx: target,
                        instr_count: func.instructions.len(),
                    });
                }
            };

            match instr {
                // Category A
                Instruction::LoadConst { dest, const_idx } => {
                    check_reg(dest, &mut errors);
                    check_const(const_idx, &mut errors);
                }
                Instruction::Move { dest, src } => {
                    check_reg(dest, &mut errors);
                    check_reg(src, &mut errors);
                }
                Instruction::LoadGlobal {
                    dest,
                    global_idx: _,
                } => {
                    check_reg(dest, &mut errors);
                }
                Instruction::StoreGlobal { global_idx: _, src } => {
                    check_reg(src, &mut errors);
                }
                Instruction::LoadNull { dest } => {
                    check_reg(dest, &mut errors);
                }

                // Category B
                Instruction::Add { dest, lhs, rhs }
                | Instruction::Sub { dest, lhs, rhs }
                | Instruction::Mul { dest, lhs, rhs }
                | Instruction::Div { dest, lhs, rhs }
                | Instruction::Rem { dest, lhs, rhs }
                | Instruction::Pow { dest, lhs, rhs }
                | Instruction::Shl { dest, lhs, rhs }
                | Instruction::Shr { dest, lhs, rhs }
                | Instruction::And { dest, lhs, rhs }
                | Instruction::Or { dest, lhs, rhs }
                | Instruction::Xor { dest, lhs, rhs }
                | Instruction::Eq { dest, lhs, rhs }
                | Instruction::Ne { dest, lhs, rhs }
                | Instruction::Lt { dest, lhs, rhs }
                | Instruction::Le { dest, lhs, rhs }
                | Instruction::Gt { dest, lhs, rhs }
                | Instruction::Ge { dest, lhs, rhs } => {
                    check_reg(dest, &mut errors);
                    check_reg(lhs, &mut errors);
                    check_reg(rhs, &mut errors);
                }
                Instruction::Neg { dest, src }
                | Instruction::Not { dest, src }
                | Instruction::BitNot { dest, src } => {
                    check_reg(dest, &mut errors);
                    check_reg(src, &mut errors);
                }
                Instruction::Fallback {
                    dest,
                    src,
                    fallback,
                } => {
                    check_reg(dest, &mut errors);
                    check_reg(src, &mut errors);
                    check_reg(fallback, &mut errors);
                }

                // Category C
                Instruction::Jump { offset } => {
                    check_jump(offset, &mut errors);
                }
                Instruction::JumpTrue { cond, offset }
                | Instruction::JumpFalse { cond, offset } => {
                    check_reg(cond, &mut errors);
                    check_jump(offset, &mut errors);
                }
                Instruction::JumpNull { val, offset } => {
                    check_reg(val, &mut errors);
                    check_jump(offset, &mut errors);
                }
                Instruction::Call {
                    dest,
                    func: func_idx,
                    args_start,
                    arg_count,
                } => {
                    check_reg(dest, &mut errors);
                    check_func(func_idx, &mut errors);
                    if arg_count > 0 {
                        check_reg(args_start, &mut errors);
                        let end_reg = args_start.raw() as u32 + arg_count as u32 - 1;
                        if end_reg >= max_regs as u32 {
                            errors.push(ImageValidationError::InvalidRegister {
                                func_name: func_name.clone(),
                                instr_idx,
                                reg: Reg(end_reg as u16),
                                max_allowed: max_regs,
                            });
                        }
                    }
                }
                Instruction::CallMethod {
                    dest,
                    obj,
                    name_const,
                    args_start,
                    arg_count,
                } => {
                    check_reg(dest, &mut errors);
                    check_reg(obj, &mut errors);
                    check_const(name_const, &mut errors);
                    if arg_count > 1 {
                        check_reg(Reg(args_start.raw() + arg_count as u16 - 2), &mut errors);
                    }
                }
                Instruction::CallDynamic {
                    dest,
                    func_reg,
                    args_start,
                    arg_count,
                } => {
                    check_reg(dest, &mut errors);
                    check_reg(func_reg, &mut errors);
                    if arg_count > 0 {
                        check_reg(args_start, &mut errors);
                        let end_reg = args_start.raw() as u32 + arg_count as u32 - 1;
                        if end_reg >= max_regs as u32 {
                            errors.push(ImageValidationError::InvalidRegister {
                                func_name: func_name.clone(),
                                instr_idx,
                                reg: Reg(end_reg as u16),
                                max_allowed: max_regs,
                            });
                        }
                    }
                }
                Instruction::Ret { src } => {
                    check_reg(src, &mut errors);
                }
                Instruction::RetNull => {}
                Instruction::Panic { const_idx } => {
                    check_const(const_idx, &mut errors);
                }

                // Category D
                Instruction::AllocLocal { dest, type_idx }
                | Instruction::AllocShared { dest, type_idx } => {
                    check_reg(dest, &mut errors);
                    check_type(type_idx, &mut errors);
                    if (type_idx.raw() as usize) < image.types.len() {
                        match &image.types[type_idx.raw() as usize] {
                            ImageType::Struct(layout_idx) => {
                                if layout_idx.raw() as usize >= image.struct_layouts.len() {
                                    errors.push(ImageValidationError::LayoutOutOfBounds {
                                        func_name: func_name.clone(),
                                        instr_idx,
                                        expected: "StructLayout",
                                        layout_idx: layout_idx.raw(),
                                    });
                                }
                            }
                            _ => {
                                errors.push(ImageValidationError::TypeMismatchAlloc {
                                    func_name: func_name.clone(),
                                    instr_idx,
                                    expected: "Struct",
                                    found: type_idx,
                                });
                            }
                        }
                    }
                }
                Instruction::LoadField { dest, obj, field } => {
                    check_reg(dest, &mut errors);
                    check_reg(obj, &mut errors);
                    if field.raw() as usize >= max_fields {
                        errors.push(ImageValidationError::FieldOutOfBounds {
                            func_name: func_name.clone(),
                            instr_idx,
                            field_idx: field,
                        });
                    }
                }
                Instruction::StoreField { obj, field, val } => {
                    check_reg(obj, &mut errors);
                    check_reg(val, &mut errors);
                    if field.raw() as usize >= max_fields {
                        errors.push(ImageValidationError::FieldOutOfBounds {
                            func_name: func_name.clone(),
                            instr_idx,
                            field_idx: field,
                        });
                    }
                }
                Instruction::NewArray {
                    dest,
                    type_idx,
                    len_reg,
                } => {
                    check_reg(dest, &mut errors);
                    check_type(type_idx, &mut errors);
                    check_reg(len_reg, &mut errors);
                }
                Instruction::LoadIndex { dest, arr, idx } => {
                    check_reg(dest, &mut errors);
                    check_reg(arr, &mut errors);
                    check_reg(idx, &mut errors);
                }
                Instruction::StoreIndex { arr, idx, val } => {
                    check_reg(arr, &mut errors);
                    check_reg(idx, &mut errors);
                    check_reg(val, &mut errors);
                }
                Instruction::NewTuple {
                    dest,
                    type_idx,
                    start,
                    count,
                } => {
                    check_reg(dest, &mut errors);
                    check_type(type_idx, &mut errors);
                    if count > 0 {
                        check_reg(start, &mut errors);
                        let end_reg = start.raw() as u32 + count as u32 - 1;
                        if end_reg >= max_regs as u32 {
                            errors.push(ImageValidationError::InvalidRegister {
                                func_name: func_name.clone(),
                                instr_idx,
                                reg: Reg(end_reg as u16),
                                max_allowed: max_regs,
                            });
                        }
                    }
                    if (type_idx.raw() as usize) < image.types.len() {
                        match &image.types[type_idx.raw() as usize] {
                            ImageType::Tuple(elements) => {
                                if elements.len() != count as usize {
                                    errors.push(ImageValidationError::TupleCountMismatch {
                                        func_name: func_name.clone(),
                                        instr_idx,
                                        expected_count: elements.len(),
                                        found_count: count as usize,
                                    });
                                }
                            }
                            _ => {
                                errors.push(ImageValidationError::TypeMismatchAlloc {
                                    func_name: func_name.clone(),
                                    instr_idx,
                                    expected: "Tuple",
                                    found: type_idx,
                                });
                            }
                        }
                    }
                }
                Instruction::NewChoice {
                    dest,
                    type_idx,
                    variant_idx,
                    payload,
                } => {
                    check_reg(dest, &mut errors);
                    check_type(type_idx, &mut errors);
                    check_reg(payload, &mut errors);
                    if (type_idx.raw() as usize) < image.types.len() {
                        match &image.types[type_idx.raw() as usize] {
                            ImageType::Choice(layout_idx) => {
                                if layout_idx.raw() as usize >= image.choice_layouts.len() {
                                    errors.push(ImageValidationError::LayoutOutOfBounds {
                                        func_name: func_name.clone(),
                                        instr_idx,
                                        expected: "ChoiceLayout",
                                        layout_idx: layout_idx.raw(),
                                    });
                                } else {
                                    let layout = &image.choice_layouts[layout_idx.raw() as usize];
                                    if variant_idx as usize >= layout.variants.len() {
                                        errors.push(
                                            ImageValidationError::ChoiceVariantOutOfBounds {
                                                func_name: func_name.clone(),
                                                instr_idx,
                                                variant_idx,
                                                variant_count: layout.variants.len(),
                                            },
                                        );
                                    }
                                }
                            }
                            _ => {
                                errors.push(ImageValidationError::TypeMismatchAlloc {
                                    func_name: func_name.clone(),
                                    instr_idx,
                                    expected: "Choice",
                                    found: type_idx,
                                });
                            }
                        }
                    }
                }
                Instruction::Cast {
                    dest,
                    src,
                    type_idx,
                } => {
                    check_reg(dest, &mut errors);
                    check_reg(src, &mut errors);
                    check_type(type_idx, &mut errors);
                }
                Instruction::Copy { dest, src } => {
                    check_reg(dest, &mut errors);
                    check_reg(src, &mut errors);
                }
                Instruction::Instanceof {
                    dest,
                    src,
                    type_idx,
                } => {
                    check_reg(dest, &mut errors);
                    check_reg(src, &mut errors);
                    check_type(type_idx, &mut errors);
                }

                // Category E
                Instruction::Drop { reg } => {
                    check_reg(reg, &mut errors);
                }

                // Category F
                Instruction::TxStart { key_reg } => {
                    check_reg(key_reg, &mut errors);
                }
                Instruction::TxLoad { dest, obj, field } => {
                    check_reg(dest, &mut errors);
                    check_reg(obj, &mut errors);
                    if field.raw() as usize >= max_fields {
                        errors.push(ImageValidationError::FieldOutOfBounds {
                            func_name: func_name.clone(),
                            instr_idx,
                            field_idx: field,
                        });
                    }
                }
                Instruction::TxStore { obj, field, val } => {
                    check_reg(obj, &mut errors);
                    check_reg(val, &mut errors);
                    if field.raw() as usize >= max_fields {
                        errors.push(ImageValidationError::FieldOutOfBounds {
                            func_name: func_name.clone(),
                            instr_idx,
                            field_idx: field,
                        });
                    }
                }
                Instruction::TxCommit { dest_reg } => {
                    check_reg(dest_reg, &mut errors);
                }
                Instruction::TxRollback => {}
                Instruction::Write { src } => {
                    check_reg(src, &mut errors);
                }
                Instruction::Read { dest, terminator } => {
                    check_reg(dest, &mut errors);
                    check_reg(terminator, &mut errors);
                }
                Instruction::Len { dest, src } => {
                    check_reg(dest, &mut errors);
                    check_reg(src, &mut errors);
                }
                Instruction::CopyArray {
                    dest,
                    dest_start,
                    src,
                } => {
                    check_reg(dest, &mut errors);
                    check_reg(dest_start, &mut errors);
                    check_reg(src, &mut errors);
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
