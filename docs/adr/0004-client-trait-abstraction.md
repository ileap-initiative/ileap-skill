# ADR-0004: Do not introduce an `ApiClient` trait; extract pure logic instead

## Status

Proposed

## Context

`Client` is a concrete struct wrapping `reqwest::Client` with no trait
abstraction (`src/client.rs:12-16`). Every command test that exercises
HTTP behaviour currently relies on `wiremock` spinning up a real TCP
server inside the process.

**Fact — `Client`'s public async resource methods.**
Seven public async fns, all with the same shape
(`src/client.rs:261-322`):

```
pub async fn footprints(&self, limit, offset, filter) -> Result<Value>
pub async fn footprint(&self, id) -> Result<Value>
pub async fn shipments(&self, limit, offset, filters) -> Result<Value>
pub async fn tocs(&self, limit, offset, filters) -> Result<Value>
pub async fn hocs(&self, limit, offset, filters) -> Result<Value>
pub async fn tad(&self, limit, offset, filters) -> Result<Value>
pub async fn aed(&self, limit, offset, filters) -> Result<Value>
```

There are also matching `*_dry_run` methods that are synchronous and
return `Value` directly without touching the network
(`src/client.rs:233-259`).

**Fact — retry/backoff lives inside the private `get` method.**
`Client::get` (`src/client.rs:124-190`) contains the entire retry loop:
MAX_RETRIES = 2, exponential backoff with jitter, special-casing for
429 and 5xx, and the `tokio::time::sleep` call at line 166. Any fake or
mock that bypasses HTTP also bypasses this logic entirely.

**Fact — `run_cmd` takes `&Client` (concrete), not a trait object.**
`commands::run_cmd` is declared `pub async fn run_cmd(client: &Client,
...)` (`src/commands.rs:11`). Each command arm passes a closure
`|off| client.<method>(args.limit, off, &args.filter)` to `run_list`
(`src/commands.rs:22-24`, `46-48`, `62-64`, `78-80`, `94-96`,
`110-112`).

**Fact — `run_list` and `merge_pages` are already decoupled from
`Client`.**
`run_list` (`src/commands.rs:124-172`) takes a generic `F: Fn(u32) ->
Fut` closure — it has no knowledge of `Client` at all. `merge_pages`
(`src/commands.rs:174-197`) is a pure free function over `Vec<Value>`.

**Fact — `merge_pages` and pagination decisions ARE already unit-tested
without HTTP.**
`src/commands.rs:200-229` contains three synchronous `#[test]` cases
for `merge_pages` (single passthrough, multi-object format,
multi-array format) — no `wiremock`, no `tokio::test`.

**Fact — wiremock tests exist for retry logic at the `Client` level.**
`src/client.rs:342-429` has five `#[tokio::test]` tests covering
retry-on-429, retry-on-500, no-retry-on-404, auth-401 exit code, and
retries-exhausted — all using `wiremock`. These test the behaviour
that lives inside `Client::get`.

**Fact — `run_list` pagination is tested only at subprocess level.**
`tests/integration.rs:128-208` — `auto_mode_merges_pages` and
`max_pages_caps_pagination` use `assert_cmd` + `wiremock` to drive the
full binary. There is no in-process unit test for the `run_list`
pagination loop itself.

**Inference — the untested seam.**
The gap is not `merge_pages` (already tested) or `Client::get` retry
logic (tested via wiremock in `src/client.rs`). It is the `run_list`
pagination loop: when does it stop fetching? Does it respect
`max_pages`? Does it handle a partial last page? These decisions live
entirely in `run_list`'s loop body (`src/commands.rs:141-153`) but can
only be exercised today by standing up a wiremock server and running
`ileap` as a subprocess — a slow, binary-level test.

### The async-fn-in-traits constraint

**Claim** — As of Rust / edition 2024, `async fn` in traits is stable
for *static dispatch* only. A `&dyn ApiClient` requires either the
`async-trait` crate (which boxes every return value as
`Pin<Box<dyn Future + Send>>`) or returning `Pin<Box<...>>` explicitly.
A generic `<C: ApiClient>` avoids boxing but viralizes the type
parameter through `run_cmd`, `run_list`, `main`, and any future
callers.

## Decision

**Do not introduce an `ApiClient` trait. Instead, extract the pure
pagination and merge logic into free functions that can be unit-tested
without any HTTP stack.**

`run_list`'s closure parameter already decouples it from `Client`.
The missing piece is making `run_list` — or an inner function extracted
from it — `pub(crate)` and callable from a `#[test]` with a canned
closure. This gives in-process unit tests for every pagination
decision without touching the trait or the network.

**Interaction with ADR-0005 (typed errors).** While editing `run_list`'s
signature, this ADR also **generalizes its error bound** so the closure may
return *any* error convertible into `anyhow::Error`, not only `anyhow::Result`.
This is required for ADR-0005: that ADR changes the `Client` resource methods to
return `Result<_, CliError>`, and `run_cmd`'s closures pass those futures into
`run_list` with no `?` at the boundary — so the current bound
`Fut: Future<Output = Result<Value>>` (`commands.rs:133`) would fail to compile.
Generalizing the bound here is the single change that lets ADR-0003 (macro) and
ADR-0005 (typed errors) compose without any `.map_err` shims at the call sites.
It is a no-op when ADR-0005 is absent (`anyhow::Result` trivially satisfies the
generalized bound).

`Client::get` retry logic is and should remain covered by wiremock
tests, because it is inherently about HTTP status codes and cannot
meaningfully be tested without a server.

## Considered Options

- **A — `trait ApiClient` + static-dispatch generics.** Define a trait
  with the seven async resource methods. Make `run_cmd` and `run_list`
  generic over `<C: ApiClient>`. Write a `FakeClient` struct in
  `#[cfg(test)]`. **Rejected as primary path:** the type parameter
  viralizes through the call tree; retry logic still needs wiremock; the
  seven-method trait surface is non-trivial to keep in sync with the
  concrete `Client`; for a small, stable API surface this is
  over-abstraction with limited return.

- **B — `#[async_trait]` + `&dyn ApiClient`.** Same trait but
  object-safe via `async-trait`'s boxing. Simpler call sites (no
  generics), but adds a crate dependency, boxes every future (a runtime
  cost on every resource call), and still does not cover retry logic.
  **Rejected for the same reasons as A**, plus the boxing overhead.

- **C — `mockall` auto-derive.** Use the `mockall` crate to derive a
  mock from the trait. Still requires the trait (option A or B first).
  Adds a second dev-dependency and generates code that is harder to
  read than a hand-written fake. **Rejected:** `mockall` adds more
  machinery than a 20-line `FakeClient` struct would; also inherits A/B
  problems.

- **D — Extract pure logic; keep wiremock for the HTTP layer
  (chosen).** No new trait, no new crate dependency, no type-parameter
  viral spread. `run_list`'s closure-based design already supports
  calling it from a test with a hand-written `Fn(u32) -> Fut`. Expose
  `run_list` as `pub(crate)` and add synchronous-or-async unit tests
  for the pagination loop. Retry logic stays under wiremock where it
  belongs. `merge_pages` is already `fn merge_pages` tested in-process.

## Consequences

**Positive**
- No new trait, no new crate dependency, no viral type parameters.
- Pagination decision logic in `run_list` becomes unit-testable
  in-process, with sub-millisecond feedback.
- Retry behaviour in `Client::get` stays under wiremock — the only
  correct test layer for HTTP-status-driven logic.
- `merge_pages` is already a tested pure function; this ADR confirms
  and extends the pattern rather than reversing it.
- Zero refactoring cost to `run_cmd` or `main.rs`.

**Negative / risks**
- `run_cmd` itself (the `match cmd` dispatch) cannot be tested without
  either a real `Client` or a trait seam. If command dispatch logic
  grows substantially (e.g., conditional dry-run paths, fallback
  strategies), the lack of a trait will become a genuine pain point —
  reconsider options A or B at that point.
- `run_list` contains a TTY check (`std::io::stdin().is_terminal()`,
  `src/commands.rs:135`) that affects control flow. Unit tests of
  `run_list` must pass `yes: true` to force the non-interactive branch,
  or mock stdin — an impurity that a trait seam would have hidden.

**Neutral**
- The generalized `run_list` error bound (Changes §1) is the agreed home for
  the fix that lets ADR-0005's typed `Client` errors flow through `run_cmd`'s
  closures. Sequencing: land this ADR before ADR-0005.
- The wiremock dev-dependency is retained and remains the right tool
  for the HTTP layer; this ADR does not reduce that dependency.
- If the team later needs to swap HTTP backends (e.g. for SDK
  generation), introducing a trait at that point is straightforward
  because `run_list` is already closure-generic.

## Changes (for coding agent)

### 1. Expose `run_list` and generalize its error bound

In `src/commands.rs`, make `run_list` `pub(crate)` **and** widen its closure's
error type so it accepts any `Into<anyhow::Error>` (required for ADR-0005; see
Decision):

```rust
// before (src/commands.rs:124-133)
async fn run_list<F, Fut>(
    yes: bool,
    max_pages: Option<u32>,
    limit: Option<u32>,
    output: &OutputFormat,
    fetch: F,
) -> Result<()>
where
    F: Fn(u32) -> Fut,
    Fut: Future<Output = Result<Value>>,

// after
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
```

The body is unchanged: `fetch(offset).await?` (`commands.rs:142,159`) still
compiles because `run_list` returns `anyhow::Result<()>` and `?` applies the
`Into<anyhow::Error>` conversion. With `E = anyhow::Error` (the current callers,
and the state before ADR-0005), the bound is satisfied trivially, so this is
backward-compatible on its own.

### 2. Add unit tests for `run_list` pagination

Add a new `#[cfg(test)]` block below the existing `merge_pages` tests
in `src/commands.rs`. Each test provides a canned async closure that
returns pre-built `Value` pages. Keep `yes: true` to force the
non-interactive branch (sidesteps the `is_terminal` check). Example
skeleton:

```rust
#[cfg(test)]
mod run_list_tests {
    use super::*;
    use serde_json::json;

    async fn pages_fetch(pages: Vec<Value>) -> impl Fn(u32) ->
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>>>>
    {
        let pages = std::sync::Arc::new(pages);
        move |offset: u32| {
            let pages = pages.clone();
            Box::pin(async move {
                // return page matching offset / limit, or empty last page
                Ok(pages
                    .iter()
                    .find(|_| true) // replace with real offset lookup
                    .cloned()
                    .unwrap_or_else(|| json!({"data": []})))
            })
        }
    }

    /// Stops after one page when item count < limit (short last page).
    #[tokio::test]
    async fn stops_on_partial_last_page() {
        let fetch_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let fc = fetch_count.clone();
        let fetch = move |_off: u32| {
            fc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let v = json!({"data": [{"id": "a"}]}); // 1 item < limit 5
            Box::pin(async move { Ok(v) })
                as std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>>>>
        };
        run_list(true, None, Some(5), &OutputFormat::Compact, fetch)
            .await
            .unwrap();
        assert_eq!(fetch_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    /// `--max-pages 1` stops after exactly one fetch even if at boundary.
    #[tokio::test]
    async fn max_pages_1_stops_after_first_fetch() {
        let fetch_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let fc = fetch_count.clone();
        let fetch = move |_off: u32| {
            fc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let v = json!({"data": [{"id": "a"}, {"id": "b"}]}); // exactly limit
            Box::pin(async move { Ok(v) })
                as std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>>>>
        };
        run_list(true, Some(1), Some(2), &OutputFormat::Compact, fetch)
            .await
            .unwrap();
        assert_eq!(fetch_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
```

Adjust the `OutputFormat` import and `Box::pin` wrapper as needed for
the actual types. The goal is to call `run_list` directly — no
`wiremock`, no subprocess.

### 3. Verify

```
cargo test                  # all existing tests still pass
cargo test run_list_tests   # new in-process pagination tests pass
cargo clippy                # no warnings about unused pub(crate)
```

No changes to `src/client.rs`, `src/main.rs`, `Cargo.toml`, or
`tests/integration.rs` are required.
