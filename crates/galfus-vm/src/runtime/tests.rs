use super::*;
use galfus_core::image::instruction::{ConstIdx, FieldIdx};
use galfus_core::image::{
    ChoiceLayout, ChoiceVariantLayout, ConstantPool, FieldLayout, ImageFunction, OwnershipKind,
    StructLayout,
};

fn create_test_image(instructions: Vec<Instruction>, constants: Vec<Constant>) -> ModuleImage {
    ModuleImage {
        name: "test".to_string(),
        constants: ConstantPool { constants },
        functions: vec![ImageFunction {
            name: "main".to_string(),
            param_count: 0,
            local_count: 8,
            temp_count: 8,
            return_ty: TypeIdx(0),
            instructions,
        }],
        types: vec![
            ImageType::Int64,                               // TypeIdx(0)
            ImageType::Bool,                                // TypeIdx(1)
            ImageType::Null,                                // TypeIdx(2)
            ImageType::Struct(StructLayoutIdx(0)),          // TypeIdx(3)
            ImageType::Array(TypeIdx(0)),                   // TypeIdx(4)
            ImageType::Tuple(vec![TypeIdx(0), TypeIdx(1)]), // TypeIdx(5)
            ImageType::Choice(ChoiceLayoutIdx(0)),          // TypeIdx(6)
            ImageType::Uint8,                               // TypeIdx(7)
        ],
        struct_layouts: vec![StructLayout {
            name: "Point".to_string(),
            fields: vec![
                FieldLayout {
                    name: "x".to_string(),
                    ty: TypeIdx(0),
                    offset: 0,
                    ownership: OwnershipKind::Value,
                },
                FieldLayout {
                    name: "y".to_string(),
                    ty: TypeIdx(0),
                    offset: 8,
                    ownership: OwnershipKind::Value,
                },
            ],
        }],
        choice_layouts: vec![ChoiceLayout {
            name: "OptionInt".to_string(),
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
        imports: Vec::new(),
        exports: Vec::new(),
        init_func_idx: None,
    }
}

#[test]
fn test_basic_arithmetic() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0),
        },
        Instruction::LoadConst {
            dest: Reg(2),
            const_idx: ConstIdx(1),
        },
        Instruction::Add {
            dest: Reg(3),
            lhs: Reg(1),
            rhs: Reg(2),
        },
        Instruction::Ret { src: Reg(3) },
    ];
    let image = create_test_image(instrs, vec![Constant::Int(10), Constant::Int(20)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Int64(30));
}

#[test]
fn test_sub_mul_div_rem_pow() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 15
        },
        Instruction::LoadConst {
            dest: Reg(2),
            const_idx: ConstIdx(1), // 4
        },
        Instruction::Sub {
            dest: Reg(3),
            lhs: Reg(1),
            rhs: Reg(2),
        }, // 11
        Instruction::Mul {
            dest: Reg(4),
            lhs: Reg(3),
            rhs: Reg(2),
        }, // 44
        Instruction::Div {
            dest: Reg(5),
            lhs: Reg(4),
            rhs: Reg(2),
        }, // 11
        Instruction::Rem {
            dest: Reg(6),
            lhs: Reg(5),
            rhs: Reg(2),
        }, // 3
        Instruction::Pow {
            dest: Reg(7),
            lhs: Reg(6),
            rhs: Reg(2),
        }, // 3^4 = 81
        Instruction::Ret { src: Reg(7) },
    ];
    let image = create_test_image(instrs, vec![Constant::Int(15), Constant::Int(4)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Int64(81));
}

#[test]
fn test_neg() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 5
        },
        Instruction::Neg {
            dest: Reg(2),
            src: Reg(1),
        }, // -5
        Instruction::Ret { src: Reg(2) },
    ];
    let image = create_test_image(instrs, vec![Constant::Int(5)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Int64(-5));
}

#[test]
fn test_not() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // true
        },
        Instruction::Not {
            dest: Reg(2),
            src: Reg(1),
        }, // false
        Instruction::Ret { src: Reg(2) },
    ];
    let image = create_test_image(instrs, vec![Constant::Bool(true)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Bool(false));
}

#[test]
fn test_bitnot() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 5
        },
        Instruction::BitNot {
            dest: Reg(2),
            src: Reg(1),
        }, // !5
        Instruction::Ret { src: Reg(2) },
    ];
    let image = create_test_image(instrs, vec![Constant::Int(5)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Int64(!5));
}

#[test]
fn test_shl_shr_and_or_xor() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 8
        },
        Instruction::LoadConst {
            dest: Reg(2),
            const_idx: ConstIdx(1), // 2
        },
        Instruction::Shl {
            dest: Reg(3),
            lhs: Reg(1),
            rhs: Reg(2),
        }, // 32
        Instruction::Shr {
            dest: Reg(4),
            lhs: Reg(3),
            rhs: Reg(2),
        }, // 8
        Instruction::And {
            dest: Reg(5),
            lhs: Reg(4),
            rhs: Reg(1),
        }, // 8
        Instruction::Or {
            dest: Reg(6),
            lhs: Reg(5),
            rhs: Reg(2),
        }, // 8 | 2 = 10
        Instruction::Xor {
            dest: Reg(7),
            lhs: Reg(6),
            rhs: Reg(2),
        }, // 10 ^ 2 = 8
        Instruction::Ret { src: Reg(7) },
    ];
    let image = create_test_image(instrs, vec![Constant::Int(8), Constant::Int(2)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Int64(8));
}

#[test]
fn test_comparison_lt() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 100
        },
        Instruction::LoadConst {
            dest: Reg(2),
            const_idx: ConstIdx(1), // 200
        },
        Instruction::Lt {
            dest: Reg(3),
            lhs: Reg(1),
            rhs: Reg(2),
        }, // true
        Instruction::Ret { src: Reg(3) },
    ];
    let image = create_test_image(instrs, vec![Constant::Int(100), Constant::Int(200)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Bool(true));
}

#[test]
fn test_comparison_le() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 100
        },
        Instruction::LoadConst {
            dest: Reg(2),
            const_idx: ConstIdx(1), // 200
        },
        Instruction::Le {
            dest: Reg(3),
            lhs: Reg(2),
            rhs: Reg(1),
        }, // false
        Instruction::Ret { src: Reg(3) },
    ];
    let image = create_test_image(instrs, vec![Constant::Int(100), Constant::Int(200)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Bool(false));
}

#[test]
fn test_fallback() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 100
        },
        Instruction::LoadNull { dest: Reg(2) },
        Instruction::Fallback {
            dest: Reg(3),
            src: Reg(2),
            fallback: Reg(1),
        }, // 100
        Instruction::Ret { src: Reg(3) },
    ];
    let image = create_test_image(instrs, vec![Constant::Int(100)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Int64(100));
}

#[test]
fn test_control_flow_jumps() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // false
        },
        Instruction::JumpFalse {
            cond: Reg(1),
            offset: 3,
        },
        Instruction::LoadConst {
            dest: Reg(2),
            const_idx: ConstIdx(1), // 999
        },
        Instruction::Ret { src: Reg(2) },
        // Target of jump
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(2), // 888
        },
        Instruction::Ret { src: Reg(3) },
    ];
    let image = create_test_image(
        instrs,
        vec![
            Constant::Bool(false),
            Constant::Int(999),
            Constant::Int(888),
        ],
    );
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Int64(888));
}

#[test]
fn test_structs_load_store() {
    let instrs = vec![
        Instruction::AllocLocal {
            dest: Reg(1),
            type_idx: TypeIdx(3), // Struct Point
        },
        Instruction::LoadConst {
            dest: Reg(2),
            const_idx: ConstIdx(0), // 42
        },
        Instruction::StoreField {
            obj: Reg(1),
            field: FieldIdx(0), // field x
            val: Reg(2),
        },
        Instruction::LoadField {
            dest: Reg(3),
            obj: Reg(1),
            field: FieldIdx(0),
        },
        Instruction::Ret { src: Reg(3) },
    ];
    let image = create_test_image(instrs, vec![Constant::Int(42)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Int64(42));
}

#[test]
fn test_arrays_load_store() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 5
        },
        Instruction::NewArray {
            dest: Reg(2),
            type_idx: TypeIdx(4), // Array of Int64
            len_reg: Reg(1),
        },
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(1), // index 2
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(2), // value 99
        },
        Instruction::StoreIndex {
            arr: Reg(2),
            idx: Reg(3),
            val: Reg(4),
        },
        Instruction::LoadIndex {
            dest: Reg(5),
            arr: Reg(2),
            idx: Reg(3),
        },
        Instruction::Ret { src: Reg(5) },
    ];
    let image = create_test_image(
        instrs,
        vec![Constant::Int(5), Constant::Int(2), Constant::Int(99)],
    );
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Int64(99));
}

#[test]
fn test_tuples() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 10
        },
        Instruction::LoadConst {
            dest: Reg(2),
            const_idx: ConstIdx(1), // true
        },
        Instruction::NewTuple {
            dest: Reg(3),
            type_idx: TypeIdx(5),
            start: Reg(1),
            count: 2,
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(2), // index 1
        },
        Instruction::LoadIndex {
            dest: Reg(5),
            arr: Reg(3),
            idx: Reg(4),
        },
        Instruction::Ret { src: Reg(5) },
    ];
    let image = create_test_image(
        instrs,
        vec![Constant::Int(10), Constant::Bool(true), Constant::Int(1)],
    );
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Bool(true));
}

#[test]
fn test_choices() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 100
        },
        Instruction::NewChoice {
            dest: Reg(2),
            type_idx: TypeIdx(6),
            variant_idx: 1, // Some
            payload: Reg(1),
        },
        Instruction::Ret { src: Reg(2) },
    ];
    let image = create_test_image(instrs, vec![Constant::Int(100)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    if let Value::Object(obj_ref) = res {
        let heap_obj = vm.get_object(obj_ref).unwrap();
        if let HeapObject::Choice {
            variant_idx,
            payload,
            ..
        } = heap_obj
        {
            assert_eq!(*variant_idx, 1);
            assert_eq!(*payload, Value::Int64(100));
        } else {
            panic!("Expected Choice");
        }
    } else {
        panic!("Expected ObjectRef");
    }
}

#[test]
fn test_cast() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 42 (Int64)
        },
        Instruction::Cast {
            dest: Reg(2),
            src: Reg(1),
            type_idx: TypeIdx(7), // Uint8
        }, // cast 42 (Int64) to 42 (Uint8)
        Instruction::Ret { src: Reg(2) },
    ];
    let image = create_test_image(instrs, vec![Constant::Int(42)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Uint8(42));
}

#[test]
fn test_instanceof() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 42 (Int64)
        },
        Instruction::Instanceof {
            dest: Reg(2),
            src: Reg(1),
            type_idx: TypeIdx(0), // Int64
        }, // true
        Instruction::Ret { src: Reg(2) },
    ];
    let image = create_test_image(instrs, vec![Constant::Int(42)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Bool(true));
}

#[test]
fn test_division_by_zero_panic() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 10
        },
        Instruction::LoadConst {
            dest: Reg(2),
            const_idx: ConstIdx(1), // 0
        },
        Instruction::Div {
            dest: Reg(3),
            lhs: Reg(1),
            rhs: Reg(2),
        },
        Instruction::Ret { src: Reg(3) },
    ];
    let image = create_test_image(instrs, vec![Constant::Int(10), Constant::Int(0)]);
    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]);
    assert!(res.is_err());
    let panic_err = res.unwrap_err();
    assert_eq!(panic_err.error, VmError::DivisionByZero);
}

#[test]
fn test_unwinding_call_stack() {
    let instrs_main = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 5
        },
        Instruction::Call {
            dest: Reg(2),
            func: FuncIdx(1),
            args_start: Reg(1),
            arg_count: 1,
        },
        Instruction::Ret { src: Reg(2) },
    ];
    let instrs_helper = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(1), // 0
        },
        Instruction::Div {
            dest: Reg(2),
            lhs: Reg(0), // param 0 (value 5)
            rhs: Reg(1), // 0
        },
        Instruction::Ret { src: Reg(2) },
    ];

    let image = ModuleImage {
        name: "test".to_string(),
        constants: ConstantPool {
            constants: vec![Constant::Int(5), Constant::Int(0)],
        },
        functions: vec![
            ImageFunction {
                name: "main".to_string(),
                param_count: 0,
                local_count: 4,
                temp_count: 4,
                return_ty: TypeIdx(0),
                instructions: instrs_main,
            },
            ImageFunction {
                name: "helper".to_string(),
                param_count: 1,
                local_count: 4,
                temp_count: 4,
                return_ty: TypeIdx(0),
                instructions: instrs_helper,
            },
        ],
        types: vec![ImageType::Int64],
        struct_layouts: vec![],
        choice_layouts: vec![],
        imports: vec![],
        exports: vec![],
        init_func_idx: None,
    };

    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]);
    assert!(res.is_err());
    let panic_err = res.unwrap_err();
    assert_eq!(panic_err.error, VmError::DivisionByZero);
    assert_eq!(panic_err.stack_trace.len(), 2);
    assert_eq!(panic_err.stack_trace[0].function_name, "helper");
    assert_eq!(panic_err.stack_trace[1].function_name, "main");
}

#[test]
fn test_ownership_deterministic_release() {
    let image = ModuleImage {
        name: "test".to_string(),
        constants: ConstantPool { constants: vec![] },
        functions: vec![ImageFunction {
            name: "main".to_string(),
            param_count: 0,
            local_count: 8,
            temp_count: 8,
            return_ty: TypeIdx(3),
            instructions: vec![
                Instruction::AllocLocal {
                    dest: Reg(1),
                    type_idx: TypeIdx(3),
                },
                Instruction::AllocLocal {
                    dest: Reg(2),
                    type_idx: TypeIdx(3),
                },
                Instruction::StoreField {
                    obj: Reg(1),
                    field: FieldIdx(0),
                    val: Reg(2),
                },
                Instruction::Drop { reg: Reg(2) },
                Instruction::Ret { src: Reg(1) },
            ],
        }],
        types: vec![
            ImageType::Int64,                      // TypeIdx(0)
            ImageType::Null,                       // TypeIdx(1)
            ImageType::Null,                       // TypeIdx(2)
            ImageType::Struct(StructLayoutIdx(0)), // TypeIdx(3)
        ],
        struct_layouts: vec![StructLayout {
            name: "Node".to_string(),
            fields: vec![
                FieldLayout {
                    name: "next".to_string(),
                    ty: TypeIdx(3),
                    offset: 0,
                    ownership: OwnershipKind::Strong,
                },
                FieldLayout {
                    name: "val".to_string(),
                    ty: TypeIdx(0),
                    offset: 8,
                    ownership: OwnershipKind::Value,
                },
            ],
        }],
        choice_layouts: vec![],
        imports: vec![],
        exports: vec![],
        init_func_idx: None,
    };

    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    let node1_ref = match res {
        Value::Object(r) => r,
        other => panic!("expected object, got {:?}", other),
    };

    let node1 = vm.get_object(node1_ref).unwrap();
    let node2_ref = match node1 {
        HeapObject::Struct { fields, .. } => match fields[0] {
            Value::Object(r) => r,
            ref other => panic!("expected object in field 0, got {:?}", other),
        },
        other => panic!("expected struct, got {:?}", other),
    };

    assert!(vm.heap[node1_ref.raw()].is_some());
    assert!(vm.heap[node2_ref.raw()].is_some());

    vm.release_unreachable();
    assert!(vm.heap[node1_ref.raw()].is_none());
    assert!(vm.heap[node2_ref.raw()].is_none());
}

#[test]
fn test_ownership_cycle_release() {
    let image = ModuleImage {
        name: "test".to_string(),
        constants: ConstantPool { constants: vec![] },
        functions: vec![ImageFunction {
            name: "main".to_string(),
            param_count: 0,
            local_count: 8,
            temp_count: 8,
            return_ty: TypeIdx(4),
            instructions: vec![
                Instruction::AllocLocal {
                    dest: Reg(1),
                    type_idx: TypeIdx(3),
                },
                Instruction::AllocLocal {
                    dest: Reg(2),
                    type_idx: TypeIdx(3),
                },
                Instruction::StoreField {
                    obj: Reg(1),
                    field: FieldIdx(0),
                    val: Reg(2),
                },
                Instruction::StoreField {
                    obj: Reg(2),
                    field: FieldIdx(0),
                    val: Reg(1),
                },
                Instruction::NewTuple {
                    dest: Reg(3),
                    type_idx: TypeIdx(4),
                    start: Reg(1),
                    count: 2,
                },
                Instruction::Ret { src: Reg(3) },
            ],
        }],
        types: vec![
            ImageType::Int64,                               // TypeIdx(0)
            ImageType::Null,                                // TypeIdx(1)
            ImageType::Null,                                // TypeIdx(2)
            ImageType::Struct(StructLayoutIdx(0)),          // TypeIdx(3)
            ImageType::Tuple(vec![TypeIdx(3), TypeIdx(3)]), // TypeIdx(4)
        ],
        struct_layouts: vec![StructLayout {
            name: "Node".to_string(),
            fields: vec![FieldLayout {
                name: "next".to_string(),
                ty: TypeIdx(3),
                offset: 0,
                ownership: OwnershipKind::Strong,
            }],
        }],
        choice_layouts: vec![],
        imports: vec![],
        exports: vec![],
        init_func_idx: None,
    };

    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    let tuple_ref = match res {
        Value::Object(r) => r,
        other => panic!("expected object, got {:?}", other),
    };

    let (node1_ref, node2_ref) = match vm.get_object(tuple_ref).unwrap() {
        HeapObject::Tuple { elements } => {
            let n1 = match &elements[0] {
                Value::Object(r) => *r,
                other => panic!("expected object, got {:?}", other),
            };
            let n2 = match &elements[1] {
                Value::Object(r) => *r,
                other => panic!("expected object, got {:?}", other),
            };
            (n1, n2)
        }
        other => panic!("expected tuple, got {:?}", other),
    };

    assert!(vm.heap[node1_ref.raw()].is_some());
    assert!(vm.heap[node2_ref.raw()].is_some());
    assert!(vm.heap[tuple_ref.raw()].is_some());

    vm.release_unreachable();
    assert!(vm.heap[node1_ref.raw()].is_none());
    assert!(vm.heap[node2_ref.raw()].is_none());
    assert!(vm.heap[tuple_ref.raw()].is_none());
}

#[test]
fn test_ownership_weak_invalidation() {
    let image = ModuleImage {
        name: "test".to_string(),
        constants: ConstantPool { constants: vec![] },
        functions: vec![ImageFunction {
            name: "main".to_string(),
            param_count: 0,
            local_count: 8,
            temp_count: 8,
            return_ty: TypeIdx(3),
            instructions: vec![
                Instruction::AllocLocal {
                    dest: Reg(1),
                    type_idx: TypeIdx(3),
                },
                Instruction::AllocLocal {
                    dest: Reg(2),
                    type_idx: TypeIdx(3),
                },
                Instruction::StoreField {
                    obj: Reg(1),
                    field: FieldIdx(0),
                    val: Reg(2),
                },
                Instruction::StoreField {
                    obj: Reg(2),
                    field: FieldIdx(1),
                    val: Reg(1),
                },
                Instruction::Drop { reg: Reg(1) },
                Instruction::Ret { src: Reg(2) },
            ],
        }],
        types: vec![
            ImageType::Int64,                      // TypeIdx(0)
            ImageType::Null,                       // TypeIdx(1)
            ImageType::Null,                       // TypeIdx(2)
            ImageType::Struct(StructLayoutIdx(0)), // TypeIdx(3)
        ],
        struct_layouts: vec![StructLayout {
            name: "Node".to_string(),
            fields: vec![
                FieldLayout {
                    name: "next".to_string(),
                    ty: TypeIdx(3),
                    offset: 0,
                    ownership: OwnershipKind::Strong,
                },
                FieldLayout {
                    name: "parent".to_string(),
                    ty: TypeIdx(3),
                    offset: 8,
                    ownership: OwnershipKind::Weak,
                },
            ],
        }],
        choice_layouts: vec![],
        imports: vec![],
        exports: vec![],
        init_func_idx: None,
    };

    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    let node2_ref = match res {
        Value::Object(r) => r,
        other => panic!("expected object, got {:?}", other),
    };

    assert!(vm.heap[node2_ref.raw()].is_some());

    let node2 = vm.get_object(node2_ref).unwrap();
    match node2 {
        HeapObject::Struct { fields, .. } => {
            assert_eq!(fields[1], Value::Null);
        }
        other => panic!("expected struct, got {:?}", other),
    }
}

struct BufferIoHandler {
    buffer: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
}

impl IoHandler for BufferIoHandler {
    fn write(&mut self, data: &[u8]) -> Result<(), VmError> {
        let mut buf = self.buffer.lock().unwrap();
        buf.extend_from_slice(data);
        Ok(())
    }

    fn read(&mut self) -> Result<Option<u8>, VmError> {
        Ok(None)
    }
}

#[test]
fn test_io_handler_write() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0),
        },
        Instruction::Write { src: Reg(1) },
        Instruction::RetNull,
    ];
    let image = create_test_image(instrs, vec![Constant::Int(42)]);
    let buffer = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let io_handler = BufferIoHandler {
        buffer: buffer.clone(),
    };
    let mut vm = VirtualMachine::new(image).with_io_handler(Box::new(io_handler));
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    assert_eq!(res, Value::Null);
    let output = buffer.lock().unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), "42");
}

#[test]
fn test_len_and_copy_array() {
    let instrs = vec![
        // Create source array [1, 2, 3] of type idx 1 (Array of Int64)
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 3
        },
        Instruction::NewArray {
            dest: Reg(2), // src_arr
            type_idx: TypeIdx(1),
            len_reg: Reg(1),
        },
        // Populate elements
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(1), // 10
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(2), // 0 (idx)
        },
        Instruction::StoreIndex {
            arr: Reg(2),
            idx: Reg(4),
            val: Reg(3),
        },
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(3), // 20
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(4), // 1 (idx)
        },
        Instruction::StoreIndex {
            arr: Reg(2),
            idx: Reg(4),
            val: Reg(3),
        },
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(5), // 30
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(6), // 2 (idx)
        },
        Instruction::StoreIndex {
            arr: Reg(2),
            idx: Reg(4),
            val: Reg(3),
        },
        // Query length of src_arr
        Instruction::Len {
            dest: Reg(5),
            src: Reg(2),
        },
        // Allocate dest_arr of size 5
        Instruction::LoadConst {
            dest: Reg(6),
            const_idx: ConstIdx(7), // 5
        },
        Instruction::NewArray {
            dest: Reg(7), // dest_arr
            type_idx: TypeIdx(1),
            len_reg: Reg(6),
        },
        // CopyArray from src_arr to dest_arr starting at index 1
        Instruction::LoadConst {
            dest: Reg(8),
            const_idx: ConstIdx(4), // 1 (dest_start)
        },
        Instruction::CopyArray {
            dest: Reg(7),
            dest_start: Reg(8),
            src: Reg(2),
        },
        Instruction::Ret { src: Reg(7) },
    ];

    let image = ModuleImage {
        name: "test".to_string(),
        constants: ConstantPool {
            constants: vec![
                Constant::Int(3),  // 0
                Constant::Int(10), // 1
                Constant::Int(0),  // 2
                Constant::Int(20), // 3
                Constant::Int(1),  // 4
                Constant::Int(30), // 5
                Constant::Int(2),  // 6
                Constant::Int(5),  // 7
            ],
        },
        functions: vec![ImageFunction {
            name: "main".to_string(),
            param_count: 0,
            local_count: 10,
            temp_count: 10,
            return_ty: TypeIdx(1),
            instructions: instrs,
        }],
        types: vec![ImageType::Int64, ImageType::Array(TypeIdx(0))],
        struct_layouts: vec![],
        choice_layouts: vec![],
        imports: vec![],
        exports: vec![],
        init_func_idx: None,
    };

    let mut vm = VirtualMachine::new(image);
    let res = vm.run_function(FuncIdx(0), vec![]).unwrap();
    let arr_ref = match res {
        Value::Object(r) => r,
        other => panic!("expected object, got {:?}", other),
    };
    let arr_obj = vm.get_object(arr_ref).unwrap();
    match arr_obj {
        HeapObject::Array { elements, .. } => {
            assert_eq!(elements.len(), 5);
            assert_eq!(elements[0], Value::Null);
            assert_eq!(elements[1], Value::Int64(10));
            assert_eq!(elements[2], Value::Int64(20));
            assert_eq!(elements[3], Value::Int64(30));
            assert_eq!(elements[4], Value::Null);
        }
        other => panic!("expected array, got {:?}", other),
    }
}
