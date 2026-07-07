pub use check::check_file;
pub use graph::print_local_graph_file;
pub use repl::repl;
pub use run::run_project;
pub use workspace::{check_workspace, check_workspace_root, compile_workspace_to_image};

pub(crate) use module::{ModuleSource, WorkspaceResolver};

#[cfg(test)]
mod tests;

mod check;
mod diagnostic;
mod graph;
mod module;
mod print;
mod repl;
mod run;
mod workspace;
