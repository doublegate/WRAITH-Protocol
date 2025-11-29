//! Build automation tasks for WRAITH Protocol
//!
//! Run with: `cargo xtask <command>`

use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "WRAITH Protocol build automation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all tests
    Test,

    /// Run clippy lints
    Lint,

    /// Check formatting
    Fmt,

    /// Run all CI checks
    Ci,

    /// Build XDP program (requires root)
    BuildXdp,

    /// Generate documentation
    Doc,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Test => {
            run_command("cargo", &["test", "--all-features", "--workspace"])?;
        }
        Commands::Lint => {
            run_command("cargo", &["clippy", "--workspace", "--", "-D", "warnings"])?;
        }
        Commands::Fmt => {
            run_command("cargo", &["fmt", "--all", "--check"])?;
        }
        Commands::Ci => {
            println!("Running CI checks...");
            run_command("cargo", &["fmt", "--all", "--check"])?;
            run_command("cargo", &["clippy", "--workspace", "--", "-D", "warnings"])?;
            run_command("cargo", &["test", "--all-features", "--workspace"])?;
            println!("All CI checks passed!");
        }
        Commands::BuildXdp => {
            println!("Building XDP program...");
            println!("Note: This requires the wraith-xdp crate and eBPF toolchain");
            // TODO: Implement XDP build
        }
        Commands::Doc => {
            run_command("cargo", &["doc", "--workspace", "--no-deps", "--open"])?;
        }
    }

    Ok(())
}

fn run_command(program: &str, args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new(program).args(args).status()?;

    if !status.success() {
        anyhow::bail!("{} {:?} failed", program, args);
    }

    Ok(())
}
