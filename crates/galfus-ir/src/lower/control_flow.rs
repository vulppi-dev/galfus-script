use super::LowerCtx;
use crate::mir::{
    Constant as MirConstant, Instruction as MirInstruction, MirBody, MirFunction, Terminator,
};
use galfus_frontend::SyntaxNodeKind;
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
                        MirInstruction::Assign(dest, rval) => {
                            self.emit_rvalue(Reg(dest.raw() as u16), rval);
                        }
                        MirInstruction::Drop(local) => {
                            self.instructions.push(Instruction::Drop {
                                reg: Reg(local.raw() as u16),
                            });
                        }
                        MirInstruction::StoreGlobal(name, operand) => {
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
                            .get_or_create_constant(&MirConstant::String(msg.clone()));
                        self.instructions.push(Instruction::Panic { const_idx });
                    }
                    Terminator::ConstraintCall {
                        method_name,
                        obj,
                        args,
                        destination,
                    } => {
                        // Allocate contiguous registers: obj first, then extra args.
                        let obj_reg = self.alloc_temp();
                        self.load_operand_to(obj, obj_reg);

                        let mut extra_regs: Vec<Reg> = Vec::with_capacity(args.len());
                        for _ in 0..args.len() {
                            extra_regs.push(self.alloc_temp());
                        }
                        for (i, arg_op) in args.iter().enumerate() {
                            self.load_operand_to(arg_op, extra_regs[i]);
                        }

                        let name_const = self
                            .ctx
                            .get_or_create_constant(&MirConstant::String(method_name.clone()));

                        self.instructions.push(Instruction::CallMethod {
                            dest: Reg(destination.raw() as u16),
                            obj: obj_reg,
                            name_const,
                            args_start: obj_reg,
                            arg_count: (1 + args.len()) as u8,
                        });

                        self.free_temps(1 + extra_regs.len() as u16);
                    }
                    Terminator::None => {}
                    Terminator::Call {
                        func,
                        args,
                        destination,
                    } => {
                        let builtin_name = self.ctx.function_names.get(func).map(|s| s.as_str());
                        if self.is_std_buffer_create_call_func(*func) {
                            let len_reg = self.alloc_temp();
                            self.load_operand_to(&args[0], len_reg);

                            let dest_decl = self
                                .func
                                .locals
                                .iter()
                                .find(|local| local.id == *destination)
                                .unwrap();
                            let type_idx = self.ctx.lower_type(dest_decl.ty);

                            self.instructions.push(Instruction::NewArray {
                                dest: Reg(destination.raw() as u16),
                                type_idx,
                                len_reg,
                            });

                            self.free_temps(1);
                        } else if builtin_name == Some("__builtin_write") {
                            let arg_reg = self.alloc_temp();
                            self.load_operand_to(&args[0], arg_reg);
                            self.instructions.push(Instruction::Write { src: arg_reg });

                            let null_idx = self.ctx.get_or_create_constant(&MirConstant::Null);
                            self.instructions.push(Instruction::LoadConst {
                                dest: Reg(destination.raw() as u16),
                                const_idx: null_idx,
                            });

                            self.free_temps(1);
                        } else if builtin_name == Some("__builtin_read") {
                            self.instructions.push(Instruction::Read {
                                dest: Reg(destination.raw() as u16),
                            });
                        } else if builtin_name.map(|s| s.starts_with("__builtin_create_buffer")).unwrap_or(false) {
                            let len_reg = self.alloc_temp();
                            self.load_operand_to(&args[0], len_reg);

                            let dest_decl = self.func.locals.iter().find(|l| l.id == *destination).unwrap();
                            let type_idx = self.ctx.lower_type(dest_decl.ty);

                            self.instructions.push(Instruction::NewArray {
                                dest: Reg(destination.raw() as u16),
                                type_idx,
                                len_reg,
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

    fn is_std_buffer_create_call_func(&self, func: galfus_core::FunctionId) -> bool {
        const PATH_CALL_TARGET_TAG: u32 = 0x8000_0000;

        let raw = func.raw();
        if raw & PATH_CALL_TARGET_TAG == 0 {
            return false;
        }

        let node_id = galfus_core::NodeId::new(raw & !PATH_CALL_TARGET_TAG);
        let syntax = self.ctx.graph.syntax();
        let Some(node) = syntax.node(node_id) else {
            return false;
        };

        if node.kind() != SyntaxNodeKind::PathExpression {
            return false;
        }

        let Some(root_node) = node.child(0) else {
            return false;
        };
        let Some(member_node) = node.child(1) else {
            return false;
        };
        let Some(member_node_data) = syntax.node(member_node) else {
            return false;
        };

        let member_span = member_node_data.span();
        let member_name = if member_span.start() as usize <= self.ctx.source_text.len()
            && member_span.end() as usize <= self.ctx.source_text.len()
        {
            &self.ctx.source_text[member_span.start() as usize..member_span.end() as usize]
        } else {
            ""
        };

        if member_name != "create" {
            return false;
        }

        let Some(resolution) = self.ctx.graph.resolution() else {
            return false;
        };

        let root_symbol = resolution.reference_symbol(root_node).or_else(|| {
            let identifier = syntax.first_child_of_kind(root_node, SyntaxNodeKind::Identifier)?;
            resolution.reference_symbol(identifier)
        });

        root_symbol
            .and_then(|symbol| resolution.import_for_symbol(symbol))
            .and_then(|import_id| resolution.import(import_id))
            .is_some_and(|import| import.source() == "std/buffer")
    }
}
