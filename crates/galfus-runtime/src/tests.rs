use super::*;
use galfus_bytecode::instruction::{ConstIdx, FuncIdx, GlobalIdx, Instruction, Reg, TypeIdx};
use galfus_bytecode::{
    BytecodeFunction, BytecodeGraph, BytecodeModule, BytecodeNode, BytecodeType, Constant,
    ConstantPool, ExportSlot, ImportEdge, ImportSlot,
};
use galfus_core::{ModuleId, ModulePath, SemanticRevision};

fn node(id: ModuleId, path: &str, module: BytecodeModule) -> BytecodeNode {
    BytecodeNode {
        id,
        path: ModulePath::new(path).expect("valid module path"),
        semantic_revision: SemanticRevision::new(0),
        module,
        metadata: None,
    }
}

#[test]
fn run_initializes_dependencies_before_the_entry_module() {
    let dependency_id = ModuleId::new(1);
    let entry_id = ModuleId::new(2);
    let dependency = BytecodeModule {
        name: "dependency.gfs".to_string(),
        constants: ConstantPool {
            constants: vec![Constant::Int32(42)],
        },
        functions: vec![BytecodeFunction {
            name: "__init_module".to_string(),
            param_count: 0,
            local_count: 0,
            temp_count: 1,
            return_ty: TypeIdx(1),
            instructions: vec![
                Instruction::LoadConst {
                    dest: Reg(0),
                    const_idx: ConstIdx(0),
                },
                Instruction::StoreGlobal {
                    module_id: dependency_id,
                    global_idx: GlobalIdx(0),
                    src: Reg(0),
                },
                Instruction::RetNull,
            ],
        }],
        types: vec![BytecodeType::Int32, BytecodeType::Null],
        struct_layouts: vec![],
        choice_layouts: vec![],
        imports: vec![],
        exports: vec![
            ExportSlot {
                symbol_name: "marker".to_string(),
                kind: galfus_bytecode::ExportKind::Function(FuncIdx(0)),
            },
            ExportSlot {
                symbol_name: "global_0".to_string(),
                kind: galfus_bytecode::ExportKind::Global(GlobalIdx(0)),
            },
        ],
        init_func_idx: Some(FuncIdx(0)),
    };
    let entry = BytecodeModule {
        name: "main.gfs".to_string(),
        constants: ConstantPool::default(),
        functions: vec![BytecodeFunction {
            name: "main".to_string(),
            param_count: 1,
            local_count: 0,
            temp_count: 1,
            return_ty: TypeIdx(3),
            instructions: vec![
                Instruction::LoadGlobal {
                    dest: Reg(1),
                    module_id: dependency_id,
                    global_idx: GlobalIdx(0),
                },
                Instruction::Ret { src: Reg(1) },
            ],
        }],
        types: vec![
            BytecodeType::Uint8,
            BytecodeType::Array(TypeIdx(0)),
            BytecodeType::Array(TypeIdx(1)),
            BytecodeType::Int32,
        ],
        struct_layouts: vec![],
        choice_layouts: vec![],
        imports: vec![ImportSlot {
            module_name: "dependency.gfs".to_string(),
            symbol_name: "marker".to_string(),
            ty: TypeIdx(3),
            kind: galfus_bytecode::ImportKind::Function,
        }],
        exports: vec![ExportSlot {
            symbol_name: "main".to_string(),
            kind: galfus_bytecode::ExportKind::Function(FuncIdx(0)),
        }],
        init_func_idx: None,
    };
    let graph = BytecodeGraph::from_modules(
        SemanticRevision::new(0),
        vec![
            node(dependency_id, "dependency.gfs", dependency),
            node(entry_id, "main.gfs", entry),
        ],
        vec![ImportEdge {
            from: entry_id,
            to: dependency_id,
        }],
    )
    .expect("valid graph");

    struct TestExecutor {
        queue: std::sync::Mutex<std::collections::VecDeque<Box<dyn galfus_contract::RunnableTask>>>,
        next_thread_id: std::sync::atomic::AtomicU64,
    }
    impl galfus_contract::ThreadExecutor for TestExecutor {
        fn run_until_idle(&self) -> Result<i32, String> { Ok(0) }
        fn allocate_thread_id(&self) -> u64 {
            self.next_thread_id
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        }

        fn spawn(&self, task: Box<dyn galfus_contract::RunnableTask>) {
            self.queue.lock().unwrap().push_back(task);
        }
    }
    let executor = std::sync::Arc::new(TestExecutor {
        queue: std::sync::Mutex::new(std::collections::VecDeque::new()),
        next_thread_id: std::sync::atomic::AtomicU64::new(1),
    });

    let task = Runtime::new(std::sync::Arc::new(graph.clone()), None)
        .build_module_entry(entry_id, "main", &[], executor.clone())
        .expect("entry execution succeeds");
    galfus_contract::ThreadExecutor::spawn(executor.as_ref(), task);

    let mut exit_code = 0;
    loop {
        let t = executor.queue.lock().unwrap().pop_front();
        let Some(t) = t else { break };
        match t.run(100) {
            galfus_contract::ThreadResult::Yielded(t) => {
                executor.queue.lock().unwrap().push_back(t)
            }
            galfus_contract::ThreadResult::Completed(code) => exit_code = code,
            _ => {}
        }
    }
    assert_eq!(exit_code, 42);
}
