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

pub fn compile_file_to_gfb(source_path: &Path, output_path: &Path) -> Result<()> {
    use std::fs;

    let code = fs::read_to_string(source_path)?;
    let source_id = galfus_core::SourceId::new(0);
    let source_file = galfus_core::SourceFile::new(
        source_id,
        source_path.to_string_lossy().into_owned(),
        code.clone(),
    );

    let parse_result = galfus_frontend::parse(&source_file);
    let resolve_result = galfus_frontend::resolve(&source_file, parse_result.into_graph());
    let graph = resolve_result.into_graph();

    if graph.has_errors() {
        return Err(anyhow::anyhow!(
            "Compilation failed during parsing/resolution: {:?}",
            graph.diagnostics()
        ));
    }

    let type_result = galfus_frontend::check_declaration_types(&source_file, &graph);
    if type_result.has_errors() {
        return Err(anyhow::anyhow!(
            "Compilation failed during type-checking: {:?}",
            type_result.diagnostics()
        ));
    }

    let mir_module = galfus_ir::builder::MirBuilder::new(&graph, &type_result, &code).build();
    let module_image = galfus_ir::lower::lower_module(&mir_module, &type_result, &graph, &code);

    if let Err(errors) = galfus_core::image::validation::validate_module_image(&module_image) {
        return Err(anyhow::anyhow!(
            "ModuleImage validation failed: {:?}",
            errors
        ));
    }

    let gfb_bytes = galfus_core::image::gfb::serialize_to_gfb(&module_image)
        .map_err(|e| anyhow::anyhow!("Serialization error: {}", e))?;

    fs::write(output_path, gfb_bytes)?;

    Ok(())
}

pub fn load_gfb_file(path: &Path) -> Result<galfus_core::image::ModuleImage> {
    use std::fs;

    let bytes = fs::read(path)?;
    let module_image = galfus_core::image::gfb::deserialize_from_gfb(&bytes)
        .map_err(|e| anyhow::anyhow!("GFB loader error: {}", e))?;
    Ok(module_image)
}
