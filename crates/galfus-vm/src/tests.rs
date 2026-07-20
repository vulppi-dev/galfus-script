use super::*;

#[test]
fn test_vm_creation() {
    let image = galfus_bytecode::BytecodeModule {
        name: "test".to_string(),
        constants: galfus_bytecode::ConstantPool::default(),
        functions: vec![],
        types: vec![],
        struct_layouts: vec![],
        choice_layouts: vec![],
        imports: vec![],
        exports: vec![],
        init_func_idx: None,
    };
    let mut graph = galfus_bytecode::BytecodeGraph::new();
    graph.upsert(galfus_bytecode::BytecodeNode {
        id: galfus_core::ModuleId::new(0),
        path: galfus_core::ModulePath::new("test.gfs").unwrap(),
        semantic_revision: galfus_core::SemanticRevision::new(0),
        module: image,
        metadata: None,
    });
    let _vm = VirtualMachine::new(&graph);
}
