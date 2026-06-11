# ADR-0001: Keep async, minimize the tokio runtime footprint

## Status

Accepted

## Context

The CLI is built on async Rust: `#[tokio::main]`, `async fn` throughout
(`main`, `run`, `run_cmd`, `run_list`, `run_repl`, `run_auth`, and all `Client`
methods), and `tokio = { version = "1", features = ["full"] }` in `Cargo.toml`.

**Fact — how async is actually used.** Every `await` is awaited sequentially.
There is no `tokio::spawn`, `join!`, `select!`, `futures` combinator, or
streaming anywhere in the codebase. The runtime's reason for existing —
concurrency — is not exercised. The async surface reduces to three things:

1. `reqwest`'s async client (`client.rs`) — one request, awaited, returned.
2. `tokio::time::sleep` for retry backoff (`client.rs:166`).
3. `#[tokio::main]` (`main.rs:17`) and the `async` coloring those two propagate
   upward through the call tree.

**Inference — why it is here.** This is convention, not a concurrency
decision: `reqwest`'s primary API is async and the canonical Rust HTTP example
is `#[tokio::main]` + `.await`. Nothing in the program's behaviour (sequential,
one request at a time, blocking on synchronous stdin for the `auth login`
credential prompt and the pager's "Next page?" prompt) requires it. (ADR-0002,
accepted, removed the interactive REPL that was previously the most prominent
such blocking point; this only strengthens the case below, since one of the
async surface's blocking consumers is now gone.)

**Two facts constrain our options:**

- `tokio`'s `"full"` feature pulls in the entire runtime (multi-thread
  scheduler, net, fs, process, signal, sync) when the tool uses roughly three
  features. This inflates compile time and dependency surface for no benefit.
- The alternative of removing async entirely has a low payoff: `reqwest`'s
  `blocking` API still runs a current-thread tokio runtime on a background
  thread, so tokio stays in the dependency tree regardless. Worse, the test
  suite uses `wiremock`, which is async-only (`#[tokio::test]`); going fully
  blocking would force a test-infrastructure migration (→ `mockito`/`httpmock`)
  or retaining tokio as a dev-dependency anyway.

The question this ADR settles: given that the tool does not need async today
but tokio cannot be cheaply removed, what is the right runtime posture?

## Decision

**Keep async, but stop over-provisioning the runtime.**

1. Replace `features = ["full"]` with the minimal set the code uses:
   `features = ["macros", "rt", "time"]`.
   - `macros` → `#[tokio::main]` / `#[tokio::test]`
   - `rt` → the runtime itself (single-threaded; we do **not** need
     `rt-multi-thread`)
   - `time` → `tokio::time::sleep` for backoff
2. Pin the runtime to a single thread, which is the correct flavour for a
   one-request-at-a-time CLI:
   `#[tokio::main(flavor = "current_thread")]`.

We explicitly **reject removing async entirely** (see Considered Options) so the
question does not get re-litigated.

## Considered Options

- **A — Minimize the runtime (chosen).** ~3 lines changed, no test churn, trims
  feature surface and compile time, signals deliberate intent.
- **B — Remove async entirely.** Switch to `reqwest::blocking`, drop all
  `.await`, replace `tokio::time::sleep` with `std::thread::sleep`, de-color the
  call tree. **Rejected:** near-zero runtime benefit (tokio still links via
  `reqwest::blocking`), and it forces a `wiremock` → blocking-mock test
  migration. Cost exceeds benefit.
- **C — Status quo (`features = ["full"]`).** Rejected: pays compile-time and
  dependency cost for capabilities the tool does not use, and leaves intent
  unstated.

## Consequences

**Positive**
- Smaller dependency feature surface; faster clean builds.
- Single-threaded runtime accurately reflects the program's behaviour.
- Intent is now explicit in `Cargo.toml` and `main.rs`, not accidental.
- Async is retained, so a future in-process concurrent fetch (e.g. a single
  `ileap dashboard` command issuing parallel resource requests via `join_all`,
  rather than today's shell-level parallelism) remains a small change.

**Negative / risks**
- `current_thread` flavour means any *future* `tokio::spawn` of CPU-bound work
  would not get a multi-thread scheduler; revisit the flavour if in-process
  concurrency is ever added.
- Marginal: does not reduce binary size (tokio is still linked). The win is
  feature surface, compile time, and clarity — not a smaller binary.

**Neutral**
- No behavioural change to the CLI. No test changes required.

## Changes (for coding agent)

- `Cargo.toml`: change
  `tokio = { version = "1", features = ["full"] }`
  to
  `tokio = { version = "1", features = ["macros", "rt", "time"] }`.
- `src/main.rs:17`: change `#[tokio::main]` to
  `#[tokio::main(flavor = "current_thread")]`.
- Verify with `cargo build` and `cargo test` (the suite uses `#[tokio::test]`,
  which is covered by the `macros` + `rt` features). No source logic changes
  expected.
- If `cargo test` reports a missing tokio feature (e.g. wiremock requiring a
  capability beyond `rt`/`time`), add the minimal missing feature rather than
  reverting to `"full"`, and note it here.
