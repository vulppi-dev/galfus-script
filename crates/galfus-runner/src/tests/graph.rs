use crate::local_graph::local_graph_text;

#[test]
fn local_graph_prints_resolved_phase() {
    let output = local_graph_text(
        "main.gfs",
        r#"
fn main(): null {
  return
}
"#,
    );

    assert!(output.contains("phase: Resolved"));
    assert!(output.contains("syntax:"));
    assert!(output.contains("scopes:"));
    assert!(output.contains("symbols:"));
    assert!(output.contains("diagnostics: 0"));
}

#[test]
fn local_graph_prints_imports_and_exports() {
    let output = local_graph_text(
        "main.gfs",
        r#"
import user from "./user"

export fn main(): null {
  user
  return
}
"#,
    );

    assert!(output.contains("import #0 namespace `user` from `./user`"));
    assert!(output.contains("export #0 Function `main`"));
    assert!(output.contains("references:"));
}

#[test]
fn local_graph_prints_resolver_diagnostics() {
    let output = local_graph_text(
        "main.gfs",
        r#"
fn main(): null {
  return
}

fn main(): null {
  return
}
"#,
    );

    assert!(output.contains("diagnostics: 1"));
    assert!(output.contains("duplicate symbol `main`"));
}
