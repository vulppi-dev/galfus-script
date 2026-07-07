use galfus_core::Diagnostic;

use crate::check::CheckResult;

pub fn print_check_result(result: &CheckResult) {
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

pub fn print_diagnostic(result: &CheckResult, diagnostic: &Diagnostic) {
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
