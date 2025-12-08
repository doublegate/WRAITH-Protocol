//! Build automation tasks for WRAITH Protocol
//!
//! Run with: `cargo xtask <command>`
//!
//! ## Available Commands
//!
//! - `test` - Run all tests
//! - `lint` - Run clippy lints
//! - `fmt` - Check formatting
//! - `ci` - Run all CI checks
//! - `coverage` - Generate code coverage report
//! - `doc` - Generate documentation
//! - `build-xdp` - Build XDP program (requires root)

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

    /// Generate code coverage report (requires cargo-llvm-cov)
    Coverage {
        /// Generate HTML report and open in browser
        #[arg(long)]
        html: bool,

        /// Generate LCOV report for CI integration
        #[arg(long)]
        lcov: bool,

        /// Output directory for reports (default: target/coverage)
        #[arg(long, default_value = "target/coverage")]
        output_dir: String,
    },
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
        Commands::Coverage {
            html,
            lcov,
            output_dir,
        } => {
            println!("Generating code coverage report...");
            println!("Note: Requires cargo-llvm-cov (install with: cargo install cargo-llvm-cov)");

            // Check if cargo-llvm-cov is installed
            let check = Command::new("cargo")
                .args(["llvm-cov", "--version"])
                .output();

            if check.is_err() || !check.unwrap().status.success() {
                eprintln!("Error: cargo-llvm-cov is not installed.");
                eprintln!("Install it with: cargo install cargo-llvm-cov");
                eprintln!("Or with rustup: rustup component add llvm-tools-preview");
                anyhow::bail!("cargo-llvm-cov not found");
            }

            // Create output directory
            std::fs::create_dir_all(&output_dir)?;

            if html {
                // Generate HTML report
                let html_dir = format!("{output_dir}/html");
                println!("Generating HTML report in {html_dir}");
                run_command(
                    "cargo",
                    &[
                        "llvm-cov",
                        "--workspace",
                        "--all-features",
                        "--html",
                        "--output-dir",
                        &html_dir,
                    ],
                )?;
                println!("HTML report generated: {html_dir}/index.html");

                // Try to open in browser
                #[cfg(target_os = "linux")]
                let _ = Command::new("xdg-open")
                    .arg(format!("{html_dir}/index.html"))
                    .spawn();
                #[cfg(target_os = "macos")]
                let _ = Command::new("open")
                    .arg(format!("{}/index.html", html_dir))
                    .spawn();
            } else if lcov {
                // Generate LCOV report for CI
                let lcov_path = format!("{output_dir}/lcov.info");
                println!("Generating LCOV report: {lcov_path}");
                run_command(
                    "cargo",
                    &[
                        "llvm-cov",
                        "--workspace",
                        "--all-features",
                        "--lcov",
                        "--output-path",
                        &lcov_path,
                    ],
                )?;
                println!("LCOV report generated: {lcov_path}");
            } else {
                // Default: show summary in terminal
                run_command("cargo", &["llvm-cov", "--workspace", "--all-features"])?;
            }

            println!("Coverage report complete!");
        }
    }

    Ok(())
}

fn run_command(program: &str, args: &[&str]) -> anyhow::Result<()> {
    let status = Command::new(program).args(args).status()?;

    if !status.success() {
        anyhow::bail!("{program} {args:?} failed");
    }

    Ok(())
}
