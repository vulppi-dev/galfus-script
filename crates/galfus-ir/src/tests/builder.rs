use super::*;

#[test]
fn test_mir_builder_basic() {
    let source_id = SourceId::new(0);
    let code = r#"
        fn add(a: int32, b: int32): int32 {
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

    match &func.body {
        MirBody::BasicBlock(bb) => {
            assert_eq!(bb.instructions.len(), 1);
            match &bb.instructions[0] {
                Instruction::Assign(
                    dest,
                    RValue::BinaryOp(MirBinaryOp::Add, Operand::Local(lhs), Operand::Local(rhs)),
                ) => {
                    assert_eq!(lhs.raw(), 0);
                    assert_eq!(rhs.raw(), 1);
                    assert_eq!(dest.raw(), 2);
                }
                other => panic!("Unexpected instruction: {:?}", other),
            }
            match &bb.terminator {
                Terminator::Return(Some(Operand::Local(ret_local))) => {
                    assert_eq!(ret_local.raw(), 2);
                }
                other => panic!("Unexpected terminator: {:?}", other),
            }
        }
        other => panic!("Expected basic block body, found {:?}", other),
    }
}

#[test]
fn test_mir_serialization() {
    let source_id = SourceId::new(0);
    let code = r#"
        fn add(a: int32, b: int32): int32 {
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
fn test_mir_builder_lowers_concrete_typeof_branch() {
    let source_id = SourceId::new(0);
    let code = r#"
        fn label(): [uint8] {
            return typeof int32 {
                int => "number",
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

    match &func.body {
        MirBody::BasicBlock(bb) => match &bb.terminator {
            Terminator::Return(Some(Operand::Constant(Constant::String(value)))) => {
                assert_eq!(value, "number");
            }
            other => panic!("Unexpected terminator: {:?}", other),
        },
        other => panic!("Expected basic block body, found {:?}", other),
    }
}

#[test]
fn test_mir_builder_phase1() {
    let source_id = SourceId::new(0);
    let code = r#"
        fn complex_expr(x: int32): int32 {
            var a = 42;
            const b = 3.14;
            var s = "hello";
            var bl = true;
            var n = null;
            a = x + 10;
            var c = <int32>b;
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
    match &func.body {
        MirBody::BasicBlock(bb) => {
            assert!(bb.instructions.len() >= 7);

            // Check that the terminator is a return statement
            match &bb.terminator {
                Terminator::Return(Some(Operand::Local(local_id))) => {
                    // It returns `a` (which is local 1)
                    assert_eq!(local_id.raw(), 1);
                }
                other => panic!("Unexpected terminator: {:?}", other),
            }
        }
        other => panic!(
            "Expected function body to be a basic block, found: {:?}",
            other
        ),
    }
}

#[test]
fn test_mir_builder_phase2() {
    let source_id = SourceId::new(0);
    let code = r#"
        fn other_func(x: int32): int32 {
            return x * 2
        }

        fn control_flow(cond: bool, val: int32): int32 {
            var res = 0;
            if cond {
                res = other_func(val);
            } else {
                res = 100;
            }

            var i = 0;
            while i < 10 {
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

    match &func.body {
        MirBody::Block { statements, .. } => {
            let mut found_if = false;
            let mut found_while = false;
            let mut found_loop = false;
            let mut found_return = false;

            for stmt in statements {
                match stmt {
                    MirBody::If { .. } => {
                        found_if = true;
                    }
                    MirBody::Loop { .. } => {
                        if found_while {
                            found_loop = true;
                        } else {
                            found_while = true;
                        }
                    }
                    MirBody::BasicBlock(bb) => {
                        if matches!(bb.terminator, Terminator::Return(_)) {
                            found_return = true;
                        }
                    }
                    _ => {}
                }
            }

            assert!(found_if, "If statement not found in MIR");
            assert!(found_while, "While statement not found in MIR");
            assert!(found_loop, "Loop statement not found in MIR");
            assert!(found_return, "Return statement not found in MIR");
        }
        other => panic!(
            "Expected block body for control_flow function, found {:?}",
            other
        ),
    }
}

#[test]
fn test_mir_builder_phase3() {
    let source_id = SourceId::new(0);
    let code = r#"
        struct Point {
            x: int32,
            y: int32 = 42,
        }

        struct Point3D {
            ...Point,
            z: int32,
        }

        choice Shape {
            Circle(int32),
            Rect(int32, int32),
            Point,
        }

        fn test_structs_arrays_tuples(p: Point): int32 {
            var p1 = new(Point) { x: 10, y: 20 };
            var p2 = new(Point) { x: 5 }; // default y
            var p3 = new(Point3D) { ...p1, z: 100 }; // spread struct

            var arr1 = [1, 2, 3];
            var arr2: [int32; 5] = [...arr1, 4, 5]; // spread array

            var t1 = (10, 20, 30);

            var x_val = p1.x; // member access
            var arr_val = arr2[1]; // array index

            return x_val
        }

        fn test_matches(s: Shape): int32 {
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

    let type_result = check_declaration_types(&source, &graph);
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
    match &test_func.body {
        MirBody::BasicBlock(bb) => {
            // Find assignments with NewStruct, NewArray, NewTuple, MemberAccess, ArrayIndex
            let mut found_new_struct = 0;
            let mut found_new_array = 0;
            let mut found_new_tuple = 0;
            let mut found_member_access = 0;
            let mut found_array_index = 0;

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
        }
        other => panic!("Expected basic block body, found {:?}", other),
    }

    let match_func = mir_module
        .functions
        .iter()
        .find(|f| f.name == "test_matches")
        .unwrap();
    // Let's check that test_matches contains an If statement/body
    match &match_func.body {
        MirBody::Block { statements, .. } => {
            let has_if = statements
                .iter()
                .any(|stmt| matches!(stmt, MirBody::If { .. }));
            assert!(
                has_if,
                "Expected match expression to lower to nested If branches"
            );
        }
        other => panic!("Expected block body, found {:?}", other),
    }
}
