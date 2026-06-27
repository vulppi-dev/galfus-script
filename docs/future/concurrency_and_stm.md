# Galfus Future Concurrency and Transactional Shared Memory Architecture

This document defines the conceptual design for the future multithreading, concurrency, and transactional shared memory (STM) architecture of the Galfus runtime. These features are scheduled for development after the MVP.

---

## 1. Concurrency Model (Logical Threads)

To ensure the Galfus VM remains target-agnostic and capable of running across desktop, mobile, web, and embedded targets, concurrency is abstracted into **Logical Threads** (execution contexts or fibers):

1. **`ThreadContext`**: Tracks independent execution states (instruction pointer, local stack, local heap).
2. **Scheduler Adapters**: The host environment maps logical threads to target capabilities:
   * **Desktop/Server**: Spawns OS threads (`std::thread`), each running a VM loop.
   * **Web/WASM**: Cooperative green threads yielding to the browser/Node.js event loop.
   * **Embedded (Monocore)**: A simple cooperative, round-robin micro-scheduler.

---

## 2. Dual-Heap Memory Architecture

Memory is divided into two distinct regions to balance performance, local garbage collection efficiency, and cross-thread communication safety:


```txt
+-------------------------------------------------------------+
|                        Shared Heap                          |
|  (Globally shared, atomic/STM managed, holds @shared state) |
+-------------------------------------------------------------+
          ^                                         ^
          |                                         |
+-------------------+                     +-------------------+
|    Local Heap 1   |                     |    Local Heap 2   |
| (Thread-exclusive |                     | (Thread-exclusive |
|  owner_graph_core)|                     |  owner_graph_core)|
+-------------------+                     +-------------------+
```

### Local Heap

* Owned exclusively by a single `ThreadContext`.
* Managed by the single-threaded, highly optimized `owner_graph_core`.
* Requires no mutexes, atomics, or cross-thread synchronization.

### Shared Heap

* Shared globally across all logical threads.
* Holds struct instances instantiated with `new(shared)`.
* Struct fields can only reference primitives or other shared structs to prevent local reference escapes.

---

## 3. Software Transactional Memory (STM)

Mutations to the shared heap are transaction-guaranteed, implementing an **Optimistic Concurrency Control (OCC)** model using alteration logs.

### Core Concepts

* **Entry Keys**: Instantiate structs as shared (e.g., `ConfigShared`) to act as keys.
* **Alteration Log (Transaction Log)**:
  * When a thread reads a shared struct, it records the struct version in its local **Read Set**.
  * When a thread modifies a shared struct, it records the mutations in its local **Write Set** (the alteration log) without modifying the shared heap.
* **Commit/Rollback**:
  * **Commit (`TX_COMMIT`)**: Validates that none of the read keys have been modified by another thread. If valid, writes the alteration log atomically and updates versions.
  * **Rollback (`TX_ROLLBACK`)**: Discards the local alteration log.

### Syntax Demonstration

```galfus
export var config: ConfigShared

fn createConfigDefault(): ConfigShared {
  // Allocated on the Shared Heap
  return new(shared) {
    port: 3001,
    host: '127.0.0.1'
  }
}

fn main(): null {
  config = createConfigDefault()
  
  // Transaction block locks/registers the 'config' instance
  transaction config {
    config.port = 8080
  }
}
```

---

## 4. Future VM Opcode Extensions

The VM instruction set includes abstract opcodes for thread control and transactional memory:

| Opcode | Arguments | Description |
| :--- | :--- | :--- |
| **`SPAWN`** | `func_idx`, `args_reg` | Spawns a new logical thread. |
| **`YIELD`** | None | Yields execution of the current thread. |
| **`ALLOC_SHARED`** | `layout_idx`, `dest_reg` | Allocates a struct on the shared heap. |
| **`TX_START`** | `key_reg` | Starts a transaction scope, registering the shared object as an entry key. |
| **`TX_LOAD`** | `obj_reg`, `field_idx`, `dest_reg` | Reads a shared field (using local alteration log if modified, otherwise shared heap). |
| **`TX_STORE`** | `obj_reg`, `field_idx`, `val_reg` | Stores a field update in the thread's local alteration log. |
| **`TX_COMMIT`** | `result_reg` | Atomically commits the alteration log. |
| **`TX_ROLLBACK`** | None | Discards the current transaction's alteration log. |

### Implicit Rollback on Panic

If a runtime panic occurs inside a `transaction` block, the VM catches the unwinding event, executes `TX_ROLLBACK` to discard all buffered writes, and propagates the panic. This guarantees the Shared Heap remains uncorrupted.
