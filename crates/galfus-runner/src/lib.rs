use std::fs;

use anyhow::Result;
use galfus_core::{SourceFile, SourceId};
use galfus_frontend::lex;

pub fn check_file(path: &str) -> Result<()> {
    let text = fs::read_to_string(path)?;

    let source = SourceFile::new(SourceId::new(0), path.to_string(), text);

    let result = lex(&source);

    println!("tokens:");

    for token in result.tokens() {
        let text = source.slice(token.span()).unwrap_or("");

        println!(
            "  {:?} {:?} `{}`",
            token.kind(),
            token.span(),
            text.escape_debug()
        );
    }

    if result.diagnostics().is_empty() {
        println!("ok");
        return Ok(());
    }

    println!("diagnostics:");

    for diagnostic in result.diagnostics().iter() {
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
        } else {
            println!(
                "  {:?} {}: {}",
                diagnostic.severity(),
                diagnostic.code().as_str(),
                diagnostic.message()
            );
        }
    }

    Ok(())
}
