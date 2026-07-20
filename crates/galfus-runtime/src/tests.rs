use super::*;
use galfus_bytecode::{BytecodeModule, ConstantPool, ExportSlot, ImportSlot, instruction::TypeIdx};
use galfus_compiler::CompiledBytecodeModule;
use galfus_core::{ModuleId, ModulePath, SemanticRevision};

#[test]
fn test_runtime_thread_spawn() {
    let mut runtime = Runtime::new();
    let t1 = runtime.spawn_thread();
    let t2 = runtime.spawn_thread();

    assert_eq!(t1, 0);
    assert_eq!(t2, 1);
    assert_eq!(runtime.threads().len(), 2);
    assert_eq!(runtime.threads()[0].state(), ThreadState::Running);
}

#[test]
fn test_module_loading() {
    let registry = Arc::new(Mutex::new(ModuleRegistry::new()));
    let loader = RuntimeLoader::new(registry.clone());

    let image = BytecodeModule {
        name: "test_mod".to_string(),
        constants: ConstantPool::default(),
        functions: vec![],
        types: vec![],
        struct_layouts: vec![],
        choice_layouts: vec![],
        imports: vec![],
        exports: vec![],
        init_func_idx: None,
    };

    let loaded = loader.load(image);
    assert_eq!(loaded.name, "test_mod");

    let found = registry.lock().unwrap().get("test_mod");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "test_mod");
}

#[test]
fn runtime_modules_upsert_unload_and_link_import_slots() {
    let dependency = CompiledBytecodeModule {
        id: ModuleId::new(7),
        path: ModulePath::new("math.gfs").expect("valid path"),
        semantic_revision: SemanticRevision::new(1),
        image: module_image(
            "math.gfs",
            vec![],
            vec![ExportSlot {
                symbol_name: "add".to_string(),
                func_idx: galfus_bytecode::instruction::FuncIdx(3),
            }],
        ),
    };
    let main = CompiledBytecodeModule {
        id: ModuleId::new(11),
        path: ModulePath::new("main.gfs").expect("valid path"),
        semantic_revision: SemanticRevision::new(1),
        image: module_image(
            "main.gfs",
            vec![ImportSlot {
                module_name: "math.gfs".to_string(),
                symbol_name: "add".to_string(),
                ty: TypeIdx(0),
            }],
            vec![],
        ),
    };
    let replacement = CompiledBytecodeModule {
        semantic_revision: SemanticRevision::new(2),
        ..main.clone()
    };

    let mut runtime = Runtime::new();
    assert!(runtime.load(main).is_none());
    assert!(matches!(
        runtime.link_module(ModuleId::new(11)),
        Err(RuntimeLinkError::ImportModuleNotLoaded { .. })
    ));
    assert!(runtime.load(dependency).is_none());

    let link = runtime
        .link_module(ModuleId::new(11))
        .expect("links imports");
    assert_eq!(link.imports.len(), 1);
    assert_eq!(link.imports[0].module_id, ModuleId::new(7));
    assert_eq!(link.imports[0].function.raw(), 3);
    assert_eq!(
        runtime
            .initialization_order(ModuleId::new(11))
            .expect("orders dependencies"),
        vec![ModuleId::new(7), ModuleId::new(11)]
    );

    let previous = runtime.load(replacement).expect("replaces main module");
    assert_eq!(previous.semantic_revision, SemanticRevision::new(1));
    assert_eq!(
        runtime
            .modules()
            .get(ModuleId::new(11))
            .unwrap()
            .semantic_revision,
        SemanticRevision::new(2)
    );
    assert!(runtime.unload(ModuleId::new(7)).is_some());
    assert!(matches!(
        runtime.link_module(ModuleId::new(11)),
        Err(RuntimeLinkError::ImportModuleNotLoaded { .. })
    ));
}

fn module_image(name: &str, imports: Vec<ImportSlot>, exports: Vec<ExportSlot>) -> BytecodeModule {
    BytecodeModule {
        name: name.to_string(),
        constants: ConstantPool::default(),
        functions: vec![],
        types: vec![],
        struct_layouts: vec![],
        choice_layouts: vec![],
        imports,
        exports,
        init_func_idx: None,
    }
}
