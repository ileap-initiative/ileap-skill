mod auth;
mod cli;
mod client;
mod commands;
mod error;
mod output;
mod prompt;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use cli::{Cli, OutputFormat};
use error::CliError;
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut cli = Cli::parse();
    cli.base_url = cli.base_url.trim_end_matches('/').to_string();
    let output = cli.output.clone();

    if let Err(e) = run(cli).await {
        // Attempt to recover the typed error before it is fully erased.
        let (exit_code, error_type, message) = if let Some(ce) = e.downcast_ref::<CliError>() {
            (ce.exit_code(), ce.error_type(), ce.to_string())
        } else {
            // Fallback for errors that never passed through CliError
            // (e.g. I/O errors from tty or pager).
            (1, "error", e.to_string())
        };

        let json = serde_json::json!({
            "cli_error": { "type": error_type, "message": message }
        });
        match output {
            OutputFormat::Compact => eprintln!("{}", serde_json::to_string(&json).unwrap()),
            OutputFormat::Pretty => eprintln!("{}", serde_json::to_string_pretty(&json).unwrap()),
        }

        std::process::exit(exit_code);
    }
}

async fn run(cli: Cli) -> Result<()> {
    let output = cli.output;
    let timeout = cli.timeout.map(Duration::from_secs);

    match cli.command {
        None => {
            Cli::command().print_help()?;
            println!();
        }

        Some(cmd) => {
            commands::run_cmd(
                cmd,
                &cli.base_url,
                cli.token.as_deref(),
                cli.username.as_deref(),
                cli.password.as_deref(),
                timeout,
                &output,
            )
            .await?;
        }
    }

    Ok(())
}
