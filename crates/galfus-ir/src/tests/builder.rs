use super::*;
use galfus_core::FunctionId;

#[test]
fn test_mir_builder_basic() {
    let source_id = SourceId::new(0);
    let code = r#"
        fn add(a: i32, b: i32): i32 {
            return a + b
        }
    "#;
    let source = SourceFile::new(source_id, "test.gfs".to_string(), code.to_string());

    let parse_result = parse(&source);
    let resolve_result = resolve(&source, parse_result.into_graph());
    let graph = resolve_result.into_graph();

    assert!(!graph.has_errors(), "Parse or resolve errors occurred");

    // Typecheck
    let type_result = check_declaration_types(&source, &graph);
    assert!(!type_result.has_errors(), "Typecheck errors occurred");

    // Build MIR
    let builder = builder::MirBuilder::new(&graph, &type_result, code);
    let mir_module = builder.build();

    assert_eq!(mir_module.functions.len(), 1);
    let func = &mir_module.functions[0];
    assert_eq!(func.name, "add");
    assert_eq!(func.parameter_types.len(), 2);

    assert!(!func.blocks.is_empty());
    let bb = func
        .blocks
        .iter()
        .find(|b| matches!(b.terminator, Terminator::Return(_)))
        .unwrap();
    assert!(!bb.instructions.is_empty());

    // Find the Add instruction in some block
    let assign_inst = func
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .find(|inst| {
            matches!(
                inst,
                Instruction::Assign(_, RValue::BinaryOp(MirBinaryOp::Add, _, _))
            )
        })
        .unwrap();

    match assign_inst {
        Instruction::Assign(
            _dest,
            RValue::BinaryOp(MirBinaryOp::Add, Operand::Local(_lhs), Operand::Local(_rhs)),
        ) => {}
        other => panic!("Unexpected instruction: {:?}", other),
    }
    match &bb.terminator {
        Terminator::Return(Some(Operand::Local(_ret_local))) => {}
        other => panic!("Unexpected terminator: {:?}", other),
    }
}

#[test]
fn test_mir_serialization() {
    let source_id = SourceId::new(0);
    let code = r#"
        fn add(a: i32, b: i32): i32 {
            return a + b
        }
    "#;
    let source = SourceFile::new(source_id, "test.gfs".to_string(), code.to_string());

    let parse_result = parse(&source);
    let resolve_result = resolve(&source, parse_result.into_graph());
    let graph = resolve_result.into_graph();
    let type_result = check_declaration_types(&source, &graph);

    let builder = builder::MirBuilder::new(&graph, &type_result, code);
    let mir_module = builder.build();

    // Serialize using postcard
    let serialized = postcard::to_allocvec(&mir_module).expect("Serialization failed");

    // Deserialize using postcard
    let deserialized: MirModule =
        postcard::from_bytes(&serialized).expect("Deserialization failed");

    assert_eq!(deserialized.functions.len(), 1);
    assert_eq!(deserialized.functions[0].name, "add");
    assert_eq!(deserialized.functions[0].parameter_types.len(), 2);
}

#[test]
fn test_mir_builder_lowers_copy_expression() {
    let source_id = SourceId::new(0);
    let code = r#"
        struct User {
            id: i32,
        }

        fn clone_user(user: User): User {
            return copy user
        }
    "#;
    let source = SourceFile::new(source_id, "test.gfs".to_string(), code.to_string());

    let parse_result = parse(&source);
    let resolve_result = resolve(&source, parse_result.into_graph());
    let graph = resolve_result.into_graph();

    assert!(!graph.has_errors(), "Parse or resolve errors occurred");

    let type_result = check_declaration_types(&source, &graph);
    assert!(
        !type_result.has_errors(),
        "Typecheck errors occurred: {:?}",
        type_result.diagnostics()
    );

    let builder = builder::MirBuilder::new(&graph, &type_result, code);
    let mir_module = builder.build();
    let func = mir_module
        .functions
        .iter()
        .find(|function| function.name == "clone_user")
        .expect("clone_user function should be lowered");

    assert!(func.blocks.iter().any(|block| {
        block
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, Instruction::Assign(_, RValue::Copy(_))))
    }));
}

#[test]
fn test_mir_builder_lowers_concrete_typeof_branch() {
    let source_id = SourceId::new(0);
    let code = r#"
        fn label(): [u8] {
            var dummy: i32 = 0
            return instanceof dummy {
                i32 v => "number",
                _ => "other",
            }
        }
    "#;
    let source = SourceFile::new(source_id, "test.gfs".to_string(), code.to_string());

    let parse_result = parse(&source);
    let resolve_result = resolve(&source, parse_result.into_graph());
    let graph = resolve_result.into_graph();

    assert!(!graph.has_errors(), "Parse or resolve errors occurred");

    let type_result = check_declaration_types(&source, &graph);
    assert!(!type_result.has_errors(), "Typecheck errors occurred");

    let builder = builder::MirBuilder::new(&graph, &type_result, code);
    let mir_module = builder.build();

    let func = &mir_module.functions[0];

    assert!(has_string_assignment(func, "number"));
}

#[test]
fn test_mir_builder_specializes_generic_typeof_call() {
    let source_id = SourceId::new(0);
    let code = r#"
        fn label<T: i32 | bool>(dummy: T): [u8] {
            return instanceof dummy {
                i32 v => "number",
                bool v => "flag",
            }
        }

        fn main(): [u8] {
            return label<i32>(0)
        }
    "#;
    let source = SourceFile::new(source_id, "test.gfs".to_string(), code.to_string());

    let parse_result = parse(&source);
    let resolve_result = resolve(&source, parse_result.into_graph());
    let graph = resolve_result.into_graph();

    assert!(!graph.has_errors(), "Parse or resolve errors occurred");

    let type_result = check_declaration_types(&source, &graph);
    assert!(
        !type_result.has_errors(),
        "Typecheck errors occurred: {:?}",
        type_result.diagnostics()
    );

    let builder = builder::MirBuilder::new(&graph, &type_result, code);
    let mir_module = builder.build();

    let main = mir_module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main function should be lowered");
    let call_id = first_call_function_id(main).expect("main should call label");

    let specialized = mir_module
        .functions
        .iter()
        .find(|function| function.id == call_id)
        .expect("specialized function should be emitted");

    assert!(specialized.name.starts_with("label#"));

    assert!(has_string_assignment(specialized, "number"));
}

#[test]
#[ignore = "requires monomorphized typeof lowering"]
fn test_mir_builder_specializes_typeof_generic_parameter() {
    let source_id = SourceId::new(0);
    let code = r#"
        fn label<T: i32 | bool>(): [u8] {
            return typeof T {
                i32 => "number",
                bool => "flag",
            }
        }

        fn main(): [u8] {
            return label<i32>()
        }
    "#;
    let source = SourceFile::new(source_id, "test.gfs".to_string(), code.to_string());

    let parse_result = parse(&source);
    let resolve_result = resolve(&source, parse_result.into_graph());
    let graph = resolve_result.into_graph();
    assert!(!graph.has_errors(), "Parse or resolve errors occurred");

    let type_result = check_declaration_types(&source, &graph);
    assert!(
        !type_result.has_errors(),
        "Typecheck errors occurred: {:?}",
        type_result.diagnostics()
    );

    let builder = builder::MirBuilder::new(&graph, &type_result, code);
    let mir_module = builder.build();
    let main = mir_module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main function should be lowered");
    let call_id = first_call_function_id(main).expect("main should call label");
    let specialized = mir_module
        .functions
        .iter()
        .find(|function| function.id == call_id)
        .expect("specialized function should be emitted");

    assert!(has_string_assignment(specialized, "number"));
    assert!(!has_string_assignment(specialized, "flag"));
}

fn first_call_function_id(func: &MirFunction) -> Option<FunctionId> {
    for block in &func.blocks {
        for inst in &block.instructions {
            if let Instruction::Call { func, .. } = inst {
                return Some(*func);
            }
        }
    }
    None
}

fn has_string_assignment(func: &MirFunction, expected_value: &str) -> bool {
    func.blocks.iter().any(|block| {
        block.instructions.iter().any(|inst| {
            if let Instruction::Assign(_, RValue::Use(Operand::Constant(Constant::String(val)))) =
                inst
            {
                val == expected_value
            } else {
                false
            }
        })
    })
}

#[test]
fn test_mir_builder_phase1() {
    let source_id = SourceId::new(0);
    let code = r#"
        fn complex_expr(x: i32): i32 {
            var a = 42
            const b = 3.14
            var s = "hello"
            var bl = true
            var n = null
            a = x + 10
            var c = <i32>b
            return a
        }
    "#;
    let source = SourceFile::new(source_id, "test.gfs".to_string(), code.to_string());

    let parse_result = parse(&source);
    let resolve_result = resolve(&source, parse_result.into_graph());
    let graph = resolve_result.into_graph();
    assert!(!graph.has_errors(), "Parse or resolve errors occurred");

    let type_result = check_declaration_types(&source, &graph);
    assert!(!type_result.has_errors(), "Typecheck errors occurred");

    let builder = builder::MirBuilder::new(&graph, &type_result, code);
    let mir_module = builder.build();

    assert_eq!(mir_module.functions.len(), 1);
    let func = &mir_module.functions[0];
    assert_eq!(func.name, "complex_expr");
    assert_eq!(func.parameter_types.len(), 1);

    // We expect several locals:
    // 0: parameter `x`
    // 1: `a`
    // 2: `b`
    // 3: `s`
    // 4: `bl`
    // 5: `n`
    // 6: temporary for `x + 10`
    // 7: `c`
    // Let's verify instructions in the body
    let return_block = func
        .blocks
        .iter()
        .find(|b| matches!(b.terminator, Terminator::Return(_)))
        .unwrap();
    match &return_block.terminator {
        Terminator::Return(Some(Operand::Local(_local_id))) => {
            // It returns `a` (which is local 1)
        }
        other => panic!("Unexpected terminator: {:?}", other),
    }
}

#[test]
fn test_mir_builder_phase2() {
    let source_id = SourceId::new(0);
    let code = r#"
        fn other_func(x: i32): i32 {
            return x * 2
        }

        fn control_flow(cond: bool, val: i32): i32 {
            var res = 0;
            if cond {
                res = other_func(val);
            } else {
                res = 100;
            }

            var i = 0;
            loop {
                if i >= 10 {
                    break;
                }
                i = i + 1;
                if i == 5 {
                    continue;
                }
                if i == 8 {
                    break;
                }
            }

            loop {
                break;
            }

            return res
        }
    "#;
    let source = SourceFile::new(source_id, "test.gfs".to_string(), code.to_string());

    let parse_result = parse(&source);
    let resolve_result = resolve(&source, parse_result.into_graph());
    let graph = resolve_result.into_graph();
    assert!(
        !graph.has_errors(),
        "Parse or resolve errors occurred: {:?}",
        graph.diagnostics()
    );

    let type_result = check_declaration_types(&source, &graph);
    assert!(
        !type_result.has_errors(),
        "Typecheck errors occurred: {:?}",
        type_result.diagnostics()
    );

    let builder = builder::MirBuilder::new(&graph, &type_result, code);
    let mir_module = builder.build();

    assert_eq!(mir_module.functions.len(), 2);
    let func = &mir_module.functions[1];
    assert_eq!(func.name, "control_flow");

    assert!(
        func.blocks.len() > 1,
        "Control flow should generate multiple blocks"
    );
    let has_return = func
        .blocks
        .iter()
        .any(|b| matches!(b.terminator, Terminator::Return(_)));
    assert!(has_return, "Return statement not found in MIR");
}

#[test]
fn test_mir_builder_phase3() {
    let source_id = SourceId::new(0);
    let code = r#"
        struct Point {
            x: i32,
            y: i32 = 42,
        }

        struct Point3D {
            ...Point,
            z: i32,
        }

        choice Shape {
            Circle(i32),
            Rect(i32, i32),
            Point,
        }

        fn test_structs_arrays_tuples(p: Point): i32 {
            var p1 = new(Point) { x: 10, y: 20 };
            var p2 = new(Point) { x: 5 };
            var p3 = new(Point3D) { ...p1, z: 100 };

            var arr1 = [1, 2, 3];
            var arr2 = [...arr1, 4, 5];

            var t1 = (10, 20, 30);

            var x_val = p1.x;
            var arr_val = arr2[1];

            return x_val
        }

        fn test_matches(s: Shape): i32 {
            return match s {
                Shape::Circle(r) => r,
                Shape::Rect(w, h) => w + h,
                Shape::Point => 0,
            }
        }
    "#;
    let source = SourceFile::new(source_id, "test.gfs".to_string(), code.to_string());

    let parse_result = parse(&source);
    let resolve_result = resolve(&source, parse_result.into_graph());
    let graph = resolve_result.into_graph();
    assert!(
        !graph.has_errors(),
        "Parse or resolve errors occurred: {:?}",
        graph.diagnostics()
    );

    let type_result =
        check_definition_types(&source, &graph, check_declaration_types(&source, &graph));
    assert!(
        !type_result.has_errors(),
        "Typecheck errors occurred: {:?}",
        type_result.diagnostics()
    );

    let builder = builder::MirBuilder::new(&graph, &type_result, code);
    let mir_module = builder.build();

    assert!(mir_module.functions.len() >= 2);
    // Let's check that test_structs_arrays_tuples lowered successfully
    let test_func = mir_module
        .functions
        .iter()
        .find(|f| f.name == "test_structs_arrays_tuples")
        .unwrap();

    // We expect NewStruct, NewArray, NewTuple, MemberAccess, ArrayIndex in test_structs_arrays_tuples
    // Let's inspect the statements or block
    let mut found_new_struct = 0;
    let mut found_new_array = 0;
    let mut found_new_tuple = 0;
    let mut found_member_access = 0;
    let mut found_array_index = 0;

    for bb in &test_func.blocks {
        for inst in &bb.instructions {
            if let Instruction::Assign(_, rval) = inst {
                match rval {
                    RValue::NewStruct { .. } => found_new_struct += 1,
                    RValue::NewArray(..) | RValue::NewArrayDynamic(..) => found_new_array += 1,
                    RValue::NewTuple(..) => found_new_tuple += 1,
                    RValue::MemberAccess(..) => found_member_access += 1,
                    RValue::ArrayIndex(..) => found_array_index += 1,
                    _ => {}
                }
            }
        }
    }

    assert!(
        found_new_struct >= 3,
        "Expected at least 3 struct instantiations"
    );
    assert!(
        found_new_array >= 2,
        "Expected at least 2 array instantiations"
    );
    assert!(
        found_new_tuple >= 1,
        "Expected at least 1 tuple instantiation"
    );
    assert!(
        found_member_access >= 1,
        "Expected at least 1 member accesses"
    ); // p1.x
    assert!(
        found_array_index >= 1,
        "Expected at least 1 array index access"
    );

    let match_func = mir_module
        .functions
        .iter()
        .find(|f| f.name == "test_matches")
        .unwrap();
    // Verify that match_func is built with branches (more than 1 block)
    assert!(
        match_func.blocks.len() > 1,
        "Expected match expression to generate multiple blocks"
    );
}
