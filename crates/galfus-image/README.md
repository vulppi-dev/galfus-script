# galfus-image

`galfus-image` defines the static bytecode image representation, instruction format, and layout tables for the in-memory module image.

## Responsibilities

- **ModuleImage**: Represents the compiled artifact, containing constant pools, functions, types, struct/choice layouts, imports, and exports.
- **Instruction Format**: Specifies bytecode instruction opcodes, registers, and operand representations.
- **Bytecode Validation**: Validates instruction register bounds, jump target offsets, and type layout sanity.
- **In-Memory Image**: The compiled `ModuleImage` lives entirely in memory and is passed directly to the VM without file serialization.
