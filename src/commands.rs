use anyhow::Result;
use clap::CommandFactory;
use serde_json::Value;
use std::future::Future;
use std::time::Duration;

use crate::auth;
use crate::cli::{Cli, Command, FootprintsCmd, ListCmd, OutputFormat};
use crate::error::CliError;
use crate::output;

/// Count the records carried by a list response, whether the server returns a
/// `{"data": [...]}` envelope or a bare array. Used by `run_list` to decide
/// whether a page was full (so another may follow).
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

/// Single dispatcher for every command (ADR-0009). `main` parses and formats
/// errors; the credential chain lives in `auth::resolve_client`, called lazily
/// by exactly the arms that need a client — `auth` and the bare-help case do not.
pub async fn run_cmd(cli: Cli) -> Result<()> {
    let output = cli.output.clone();
    let timeout = cli.timeout.map(Duration::from_secs);

    match cli.command {
        None => {
            Cli::command().print_help()?;
            println!();
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

        Some(Command::Footprints { cmd }) => {
            let client = auth::resolve_client(
                &cli.base_url,
                cli.token.as_deref(),
                cli.username.as_deref(),
                cli.password.as_deref(),
                timeout,
            )
            .await?;
            match cmd {
                FootprintsCmd::List(args) => {
                    // PACT supports a single OData $filter expression (ADR-0008);
                    // reject extra -f flags instead of silently dropping them.
                    if args.filter.len() > 1 {
                        return Err(CliError::Other(format!(
                            "PACT footprints accepts at most one --filter; got {}. \
                             Combine conditions in one OData expression, e.g. -f \"{} and {}\"",
                            args.filter.len(),
                            args.filter[0],
                            args.filter[1]
                        ))
                        .into());
                    }
                    let filter = args.filter.first().map(String::as_str);
                    if args.dry_run {
                        output::print_value(
                            &client.footprints_dry_run(args.limit, 0, filter),
                            &output,
                        );
                        return Ok(());
                    }
                    run_list(args.max_pages, args.limit, &output, |off| {
                        client.footprints(args.limit, off, filter)
                    })
                    .await?;
                }
                FootprintsCmd::Get { id, dry_run } => {
                    if dry_run {
                        output::print_value(&client.footprint_dry_run(&id), &output);
                        return Ok(());
                    }
                    output::print_value(&client.footprint(&id).await?, &output);
                }
            }
        }

        Some(Command::Shipments {
            cmd: ListCmd::List(args),
        }) => {
            let client = auth::resolve_client(
                &cli.base_url,
                cli.token.as_deref(),
                cli.username.as_deref(),
                cli.password.as_deref(),
                timeout,
            )
            .await?;
            if args.dry_run {
                output::print_value(
                    &client.list_dry_run("/v1/ileap/shipments", args.limit, 0, &args.filter),
                    &output,
                );
                return Ok(());
            }
            run_list(args.max_pages, args.limit, &output, |off| {
                client.shipments(args.limit, off, &args.filter)
            })
            .await?;
        }

        Some(Command::Tocs {
            cmd: ListCmd::List(args),
        }) => {
            let client = auth::resolve_client(
                &cli.base_url,
                cli.token.as_deref(),
                cli.username.as_deref(),
                cli.password.as_deref(),
                timeout,
            )
            .await?;
            if args.dry_run {
                output::print_value(
                    &client.list_dry_run("/v1/ileap/tocs", args.limit, 0, &args.filter),
                    &output,
                );
                return Ok(());
            }
            run_list(args.max_pages, args.limit, &output, |off| {
                client.tocs(args.limit, off, &args.filter)
            })
            .await?;
        }

        Some(Command::Hocs {
            cmd: ListCmd::List(args),
        }) => {
            let client = auth::resolve_client(
                &cli.base_url,
                cli.token.as_deref(),
                cli.username.as_deref(),
                cli.password.as_deref(),
                timeout,
            )
            .await?;
            if args.dry_run {
                output::print_value(
                    &client.list_dry_run("/v1/ileap/hocs", args.limit, 0, &args.filter),
                    &output,
                );
                return Ok(());
            }
            run_list(args.max_pages, args.limit, &output, |off| {
                client.hocs(args.limit, off, &args.filter)
            })
            .await?;
        }

        Some(Command::Tad {
            cmd: ListCmd::List(args),
        }) => {
            let client = auth::resolve_client(
                &cli.base_url,
                cli.token.as_deref(),
                cli.username.as_deref(),
                cli.password.as_deref(),
                timeout,
            )
            .await?;
            if args.dry_run {
                output::print_value(
                    &client.list_dry_run("/v1/ileap/tad", args.limit, 0, &args.filter),
                    &output,
                );
                return Ok(());
            }
            run_list(args.max_pages, args.limit, &output, |off| {
                client.tad(args.limit, off, &args.filter)
            })
            .await?;
        }

        Some(Command::Aed {
            cmd: ListCmd::List(args),
        }) => {
            let client = auth::resolve_client(
                &cli.base_url,
                cli.token.as_deref(),
                cli.username.as_deref(),
                cli.password.as_deref(),
                timeout,
            )
            .await?;
            if args.dry_run {
                output::print_value(
                    &client.list_dry_run("/v1/ileap/aed", args.limit, 0, &args.filter),
                    &output,
                );
                return Ok(());
            }
            run_list(args.max_pages, args.limit, &output, |off| {
                client.aed(args.limit, off, &args.filter)
            })
            .await?;
        }
    }

    Ok(())
}

pub(crate) async fn run_list<F, Fut, E>(
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
    let mut pages: Vec<Value> = vec![];
    let mut offset = 0u32;
    let mut page_num = 0u32;
    loop {
        let value = fetch(offset).await.map_err(Into::into)?;
        page_num += 1;

        let more = limit.is_some_and(|l| item_count(&value) == l as usize);
        let at_max = max_pages.is_some_and(|mp| page_num >= mp);

        pages.push(value);

        if !more || at_max {
            break;
        }
        let Some(l) = limit else {
            break;
        };
        offset += l;
    }

    output::print_value(&merge_pages(pages), output);

    Ok(())
}

fn merge_pages(mut pages: Vec<Value>) -> Value {
    if pages.len() == 1 {
        return pages.remove(0);
    }
    // The merged envelope follows the first page. A server that mixes shapes
    // across pages is already misbehaving; we stay lossless but deterministic.
    let is_object = matches!(pages.first(), Some(Value::Object(_)));
    let mut all_data: Vec<Value> = vec![];
    for page in &pages {
        match page {
            Value::Object(obj) => {
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

    /// Mixed shapes across pages (misbehaving server): the envelope follows
    /// the first page; items from all pages are kept.
    #[test]
    fn merge_pages_mixed_shapes_first_page_wins() {
        let object_first = merge_pages(vec![json!({"data": [{"id": "a"}]}), json!([{"id": "b"}])]);
        let items = object_first["data"].as_array().unwrap();
        assert_eq!(items.len(), 2);

        let array_first = merge_pages(vec![json!([{"id": "a"}]), json!({"data": [{"id": "b"}]})]);
        let items = array_first.as_array().unwrap();
        assert_eq!(items.len(), 2);
    }
}

#[cfg(test)]
mod item_count_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn item_count_object_format() {
        assert_eq!(item_count(&json!({"data": [1, 2, 3]})), 3);
    }

    #[test]
    fn item_count_array_format() {
        assert_eq!(item_count(&json!([1, 2])), 2);
    }

    #[test]
    fn item_count_empty_object() {
        assert_eq!(item_count(&json!({"data": []})), 0);
    }

    #[test]
    fn item_count_non_data_value() {
        assert_eq!(item_count(&json!("irrelevant")), 0);
    }
}

#[cfg(test)]
mod filter_validation_tests {
    use super::*;
    use crate::cli::ListArgs;

    /// More than one -f for PACT footprints is an error naming both expressions,
    /// checked before any request (dry_run would short-circuit right after). A
    /// `--token` is supplied so `resolve_client` succeeds without a network call.
    #[tokio::test]
    async fn footprints_list_rejects_multiple_filters() {
        let args = ListArgs {
            filter: vec!["a eq 1".into(), "b eq 2".into()],
            dry_run: true,
            ..Default::default()
        };
        let cli = Cli {
            base_url: "http://filter-test.invalid".into(),
            token: Some("tok".into()),
            username: None,
            password: None,
            output: OutputFormat::Compact,
            timeout: None,
            command: Some(Command::Footprints {
                cmd: FootprintsCmd::List(args),
            }),
        };
        let err = run_cmd(cli).await.unwrap_err();
        let ce = err.downcast_ref::<CliError>().expect("expected CliError");
        assert!(matches!(ce, CliError::Other(_)), "got: {ce:?}");
        let msg = ce.to_string();
        assert!(
            msg.contains("a eq 1") && msg.contains("b eq 2"),
            "error must name the conflicting filters, got: {msg}"
        );
    }
}

#[cfg(test)]
mod run_list_tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering::SeqCst};

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
        run_list(None, Some(5), &OutputFormat::Compact, fetch)
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
        run_list(None, Some(2), &OutputFormat::Compact, fetch)
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
        run_list(Some(2), Some(2), &OutputFormat::Compact, fetch)
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
        run_list(None, None, &OutputFormat::Compact, fetch)
            .await
            .unwrap();
        assert_eq!(counter.load(SeqCst), 1);
    }
}
