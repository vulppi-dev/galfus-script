use super::*;
use galfus_image::{ConstantPool, ModuleImage};
use galfus_target::NativeTarget;

#[test]
fn test_runtime_thread_spawn() {
    let mut runtime = Runtime::new(Box::new(NativeTarget));
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

    let image = ModuleImage {
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
