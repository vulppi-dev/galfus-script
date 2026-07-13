use crate::LocalId;
use crate::mir::*;
use galfus_frontend::TypeCheckResult;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub message: String,
}

pub fn validate_module(
    module: &MirModule,
    _type_result: &TypeCheckResult,
) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    for func in &module.functions {
        if let Err(mut func_errors) = validate_function(func) {
            errors.append(&mut func_errors);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_function(func: &MirFunction) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    let mut blocks = HashMap::new();
    for block in &func.blocks {
        if blocks.insert(block.id, block).is_some() {
            errors.push(ValidationError {
                message: format!(
                    "Function '{}': Duplicate basic block ID {:?}",
                    func.name, block.id
                ),
            });
        }
    }

    if !blocks.contains_key(&BlockId::new(0)) {
        errors.push(ValidationError {
            message: format!("Function '{}': Missing entry block", func.name),
        });
        return Err(errors);
    }

    for block in &func.blocks {
        for target in successor_blocks(&block.terminator) {
            if !blocks.contains_key(&target) {
                errors.push(ValidationError {
                    message: format!(
                        "Function '{}': Block {:?} jumps to undeclared block {:?}",
                        func.name, block.id, target
                    ),
                });
            }
        }
    }

    let mut assigned_locals = HashSet::new();
    for i in 0..func.parameter_types.len() {
        assigned_locals.insert(LocalId::new(i as u32));
    }

    for block in &func.blocks {
        for param in &block.parameters {
            if !assigned_locals.insert(param.id) {
                errors.push(ValidationError {
                    message: format!(
                        "SSA Violation: Local {:?} assigned multiple times (block parameter)",
                        param.id
                    ),
                });
            }
        }
        for inst in &block.instructions {
            match inst {
                Instruction::Assign(dest, _)
                | Instruction::Call {
                    destination: dest, ..
                }
                | Instruction::IndirectCall {
                    destination: dest, ..
                }
                | Instruction::ConstraintCall {
                    destination: dest, ..
                }
                | Instruction::TransactionCommit { destination: dest } => {
                    if !assigned_locals.insert(*dest) {
                        errors.push(ValidationError {
                            message: format!(
                                "SSA Violation: Local {:?} assigned multiple times",
                                dest
                            ),
                        });
                    }
                }
                _ => {}
            }
        }

        match &block.terminator {
            Terminator::Jump { target, args } => {
                if let Some(target_block) = blocks.get(target)
                    && args.len() != target_block.parameters.len() {
                        errors.push(ValidationError {
                            message: "Jump arguments length mismatch".to_string(),
                        });
                    }
            }
            Terminator::Branch {
                true_block,
                true_args,
                false_block,
                false_args,
                ..
            } => {
                if let Some(tb) = blocks.get(true_block)
                    && true_args.len() != tb.parameters.len() {
                        errors.push(ValidationError {
                            message: "Branch true arguments mismatch".to_string(),
                        });
                    }
                if let Some(fb) = blocks.get(false_block)
                    && false_args.len() != fb.parameters.len() {
                        errors.push(ValidationError {
                            message: "Branch false arguments mismatch".to_string(),
                        });
                    }
            }
            _ => {}
        }
    }

    let mut entry_initialized = HashSet::new();
    for idx in 0..func.parameter_types.len() {
        entry_initialized.insert(LocalId::new(idx as u32));
    }

    let initialized_at_entry = initialized_at_block_entries(&blocks, entry_initialized);
    let all_declared = func
        .locals
        .iter()
        .map(|decl| decl.id)
        .collect::<HashSet<_>>();

    for bb in &func.blocks {
        let mut initialized = initialized_at_entry
            .get(&bb.id)
            .cloned()
            .unwrap_or_else(|| all_declared.clone());

        for param in &bb.parameters {
            initialized.insert(param.id);
        }

        validate_basic_block(bb, func, &mut initialized, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn initialized_at_block_entries(
    blocks: &HashMap<BlockId, &BasicBlock>,
    entry_initialized: HashSet<LocalId>,
) -> HashMap<BlockId, HashSet<LocalId>> {
    let entry = BlockId::new(0);
    let mut initialized = HashMap::from([(entry, entry_initialized)]);
    let mut pending = VecDeque::from([entry]);

    while let Some(block_id) = pending.pop_front() {
        let Some(block) = blocks.get(&block_id) else {
            continue;
        };
        let mut outgoing = initialized[&block_id].clone();
        apply_initialization_effects(block, &mut outgoing);

        for successor in successor_blocks(&block.terminator) {
            if !blocks.contains_key(&successor) {
                continue;
            }
            let changed = match initialized.get(&successor) {
                Some(previous) => {
                    let merged = previous
                        .intersection(&outgoing)
                        .copied()
                        .collect::<HashSet<_>>();
                    if &merged == previous {
                        false
                    } else {
                        initialized.insert(successor, merged);
                        true
                    }
                }
                None => {
                    initialized.insert(successor, outgoing.clone());
                    true
                }
            };
            if changed {
                pending.push_back(successor);
            }
        }
    }

    initialized
}

fn apply_initialization_effects(block: &BasicBlock, initialized: &mut HashSet<LocalId>) {
    for param in &block.parameters {
        initialized.insert(param.id);
    }
    for instruction in &block.instructions {
        match instruction {
            Instruction::Assign(_, rvalue) => {
                if let RValue::Use(Operand::Local(l)) = rvalue {
                    initialized.remove(l);
                }
            }
            Instruction::Call { args, .. }
            | Instruction::ConstraintCall { args, .. }
            | Instruction::IndirectCall { args, .. } => {
                for arg in args {
                    if let Operand::Local(l) = arg {
                        initialized.remove(l);
                    }
                }
            }
            Instruction::StoreGlobal(_, Operand::Local(l)) => {
                initialized.remove(l);
            }
            Instruction::StoreIndex {
                val: Operand::Local(l),
                ..
            } => {
                initialized.remove(l);
            }
            Instruction::StoreField {
                val: Operand::Local(l),
                ..
            } => {
                initialized.remove(l);
            }
            Instruction::Drop(local) => {
                initialized.remove(local);
            }
            _ => {}
        }

        match instruction {
            Instruction::Assign(destination, _)
            | Instruction::TransactionCommit { destination }
            | Instruction::Call { destination, .. }
            | Instruction::ConstraintCall { destination, .. }
            | Instruction::IndirectCall { destination, .. } => {
                initialized.insert(*destination);
            }
            _ => {}
        }
    }
    match &block.terminator {
        Terminator::Return(Some(Operand::Local(l))) => {
            initialized.remove(l);
        }
        Terminator::Branch {
            cond: Operand::Local(l),
            ..
        } => {
            initialized.remove(l);
        }
        _ => {}
    }
}

fn successor_blocks(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Jump { target, .. } => vec![*target],
        Terminator::Branch {
            true_block,
            false_block,
            ..
        } => vec![*true_block, *false_block],
        Terminator::Return(_) | Terminator::Panic(_) => Vec::new(),
    }
}

fn validate_basic_block(
    bb: &BasicBlock,
    func: &MirFunction,
    initialized: &mut HashSet<LocalId>,
    errors: &mut Vec<ValidationError>,
) {
    for inst in &bb.instructions {
        match inst {
            Instruction::Assign(dest, rvalue) => {
                if !func.locals.iter().any(|decl| decl.id == *dest) {
                    errors.push(ValidationError {
                        message: format!(
                            "Function '{}': Assignment to undeclared local ID {:?}",
                            func.name, dest
                        ),
                    });
                }
                validate_rvalue_operands(rvalue, func, initialized, errors);
                initialized.insert(*dest);
            }
            Instruction::Drop(local) => {
                if !func.locals.iter().any(|decl| decl.id == *local) {
                    errors.push(ValidationError {
                        message: format!(
                            "Function '{}': Drop of undeclared local ID {:?}",
                            func.name, local
                        ),
                    });
                }
                initialized.remove(local);
            }
            Instruction::StoreGlobal(_, val) => {
                validate_operand(val, func, initialized, errors);
            }
            Instruction::StoreIndex { arr, idx, val } => {
                validate_operand(arr, func, initialized, errors);
                validate_operand(idx, func, initialized, errors);
                validate_operand(val, func, initialized, errors);
            }
            Instruction::StoreField { obj, val, .. } => {
                validate_operand(obj, func, initialized, errors);
                validate_operand(val, func, initialized, errors);
            }
            Instruction::TransactionStart { targets } => {
                for target in targets {
                    validate_operand(target, func, initialized, errors);
                }
            }
            Instruction::TransactionCommit { destination } => {
                if !func.locals.iter().any(|decl| decl.id == *destination) {
                    errors.push(ValidationError {
                        message: format!(
                            "Function '{}': Transaction commit assigned to undeclared local ID {:?}",
                            func.name, destination
                        ),
                    });
                }
                initialized.insert(*destination);
            }
            Instruction::TransactionRollback => {}
            Instruction::Call {
                func: _,
                args,
                destination,
            } => {
                for arg in args {
                    validate_operand(arg, func, initialized, errors);
                }
                if !func.locals.iter().any(|decl| decl.id == *destination) {
                    errors.push(ValidationError {
                        message: format!(
                            "Function '{}': Call assigned to undeclared local ID {:?}",
                            func.name, destination
                        ),
                    });
                }
                initialized.insert(*destination);
            }
            Instruction::ConstraintCall {
                method_name: _,
                obj,
                args,
                destination,
            } => {
                validate_operand(obj, func, initialized, errors);
                for arg in args {
                    validate_operand(arg, func, initialized, errors);
                }
                if !func.locals.iter().any(|decl| decl.id == *destination) {
                    errors.push(ValidationError {
                        message: format!(
                            "Function '{}': Constraint call assigned to undeclared local ID {:?}",
                            func.name, destination
                        ),
                    });
                }
                initialized.insert(*destination);
            }
            Instruction::IndirectCall {
                func: func_op,
                args,
                destination,
            } => {
                validate_operand(func_op, func, initialized, errors);
                for arg in args {
                    validate_operand(arg, func, initialized, errors);
                }
                if !func.locals.iter().any(|decl| decl.id == *destination) {
                    errors.push(ValidationError {
                        message: format!(
                            "Function '{}': Indirect call assigned to undeclared local ID {:?}",
                            func.name, destination
                        ),
                    });
                }
                initialized.insert(*destination);
            }
        }
    }

    match &bb.terminator {
        Terminator::Return(Some(op)) => {
            validate_operand(op, func, initialized, errors);
        }
        Terminator::Return(None) => {}
        Terminator::Jump { args, .. } => {
            for arg in args {
                validate_operand(arg, func, initialized, errors);
            }
        }
        Terminator::Branch {
            cond,
            true_args,
            false_args,
            ..
        } => {
            validate_operand(cond, func, initialized, errors);
            for arg in true_args {
                validate_operand(arg, func, initialized, errors);
            }
            for arg in false_args {
                validate_operand(arg, func, initialized, errors);
            }
        }
        Terminator::Panic(_) => {}
    }
}

fn validate_rvalue_operands(
    rvalue: &RValue,
    func: &MirFunction,
    initialized: &HashSet<LocalId>,
    errors: &mut Vec<ValidationError>,
) {
    match rvalue {
        RValue::Use(operand)
        | RValue::UnaryOp(_, operand)
        | RValue::Len(operand)
        | RValue::Copy(operand)
        | RValue::MemberAccess(operand, _)
        | RValue::ChoiceVariantIs(operand, _)
        | RValue::Instanceof(operand, _)
        | RValue::Cast(operand, _) => {
            validate_operand(operand, func, initialized, errors);
        }
        RValue::BinaryOp(_, lhs, rhs) | RValue::ArrayIndex(lhs, rhs) => {
            validate_operand(lhs, func, initialized, errors);
            validate_operand(rhs, func, initialized, errors);
        }
        RValue::Choice(_, _, op) => {
            if let Some(op) = op {
                validate_operand(op, func, initialized, errors);
            }
        }
        RValue::NewStruct { fields, .. }
        | RValue::NewArray(_, fields)
        | RValue::NewTuple(_, fields) => {
            for op in fields {
                validate_operand(op, func, initialized, errors);
            }
        }
        RValue::NewArrayDynamic(_, elements) => {
            for elem in elements {
                match elem {
                    ArrayLiteralElement::Single(op) | ArrayLiteralElement::Spread(op) => {
                        validate_operand(op, func, initialized, errors);
                    }
                }
            }
        }
        RValue::NewArrayZeroed { .. } | RValue::LoadGlobal(_) => {}
        RValue::NewArrayZeroedDynamic { length, .. } => {
            validate_operand(length, func, initialized, errors);
        }
    }
}

fn validate_operand(
    op: &Operand,
    func: &MirFunction,
    initialized: &HashSet<LocalId>,
    errors: &mut Vec<ValidationError>,
) {
    if let Operand::Local(local_id) = op {
        if !func.locals.iter().any(|decl| decl.id == *local_id) {
            errors.push(ValidationError {
                message: format!(
                    "Function '{}': Use of undeclared local ID {:?}",
                    func.name, local_id
                ),
            });
        }
        if !initialized.contains(local_id) {
            errors.push(ValidationError {
                message: format!(
                    "Function '{}': Use of uninitialized local ID {:?}",
                    func.name, local_id
                ),
            });
        }
    }
}
