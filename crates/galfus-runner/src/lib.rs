#[cfg(test)]
mod tests;

mod check;
mod diagnostic;
mod local_graph;
mod workspace;
mod workspace_graph;

pub use check::*;
pub use diagnostic::*;
pub use local_graph::*;
pub use workspace::*;
pub use workspace_graph::*;

use anyhow::Result;
use galfus_core::Diagnostic;
use std::path::{Path, PathBuf};

fn normalize_existing_path(path: &Path) -> Result<PathBuf> {
    Ok(path.canonicalize()?)
}

fn print_check_result(result: &CheckResult) {
    println!("modules: {}", result.modules().len());

    for module in result.modules() {
        println!(
            "  {:?}: {:?}, syntax nodes: {}",
            module.path(),
            module.graph().phase(),
            module.graph().syntax().len()
        );
    }

    if result.diagnostics().is_empty() {
        println!("ok");
        return;
    }

    println!("diagnostics:");

    for diagnostic in result.diagnostics().iter() {
        print_diagnostic(result, diagnostic);
    }
}

fn print_diagnostic(result: &CheckResult, diagnostic: &Diagnostic) {
    let source = result.source_for(diagnostic.span().source_id());

    if let Some(source) = source {
        let pos = source.row_col(diagnostic.span().start());

        if let Some(pos) = pos {
            println!(
                "  {:?} {} at {}:{}:{}: {}",
                diagnostic.severity(),
                diagnostic.code().as_str(),
                source.name(),
                pos.row,
                pos.column,
                diagnostic.message()
            );
            return;
        }
    }

    println!(
        "  {:?} {}: {}",
        diagnostic.severity(),
        diagnostic.code().as_str(),
        diagnostic.message()
    );
}
