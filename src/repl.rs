use anyhow::{Context, Result};

use crate::cli::OutputFormat;
use crate::client::Client;
use crate::output;
use crate::pager::print_page;
use crate::tty::prompt;

fn prompt_limit() -> Result<Option<u32>> {
    let s = prompt("  Limit (Enter for all): ")?;
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s.parse::<u32>().context("limit must be a number")?))
    }
}

fn prompt_pact_args() -> Result<(Option<u32>, Vec<String>)> {
    let limit = prompt_limit()?;
    let f = prompt("  OData filter (Enter for none, e.g. \"created lt '2024-01-01T00:00:00Z'\"): ")?;
    let filter = if f.is_empty() { vec![] } else { vec![f] };
    Ok((limit, filter))
}

fn prompt_ileap_args() -> Result<(Option<u32>, Vec<String>)> {
    let limit = prompt_limit()?;
    eprintln!("  Filters: key=value pairs, one per line (e.g. mode=road, id=abc-123).");
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

pub async fn run_repl(client: Client, output: &OutputFormat) -> Result<()> {
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
