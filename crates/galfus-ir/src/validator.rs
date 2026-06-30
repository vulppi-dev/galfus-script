use crate::LocalId;
use crate::mir::*;
use galfus_frontend::TypeCheckResult;
use std::collections::HashSet;

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
        if let Err(mut func_errors) = validate_function(func, module) {
            errors.append(&mut func_errors);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_function(func: &MirFunction, module: &MirModule) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();
    let mut initialized = HashSet::new();

    // Parameters are initially defined/initialized
    for idx in 0..func.parameter_types.len() {
        initialized.insert(LocalId::new(idx as u32));
    }

    // Traverse function body
    validate_body(&func.body, func, module, &mut initialized, &mut errors);

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_body(
    body: &MirBody,
    func: &MirFunction,
    module: &MirModule,
    initialized: &mut HashSet<LocalId>,
    errors: &mut Vec<ValidationError>,
) {
    match body {
        MirBody::BasicBlock(bb) => {
            // Validate all instructions in basic block
            for inst in &bb.instructions {
                match inst {
                    Instruction::Assign(dest, rvalue) => {
                        // Check if dest local is declared
                        if !func.locals.iter().any(|decl| decl.id == *dest) {
                            errors.push(ValidationError {
                                message: format!(
                                    "Function '{}': Assignment to undeclared local ID {:?}",
                                    func.name, dest
                                ),
                            });
                        }

                        // Validate all operand uses in the RValue
                        validate_rvalue_operands(rvalue, func, initialized, errors);

                        // Mark dest as defined/initialized
                        initialized.insert(*dest);
                    }
                    Instruction::Drop(local) => {
                        // Check if local is declared
                        if !func.locals.iter().any(|decl| decl.id == *local) {
                            errors.push(ValidationError {
                                message: format!(
                                    "Function '{}': Drop of undeclared local ID {:?}",
                                    func.name, local
                                ),
                            });
                        }

                        // Check definition-before-use
                        if !initialized.contains(local) {
                            errors.push(ValidationError {
                                message: format!(
                                    "Function '{}': Drop of uninitialized local ID {:?}",
                                    func.name, local
                                ),
                            });
                        }

                        initialized.remove(local);
                    }
                    Instruction::StoreGlobal(name, operand) => {
                        // Check if global variable is declared in module
                        if !module.globals.iter().any(|g| g.name == *name) {
                            errors.push(ValidationError {
                                message: format!(
                                    "Function '{}': Store to undeclared global variable '{}'",
                                    func.name, name
                                ),
                            });
                        }

                        validate_operand(operand, func, initialized, errors);
                    }
                    Instruction::StoreIndex { arr, idx, val } => {
                        validate_operand(arr, func, initialized, errors);
                        validate_operand(idx, func, initialized, errors);
                        validate_operand(val, func, initialized, errors);
                    }
                }
            }

            // Validate terminator
            match &bb.terminator {
                Terminator::Return(Some(operand)) => {
                    validate_operand(operand, func, initialized, errors);
                }
                Terminator::Return(None) => {}
                Terminator::Break => {}
                Terminator::Continue => {}
                Terminator::Panic(_) => {}
                Terminator::None => {}
                Terminator::Call {
                    func: target_func_id,
                    args,
                    destination,
                } => {
                    for arg in args {
                        validate_operand(arg, func, initialized, errors);
                    }

                    // Check if destination local is declared
                    if !func.locals.iter().any(|decl| decl.id == *destination) {
                        errors.push(ValidationError {
                            message: format!(
                                "Function '{}': Call destination to undeclared local ID {:?}",
                                func.name, destination
                            ),
                        });
                    }

                    // Validate that the target function exists in module
                    if !module.functions.iter().any(|f| f.id == *target_func_id) {
                        errors.push(ValidationError {
                            message: format!(
                                "Function '{}': Call to undeclared function ID {:?}",
                                func.name, target_func_id
                            ),
                        });
                    }

                    initialized.insert(*destination);
                }
            }
        }
        MirBody::Block {
            locals: _,
            statements,
        } => {
            for stmt in statements {
                validate_body(stmt, func, module, initialized, errors);
            }
        }
        MirBody::If {
            cond,
            then_branch,
            else_branch,
        } => {
            validate_operand(cond, func, initialized, errors);

            let mut init_then = initialized.clone();
            validate_body(then_branch, func, module, &mut init_then, errors);

            if let Some(else_br) = else_branch {
                let mut init_else = initialized.clone();
                validate_body(else_br, func, module, &mut init_else, errors);

                // Variables initialized in BOTH branches are initialized afterwards
                *initialized = init_then.intersection(&init_else).copied().collect();
            }
        }
        MirBody::Loop { body } => {
            let mut init_loop = initialized.clone();
            validate_body(body, func, module, &mut init_loop, errors);
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
        // Check if local is declared
        if !func.locals.iter().any(|decl| decl.id == *local_id) {
            errors.push(ValidationError {
                message: format!(
                    "Function '{}': Use of undeclared local ID {:?}",
                    func.name, local_id
                ),
            });
        }

        // Check definition-before-use
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

fn validate_rvalue_operands(
    rvalue: &RValue,
    func: &MirFunction,
    initialized: &HashSet<LocalId>,
    errors: &mut Vec<ValidationError>,
) {
    match rvalue {
        RValue::Use(operand) => {
            validate_operand(operand, func, initialized, errors);
        }
        RValue::UnaryOp(_, operand) => {
            validate_operand(operand, func, initialized, errors);
        }
        RValue::BinaryOp(_, lhs, rhs) => {
            validate_operand(lhs, func, initialized, errors);
            validate_operand(rhs, func, initialized, errors);
        }
        RValue::Cast(operand, _) => {
            validate_operand(operand, func, initialized, errors);
        }
        RValue::NewStruct { fields, .. } => {
            for field in fields {
                validate_operand(field, func, initialized, errors);
            }
        }
        RValue::NewArray(_, elements) => {
            for element in elements {
                validate_operand(element, func, initialized, errors);
            }
        }
        RValue::NewArrayDynamic(_, elements) => {
            for element in elements {
                match element {
                    ArrayLiteralElement::Single(operand) => {
                        validate_operand(operand, func, initialized, errors);
                    }
                    ArrayLiteralElement::Spread(operand) => {
                        validate_operand(operand, func, initialized, errors);
                    }
                }
            }
        }
        RValue::NewTuple(_, elements) => {
            for element in elements {
                validate_operand(element, func, initialized, errors);
            }
        }
        RValue::MemberAccess(operand, _) => {
            validate_operand(operand, func, initialized, errors);
        }
        RValue::ArrayIndex(operand, index) => {
            validate_operand(operand, func, initialized, errors);
            validate_operand(index, func, initialized, errors);
        }
        RValue::Choice(_, _, payload) => {
            if let Some(op) = payload {
                validate_operand(op, func, initialized, errors);
            }
        }
        RValue::Instanceof(operand, _) => {
            validate_operand(operand, func, initialized, errors);
        }
        RValue::LoadGlobal(_) => {}
        RValue::Len(operand) => {
            validate_operand(operand, func, initialized, errors);
        }
        RValue::NewArrayZeroed { .. } => {
            // No operands — size and types are compile-time constants.
        }
    }
}
