use super::*;
use galfus_bytecode::BytecodeModule;
use galfus_bytecode::instruction::{ConstIdx, FieldIdx};
use galfus_bytecode::{
    BytecodeFunction, ChoiceLayout, ChoiceVariantLayout, ConstantPool, FieldLayout, OwnershipKind,
    StructLayout,
};

fn create_test_module(instructions: Vec<Instruction>, constants: Vec<Constant>) -> BytecodeModule {
    BytecodeModule {
        name: "test".to_string(),
        constants: ConstantPool { constants },
        functions: vec![BytecodeFunction {
            name: "main".to_string(),
            param_count: 0,
            local_count: 8,
            temp_count: 8,
            return_ty: TypeIdx(0),
            instructions,
        }],
        types: vec![
            BytecodeType::Int64,                               // TypeIdx(0)
            BytecodeType::Bool,                                // TypeIdx(1)
            BytecodeType::Null,                                // TypeIdx(2)
            BytecodeType::Struct(StructLayoutIdx(0)),          // TypeIdx(3)
            BytecodeType::Array(TypeIdx(0)),                   // TypeIdx(4)
            BytecodeType::Tuple(vec![TypeIdx(0), TypeIdx(1)]), // TypeIdx(5)
            BytecodeType::Choice(ChoiceLayoutIdx(0)),          // TypeIdx(6)
            BytecodeType::Uint8,                               // TypeIdx(7)
        ],
        struct_layouts: vec![StructLayout {
            name: "Point".to_string(),
            fields: vec![
                FieldLayout {
                    name: "x".to_string(),
                    ty: TypeIdx(0),
                    offset: 0,
                    ownership: OwnershipKind::Value,
                },
                FieldLayout {
                    name: "y".to_string(),
                    ty: TypeIdx(0),
                    offset: 8,
                    ownership: OwnershipKind::Value,
                },
            ],
            constraints: vec![],
        }],
        choice_layouts: vec![ChoiceLayout {
            name: "OptionInt".to_string(),
            variants: vec![
                ChoiceVariantLayout {
                    name: "None".to_string(),
                    payload_ty: None,
                },
                ChoiceVariantLayout {
                    name: "Some".to_string(),
                    payload_ty: Some(TypeIdx(0)),
                },
            ],
        }],
        imports: Vec::new(),
        exports: Vec::new(),
        init_func_idx: None,
    }
}

mod arithmetic_and_control;
mod io_and_arrays;
mod objects_and_types;
mod ownership;
