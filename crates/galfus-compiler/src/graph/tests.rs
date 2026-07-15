use super::*;
use galfus_image::ConstantPool;

fn compiled_image(id: ModuleId, revision: SemanticRevision) -> CompiledModuleImage {
    CompiledModuleImage {
        id,
        path: ModulePath::new(format!("src/{}.gfs", id.raw()).as_str()).expect("valid path"),
        semantic_revision: revision,
        image: ModuleImage {
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
    }
}

#[test]
fn graph_keys_images_and_edges_by_stable_module_id() {
    let main = ModuleId::new(41);
    let utilities = ModuleId::new(7);
    let mut graph = CompiledModuleGraph::new();

    graph.upsert(compiled_image(main, SemanticRevision::new(3)));
    graph.upsert(compiled_image(utilities, SemanticRevision::new(2)));
    graph.set_edges(vec![CompiledImportEdge {
        from: main,
        to: utilities,
    }]);

    assert_eq!(
        graph.get(main).map(CompiledModuleImage::semantic_revision),
        Some(SemanticRevision::new(3))
    );
    assert_eq!(graph.deps_of(main).collect::<Vec<_>>(), vec![utilities]);
    assert_eq!(graph.dependents_of(utilities), vec![main]);
}
