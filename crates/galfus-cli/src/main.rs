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
    Graph { file: String },
    Repl,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Run { file } => {
            println!("galfus run {file}");
        }
        Command::Check { file } => {
            galfus_runner::check_file(&file)?;
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
