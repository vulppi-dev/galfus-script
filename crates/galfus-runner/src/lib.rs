use std::fs;

use anyhow::Result;
use galfus_core::{SourceFile, SourceId};
use galfus_frontend::parse;

pub fn check_file(path: &str) -> Result<()> {
    let text = fs::read_to_string(path)?;

    let source = SourceFile::new(SourceId::new(0), path.to_string(), text);

    let result = parse(&source);
    let graph = result.graph();

    println!("phase: {:?}", graph.phase());
    println!("tokens: {}", graph.syntax().tokens().len());
    println!("syntax nodes: {}", graph.syntax().len());

    if let Some(root) = graph.syntax().root() {
        let root_node = graph.syntax().node(root).expect("root node should exist");

        println!("root: {:?}", root_node.kind());
    }

    println!("tokens:");

    for token in graph.syntax().tokens() {
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
