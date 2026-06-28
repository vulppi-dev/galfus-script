# galfus-image

`galfus-image` defines the static bytecode image representation, instruction format, layout tables, and the `.gfb` binary format serialization/deserialization.

## Responsibilities

- **ModuleImage**: Represents the compiled artifact, containing constant pools, functions, types, struct/choice layouts, imports, and exports.
- **Instruction Format**: Specifies bytecode instruction opcodes, registers, and operand representations.
- **Bytecode Validation**: Validates instruction register bounds, jump target offsets, and type layout sanity.
- **GFB Serialization**: Provides tools to read and write `.gfb` (Galfus Binary Format) files.
