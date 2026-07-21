use super::RuntimeTask;
use crate::queue::BlockedQueue;
use crate::registry::{ThreadId, ThreadRegistry};
use galfus_bytecode::instruction::{FuncIdx, Reg, TypeIdx};
use galfus_bytecode::{
    BytecodeFunction, BytecodeGraph, BytecodeModule, BytecodeNode, BytecodeType, Instruction,
};
use galfus_contract::{RunnableTask, ThreadExecutor, ThreadResult};
use galfus_core::{ModuleId, ModulePath, SemanticRevision};
use galfus_vm::thread::VirtualThread;
use galfus_vm::{ExecutionStep, VirtualMachine, VmValue};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct TestExecutor {
    tasks: Mutex<VecDeque<Box<dyn RunnableTask>>>,
}

impl TestExecutor {
    fn take_task(&self) -> Option<Box<dyn RunnableTask>> {
        self.tasks.lock().unwrap().pop_front()
    }
}

impl ThreadExecutor for TestExecutor {
    fn allocate_thread_id(&self) -> u64 {
        1
    }

    fn spawn(&self, task: Box<dyn RunnableTask>) {
        self.tasks.lock().unwrap().push_back(task);
    }
}

#[test]
fn receive_timeout_resumes_with_null() {
    let module_id = ModuleId::new(0);
    let module = BytecodeModule {
        name: "test.gfs".to_string(),
        constants: Default::default(),
        functions: vec![BytecodeFunction {
            name: "wait".to_string(),
            param_count: 0,
            local_count: 3,
            temp_count: 0,
            return_ty: TypeIdx(1),
            instructions: vec![
                Instruction::ReceiveFilter {
                    dest: Reg(0),
                    sender: Reg(1),
                    timeout: Reg(2),
                },
                Instruction::Ret { src: Reg(0) },
            ],
        }],
        types: vec![
            BytecodeType::Uint8,
            BytecodeType::Array(TypeIdx(0)),
            BytecodeType::Null,
        ],
        struct_layouts: vec![],
        choice_layouts: vec![],
        imports: vec![],
        exports: vec![],
        init_func_idx: None,
    };
    let graph = BytecodeGraph::from_modules(
        SemanticRevision::new(0),
        vec![BytecodeNode {
            id: module_id,
            path: ModulePath::new("test.gfs").expect("valid path"),
            semantic_revision: SemanticRevision::new(0),
            module,
            metadata: None,
        }],
        vec![],
    )
    .expect("valid graph");
    let vm = VirtualMachine::new(Arc::new(graph));
    let thread_id = ThreadId::from_executor(1).expect("non-zero ID");
    let mut waiting_thread = VirtualThread::new();
    vm.prepare_function(&mut waiting_thread, module_id, FuncIdx(0), vec![])
        .expect("function prepares");
    waiting_thread
        .write_reg(Reg(1), VmValue::Int64(7))
        .expect("sender register exists");
    waiting_thread
        .write_reg(Reg(2), VmValue::Int32(1))
        .expect("timeout register exists");
    assert!(matches!(
        vm.execute_with_budget(&mut waiting_thread, 10),
        Ok(ExecutionStep::ReceiveFilter { .. })
    ));

    let registry = Arc::new(Mutex::new(ThreadRegistry::new()));
    registry.lock().unwrap().register(thread_id, waiting_thread);
    let blocked = Arc::new(Mutex::new(BlockedQueue::new()));
    blocked.lock().unwrap().block_with_timeout(thread_id, 1);
    let executor = Arc::new(TestExecutor {
        tasks: Mutex::new(VecDeque::new()),
    });
    let task = RuntimeTask {
        thread_id,
        thread: VirtualThread::new(),
        vm,
        registry,
        blocked,
        executor: executor.clone(),
    };

    task.schedule_receive_timeout(Reg(0), 1);
    std::thread::sleep(Duration::from_millis(20));

    let timed_out_task = executor.take_task().expect("timeout wakes the task");
    assert!(matches!(timed_out_task.run(10), ThreadResult::Completed(0)));
}
