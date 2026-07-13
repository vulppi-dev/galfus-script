use super::LowerCtx;
use crate::mir::{
    Constant as MirConstant, Instruction as MirInstruction, MirFunction, Operand, Terminator,
};
use galfus_image::Instruction;
use galfus_image::instruction::{GlobalIdx, Reg};

#[allow(dead_code)]
pub enum JumpKind {
    Unconditional,
    IfTrue(Reg),
    IfFalse(Reg),
}

pub struct FnEmitter<'a, 'b> {
    pub ctx: &'b mut LowerCtx<'a>,
    pub func: &'a MirFunction,
    pub param_count: u16,
    pub local_count: u16,
    pub instructions: Vec<Instruction>,
    pub temp_count_current: u16,
    pub temp_count_max: u16,
    next_label_id: usize,
    label_pcs: std::collections::HashMap<usize, usize>,
    pending_jumps: Vec<(usize, usize, JumpKind)>,
}

impl<'a, 'b> FnEmitter<'a, 'b> {
    pub fn new(
        ctx: &'b mut LowerCtx<'a>,
        func: &'a MirFunction,
        param_count: u16,
        local_count: u16,
    ) -> Self {
        Self {
            ctx,
            func,
            param_count,
            local_count,
            instructions: Vec::new(),
            temp_count_current: 0,
            temp_count_max: 0,
            next_label_id: 0,
            label_pcs: std::collections::HashMap::new(),
            pending_jumps: Vec::new(),
        }
    }

    pub fn alloc_temp(&mut self) -> Reg {
        let id = self.param_count + self.local_count + self.temp_count_current;
        self.temp_count_current += 1;
        if self.temp_count_current > self.temp_count_max {
            self.temp_count_max = self.temp_count_current;
        }
        Reg(id)
    }

    pub fn free_temps(&mut self, count: u16) {
        self.temp_count_current = self.temp_count_current.saturating_sub(count);
    }

    pub fn new_label(&mut self) -> usize {
        let id = self.next_label_id;
        self.next_label_id += 1;
        id
    }

    pub fn emit_label(&mut self, label: usize) {
        let pc = self.instructions.len();
        self.label_pcs.insert(label, pc);
    }

    pub fn emit_jump(&mut self, target_label: usize, kind: JumpKind) {
        let pc = self.instructions.len();
        self.pending_jumps.push((pc, target_label, kind));
        self.instructions.push(Instruction::RetNull);
    }

    pub fn emit(&mut self) -> Vec<Instruction> {
        let mut block_labels = std::collections::HashMap::new();
        for bb in &self.func.blocks {
            block_labels.insert(bb.id, self.new_label());
        }

        for bb in &self.func.blocks {
            let label = block_labels[&bb.id];
            self.emit_label(label);

            for inst in &bb.instructions {
                match inst {
                    MirInstruction::Assign(dest, rvalue) => {
                        self.emit_rvalue(Reg(dest.raw() as u16), rvalue);
                    }
                    MirInstruction::Drop(local) => {
                        self.instructions.push(Instruction::Drop {
                            reg: Reg(local.raw() as u16),
                        });
                    }
                    MirInstruction::StoreGlobal(_name, op) => {
                        let global_idx = 0;
                        let val_reg = self.operand_reg(op);
                        self.instructions.push(Instruction::StoreGlobal {
                            global_idx: GlobalIdx(global_idx as u16),
                            src: val_reg,
                        });
                        self.free_temp_if_operand(op);
                    }
                    MirInstruction::StoreIndex { arr, idx, val } => {
                        let arr_reg = self.operand_reg(arr);
                        let idx_reg = self.operand_reg(idx);
                        let val_reg = self.operand_reg(val);

                        self.instructions.push(Instruction::StoreIndex {
                            arr: arr_reg,
                            idx: idx_reg,
                            val: val_reg,
                        });

                        self.free_temp_if_operand(val);
                        self.free_temp_if_operand(idx);
                        self.free_temp_if_operand(arr);
                    }
                    MirInstruction::StoreField {
                        obj,
                        field_name,
                        val,
                    } => {
                        let obj_reg = self.operand_reg(obj);
                        let val_reg = self.operand_reg(val);
                        let field = self.field_idx_for_member(obj, field_name);

                        self.instructions.push(Instruction::StoreField {
                            obj: obj_reg,
                            field,
                            val: val_reg,
                        });

                        self.free_temp_if_operand(val);
                        self.free_temp_if_operand(obj);
                    }
                    MirInstruction::TransactionStart { targets } => {
                        let key = targets
                            .first()
                            .cloned()
                            .unwrap_or(Operand::Constant(MirConstant::Null));
                        let key_reg = self.operand_reg(&key);
                        self.instructions.push(Instruction::TxStart { key_reg });
                        self.free_temp_if_operand(&key);
                    }
                    MirInstruction::TransactionCommit { destination } => {
                        self.instructions.push(Instruction::TxCommit {
                            dest_reg: Reg(destination.raw() as u16),
                        });
                    }
                    MirInstruction::TransactionRollback => {
                        self.instructions.push(Instruction::TxRollback);
                    }
                    MirInstruction::Call {
                        func,
                        args,
                        destination,
                    } => {
                        let builtin_name = self.ctx.function_names.get(func).map(|s| s.as_str());
                        if builtin_name == Some("__builtin_write") {
                            let arg_reg = self.alloc_temp();
                            self.load_operand_to(&args[0], arg_reg);
                            self.instructions.push(Instruction::Write { src: arg_reg });

                            let null_idx = crate::lower::constants::get_or_create_constant(
                                &mut self.ctx,
                                &MirConstant::Null,
                            );
                            self.instructions.push(Instruction::LoadConst {
                                dest: Reg(destination.raw() as u16),
                                const_idx: null_idx,
                            });

                            self.free_temps(1);
                        } else if builtin_name == Some("__builtin_read") {
                            self.instructions.push(Instruction::Read {
                                dest: Reg(destination.raw() as u16),
                            });
                        } else {
                            let start_reg = self.alloc_temp();
                            let mut temp_regs = vec![start_reg];
                            for _ in 1..args.len() {
                                temp_regs.push(self.alloc_temp());
                            }

                            for (i, arg_op) in args.iter().enumerate() {
                                self.load_operand_to(arg_op, temp_regs[i]);
                            }

                            let func_idx = *self.ctx.function_map.get(func).unwrap_or_else(|| {
                                panic!(
                                    "missing lowered function mapping for {:?} while emitting {} ({:?})",
                                    func, self.func.name, self.func.id
                                )
                            });
                            self.instructions.push(Instruction::Call {
                                dest: Reg(destination.raw() as u16),
                                func: func_idx,
                                args_start: start_reg,
                                arg_count: args.len() as u8,
                            });

                            self.free_temps(args.len() as u16);
                        }
                    }
                    MirInstruction::ConstraintCall {
                        method_name,
                        obj,
                        args,
                        destination,
                    } => {
                        let obj_reg = self.alloc_temp();
                        self.load_operand_to(obj, obj_reg);

                        let mut extra_regs: Vec<Reg> = Vec::with_capacity(args.len());
                        for _ in 0..args.len() {
                            extra_regs.push(self.alloc_temp());
                        }
                        for (i, arg_op) in args.iter().enumerate() {
                            self.load_operand_to(arg_op, extra_regs[i]);
                        }

                        let name_const = crate::lower::constants::get_or_create_constant(
                            &mut self.ctx,
                            &MirConstant::String(method_name.clone()),
                        );

                        self.instructions.push(Instruction::CallMethod {
                            dest: Reg(destination.raw() as u16),
                            obj: obj_reg,
                            name_const,
                            args_start: obj_reg,
                            arg_count: (1 + args.len()) as u8,
                        });

                        self.free_temps(1 + extra_regs.len() as u16);
                    }
                    MirInstruction::IndirectCall {
                        func,
                        args,
                        destination,
                    } => {
                        let func_reg = self.alloc_temp();
                        self.load_operand_to(func, func_reg);

                        let start_reg = self.alloc_temp();
                        let mut temp_regs = vec![start_reg];
                        for _ in 1..args.len() {
                            temp_regs.push(self.alloc_temp());
                        }

                        for (i, arg_op) in args.iter().enumerate() {
                            self.load_operand_to(arg_op, temp_regs[i]);
                        }

                        self.instructions.push(Instruction::CallDynamic {
                            dest: Reg(destination.raw() as u16),
                            func_reg,
                            args_start: start_reg,
                            arg_count: args.len() as u8,
                        });

                        self.free_temps(1 + args.len() as u16);
                        self.free_temp_if_operand(func);
                    }
                }
            }

            match &bb.terminator {
                Terminator::Return(opt_operand) => {
                    if let Some(op) = opt_operand {
                        let src = self.operand_reg(op);
                        self.instructions.push(Instruction::Ret { src });
                        self.free_temp_if_operand(op);
                    } else {
                        self.instructions.push(Instruction::RetNull);
                    }
                }

                Terminator::Panic(msg) => {
                    let const_idx = crate::lower::constants::get_or_create_constant(
                        &mut self.ctx,
                        &MirConstant::String(msg.clone()),
                    );
                    self.instructions.push(Instruction::Panic { const_idx });
                }
                Terminator::Jump { target, args: _ } => {
                    self.emit_jump(block_labels[target], JumpKind::Unconditional);
                }
                Terminator::Branch {
                    cond,
                    true_block,
                    true_args: _,
                    false_block,
                    false_args: _,
                } => {
                    let cond_reg = self.operand_reg(cond);
                    self.emit_jump(block_labels[true_block], JumpKind::IfTrue(cond_reg));
                    self.emit_jump(block_labels[false_block], JumpKind::Unconditional);
                    self.free_temp_if_operand(cond);
                }
            }
        }

        for (pc, target_label, kind) in &self.pending_jumps {
            let target_pc = self.label_pcs[target_label];
            let offset = target_pc as i32 - (*pc as i32 + 1);
            let patched_instr = match kind {
                JumpKind::Unconditional => Instruction::Jump { offset },
                JumpKind::IfTrue(cond) => Instruction::JumpTrue {
                    cond: *cond,
                    offset,
                },
                JumpKind::IfFalse(cond) => Instruction::JumpFalse {
                    cond: *cond,
                    offset,
                },
            };
            self.instructions[*pc] = patched_instr;
        }

        std::mem::take(&mut self.instructions)
    }
}
