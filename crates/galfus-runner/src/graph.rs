use std::fmt::Write as _;
use std::fs;

use anyhow::Result;
use galfus_core::{NodeId, SourceFile, SourceId, Span};
use galfus_frontend::{ImportKind, ModuleGraph, ResolutionLayer, Scope, parse, resolve};

pub fn print_local_graph_file(path: &str) -> Result<()> {
    let output = local_graph_file_text(path)?;
    print!("{output}");
    Ok(())
}

pub fn local_graph_file_text(path: &str) -> Result<String> {
    let text = fs::read_to_string(path)?;
    Ok(local_graph_text(path, text.as_str()))
}

pub(crate) fn local_graph_text(name: &str, text: &str) -> String {
    let source = SourceFile::new(SourceId::new(0), name.to_string(), text.to_string());

    let parse_result = parse(&source);
    let resolve_result = resolve(&source, parse_result.into_graph());
    let graph = resolve_result.into_graph();

    format_local_graph(&source, &graph)
}

fn format_local_graph(source: &SourceFile, graph: &ModuleGraph) -> String {
    let mut out = String::new();

    writeln!(out, "local graph: {}", source.name()).unwrap();
    writeln!(out, "phase: {:?}", graph.phase()).unwrap();
    writeln!(out).unwrap();

    write_syntax_summary(&mut out, graph);
    writeln!(out).unwrap();

    write_syntax_tree(&mut out, source, graph);
    writeln!(out).unwrap();

    match graph.resolution() {
        Some(resolution) => {
            write_scopes(&mut out, resolution);
            writeln!(out).unwrap();

            write_symbols(&mut out, source, graph, resolution);
            writeln!(out).unwrap();

            write_imports(&mut out, resolution);
            writeln!(out).unwrap();

            write_exports(&mut out, resolution);
            writeln!(out).unwrap();

            write_references(&mut out, source, graph, resolution);
            writeln!(out).unwrap();
        }
        None => {
            writeln!(out, "resolution: <missing>").unwrap();
            writeln!(out).unwrap();
        }
    }

    write_diagnostics(&mut out, source, graph);

    out
}

fn write_syntax_summary(out: &mut String, graph: &ModuleGraph) {
    writeln!(out, "syntax:").unwrap();
    writeln!(out, "  tokens: {}", graph.syntax().tokens().len()).unwrap();
    writeln!(out, "  nodes: {}", graph.syntax().len()).unwrap();

    match graph.syntax().root() {
        Some(root) => {
            let kind = graph
                .syntax()
                .node(root)
                .map(|node| format!("{:?}", node.kind()))
                .unwrap_or_else(|| "<missing>".to_string());

            writeln!(out, "  root: {kind}").unwrap();
        }
        None => {
            writeln!(out, "  root: <missing>").unwrap();
        }
    }
}

fn write_syntax_tree(out: &mut String, source: &SourceFile, graph: &ModuleGraph) {
    writeln!(out, "tree:").unwrap();

    let Some(root) = graph.syntax().root() else {
        writeln!(out, "  <missing>").unwrap();
        return;
    };

    write_syntax_tree_node(out, source, graph, root, 1);
}

fn write_syntax_tree_node(
    out: &mut String,
    source: &SourceFile,
    graph: &ModuleGraph,
    node: NodeId,
    depth: usize,
) {
    let Some(syntax_node) = graph.syntax().node(node) else {
        return;
    };

    let indent = "  ".repeat(depth);
    let location = span_location(source, syntax_node.span());
    let snippet = source
        .slice(syntax_node.span())
        .map(compact_snippet)
        .filter(|snippet| !snippet.is_empty())
        .map(|snippet| format!(" `{snippet}`"))
        .unwrap_or_default();

    writeln!(
        out,
        "{indent}node #{} {:?} at {}{}",
        node.raw(),
        syntax_node.kind(),
        location,
        snippet,
    )
    .unwrap();

    for child in syntax_node.children() {
        write_syntax_tree_node(out, source, graph, *child, depth + 1);
    }
}

fn write_scopes(out: &mut String, resolution: &ResolutionLayer) {
    writeln!(out, "scopes: {}", resolution.scopes().len()).unwrap();

    for scope in resolution.scopes() {
        write_scope(out, scope);
    }
}

fn write_scope(out: &mut String, scope: &Scope) {
    let parent = scope
        .parent()
        .map(|id| format!("#{}", id.raw()))
        .unwrap_or_else(|| "-".to_string());

    let owner = scope
        .owner()
        .map(|id| format!("#{}", id.raw()))
        .unwrap_or_else(|| "-".to_string());

    writeln!(
        out,
        "  scope #{} {:?} parent={} syntax_owner={}",
        scope.id().raw(),
        scope.kind(),
        parent,
        owner,
    )
    .unwrap();

    let mut symbols: Vec<_> = scope.symbols().iter().collect();
    symbols.sort_by(|left, right| left.0.cmp(right.0));

    for (name, symbol) in symbols {
        writeln!(out, "    {name} -> symbol #{}", symbol.raw()).unwrap();
    }
}

fn write_symbols(
    out: &mut String,
    source: &SourceFile,
    graph: &ModuleGraph,
    resolution: &ResolutionLayer,
) {
    writeln!(out, "symbols: {}", resolution.symbols().len()).unwrap();

    for symbol in resolution.symbols() {
        let location = node_location(source, graph, symbol.declaration());

        writeln!(
            out,
            "  symbol #{} {:?} `{}` scope=#{} declared at {}",
            symbol.id().raw(),
            symbol.kind(),
            symbol.name(),
            symbol.scope().raw(),
            location,
        )
        .unwrap();
    }
}

fn write_imports(out: &mut String, resolution: &ResolutionLayer) {
    writeln!(out, "imports: {}", resolution.imports().len()).unwrap();

    for import in resolution.imports() {
        match import.kind() {
            ImportKind::Namespace => {
                writeln!(
                    out,
                    "  import #{} namespace `{}` from `{}` -> symbol #{}",
                    import.id().raw(),
                    import.local_name(),
                    import.source(),
                    import.local_symbol().raw(),
                )
                .unwrap();
            }
            ImportKind::Named => {
                let imported = import.imported_name().unwrap_or("<missing>");

                writeln!(
                    out,
                    "  import #{} named `{}` as `{}` from `{}` -> symbol #{}",
                    import.id().raw(),
                    imported,
                    import.local_name(),
                    import.source(),
                    import.local_symbol().raw(),
                )
                .unwrap();
            }
        }
    }
}

fn write_exports(out: &mut String, resolution: &ResolutionLayer) {
    writeln!(out, "exports: {}", resolution.exports().len()).unwrap();

    for export in resolution.exports() {
        writeln!(
            out,
            "  export #{} {:?} `{}` -> symbol #{}",
            export.id().raw(),
            export.kind(),
            export.name(),
            export.symbol().raw(),
        )
        .unwrap();
    }
}

fn write_references(
    out: &mut String,
    source: &SourceFile,
    graph: &ModuleGraph,
    resolution: &ResolutionLayer,
) {
    let mut references: Vec<_> = resolution.references().iter().collect();
    references.sort_by_key(|(node, _)| node.raw());

    writeln!(out, "references: {}", references.len()).unwrap();

    for (node, symbol) in references {
        let location = node_location(source, graph, *node);

        writeln!(
            out,
            "  node #{} at {} -> symbol #{}",
            node.raw(),
            location,
            symbol.raw(),
        )
        .unwrap();
    }
}

fn write_diagnostics(out: &mut String, source: &SourceFile, graph: &ModuleGraph) {
    writeln!(out, "diagnostics: {}", graph.diagnostics().len()).unwrap();

    for diagnostic in graph.diagnostics().iter() {
        let location = span_location(source, diagnostic.span());

        writeln!(
            out,
            "  {:?} {} at {}: {}",
            diagnostic.severity(),
            diagnostic.code().as_str(),
            location,
            diagnostic.message(),
        )
        .unwrap();
    }
}

fn node_location(source: &SourceFile, graph: &ModuleGraph, node: NodeId) -> String {
    graph
        .syntax()
        .node(node)
        .map(|node| span_location(source, node.span()))
        .unwrap_or_else(|| "<missing>".to_string())
}

fn span_location(source: &SourceFile, span: Span) -> String {
    source
        .row_col(span.start())
        .map(|position| format!("{}:{}", position.row, position.column))
        .unwrap_or_else(|| format!("{}..{}", span.start(), span.end()))
}

fn compact_snippet(text: &str) -> String {
    let snippet = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if snippet.chars().count() <= 80 {
        return snippet;
    }

    let mut compact = snippet.chars().take(77).collect::<String>();
    compact.push_str("...");
    compact
}
