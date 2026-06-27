use super::*;
use galfus_core::{SourceFile, SourceId};
use galfus_frontend::{check_declaration_types, parse, resolve};

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
