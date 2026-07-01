use super::*;

fn create_dummy_image(instructions: Vec<Instruction>) -> ModuleImage {
    ModuleImage {
        name: "test".to_string(),
        constants: ConstantPool {
            constants: vec![Constant::Int(42)],
        },
        functions: vec![ImageFunction {
            name: "main".to_string(),
            param_count: 1,
            local_count: 2,
            temp_count: 2,
            return_ty: TypeIdx(0),
            instructions,
        }],
        types: vec![
            ImageType::Int64,
            ImageType::Struct(StructLayoutIdx(0)),
            ImageType::Tuple(vec![TypeIdx(0), TypeIdx(0)]),
            ImageType::Choice(ChoiceLayoutIdx(0)),
        ],
        struct_layouts: vec![StructLayout {
            name: "Point".to_string(),
            fields: vec![FieldLayout {
                name: "x".to_string(),
                ty: TypeIdx(0),
                offset: 0,
                ownership: OwnershipKind::Value,
            }],
            constraints: vec![],
        }],
        choice_layouts: vec![ChoiceLayout {
            name: "Option".to_string(),
            variants: vec![
                ChoiceVariantLayout {
                    name: "None".to_string(),
                    payload_ty: None,
                },
                ChoiceVariantLayout {
                    name: "Some".to_string(),
                    payload_ty: Some(TypeIdx(0)),
                },
            ],
        }],
        imports: vec![],
        exports: vec![],
        init_func_idx: None,
    }
}

#[test]
fn test_valid_image() {
    let image = create_dummy_image(vec![
        Instruction::Move {
            dest: Reg(1),
            src: Reg(0),
        },
        Instruction::Ret { src: Reg(1) },
    ]);
    assert!(validate_module_image(&image).is_ok());
}

#[test]
fn test_invalid_register() {
    let image = create_dummy_image(vec![Instruction::Move {
        dest: Reg(10),
        src: Reg(0),
    }]);
    let res = validate_module_image(&image);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert!(matches!(
        errs[0],
        ImageValidationError::InvalidRegister { ref reg, .. } if reg.raw() == 10
    ));
}

#[test]
fn test_invalid_constant_index() {
    let image = create_dummy_image(vec![Instruction::LoadConst {
        dest: Reg(1),
        const_idx: ConstIdx(5),
    }]);
    let res = validate_module_image(&image);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert!(matches!(
        errs[0],
        ImageValidationError::InvalidConstantIndex {
            index: ConstIdx(5),
            ..
        }
    ));
}

#[test]
fn test_invalid_jump_offset() {
    let image = create_dummy_image(vec![Instruction::Jump { offset: 10 }]);
    let res = validate_module_image(&image);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert!(matches!(
        errs[0],
        ImageValidationError::InvalidJumpOffset { target_idx: 11, .. }
    ));
}

#[test]
fn test_type_mismatch_alloc() {
    // TypeIdx(0) is Int64, not Struct
    let image = create_dummy_image(vec![Instruction::AllocLocal {
        dest: Reg(1),
        type_idx: TypeIdx(0),
    }]);
    let res = validate_module_image(&image);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert!(matches!(
        errs[0],
        ImageValidationError::TypeMismatchAlloc {
            found: TypeIdx(0),
            expected: "Struct",
            ..
        }
    ));
}

#[test]
fn test_tuple_count_mismatch() {
    // TypeIdx(2) is Tuple with 2 elements, but count: 3 is requested
    let image = create_dummy_image(vec![Instruction::NewTuple {
        dest: Reg(1),
        type_idx: TypeIdx(2),
        start: Reg(0),
        count: 3,
    }]);
    let res = validate_module_image(&image);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert!(matches!(
        errs[0],
        ImageValidationError::TupleCountMismatch {
            expected_count: 2,
            found_count: 3,
            ..
        }
    ));
}

#[test]
fn test_invalid_function_index() {
    let image = create_dummy_image(vec![Instruction::Call {
        dest: Reg(1),
        func: FuncIdx(10),
        args_start: Reg(0),
        arg_count: 0,
    }]);
    let res = validate_module_image(&image);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert!(matches!(
        errs[0],
        ImageValidationError::InvalidFunctionIndex {
            index: FuncIdx(10),
            ..
        }
    ));
}

#[test]
fn test_choice_variant_out_of_bounds() {
    // TypeIdx(3) is Choice Option, which only has variants 0 and 1
    let image = create_dummy_image(vec![Instruction::NewChoice {
        dest: Reg(1),
        type_idx: TypeIdx(3),
        variant_idx: 5,
        payload: Reg(0),
    }]);
    let res = validate_module_image(&image);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert!(matches!(
        errs[0],
        ImageValidationError::ChoiceVariantOutOfBounds {
            variant_idx: 5,
            variant_count: 2,
            ..
        }
    ));
}

#[test]
fn test_field_out_of_bounds() {
    // Point struct layout only has field 0
    let image = create_dummy_image(vec![Instruction::LoadField {
        dest: Reg(1),
        obj: Reg(0),
        field: FieldIdx(2),
    }]);
    let res = validate_module_image(&image);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert!(matches!(
        errs[0],
        ImageValidationError::FieldOutOfBounds {
            field_idx: FieldIdx(2),
            ..
        }
    ));
}

#[test]
fn test_export_function_out_of_bounds() {
    let mut image = create_dummy_image(vec![Instruction::Ret { src: Reg(0) }]);
    image.exports.push(ExportSlot {
        symbol_name: "compute".to_string(),
        func_idx: FuncIdx(10),
    });
    let res = validate_module_image(&image);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert!(matches!(
        errs[0],
        ImageValidationError::ExportFunctionOutOfBounds {
            func_idx: FuncIdx(10),
            ..
        }
    ));
}

#[test]
fn test_init_function_out_of_bounds() {
    let mut image = create_dummy_image(vec![Instruction::Ret { src: Reg(0) }]);
    image.init_func_idx = Some(FuncIdx(10));
    let res = validate_module_image(&image);
    assert!(res.is_err());
    let errs = res.unwrap_err();
    assert!(matches!(
        errs[0],
        ImageValidationError::InitFunctionOutOfBounds {
            func_idx: FuncIdx(10),
            ..
        }
    ));
}
