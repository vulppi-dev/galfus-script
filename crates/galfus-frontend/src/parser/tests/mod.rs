use super::*;
use galfus_core::{SourceFile, SourceId};

mod array_literals;
mod assignment_statements;
mod binary;
mod calls_members;
mod compound_assignment_statements;
mod control_flow_statements;
mod core;
mod default_parameters;
mod expression_statements;
mod for_statements;
mod functions;
mod grouping_unary;
mod if_statements;
mod imports_exports;
mod index_expressions;
mod loop_statements;
mod match_statements;
mod rest_parameters;
mod spread_expressions;
mod statements;
mod struct_literals;
mod structs_choices;
mod types;
mod variant_construction;
mod while_statements;

fn source(text: &str) -> SourceFile {
    SourceFile::new(SourceId::new(0), "test.gfs".to_string(), text.to_string())
}
