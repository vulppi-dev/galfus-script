use super::*;

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
            constraints: vec![],
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
            constraints: vec![],
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
            constraints: vec![],
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
