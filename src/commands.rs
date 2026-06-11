use anyhow::Result;
use serde_json::Value;
use std::future::Future;
use std::io::IsTerminal;

use crate::cli::{Command, FootprintsCmd, ListCmd, OutputFormat};
use crate::client::Client;
use crate::output;
use crate::pager::{item_count, print_page};

pub async fn run_cmd(client: &Client, cmd: Command, output: &OutputFormat) -> Result<()> {
    match cmd {
        Command::Footprints { cmd } => match cmd {
            FootprintsCmd::List(args) => {
                if args.dry_run {
                    output::print_value(
                        &client.footprints_dry_run(args.limit, 0, &args.filter),
                        output,
                    );
                    return Ok(());
                }
                run_list(args.yes, args.max_pages, args.limit, output, |off| {
                    client.footprints(args.limit, off, &args.filter)
                })
                .await?;
            }
            FootprintsCmd::Get { id, dry_run } => {
                if dry_run {
                    output::print_value(&client.footprint_dry_run(&id), output);
                    return Ok(());
                }
                output::print_value(&client.footprint(&id).await?, output);
            }
        },

        Command::Shipments {
            cmd: ListCmd::List(args),
        } => {
            if args.dry_run {
                output::print_value(
                    &client.list_dry_run("/v1/ileap/shipments", args.limit, 0, &args.filter),
                    output,
                );
                return Ok(());
            }
            run_list(args.yes, args.max_pages, args.limit, output, |off| {
                client.shipments(args.limit, off, &args.filter)
            })
            .await?;
        }

        Command::Tocs {
            cmd: ListCmd::List(args),
        } => {
            if args.dry_run {
                output::print_value(
                    &client.list_dry_run("/v1/ileap/tocs", args.limit, 0, &args.filter),
                    output,
                );
                return Ok(());
            }
            run_list(args.yes, args.max_pages, args.limit, output, |off| {
                client.tocs(args.limit, off, &args.filter)
            })
            .await?;
        }

        Command::Hocs {
            cmd: ListCmd::List(args),
        } => {
            if args.dry_run {
                output::print_value(
                    &client.list_dry_run("/v1/ileap/hocs", args.limit, 0, &args.filter),
                    output,
                );
                return Ok(());
            }
            run_list(args.yes, args.max_pages, args.limit, output, |off| {
                client.hocs(args.limit, off, &args.filter)
            })
            .await?;
        }

        Command::Tad {
            cmd: ListCmd::List(args),
        } => {
            if args.dry_run {
                output::print_value(
                    &client.list_dry_run("/v1/ileap/tad", args.limit, 0, &args.filter),
                    output,
                );
                return Ok(());
            }
            run_list(args.yes, args.max_pages, args.limit, output, |off| {
                client.tad(args.limit, off, &args.filter)
            })
            .await?;
        }

        Command::Aed {
            cmd: ListCmd::List(args),
        } => {
            if args.dry_run {
                output::print_value(
                    &client.list_dry_run("/v1/ileap/aed", args.limit, 0, &args.filter),
                    output,
                );
                return Ok(());
            }
            run_list(args.yes, args.max_pages, args.limit, output, |off| {
                client.aed(args.limit, off, &args.filter)
            })
            .await?;
        }

        Command::Auth { .. } => {
            unreachable!("auth command is handled before run_cmd")
        }
    }

    Ok(())
}

pub(crate) async fn run_list<F, Fut, E>(
    yes: bool,
    max_pages: Option<u32>,
    limit: Option<u32>,
    output: &OutputFormat,
    fetch: F,
) -> Result<()>
where
    F: Fn(u32) -> Fut,
    Fut: Future<Output = std::result::Result<Value, E>>,
    E: Into<anyhow::Error>,
{
    let non_interactive = yes || !std::io::stdin().is_terminal();

    if non_interactive {
        let mut pages: Vec<Value> = vec![];
        let mut offset = 0u32;
        let mut page_num = 0u32;
        loop {
            let value = fetch(offset).await.map_err(Into::into)?;
            let at_boundary = limit.is_some_and(|l| item_count(&value) == l as usize);
            page_num += 1;
            pages.push(value);
            if !at_boundary || max_pages.is_some_and(|mp| page_num >= mp) {
                break;
            }
            let Some(l) = limit else {
                break;
            };
            offset += l;
        }
        output::print_value(&merge_pages(pages), output);
    } else {
        let mut offset = 0u32;
        let mut page_num = 0u32;
        loop {
            let value = fetch(offset).await.map_err(Into::into)?;
            page_num += 1;
            let at_max = max_pages.is_some_and(|mp| page_num >= mp);
            if !print_page(&value, limit, output)? || at_max {
                break;
            }
            let Some(l) = limit else {
                break;
            };
            offset += l;
        }
    }
    Ok(())
}

fn merge_pages(mut pages: Vec<Value>) -> Value {
    if pages.len() == 1 {
        return pages.remove(0);
    }
    let mut all_data: Vec<Value> = vec![];
    let mut is_object = false;
    for page in &pages {
        match page {
            Value::Object(obj) => {
                is_object = true;
                if let Some(Value::Array(data)) = obj.get("data") {
                    all_data.extend(data.iter().cloned());
                }
            }
            Value::Array(arr) => all_data.extend(arr.iter().cloned()),
            _ => {}
        }
    }
    if is_object {
        serde_json::json!({"data": all_data})
    } else {
        Value::Array(all_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn merge_pages_single_passthrough() {
        let page = json!({"data": [{"id": "a"}]});
        assert_eq!(merge_pages(vec![page.clone()]), page);
    }

    #[test]
    fn merge_pages_multiple_object_format() {
        let p1 = json!({"data": [{"id": "a"}, {"id": "b"}]});
        let p2 = json!({"data": [{"id": "c"}]});
        let merged = merge_pages(vec![p1, p2]);
        let items = merged["data"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0]["id"], "a");
        assert_eq!(items[2]["id"], "c");
    }

    #[test]
    fn merge_pages_multiple_array_format() {
        let p1 = json!([{"id": "a"}]);
        let p2 = json!([{"id": "b"}]);
        let merged = merge_pages(vec![p1, p2]);
        let items = merged.as_array().unwrap();
        assert_eq!(items.len(), 2);
    }
}

#[cfg(test)]
mod run_list_tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU32, Ordering::SeqCst};
    use std::sync::Arc;

    /// One page with fewer items than limit → exactly 1 fetch (short last page stops immediately).
    #[tokio::test]
    async fn run_list_stops_on_partial_last_page() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let fetch = move |_off: u32| {
            let c = c.clone();
            async move {
                c.fetch_add(1, SeqCst);
                Ok::<_, anyhow::Error>(json!({"data": [{"id": "a"}]})) // 1 item < limit 5
            }
        };
        run_list(true, None, Some(5), &OutputFormat::Compact, fetch)
            .await
            .unwrap();
        assert_eq!(counter.load(SeqCst), 1);
    }

    /// Three pages: offset 0 → 2 items, offset 2 → 2 items, offset 4 → 1 item → 3 fetches.
    #[tokio::test]
    async fn run_list_paginates_until_short_page() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let fetch = move |off: u32| {
            let c = c.clone();
            async move {
                c.fetch_add(1, SeqCst);
                let page = match off {
                    0 => json!({"data": [{"id": "a"}, {"id": "b"}]}),
                    2 => json!({"data": [{"id": "c"}, {"id": "d"}]}),
                    _ => json!({"data": [{"id": "e"}]}), // 1 item < limit 2 → stop
                };
                Ok::<_, anyhow::Error>(page)
            }
        };
        run_list(true, None, Some(2), &OutputFormat::Compact, fetch)
            .await
            .unwrap();
        assert_eq!(counter.load(SeqCst), 3);
    }

    /// max_pages=Some(2) with always-full pages (2 items, limit=2) → exactly 2 fetches.
    #[tokio::test]
    async fn run_list_max_pages_caps_fetches() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let fetch = move |_off: u32| {
            let c = c.clone();
            async move {
                c.fetch_add(1, SeqCst);
                Ok::<_, anyhow::Error>(json!({"data": [{"id": "a"}, {"id": "b"}]}))
            }
        };
        run_list(true, Some(2), Some(2), &OutputFormat::Compact, fetch)
            .await
            .unwrap();
        assert_eq!(counter.load(SeqCst), 2);
    }

    /// limit=None → boundary check is always false → exactly 1 fetch (guards against infinite loop).
    #[tokio::test]
    async fn run_list_no_limit_single_fetch() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let fetch = move |_off: u32| {
            let c = c.clone();
            async move {
                c.fetch_add(1, SeqCst);
                Ok::<_, anyhow::Error>(json!({"data": [{"id": "a"}, {"id": "b"}]}))
            }
        };
        run_list(true, None, None, &OutputFormat::Compact, fetch)
            .await
            .unwrap();
        assert_eq!(counter.load(SeqCst), 1);
    }
}
