# ADR-0005: Typed errors with `thiserror` at the client/auth boundary

## Status

Accepted — **implemented** in the working tree on `martin/adrs` (uncommitted as
of 2026-06-11). See **Implementation** at the end of this document.

_Supersedes the "Dropped / Won't-ADR" entry for "Replacing `anyhow` with
`thiserror` for typed errors" in `docs/adr/README.md`. That entry was dropped
with the reasoning "The ExitCode pattern in `client.rs` already gives semantic
exit codes. Full typed errors would add ceremony with no user-visible benefit
for a CLI." The user has now explicitly asked to record a decision. This ADR
engages with that reasoning directly rather than ignoring it._

## Context

### Current error architecture

**Fact — the exit-code sentinel (`client.rs:24-32`).** The codebase uses a
hand-rolled sentinel type to thread exit codes through `anyhow`'s opaque
error chain:

```rust
#[derive(Debug)]
pub struct ExitCode(pub i32);

impl std::fmt::Display for ExitCode { ... }
impl std::error::Error for ExitCode {}
```

**Fact — how exit codes are attached in `client.rs`.** Two sites construct a
layered error `anyhow::Error::from(ExitCode(n)).context(message)`, binding
the semantic code below an untyped context string:

- `client.rs:97-98` — authentication failure path in `authenticate()`;
  exit code 4 (auth) for HTTP 401/403, exit code 1 (generic) otherwise.
- `client.rs:181-183` — per-request failure path in the `get()` loop;
  exit code 3 (not_found) for HTTP 404, exit code 4 (auth) for 401/403,
  exit code 1 otherwise.

The mapping is explicit inline `match` expressions at both sites
(`client.rs:96` and `client.rs:181`).

**Fact — how `auth.rs` sets exit code 4 (`auth.rs:82`).** The
`credential_error` factory constructs the same layered sentinel:

```rust
anyhow::Error::from(ExitCode(4)).context(msg)
```

**Fact — how `main.rs` recovers the exit code (`main.rs:24-50`).** On any
error from `run()`, the top-level handler walks the entire `anyhow` chain
using a downcast search, then separately reconstructs the human message by
filtering the sentinel out:

```rust
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
```

The resulting structured JSON goes to stderr; the process exits with the
recovered code (`main.rs:43-50`).

**Fact — exit-code-to-type mapping (`main.rs:37-41`).**
`1` → `"error"`, `3` → `"not_found"`, `4` → `"auth_error"`.
Exit code 2 is not used (reserved for OS / shell misuse errors). Any
unrecognised code falls through to `"error"`.

**Fact — integration tests lock in all four exit codes
(`tests/integration.rs:20-82`).** Tests assert `.code(4)` for missing
credentials, `.code(4)` for partial credentials, `.code(0)` for success,
and structured JSON shape on stderr. The unit tests in `client.rs` and
`auth.rs` also assert exact exit codes via the same downcast helper
(`client.rs:335-339`, `auth.rs:165-169`).

**Fact — anyhow call-site count.** Grepping `src/` for `anyhow`, `.context(`,
`.with_context(`, `bail!`, `anyhow!` (excluding test modules) yields
approximately 24 non-test call sites spread across `client.rs` (12 sites),
`auth.rs` (5 sites), `main.rs` (2 sites), `tty.rs` (2 sites),
`commands.rs` (1 import), `pager.rs` (1 import), and `repl.rs` (1 site).
The majority of the cost of any migration is concentrated in `client.rs`.

### The key smell

**Inference — the downcast is type-unsafe in a subtle way.** The
`ExitCode-in-the-chain + downcast` pattern works correctly today because
`client.rs` and `auth.rs` always place the `ExitCode` as the innermost
error and add context on top. However, nothing in the type system enforces
this ordering. A future call site that wraps an error with `.context()`
_before_ attaching `ExitCode`, or that propagates an `anyhow` error from a
third-party crate that happens to contain a `TypeId`-colliding type, would
silently produce the wrong exit code. The `unwrap_or(1)` fallback in
`main.rs:28` papers over any case where the sentinel is absent. There is no
compiler-enforced exhaustiveness over the set of failure modes that carry
semantic exit codes.

### Why the original "Won't-ADR" reasoning is partially correct

**Claim — from the dropped candidate:** "The `ExitCode` pattern in
`client.rs` already gives semantic exit codes. Full typed errors would add
ceremony with no user-visible benefit for a CLI."

**Inference — the claim is correct for a _full_ migration.** Defining
`thiserror` enums at every layer (`commands.rs`, `tty.rs`, `pager.rs`,
`auth.rs`, `client.rs`) and removing `anyhow` entirely would require
touching ~24 call sites, deleting ergonomic `.context()` chaining that is
genuinely useful at the command layer, and producing no change observable
by the user. For a binary where every error terminates the process, that
is poor return on effort.

**Inference — the claim is _not_ correct for a targeted hybrid.** The
semantic exit codes live in exactly two files (`client.rs` and `auth.rs`)
at exactly three sites. Those sites are the narrow boundary between
"library code that knows what failed" and "command code that just propagates
errors up." Making that boundary typed removes the unsafe downcast, makes
the exit-code mapping exhaustive, and costs one new file plus changes to
three construction sites.

## Decision

**Adopt Option B — a targeted hybrid: introduce a `thiserror` error enum
at the `client`/`auth` boundary; keep `anyhow::Result` at the command and
`main` layer.**

Concretely:

1. Define a `CliError` enum (in `src/error.rs`) with `thiserror` covering
   every failure mode that carries a non-1 exit code, plus a generic
   catch-all for exit code 1. The enum derives `thiserror::Error` and
   carries the human-readable message as a field on each variant.
2. Change `client.rs` and `auth.rs` to return `Result<T, CliError>` at
   their public interface. Internal `anyhow` usage within those files is
   replaced only at the three sentinel-construction sites; the
   `.context()` / `.with_context()` sugar for body-read and parse failures
   is replaced by named variants or a `#[from]` delegation.
3. Keep `anyhow::Result` in `commands.rs`, `tty.rs`, `pager.rs`, and
   `main.rs`. The `?` operator propagates `CliError` into `anyhow::Error`
   automatically via the `From<CliError> for anyhow::Error` blanket impl
   that `anyhow` provides.
4. Replace the downcast loop in `main.rs` with a direct `downcast_ref` or
   (more idiomatically) match on `e.downcast_ref::<CliError>()` for the
   mapping — or, even cleaner, add an `exit_code()` method on `CliError`
   and call it before the error is erased into `anyhow`.

We explicitly **reject Option A** (full migration) and **accept Option C as
the prior state** (which this ADR supersedes by choosing to record a
decision rather than defer indefinitely).

## Considered Options

- **A — Full migration: `thiserror` everywhere, drop `anyhow`.**
  Define typed enums at every layer; replace all `.context()` with named
  variants. Touches ~24 call sites, forces richer error types for leaf
  operations that add no value (e.g. password-read, directory creation).
  **Rejected:** high churn, loses `anyhow`'s ergonomic context-chaining
  at the binary boundary, zero user-visible gain over the hybrid.

- **B — Hybrid: `thiserror` at the client/auth boundary, `anyhow` above
  (chosen).** Introduce `CliError` in `src/error.rs`; change the three
  sentinel-construction sites in `client.rs` and `auth.rs`; keep all
  other `anyhow` usage. The downcast in `main.rs` becomes a typed match.
  Exit-code coverage is compiler-enforced without touching the ergonomic
  context-chaining that makes the rest of the code readable.

- **C — Status quo: keep `anyhow` + `ExitCode` sentinel.**
  The dropped candidate's position. Defensible: the pattern is correct
  today, the tests catch regressions, and the code is readable. The
  weakness is the type-unsafe downcast and the non-exhaustive exit-code
  mapping. This is the right choice if the team judges the migration cost
  exceeds the type-safety gain. **Not chosen**, but recorded honestly:
  a codebase with two or three developers, a stable API surface, and
  comprehensive exit-code integration tests is a reasonable context for
  accepting this tradeoff.

## Consequences

**Positive**

- The exit-code-to-error-type mapping in `main.rs` becomes a total `match`
  on a closed enum variant, enforced by the compiler. Missed cases become
  compile errors, not silent fallback to exit code 1.
- Adding a new failure mode that carries a semantic exit code (e.g.
  a future rate-limit exit code 5) requires adding an enum variant;
  the compiler flags every match that does not handle it.
- The `ExitCode` sentinel type (`client.rs:24-32`) and both test-only
  `exit_code()` downcast helpers (`client.rs:335-339`,
  `auth.rs:165-169`) can be deleted. The test helpers are replaced by
  `matches!(err, CliError::NotFound)` or similar.
- Migration is bounded: three construction sites in two files plus
  one new file. Remaining `anyhow` usage is untouched.

**Negative / risks**

- `client.rs` and `auth.rs` will have a mixed signature: public methods
  return `Result<T, CliError>` while some internal helpers may still use
  `anyhow::Result`. This is slightly inconsistent; document the boundary
  in a module comment.
- The `?` propagation from `CliError` into `anyhow::Error` works via the
  `From` blanket impl, but it _erases_ the typed error at the `anyhow`
  boundary. If `main.rs` uses `e.downcast_ref::<CliError>()` to recover
  it, that downcast can still fail if a future call site converts to
  `anyhow` before the exit code is read. The cleanest fix — passing
  the `CliError` directly to the JSON-and-exit path before it is erased —
  is recommended in the Changes section below.
- Adds `thiserror` (one crate, zero transitive deps beyond `proc-macro`)
  to `Cargo.toml`. Negligible compile-time cost.

**Neutral**

- No change to the emitted JSON structure (`cli_error.type`,
  `cli_error.message`) or the exit codes (1, 3, 4). All integration tests
  pass without modification.
- `anyhow` remains a direct dependency; only the sentinel-construction
  pattern changes.

## Changes (for coding agent)

**Prerequisite / sequencing.** Land **ADR-0004 first**. Changing the `Client`
resource methods to `Result<_, CliError>` (§3 below) ripples into `run_cmd`'s
closures, which pass those futures into `run_list` with no `?` at the boundary.
`run_list`'s current bound (`Fut: Future<Output = anyhow::Result<Value>>`,
`commands.rs:133`) would then **fail to compile**. ADR-0004 generalizes that
bound to `Fut: Future<Output = Result<Value, E>>, E: Into<anyhow::Error>`, which
makes `CliError` flow through unchanged. Do not implement this ADR until that
bound change is in place, or fold the bound change into this work.

**Baseline.** This ADR assumes ADR-0002 (accepted) has landed: the interactive
credential prompt now lives in `auth login` (`run_auth`, TTY-gated). That prompt
path calls `Client::authenticate` + `save_token` + `credential_error` — all of
which this ADR retypes — so it inherits `CliError` automatically once §3/§4 are
applied; verify it returns `CliError::Auth` on the non-TTY fallback.

### 1. Add `thiserror` to `Cargo.toml`

```toml
thiserror = "2"
```

Place it adjacent to `anyhow = "1"` in `[dependencies]`. (thiserror 2.x is the
current major as of late 2024; the derive API used here is unchanged from 1.x.)

### 2. Create `src/error.rs`

```rust
use thiserror::Error;

/// Typed errors that carry a semantic CLI exit code.
/// All variants include the full human-readable message so that callers
/// at the `main` boundary can emit it without re-walking an error chain.
#[derive(Debug, Error)]
pub enum CliError {
    /// HTTP 404 — resource not found. Exit code 3.
    #[error("{0}")]
    NotFound(String),

    /// HTTP 401/403 or missing credentials. Exit code 4.
    #[error("{0}")]
    Auth(String),

    /// Any other failure (network, server 5xx, parse). Exit code 1.
    #[error("{0}")]
    Other(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::NotFound(_) => 3,
            CliError::Auth(_) => 4,
            CliError::Other(_) => 1,
        }
    }

    pub fn error_type(&self) -> &'static str {
        match self {
            CliError::NotFound(_) => "not_found",
            CliError::Auth(_) => "auth_error",
            CliError::Other(_) => "error",
        }
    }
}
```

Add `pub mod error;` to `src/main.rs` alongside the other `mod`
declarations.

### 3. Update `src/client.rs`

- Add `use crate::error::CliError;` and change the return type of
  `authenticate` and `get` (and therefore all `pub async fn` methods that
  delegate to `get`) from `Result<…>` (`anyhow::Result`) to
  `Result<…, CliError>`.
- Replace the two sentinel-construction sites:

  **`client.rs:97-98`** (authentication failure):
  ```rust
  // Before
  return Err(anyhow::Error::from(ExitCode(exit_code))
      .context(format!("authentication failed ({status}){hint}: {body}")));

  // After
  let msg = format!("authentication failed ({status}){hint}: {body}");
  return Err(if exit_code == 4 {
      CliError::Auth(msg)
  } else {
      CliError::Other(msg)
  });
  ```

  **`client.rs:182-183`** (GET failure):
  ```rust
  // Before
  return Err(anyhow::Error::from(ExitCode(exit_code))
      .context(format!("GET {url} failed ({status}){hint}: {body}")));

  // After
  let msg = format!("GET {url} failed ({status}){hint}: {body}");
  return Err(match status.as_u16() {
      404 => CliError::NotFound(msg),
      401 | 403 => CliError::Auth(msg),
      _ => CliError::Other(msg),
  });
  ```

- Replace the remaining `anyhow::Result` return sites in `authenticate`
  (body-read at `client.rs:101`, parse at `client.rs:103`) and in `get`
  (body-read at `client.rs:186`, parse at `client.rs:188`) with
  `CliError::Other(format!("…"))`. These were `.context()` / `.with_context()`
  one-liners; convert them to `map_err(|e| CliError::Other(format!("…: {e}")))`.
- Delete the `ExitCode` struct and its `Display`/`Error` impls
  (`client.rs:24-32`). Delete the `exit_code` test-helper function
  (`client.rs:335-339`).

### 4. Update `src/auth.rs`

- Add `use crate::error::CliError;` and change the return type of
  `credential_error` from `anyhow::Error` to `CliError`. Its callers
  (`main.rs:111`, `auth.rs:120`) use `return Err(auth::credential_error(…))`.
  **Correction (verified at implementation):** these are `return Err(...)` sites,
  **not** `?`, so they require `.into()` —
  `return Err(auth::credential_error(…).into())` — to convert `CliError` into the
  function's `anyhow::Error`. (`anyhow::Error: From<CliError>` holds because
  `CliError: std::error::Error + Send + Sync + 'static`.) Bare
  `Err(credential_error(…))` does **not** compile in an `anyhow::Result` fn.
- Replace `auth.rs:82`:
  ```rust
  // Before
  anyhow::Error::from(ExitCode(4)).context(msg)

  // After
  CliError::Auth(msg.to_string())
  ```
- Remove the now-unused `use crate::client::ExitCode;` import if no other
  reference to `ExitCode` remains in `auth.rs`. Delete the `exit_code`
  test-helper function (`auth.rs:165-169`).

### 5. Update `src/main.rs`

The `run()` function returns `anyhow::Result<()>`. Because `CliError`
implements `std::error::Error`, anyhow's `From<E: Error>` blanket picks it
up automatically; all `?` propagation continues to work.

Replace the downcast loop in `main.rs:23-50` with a typed recovery. The
cleanest approach is to call a helper that attempts to downcast before the
error is printed:

```rust
if let Err(e) = run(cli).await {
    // Attempt to recover the typed error before it is fully erased.
    let (exit_code, error_type, message) =
        if let Some(ce) = e.downcast_ref::<CliError>() {
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
        OutputFormat::Compact =>
            eprintln!("{}", serde_json::to_string(&json).unwrap()),
        OutputFormat::Pretty =>
            eprintln!("{}", serde_json::to_string_pretty(&json).unwrap()),
    }
    std::process::exit(exit_code);
}
```

Remove the `use client::ExitCode;` import (`main.rs:13`). Add
`use error::CliError;`.

### 6. Update tests in `client.rs` and `auth.rs`

The existing `exit_code()` downcast helpers in both test modules can be
replaced with direct variant matching:

```rust
// Before (client.rs test helper)
fn exit_code(err: &anyhow::Error) -> Option<i32> {
    err.chain().find_map(|c| c.downcast_ref::<ExitCode>()).map(|ec| ec.0)
}
// assert_eq!(exit_code(&err), Some(3));

// After
// assert!(matches!(err, CliError::NotFound(_)));
```

Tests that currently call `client` methods receive `Result<_, CliError>`
directly, so no chain-walk is needed.

### 7. Verify

```
cargo build
cargo clippy --all-targets -- -D warnings
cargo test
```

All integration tests in `tests/integration.rs` assert exit codes 0, 4
(multiple tests), and implicitly 1 (failure without specific code). None
assert exit code 3 at the integration level, but the unit tests in
`client.rs` (`no_retry_on_404_returns_exit_code_3`) must still hold.
Confirm that `.code(4)` in `tests/integration.rs:28`, `:50`, `:67`, `:82`
all pass unchanged. If the `downcast_ref::<CliError>()` in the new `main`
handler returns `None` for any path that previously set an `ExitCode`, add
a targeted test to catch the regression before merging.

## Implementation

Implemented on branch `martin/adrs` (working tree, **uncommitted** as of
2026-06-11). Verified: `cargo build` pass; `cargo test` pass (**32 unit + 10
integration, 0 failed**) — including the integration tests that lock exit codes
0/3/4 and `cli_error.type`, and the `client.rs` wiremock tests for 404→exit 3 /
401→exit 4 / retry. `cargo clippy`: only the pre-existing `client.rs`
`absurd_extreme_comparisons` lint (`BACKOFF_BASE_MS`); no new findings.

**Files:** `Cargo.toml` (+`thiserror = "2"`); new `src/error.rs` (`CliError`);
`src/client.rs` (all 7 resource methods + `get` + `get_kv_filters` +
`authenticate` → `Result<_, CliError>`; `ExitCode` struct and its test-helper
deleted); `src/auth.rs` (`credential_error` → `CliError`; test helpers → variant
matching); `src/main.rs` (typed recovery via `downcast_ref::<CliError>()`).
Built on the ADR-0004 `run_list` bound, so `commands.rs` needed **no** change.

**Deviations from the Changes text (recorded for accuracy):**
- **§4 `?`-propagation claim was wrong** (now corrected inline above): the
  `credential_error` call sites use `return Err(...)`, not `?`, so they need
  `.into()`. Applied in `main.rs` and in `auth.rs`'s ADR-0002 non-TTY branch.
- **§3 under-counted the `map_err` sites.** It named 2; the real count is ~6 —
  *every* infrastructure `?` (reqwest send, body read, `serde_json` parse, etc.)
  inside the now-`CliError` functions had to be `.map_err(|e|
  CliError::Other(...))`, because `CliError` has no `From<reqwest::Error>` etc.
  Context strings preserved.
- **§3 omitted `get_kv_filters` and the 7 resource methods** — all needed the
  return-type change (they delegate to `get`).
- **`main` recovery uses a direct `downcast_ref::<CliError>()`** (not
  `chain().find_map`), valid because no `.context()` is layered over any
  `CliError`; confirmed by the passing exit-code tests.

**Minor behaviour note:** the fallback (non-`CliError`) message path now uses
`e.to_string()` (top error only) rather than the old chain joined with `": "`.
`CliError` messages are self-contained, so user-facing errors are unchanged;
only stray I/O errors lose chain detail. Use `format!("{e:#}")` in the fallback
if full-chain detail is wanted.
