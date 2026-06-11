mod auth;
mod cli;
mod client;
mod commands;
mod output;
mod pager;
mod repl;
mod tty;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command, OutputFormat};
use client::ExitCode;
use std::io::IsTerminal;
use std::time::Duration;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut cli = Cli::parse();
    cli.base_url = cli.base_url.trim_end_matches('/').to_string();
    let output = cli.output.clone();

    if let Err(e) = run(cli).await {
        let exit_code = e
            .chain()
            .find_map(|c| c.downcast_ref::<ExitCode>())
            .map(|ec| ec.0)
            .unwrap_or(1);

        let message: Vec<String> = e
            .chain()
            .filter(|c| c.downcast_ref::<ExitCode>().is_none())
            .map(|c| c.to_string())
            .collect();
        let message = message.join(": ");

        let error_type = match exit_code {
            3 => "not_found",
            4 => "auth_error",
            _ => "error",
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
            if !std::io::stdin().is_terminal() {
                anyhow::bail!(
                    "no command provided and stdin is not a terminal — use a subcommand (run with --help to see available commands)"
                );
            }
            let client = match auth::load_saved_token(&cli.base_url) {
                Ok(Some(t)) => client::Client::from_token(&cli.base_url, t, timeout),
                Ok(None) => {
                    let username = tty::prompt("Username: ")?;
                    let password = tty::prompt_password("Password: ")?;
                    let c = client::Client::authenticate(&cli.base_url, &username, &password, timeout)
                        .await?;
                    auth::save_token(&cli.base_url, c.token())?;
                    c
                }
                Err(e) => {
                    eprintln!("warning: {e}; continuing with interactive login");
                    let username = tty::prompt("Username: ")?;
                    let password = tty::prompt_password("Password: ")?;
                    let c = client::Client::authenticate(&cli.base_url, &username, &password, timeout)
                        .await?;
                    auth::save_token(&cli.base_url, c.token())?;
                    c
                }
            };
            repl::run_repl(client, &output).await?;
        }

        Some(Command::Auth { cmd }) => {
            auth::run_auth(
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

        Some(cmd) => {
            let client = if let Some(t) = cli.token {
                client::Client::from_token(&cli.base_url, t, timeout)
            } else if let Some(t) = auth::load_saved_token(&cli.base_url)? {
                client::Client::from_token(&cli.base_url, t, timeout)
            } else {
                match (cli.username, cli.password) {
                    (Some(u), Some(p)) => {
                        client::Client::authenticate(&cli.base_url, &u, &p, timeout).await?
                    }
                    (u, p) => return Err(auth::credential_error(u.as_deref(), p.as_deref())),
                }
            };
            commands::run_cmd(&client, cmd, &output).await?;
        }
    }

    Ok(())
}
