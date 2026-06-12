use super::*;
use galfus_core::{SourceFile, SourceId};

mod anchored_functions;
mod assignment_statements;
mod binding_patterns;
mod compound_assignment_statements;
mod constraint_items;
mod control_flow_statements;
mod default_parameters;
mod expression_statements;
mod expressions;
mod for_statements;
mod function_types;
mod functions;
mod generic_constraints;
mod generic_declarations;
mod generic_types;
mod grouped_types;
mod if_statements;
mod instanceof_statements;
mod loop_statements;
mod match_statements;
mod module_items;
mod parser_core;
mod regex_literals;
mod rest_parameters;
mod return_statements;
mod struct_and_choice_items;
mod struct_fields;
mod struct_satisfies_clauses;
mod type_paths;
mod types;
mod variable_declarations;
mod while_statements;

fn source(text: &str) -> SourceFile {
    SourceFile::new(SourceId::new(0), "test.gfs".to_string(), text.to_string())
}
