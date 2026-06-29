use super::*;

#[test]
fn test_vm_creation() {
    let image = galfus_image::ModuleImage {
        name: "test".to_string(),
        constants: galfus_image::ConstantPool::default(),
        functions: vec![],
        types: vec![],
        struct_layouts: vec![],
        choice_layouts: vec![],
        imports: vec![],
        exports: vec![],
        init_func_idx: None,
    };
    let vm = VirtualMachine::new(image);
    assert_eq!(vm.image.name, "test");
}
