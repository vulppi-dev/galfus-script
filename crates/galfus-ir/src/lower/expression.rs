use super::control_flow::FnEmitter;
use crate::mir::{Constant as MirConstant, MirBinaryOp, MirUnaryOp, Operand, RValue};
use galfus_core::TypeId;
use galfus_core::image::Instruction;
use galfus_core::image::instruction::{FieldIdx, GlobalIdx, Reg};
use galfus_frontend::{PrimitiveType, SymbolKind, TypeKind};

impl<'a, 'b> FnEmitter<'a, 'b> {
    pub fn emit_rvalue(&mut self, dest: Reg, rvalue: &RValue) {
        match rvalue {
            RValue::Use(operand) => {
                self.load_operand_to(operand, dest);
            }
            RValue::UnaryOp(op, operand) => {
                let src = self.operand_reg(operand);
                let instr = match op {
                    MirUnaryOp::Negate => Instruction::Neg { dest, src },
                    MirUnaryOp::Not => Instruction::Not { dest, src },
                    MirUnaryOp::BitwiseNot => Instruction::BitNot { dest, src },
                };
                self.instructions.push(instr);
                self.free_temp_if_operand(operand);
            }
            RValue::BinaryOp(op, lhs, rhs) => {
                let lhs_reg = self.operand_reg(lhs);
                let rhs_reg = self.operand_reg(rhs);
                let instr = match op {
                    MirBinaryOp::Add => Instruction::Add {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::Subtract => Instruction::Sub {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::Multiply => Instruction::Mul {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::Divide => Instruction::Div {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::Remainder => Instruction::Rem {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::Power => Instruction::Pow {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::ShiftLeft => Instruction::Shl {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::ShiftRight => Instruction::Shr {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::BitwiseAnd => Instruction::And {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::BitwiseOr => Instruction::Or {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::BitwiseXor => Instruction::Xor {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::Equal => Instruction::Eq {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::NotEqual => Instruction::Ne {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::Less => Instruction::Lt {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::LessEqual => Instruction::Le {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::Greater => Instruction::Gt {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::GreaterEqual => Instruction::Ge {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::LogicalAnd => Instruction::And {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::LogicalOr => Instruction::Or {
                        dest,
                        lhs: lhs_reg,
                        rhs: rhs_reg,
                    },
                    MirBinaryOp::NullFallback => Instruction::Fallback {
                        dest,
                        src: lhs_reg,
                        fallback: rhs_reg,
                    },
                };
                self.instructions.push(instr);
                self.free_temp_if_operand(rhs);
                self.free_temp_if_operand(lhs);
            }
            RValue::Cast(operand, ty) => {
                let src = self.operand_reg(operand);
                let type_idx = self.ctx.lower_type(*ty);
                self.instructions.push(Instruction::Cast {
                    dest,
                    src,
                    type_idx,
                });
                self.free_temp_if_operand(operand);
            }
            RValue::Instanceof(operand, ty) => {
                let src = self.operand_reg(operand);
                let type_idx = self.ctx.lower_type(*ty);
                self.instructions.push(Instruction::Instanceof {
                    dest,
                    src,
                    type_idx,
                });
                self.free_temp_if_operand(operand);
            }
            RValue::LoadGlobal(name) => {
                // Find global index. For simplicity, we can just use 0 if not found,
                // or we can map it to global index.
                let global_idx = self
                    .ctx
                    .graph
                    .resolution()
                    .and_then(|res| {
                        res.symbols()
                            .iter()
                            .position(|s| s.name() == name)
                            .map(|idx| idx as u16)
                    })
                    .unwrap_or(0);
                self.instructions.push(Instruction::LoadGlobal {
                    dest,
                    global_idx: GlobalIdx(global_idx),
                });
            }
            RValue::NewStruct {
                struct_type,
                fields,
                storage_meta: _,
            } => {
                let type_idx = self.ctx.lower_type(*struct_type);
                self.instructions
                    .push(Instruction::AllocLocal { dest, type_idx });

                // Find fields list to map field names to indices
                let _struct_symbol = self.struct_symbol_for_type(*struct_type).unwrap();

                for (i, val_operand) in fields.iter().enumerate() {
                    let val_reg = self.operand_reg(val_operand);
                    self.instructions.push(Instruction::StoreField {
                        obj: dest,
                        field: FieldIdx(i as u16),
                        val: val_reg,
                    });
                    self.free_temp_if_operand(val_operand);
                }
            }
            RValue::NewArray(element_type, elements) => {
                let type_idx = self.ctx.lower_type(*element_type);
                let size_const = self
                    .ctx
                    .get_or_create_constant(&MirConstant::Int(elements.len() as i64));
                let size_reg = self.alloc_temp();
                self.instructions.push(Instruction::LoadConst {
                    dest: size_reg,
                    const_idx: size_const,
                });

                self.instructions.push(Instruction::NewArray {
                    dest,
                    type_idx,
                    len_reg: size_reg,
                });
                self.free_temps(1);

                for (i, elem_operand) in elements.iter().enumerate() {
                    let idx_const = self.ctx.get_or_create_constant(&MirConstant::Int(i as i64));
                    let idx_reg = self.alloc_temp();
                    self.instructions.push(Instruction::LoadConst {
                        dest: idx_reg,
                        const_idx: idx_const,
                    });

                    let val_reg = self.operand_reg(elem_operand);
                    self.instructions.push(Instruction::StoreIndex {
                        arr: dest,
                        idx: idx_reg,
                        val: val_reg,
                    });
                    self.free_temp_if_operand(elem_operand);
                    self.free_temps(1);
                }
            }
            RValue::NewTuple(tuple_type, elements) => {
                let type_idx = self.ctx.lower_type(*tuple_type);
                let start_reg = self.alloc_temp();

                // Allocate remaining contiguous temps
                let mut temp_regs = vec![start_reg];
                for _ in 1..elements.len() {
                    temp_regs.push(self.alloc_temp());
                }

                // Load each operand into contiguous registers
                for (i, elem_operand) in elements.iter().enumerate() {
                    self.load_operand_to(elem_operand, temp_regs[i]);
                }

                self.instructions.push(Instruction::NewTuple {
                    dest,
                    type_idx,
                    start: start_reg,
                    count: elements.len() as u8,
                });

                self.free_temps(elements.len() as u16);
            }
            RValue::ArrayIndex(arr_operand, idx_operand) => {
                let arr = self.operand_reg(arr_operand);
                let idx = self.operand_reg(idx_operand);
                self.instructions
                    .push(Instruction::LoadIndex { dest, arr, idx });
                self.free_temp_if_operand(idx_operand);
                self.free_temp_if_operand(arr_operand);
            }
            RValue::MemberAccess(obj_operand, field_name) => {
                let obj = self.operand_reg(obj_operand);
                let obj_type = self.get_operand_type(obj_operand);
                let table = self.ctx.type_result.layer().table();
                let resolved_type = self.ctx.resolve_alias_type(obj_type);

                let field_idx = if matches!(table.kind(resolved_type), Some(TypeKind::Tuple { .. }))
                {
                    field_name.parse::<u16>().unwrap_or(0)
                } else if let Some(symbol) = self.struct_symbol_for_type(obj_type) {
                    let struct_fields = self.ctx.get_struct_fields(symbol);
                    struct_fields
                        .iter()
                        .position(|(name, _)| name == field_name)
                        .unwrap_or(0) as u16
                } else {
                    0
                };

                self.instructions.push(Instruction::LoadField {
                    dest,
                    obj,
                    field: FieldIdx(field_idx),
                });
                self.free_temp_if_operand(obj_operand);
            }
            RValue::Choice(choice_type, variant_name, payload_operand) => {
                let type_idx = self.ctx.lower_type(*choice_type);
                let choice_symbol = self.struct_symbol_for_type(*choice_type).unwrap();
                let variants = self.ctx.get_choice_variants(choice_symbol);
                let variant_idx = variants
                    .iter()
                    .position(|(name, _)| name == variant_name)
                    .unwrap_or(0);

                let payload_reg = if let Some(op) = payload_operand {
                    let reg = self.operand_reg(op);
                    reg
                } else {
                    let reg = self.alloc_temp();
                    self.instructions.push(Instruction::LoadNull { dest: reg });
                    reg
                };

                self.instructions.push(Instruction::NewChoice {
                    dest,
                    type_idx,
                    variant_idx: variant_idx as u16,
                    payload: payload_reg,
                });

                if payload_operand.is_some() {
                    self.free_temp_if_operand(payload_operand.as_ref().unwrap());
                } else {
                    self.free_temps(1);
                }
            }
        }
    }

    pub fn operand_reg(&mut self, operand: &Operand) -> Reg {
        match operand {
            Operand::Local(local_id) => Reg(local_id.raw() as u16),
            Operand::Constant(constant) => {
                let const_idx = self.ctx.get_or_create_constant(constant);
                let temp = self.alloc_temp();
                self.instructions.push(Instruction::LoadConst {
                    dest: temp,
                    const_idx,
                });
                temp
            }
        }
    }

    pub fn load_operand_to(&mut self, operand: &Operand, dest: Reg) {
        match operand {
            Operand::Local(local_id) => {
                let src = Reg(local_id.raw() as u16);
                if src != dest {
                    self.instructions.push(Instruction::Move { dest, src });
                }
            }
            Operand::Constant(constant) => {
                let const_idx = self.ctx.get_or_create_constant(constant);
                self.instructions
                    .push(Instruction::LoadConst { dest, const_idx });
            }
        }
    }

    pub fn free_temp_if_operand(&mut self, operand: &Operand) {
        if matches!(operand, Operand::Constant(_)) {
            self.free_temps(1);
        }
    }

    fn get_operand_type(&self, operand: &Operand) -> TypeId {
        match operand {
            Operand::Local(local_id) => {
                let local_decl = self.func.locals.iter().find(|l| l.id == *local_id).unwrap();
                local_decl.ty
            }
            Operand::Constant(constant) => {
                let layer = self.ctx.type_result.layer();
                let table = layer.table();
                // Simple mapping for constants
                let prim = match constant {
                    MirConstant::Null => PrimitiveType::Null,
                    MirConstant::Bool(_) => PrimitiveType::Bool,
                    MirConstant::Int(_) => PrimitiveType::Int64,
                    MirConstant::Float(_) => PrimitiveType::Float64,
                    MirConstant::String(_) => {
                        // Find String type in type table
                        for i in 0..table.len() {
                            let ty_id = TypeId::new(i as u32);
                            if matches!(
                                table.kind(ty_id),
                                Some(TypeKind::Primitive(PrimitiveType::Uint8))
                            ) {
                                // Fallback to Int64 if not found
                            }
                        }
                        PrimitiveType::Int64
                    }
                };
                for i in 0..table.len() {
                    let ty_id = TypeId::new(i as u32);
                    if matches!(table.kind(ty_id), Some(TypeKind::Primitive(p)) if *p == prim) {
                        return ty_id;
                    }
                }
                TypeId::new(0)
            }
        }
    }

    fn struct_symbol_for_type(&self, ty: TypeId) -> Option<galfus_core::SymbolId> {
        let ty = self.ctx.resolve_alias_type(ty);
        let layer = self.ctx.type_result.layer();
        let table = layer.table();
        let mut current = ty;
        loop {
            match table.kind(current) {
                Some(TypeKind::Named { symbol }) => {
                    let resolution = self.ctx.graph.resolution()?;
                    let is_struct_or_choice = resolution.symbol(*symbol).is_some_and(|sd| {
                        sd.kind() == SymbolKind::Struct || sd.kind() == SymbolKind::Choice
                    });
                    if is_struct_or_choice {
                        return Some(*symbol);
                    }
                    break;
                }
                Some(TypeKind::GenericInstance { base, .. }) => {
                    current = *base;
                }
                _ => break,
            }
        }
        None
    }
}
