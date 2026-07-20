use super::*;
use crate::ConstantPool;

fn compiled_image(id: ModuleId, revision: SemanticRevision) -> BytecodeNode {
    BytecodeNode {
        id,
        path: ModulePath::new(format!("src/{}.gfs", id.raw()).as_str()).expect("valid path"),
        semantic_revision: revision,
        module: BytecodeModule {
            name: id.raw().to_string(),
            constants: ConstantPool::default(),
            functions: Vec::new(),
            types: Vec::new(),
            struct_layouts: Vec::new(),
            choice_layouts: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            init_func_idx: None,
        },
        metadata: None,
    }
}

#[test]
fn graph_keys_modules_and_edges_by_stable_module_id() {
    let main = ModuleId::new(41);
    let utilities = ModuleId::new(7);
    let mut graph = BytecodeGraph::new();

    let img1 = compiled_image(main, SemanticRevision::new(3));
    let img2 = compiled_image(utilities, SemanticRevision::new(2));
    graph.apply(BytecodeGraphTransaction {
        upserted_modules: vec![img1, img2],
        removed_modules: vec![],
        edges: vec![ImportEdge {
            from: main,
            to: utilities,
        }],
    });

    assert_eq!(
        graph.get(main).map(BytecodeNode::semantic_revision),
        Some(SemanticRevision::new(3))
    );
    assert_eq!(graph.deps_of(main).collect::<Vec<_>>(), vec![utilities]);
    assert_eq!(graph.dependents_of(utilities), vec![main]);
}
