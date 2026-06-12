use super::*;
use galfus_core::{SourceFile, SourceId};

mod anchored_functions;
mod binding_patterns;
mod constraint_items;
mod default_parameters;
mod expressions;
mod function_types;
mod functions;
mod generic_constraints;
mod generic_declarations;
mod generic_types;
mod grouped_types;
mod module_items;
mod parser_core;
mod ranges;
mod regex_literals;
mod rest_parameters;
mod statements;
mod struct_and_choice_items;
mod struct_fields;
mod struct_satisfies_clauses;
mod tuples;
mod type_paths;
mod types;
mod variable_declarations;

fn source(text: &str) -> SourceFile {
    SourceFile::new(SourceId::new(0), "test.gfs".to_string(), text.to_string())
}
