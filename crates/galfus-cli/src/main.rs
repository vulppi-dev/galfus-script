use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "galfus")]
#[command(about = "Galfus Script runner and tooling")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run { file: String },
    Check { file: String },
    CheckWorkspace { root: String },
    Graph { file: String },
    Repl,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run { file } => {
            galfus_runner::run_project(&file)?;
        }
        Command::Check { file } => {
            galfus_runner::check_file(&file)?;
        }
        Command::CheckWorkspace { root } => {
            galfus_runner::check_workspace_root(&root)?;
        }
        Command::Graph { file } => {
            galfus_runner::print_local_graph_file(&file)?;
        }
        Command::Repl => {
            println!("galfus repl");
        }
    }

    Ok(())
}
