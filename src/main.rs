mod cli;
mod client;
mod output;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Command, FootprintsCmd, OutputFormat};
use std::io::BufRead;
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Token persistence
// ---------------------------------------------------------------------------

fn token_file(base_url: &str) -> PathBuf {
    // Derive a filesystem-safe name from the base URL so tokens are per-server.
    let name = base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .replace(['/', ':', '.', '-'], "_");
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ileap")
        .join(format!("token_{name}"))
}

fn save_token(base_url: &str, token: &str) -> Result<()> {
    let path = token_file(base_url);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).context("failed to create config directory")?;
    }
    std::fs::write(&path, token).with_context(|| format!("failed to save token to {}", path.display()))
}

fn jwt_exp(token: &str) -> Option<u64> {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let json: Value = serde_json::from_slice(&bytes).ok()?;
    json.get("exp").and_then(|v| v.as_u64())
}

fn load_saved_token(base_url: &str) -> Option<String> {
    let token = std::fs::read_to_string(token_file(base_url))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;

    if let Some(exp) = jwt_exp(&token) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        if exp <= now + 60 {
            return None;
        }
    }

    Some(token)
}

// ---------------------------------------------------------------------------
// Interactive prompt helpers
// ---------------------------------------------------------------------------

fn prompt(msg: &str) -> Result<String> {
    eprint!("{msg}");
    std::io::stderr().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn prompt_password(msg: &str) -> Result<String> {
    rpassword::prompt_password(msg).context("failed to read password")
}

fn prompt_limit() -> Result<Option<u32>> {
    let s = prompt("  Limit (Enter for all): ")?;
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s.parse::<u32>().context("limit must be a number")?))
    }
}

/// For PACT-based endpoints: single OData $filter expression.
fn prompt_pact_args() -> Result<(Option<u32>, Vec<String>)> {
    let limit = prompt_limit()?;
    let f = prompt("  OData filter (Enter for none, e.g. \"created lt '2024-01-01T00:00:00Z'\"): ")?;
    let filter = if f.is_empty() { vec![] } else { vec![f] };
    Ok((limit, filter))
}

/// For iLEAP standalone endpoints: repeatable key=value filters.
fn prompt_ileap_args() -> Result<(Option<u32>, Vec<String>)> {
    let limit = prompt_limit()?;
    eprintln!("  Filters: key=value pairs, one per line (e.g. mode=road, created=gt:2024-01-01T00:00:00Z).");
    eprintln!("  Press Enter on an empty line when done.");
    let mut filters = vec![];
    loop {
        let f = prompt(&format!("  Filter {} (Enter to finish): ", filters.len() + 1))?;
        if f.is_empty() {
            break;
        }
        filters.push(f);
    }
    Ok((limit, filters))
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let mut cli = Cli::parse();
    cli.base_url = cli.base_url.trim_end_matches('/').to_string();
    let output = cli.output;

    match cli.command {
        None => {
            let client = if let Some(t) = load_saved_token(&cli.base_url) {
                client::Client::from_token(&cli.base_url, t)
            } else {
                let username = prompt("Username: ")?;
                let password = prompt_password("Password: ")?;
                let client =
                    client::Client::authenticate(&cli.base_url, &username, &password).await?;
                save_token(&cli.base_url, client.token())?;
                client
            };
            run_repl(client, &output).await?;
        }
        Some(cmd) => {
            let client = if let Some(t) = cli.token {
                client::Client::from_token(&cli.base_url, t)
            } else if let Some(t) = load_saved_token(&cli.base_url) {
                client::Client::from_token(&cli.base_url, t)
            } else if let (Some(u), Some(p)) = (cli.username, cli.password) {
                client::Client::authenticate(&cli.base_url, &u, &p).await?
            } else {
                anyhow::bail!(
                    "not authenticated — provide --token / --username and --password, \
                     or set ILEAP_TOKEN / ILEAP_USERNAME + ILEAP_PASSWORD"
                );
            };
            run_cmd(&client, cmd, &output).await?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Interactive REPL
// ---------------------------------------------------------------------------

async fn run_repl(client: client::Client, output: &OutputFormat) -> Result<()> {
    loop {
        eprintln!("Welcome to the iLEAP CLI!\n============================");
        eprintln!("\nWhat would you like to do?");
        eprintln!("  1. List PACT-based footprints (with optional limit and OData filter)");
        eprintln!("  2. Get PACT-based footprint by ID");
        eprintln!("  3. List iLEAP standalone ShipmentFootprints (with optional limit and filters)");
        eprintln!("  4. List iLEAP standalone TOCs (with optional limit and filters)");
        eprintln!("  5. List iLEAP standalone HOCs (with optional limit and filters)");
        eprintln!("  6. List iLEAP standalone Transport Activity Data (TAD) (with optional limit and filters)");
        eprintln!("  7. List iLEAP standalone Aggregated Emissions Data (AED) (with optional limit and filters)");
        eprintln!("  0. Exit");

        let choice = prompt("\n> ")?;

        match choice.as_str() {
            "0" | "exit" | "quit" | "q" | "" => break,

            "1" => {
                let (limit, filter) = prompt_pact_args()?;
                let mut offset = 0u32;
                loop {
                    let value = client.footprints(limit, offset, &filter).await?;
                    if !print_page(&value, limit, output)? {
                        break;
                    }
                    offset += limit.unwrap();
                }
            }

            "2" => {
                let id = prompt("  Footprint ID: ")?;
                if !id.is_empty() {
                    output::print_value(&client.footprint(&id).await?, output);
                }
            }

            "3" => {
                let (limit, filter) = prompt_ileap_args()?;
                let mut offset = 0u32;
                loop {
                    let value = client.shipments(limit, offset, &filter).await?;
                    if !print_page(&value, limit, output)? {
                        break;
                    }
                    offset += limit.unwrap();
                }
            }

            "4" => {
                let (limit, filter) = prompt_ileap_args()?;
                let mut offset = 0u32;
                loop {
                    let value = client.tocs(limit, offset, &filter).await?;
                    if !print_page(&value, limit, output)? {
                        break;
                    }
                    offset += limit.unwrap();
                }
            }

            "5" => {
                let (limit, filter) = prompt_ileap_args()?;
                let mut offset = 0u32;
                loop {
                    let value = client.hocs(limit, offset, &filter).await?;
                    if !print_page(&value, limit, output)? {
                        break;
                    }
                    offset += limit.unwrap();
                }
            }

            "6" => {
                let (limit, filter) = prompt_ileap_args()?;
                let mut offset = 0u32;
                loop {
                    let value = client.tad(limit, offset, &filter).await?;
                    if !print_page(&value, limit, output)? {
                        break;
                    }
                    offset += limit.unwrap();
                }
            }

            "7" => {
                let (limit, filter) = prompt_ileap_args()?;
                let mut offset = 0u32;
                loop {
                    let value = client.aed(limit, offset, &filter).await?;
                    if !print_page(&value, limit, output)? {
                        break;
                    }
                    offset += limit.unwrap();
                }
            }

            _ => eprintln!("Invalid choice — please enter a number from the menu."),
        }
    }

    eprintln!("Thanks for using iLEAP CLI. Goodbye!");
    Ok(())
}

// ---------------------------------------------------------------------------
// Non-interactive command execution (used for direct subcommands)
// ---------------------------------------------------------------------------

async fn run_cmd(client: &client::Client, cmd: Command, output: &OutputFormat) -> Result<()> {
    match cmd {
        Command::Footprints { cmd } => match cmd {
            FootprintsCmd::List(args) => {
                let mut offset = 0u32;
                loop {
                    let value = client.footprints(args.limit, offset, &args.filter).await?;
                    if !print_page(&value, args.limit, output)? {
                        break;
                    }
                    offset += args.limit.unwrap();
                }
            }
            FootprintsCmd::Get { id } => {
                output::print_value(&client.footprint(&id).await?, output);
            }
        },

        Command::Shipments(args) => {
            let mut offset = 0u32;
            loop {
                let value = client.shipments(args.limit, offset, &args.filter).await?;
                if !print_page(&value, args.limit, output)? {
                    break;
                }
                offset += args.limit.unwrap();
            }
        }

        Command::Tocs(args) => {
            let mut offset = 0u32;
            loop {
                let value = client.tocs(args.limit, offset, &args.filter).await?;
                if !print_page(&value, args.limit, output)? {
                    break;
                }
                offset += args.limit.unwrap();
            }
        }

        Command::Hocs(args) => {
            let mut offset = 0u32;
            loop {
                let value = client.hocs(args.limit, offset, &args.filter).await?;
                if !print_page(&value, args.limit, output)? {
                    break;
                }
                offset += args.limit.unwrap();
            }
        }

        Command::Tad(args) => {
            let mut offset = 0u32;
            loop {
                let value = client.tad(args.limit, offset, &args.filter).await?;
                if !print_page(&value, args.limit, output)? {
                    break;
                }
                offset += args.limit.unwrap();
            }
        }

        Command::Aed(args) => {
            let mut offset = 0u32;
            loop {
                let value = client.aed(args.limit, offset, &args.filter).await?;
                if !print_page(&value, args.limit, output)? {
                    break;
                }
                offset += args.limit.unwrap();
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Pagination helpers
// ---------------------------------------------------------------------------

fn print_page(value: &Value, limit: Option<u32>, output: &OutputFormat) -> Result<bool> {
    output::print_value(value, output);
    let at_boundary = limit.is_some_and(|l| item_count(value) == l as usize);
    if at_boundary {
        let answer = prompt("Next page? [y/N] ")?;
        Ok(matches!(answer.to_lowercase().as_str(), "y" | "yes"))
    } else {
        Ok(false)
    }
}

fn item_count(value: &Value) -> usize {
    match value {
        Value::Object(obj) => obj
            .get("data")
            .and_then(|d| d.as_array())
            .map(|a| a.len())
            .unwrap_or(0),
        Value::Array(arr) => arr.len(),
        _ => 0,
    }
}
