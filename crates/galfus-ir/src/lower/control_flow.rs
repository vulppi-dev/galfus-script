use super::LowerCtx;
use crate::mir::{MirBody, MirFunction, Terminator};
use galfus_image::Instruction;
use galfus_image::instruction::{GlobalIdx, Reg};

pub struct LoopLabels {
    pub start: usize,
    pub end: usize,
}

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
    pub loop_stack: Vec<LoopLabels>,
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
            loop_stack: Vec::new(),
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
        self.emit_body(&self.func.body);

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

        if !matches!(
            self.instructions.last(),
            Some(Instruction::Ret { .. })
                | Some(Instruction::RetNull)
                | Some(Instruction::Panic { .. })
        ) {
            self.instructions.push(Instruction::RetNull);
        }

        std::mem::take(&mut self.instructions)
    }

    fn emit_body(&mut self, body: &MirBody) {
        match body {
            MirBody::BasicBlock(bb) => {
                for instr in &bb.instructions {
                    match instr {
                        crate::mir::Instruction::Assign(dest, rval) => {
                            self.emit_rvalue(Reg(dest.raw() as u16), rval);
                        }
                        crate::mir::Instruction::Drop(local) => {
                            self.instructions.push(Instruction::Drop {
                                reg: Reg(local.raw() as u16),
                            });
                        }
                        crate::mir::Instruction::StoreGlobal(name, operand) => {
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
                            let src = self.operand_reg(operand);
                            self.instructions.push(Instruction::StoreGlobal {
                                global_idx: GlobalIdx(global_idx),
                                src,
                            });
                            self.free_temp_if_operand(operand);
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
                    Terminator::Break => {
                        let loop_end = self.loop_stack.last().unwrap().end;
                        self.emit_jump(loop_end, JumpKind::Unconditional);
                    }
                    Terminator::Continue => {
                        let loop_start = self.loop_stack.last().unwrap().start;
                        self.emit_jump(loop_start, JumpKind::Unconditional);
                    }
                    Terminator::Panic(msg) => {
                        let const_idx = self
                            .ctx
                            .get_or_create_constant(&crate::mir::Constant::String(msg.clone()));
                        self.instructions.push(Instruction::Panic { const_idx });
                    }
                    Terminator::None => {}
                    Terminator::Call {
                        func,
                        args,
                        destination,
                    } => {
                        if self.ctx.function_names.get(func).map(|s| s.as_str())
                            == Some("__builtin_write")
                        {
                            let arg_reg = self.alloc_temp();
                            self.load_operand_to(&args[0], arg_reg);
                            self.instructions.push(Instruction::Write { src: arg_reg });

                            let null_idx =
                                self.ctx.get_or_create_constant(&crate::mir::Constant::Null);
                            self.instructions.push(Instruction::LoadConst {
                                dest: Reg(destination.raw() as u16),
                                const_idx: null_idx,
                            });

                            self.free_temps(1);
                        } else {
                            let start_reg = self.alloc_temp();
                            let mut temp_regs = vec![start_reg];
                            for _ in 1..args.len() {
                                temp_regs.push(self.alloc_temp());
                            }

                            for (i, arg_op) in args.iter().enumerate() {
                                self.load_operand_to(arg_op, temp_regs[i]);
                            }

                            let func_idx = self.ctx.function_map[func];
                            self.instructions.push(Instruction::Call {
                                dest: Reg(destination.raw() as u16),
                                func: func_idx,
                                args_start: start_reg,
                                arg_count: args.len() as u8,
                            });

                            self.free_temps(args.len() as u16);
                        }
                    }
                }
            }
            MirBody::Block {
                locals: _,
                statements,
            } => {
                for stmt in statements {
                    self.emit_body(stmt);
                }
            }
            MirBody::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let else_label = self.new_label();
                let end_label = self.new_label();

                let cond_reg = self.operand_reg(cond);
                self.emit_jump(else_label, JumpKind::IfFalse(cond_reg));
                self.free_temp_if_operand(cond);

                self.emit_body(then_branch);
                self.emit_jump(end_label, JumpKind::Unconditional);

                self.emit_label(else_label);
                if let Some(else_b) = else_branch {
                    self.emit_body(else_b);
                }
                self.emit_label(end_label);
            }
            MirBody::Loop { body } => {
                let start_label = self.new_label();
                let end_label = self.new_label();

                self.loop_stack.push(LoopLabels {
                    start: start_label,
                    end: end_label,
                });

                self.emit_label(start_label);
                self.emit_body(body);
                self.emit_jump(start_label, JumpKind::Unconditional);
                self.emit_label(end_label);

                self.loop_stack.pop();
            }
        }
    }
}
