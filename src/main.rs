//! orksworksorks — conventions CLI.
//!
//! Ships the convention spine (typed errors, colored output, JSON envelope,
//! TOML config) behind the `orksworksorks init` command.

#![warn(missing_docs)]

mod commands;
mod errors;
mod format;

use clap::Parser;
use errors::Error;

/// The result of running a command, ready for the output layer.
pub struct CommandResult {
    /// Command output: `Ok` data on success, typed error on failure.
    pub result: Result<String, Error>,
    /// Ring the terminal bell on success.
    pub bell_success: bool,
    /// Ring the terminal bell on failure.
    pub bell_failure: bool,
    /// Emit the result as a JSON envelope instead of text.
    pub json: bool,
}

/// Print the command result as text (stdout on success, stderr on error).
fn output_text(result: &Result<String, Error>) {
    match result {
        Ok(data) => {
            println!("{data}");
            // Terminal bell on success
            print!("\x07");
        }
        Err(e) => {
            eprintln!("\n\n{e}");
            // Terminal bell on failure
            print!("\x07");
        }
    }
}

/// Print the command result as a JSON envelope.
fn output_json(result: &Result<String, Error>) {
    match result {
        Ok(data) => {
            let json = serde_json::json!({"data": data});
            println!("{json}");
        }
        Err(e) => {
            let json = serde_json::json!({"error": {"message": e.message, "source": e.source}});
            println!("{json}");
        }
    }
}

/// Dispatch to text or JSON output based on the command result's json flag.
fn output_result(cr: &CommandResult) {
    if cr.json {
        output_json(&cr.result);
    } else {
        output_text(&cr.result);
    }
}

/// Parse CLI, run the command, and output the result.
fn run_command(cli: commands::Cli) {
    let json = cli.json;
    let result = commands::select_command(&cli);
    let cr = match result {
        Ok(data) => CommandResult {
            result: Ok(data),
            bell_success: true,
            bell_failure: false,
            json,
        },
        Err(e) => CommandResult {
            result: Err(e),
            bell_success: false,
            bell_failure: true,
            json,
        },
    };
    output_result(&cr);
    if cr.result.is_err() {
        std::process::exit(1);
    }
}

#[tokio::main]
async fn main() {
    let cli = commands::Cli::parse();
    run_command(cli);
}
