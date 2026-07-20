use super::*;
use galfus_bytecode::BytecodeModule;
use galfus_host::{IoProvider, IoProviderError, IoRead, Providers};

struct BufferIo {
    buffer: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    input: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    terminator: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
}

impl IoProvider for BufferIo {
    fn read(&mut self, terminator: &[u8]) -> Result<IoRead, IoProviderError> {
        *self.terminator.lock().unwrap() = terminator.to_vec();
        let mut input = self.input.lock().unwrap();
        if input.is_empty() {
            Ok(IoRead::EndOfInput)
        } else {
            Ok(IoRead::Bytes(std::mem::take(&mut *input)))
        }
    }

    fn write(&mut self, data: &[u8]) -> Result<(), IoProviderError> {
        self.buffer.lock().unwrap().extend_from_slice(data);
        Ok(())
    }
}

#[test]
fn test_target_write() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0),
        },
        Instruction::Write { src: Reg(1) },
        Instruction::RetNull,
    ];
    let image = create_test_image(instrs, vec![Constant::Int64(42)]);
    let buffer = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let provider = BufferIo {
        buffer: buffer.clone(),
        input: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        terminator: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
    };
    let mut graph = galfus_bytecode::BytecodeGraph::new();
    graph.upsert(galfus_bytecode::BytecodeNode {
        id: galfus_core::ModuleId::new(0),
        path: galfus_core::ModulePath::new("test.gfs").unwrap(),
        semantic_revision: galfus_core::SemanticRevision::new(0),
        image,
        metadata: None,
    });
    let mut vm =
        VirtualMachine::new(&graph).with_providers(Some(Providers::with_io(Box::new(provider))));
    let res = vm
        .run_function(galfus_core::ModuleId::new(0), FuncIdx(0), vec![])
        .unwrap();
    assert_eq!(res, Value::Null);
    let output = buffer.lock().unwrap();
    assert_eq!(std::str::from_utf8(&output).unwrap(), "42");
}

#[test]
fn test_target_read() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(2),
            const_idx: ConstIdx(0),
        },
        Instruction::Read {
            dest: Reg(1),
            terminator: Reg(2),
        },
        Instruction::Ret { src: Reg(1) },
    ];
    let image = create_test_image(instrs, vec![Constant::String("!".to_string())]);
    let input = std::sync::Arc::new(std::sync::Mutex::new(b"abc".to_vec()));
    let terminator = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let provider = BufferIo {
        buffer: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        input,
        terminator: terminator.clone(),
    };
    let mut graph = galfus_bytecode::BytecodeGraph::new();
    graph.upsert(galfus_bytecode::BytecodeNode {
        id: galfus_core::ModuleId::new(0),
        path: galfus_core::ModulePath::new("test.gfs").unwrap(),
        semantic_revision: galfus_core::SemanticRevision::new(0),
        image,
        metadata: None,
    });
    let mut vm =
        VirtualMachine::new(&graph).with_providers(Some(Providers::with_io(Box::new(provider))));
    let res = vm
        .run_function(galfus_core::ModuleId::new(0), FuncIdx(0), vec![])
        .unwrap();
    let arr_ref = match res {
        Value::Object(r) => r,
        other => panic!("expected object, got {:?}", other),
    };
    let arr_obj = vm.get_object(arr_ref).unwrap();
    match arr_obj {
        HeapObject::Array { elements, .. } => {
            assert_eq!(
                elements,
                &vec![Value::Uint8(b'a'), Value::Uint8(b'b'), Value::Uint8(b'c')]
            );
        }
        other => panic!("expected array, got {:?}", other),
    }
    assert_eq!(*terminator.lock().unwrap(), b"!");
}

#[test]
fn test_read_requires_an_io_provider_when_executed() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(2),
            const_idx: ConstIdx(0),
        },
        Instruction::Read {
            dest: Reg(1),
            terminator: Reg(2),
        },
        Instruction::Ret { src: Reg(1) },
    ];
    let image = create_test_image(instrs, vec![Constant::String("\n".to_string())]);
    let mut graph = galfus_bytecode::BytecodeGraph::new();
    graph.upsert(galfus_bytecode::BytecodeNode {
        id: galfus_core::ModuleId::new(0),
        path: galfus_core::ModulePath::new("test.gfs").unwrap(),
        semantic_revision: galfus_core::SemanticRevision::new(0),
        image,
        metadata: None,
    });
    let mut vm = VirtualMachine::new(&graph);

    let panic = vm
        .run_function(galfus_core::ModuleId::new(0), FuncIdx(0), vec![])
        .expect_err("read without I/O provider fails");

    assert_eq!(
        panic.error,
        VmError::IoProviderUnavailable { operation: "read" }
    );
}

#[test]
fn test_len_and_copy_array() {
    let instrs = vec![
        // Create source array [1, 2, 3] of type idx 1 (Array of Int64)
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0), // 3
        },
        Instruction::NewArray {
            dest: Reg(2), // src_arr
            type_idx: TypeIdx(1),
            len_reg: Reg(1),
        },
        // Populate elements
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(1), // 10
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(2), // 0 (idx)
        },
        Instruction::StoreIndex {
            arr: Reg(2),
            idx: Reg(4),
            val: Reg(3),
        },
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(3), // 20
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(4), // 1 (idx)
        },
        Instruction::StoreIndex {
            arr: Reg(2),
            idx: Reg(4),
            val: Reg(3),
        },
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(5), // 30
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(6), // 2 (idx)
        },
        Instruction::StoreIndex {
            arr: Reg(2),
            idx: Reg(4),
            val: Reg(3),
        },
        // Query length of src_arr
        Instruction::Len {
            dest: Reg(5),
            src: Reg(2),
        },
        // Allocate dest_arr of size 5
        Instruction::LoadConst {
            dest: Reg(6),
            const_idx: ConstIdx(7), // 5
        },
        Instruction::NewArray {
            dest: Reg(7), // dest_arr
            type_idx: TypeIdx(1),
            len_reg: Reg(6),
        },
        // CopyArray from src_arr to dest_arr starting at index 1
        Instruction::LoadConst {
            dest: Reg(8),
            const_idx: ConstIdx(4), // 1 (dest_start)
        },
        Instruction::CopyArray {
            dest: Reg(7),
            dest_start: Reg(8),
            src: Reg(2),
        },
        Instruction::Ret { src: Reg(7) },
    ];

    let image = BytecodeModule {
        name: "test".to_string(),
        constants: ConstantPool {
            constants: vec![
                Constant::Int64(3),  // 0
                Constant::Int64(10), // 1
                Constant::Int64(0),  // 2
                Constant::Int64(20), // 3
                Constant::Int64(1),  // 4
                Constant::Int64(30), // 5
                Constant::Int64(2),  // 6
                Constant::Int64(5),  // 7
            ],
        },
        functions: vec![ImageFunction {
            name: "main".to_string(),
            param_count: 0,
            local_count: 10,
            temp_count: 10,
            return_ty: TypeIdx(1),
            instructions: instrs,
        }],
        types: vec![ImageType::Int64, ImageType::Array(TypeIdx(0))],
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
        image,
        metadata: None,
    });
    let mut vm = VirtualMachine::new(&graph);
    let res = vm
        .run_function(galfus_core::ModuleId::new(0), FuncIdx(0), vec![])
        .unwrap();
    let arr_ref = match res {
        Value::Object(r) => r,
        other => panic!("expected object, got {:?}", other),
    };
    let arr_obj = vm.get_object(arr_ref).unwrap();
    match arr_obj {
        HeapObject::Array { elements, .. } => {
            assert_eq!(elements.len(), 5);
            assert_eq!(elements[0], Value::Int64(0));
            assert_eq!(elements[1], Value::Int64(10));
            assert_eq!(elements[2], Value::Int64(20));
            assert_eq!(elements[3], Value::Int64(30));
            assert_eq!(elements[4], Value::Int64(0));
        }
        other => panic!("expected array, got {:?}", other),
    }
}

#[test]
fn test_load_index_accepts_negative_index() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0),
        },
        Instruction::NewArray {
            dest: Reg(2),
            type_idx: TypeIdx(4),
            len_reg: Reg(1),
        },
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(1),
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(2),
        },
        Instruction::StoreIndex {
            arr: Reg(2),
            idx: Reg(4),
            val: Reg(3),
        },
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(3),
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(4),
        },
        Instruction::StoreIndex {
            arr: Reg(2),
            idx: Reg(4),
            val: Reg(3),
        },
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(5),
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(6),
        },
        Instruction::StoreIndex {
            arr: Reg(2),
            idx: Reg(4),
            val: Reg(3),
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(7),
        },
        Instruction::LoadIndex {
            dest: Reg(5),
            arr: Reg(2),
            idx: Reg(4),
        },
        Instruction::Ret { src: Reg(5) },
    ];

    let image = create_test_image(
        instrs,
        vec![
            Constant::Int64(3),
            Constant::Int64(10),
            Constant::Int64(0),
            Constant::Int64(20),
            Constant::Int64(1),
            Constant::Int64(30),
            Constant::Int64(2),
            Constant::Int64(-1),
        ],
    );

    let mut graph = galfus_bytecode::BytecodeGraph::new();
    graph.upsert(galfus_bytecode::BytecodeNode {
        id: galfus_core::ModuleId::new(0),
        path: galfus_core::ModulePath::new("test.gfs").unwrap(),
        semantic_revision: galfus_core::SemanticRevision::new(0),
        image,
        metadata: None,
    });
    let mut vm = VirtualMachine::new(&graph);
    let res = vm
        .run_function(galfus_core::ModuleId::new(0), FuncIdx(0), vec![])
        .unwrap();

    assert_eq!(res, Value::Int64(30));
}

#[test]
fn test_load_index_out_of_bounds_returns_null() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0),
        },
        Instruction::NewArray {
            dest: Reg(2),
            type_idx: TypeIdx(4),
            len_reg: Reg(1),
        },
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(1),
        },
        Instruction::LoadIndex {
            dest: Reg(4),
            arr: Reg(2),
            idx: Reg(3),
        },
        Instruction::Ret { src: Reg(4) },
    ];

    let image = create_test_image(instrs, vec![Constant::Int64(3), Constant::Int64(99)]);

    let mut graph = galfus_bytecode::BytecodeGraph::new();
    graph.upsert(galfus_bytecode::BytecodeNode {
        id: galfus_core::ModuleId::new(0),
        path: galfus_core::ModulePath::new("test.gfs").unwrap(),
        semantic_revision: galfus_core::SemanticRevision::new(0),
        image,
        metadata: None,
    });
    let mut vm = VirtualMachine::new(&graph);
    let res = vm
        .run_function(galfus_core::ModuleId::new(0), FuncIdx(0), vec![])
        .unwrap();

    assert_eq!(res, Value::Null);
}

#[test]
fn test_store_index_accepts_negative_index() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0),
        },
        Instruction::NewArray {
            dest: Reg(2),
            type_idx: TypeIdx(4),
            len_reg: Reg(1),
        },
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(1),
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(2),
        },
        Instruction::StoreIndex {
            arr: Reg(2),
            idx: Reg(4),
            val: Reg(3),
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(3),
        },
        Instruction::LoadIndex {
            dest: Reg(5),
            arr: Reg(2),
            idx: Reg(4),
        },
        Instruction::Ret { src: Reg(5) },
    ];

    let image = create_test_image(
        instrs,
        vec![
            Constant::Int64(3),
            Constant::Int64(99),
            Constant::Int64(-1),
            Constant::Int64(2),
        ],
    );

    let mut graph = galfus_bytecode::BytecodeGraph::new();
    graph.upsert(galfus_bytecode::BytecodeNode {
        id: galfus_core::ModuleId::new(0),
        path: galfus_core::ModulePath::new("test.gfs").unwrap(),
        semantic_revision: galfus_core::SemanticRevision::new(0),
        image,
        metadata: None,
    });
    let mut vm = VirtualMachine::new(&graph);
    let res = vm
        .run_function(galfus_core::ModuleId::new(0), FuncIdx(0), vec![])
        .unwrap();

    assert_eq!(res, Value::Int64(99));
}

#[test]
fn test_store_index_out_of_bounds_returns_error() {
    let instrs = vec![
        Instruction::LoadConst {
            dest: Reg(1),
            const_idx: ConstIdx(0),
        },
        Instruction::NewArray {
            dest: Reg(2),
            type_idx: TypeIdx(4),
            len_reg: Reg(1),
        },
        Instruction::LoadConst {
            dest: Reg(3),
            const_idx: ConstIdx(1),
        },
        Instruction::LoadConst {
            dest: Reg(4),
            const_idx: ConstIdx(2),
        },
        Instruction::StoreIndex {
            arr: Reg(2),
            idx: Reg(4),
            val: Reg(3),
        },
        Instruction::RetNull,
    ];

    let image = create_test_image(
        instrs,
        vec![Constant::Int64(3), Constant::Int64(99), Constant::Int64(3)],
    );

    let mut graph = galfus_bytecode::BytecodeGraph::new();
    graph.upsert(galfus_bytecode::BytecodeNode {
        id: galfus_core::ModuleId::new(0),
        path: galfus_core::ModulePath::new("test.gfs").unwrap(),
        semantic_revision: galfus_core::SemanticRevision::new(0),
        image,
        metadata: None,
    });
    let mut vm = VirtualMachine::new(&graph);
    let err = vm
        .run_function(galfus_core::ModuleId::new(0), FuncIdx(0), vec![])
        .unwrap_err();

    assert!(matches!(
        err.error,
        VmError::IndexOutOfBounds { index: 3, len: 3 }
    ));
}
