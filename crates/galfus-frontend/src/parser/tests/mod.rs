use super::*;
use galfus_core::{SourceFile, SourceId};

mod binary;
mod calls_members;
mod core;
mod expression_statements;
mod functions;
mod grouping_unary;
mod if_statements;
mod imports_exports;
mod statements;
mod structs_choices;
mod types;

fn source(text: &str) -> SourceFile {
    SourceFile::new(SourceId::new(0), "test.gfs".to_string(), text.to_string())
}
