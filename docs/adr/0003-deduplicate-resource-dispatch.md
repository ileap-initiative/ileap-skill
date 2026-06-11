# ADR-0003: Deduplicate the 5× iLEAP resource dispatch in commands.rs

## Status

Rejected (2026-06-11). The decision below was not adopted; the five explicit
iLEAP dispatch arms in `src/commands.rs` remain as-is (Considered Options "C —
status quo").

**Rationale:** the `macro_rules!` approach hurts transparency, understandability,
and readability. A reader has to mentally expand the macro to see what each arm
actually does, whereas the five explicit arms are immediately self-contained and
greppable. For a small, stable set of resources the modest line savings do not
justify trading away that directness — explicit repetition is the clearer choice
here.

## Context

`src/commands.rs` dispatches six `Command` variants inside `run_cmd`. Five of
them — `Shipments`, `Tocs`, `Hocs`, `Tad`, `Aed` — are structurally identical:

```rust
Command::Tocs { cmd: ListCmd::List(args) } => {
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
```

**Fact — five arms are token-for-token identical modulo the path string and
the client method name.** (`src/commands.rs:36-114`). Every arm:
1. destructures `{ cmd: ListCmd::List(args) }`,
2. guards on `args.dry_run` and delegates to `client.list_dry_run(path, ...)`,
3. falls through to `run_list(args.yes, args.max_pages, args.limit, output,
   |off| client.<method>(args.limit, off, &args.filter))`.

**Fact — the five iLEAP client methods are thin wrappers around one helper.**
`client.shipments`, `client.tocs`, `client.hocs`, `client.tad`, and `client.aed`
each call `self.get_kv_filters(path, limit, offset, filters)` with a fixed path
literal (`src/client.rs:279-322`). Their signatures are identical:
`async fn(limit: Option<u32>, offset: u32, filters: &[String]) -> Result<Value>`.

**Fact — `footprints` is a genuine outlier.** It has its own dispatch arm
(`src/commands.rs:13-34`) that differs in three ways:
- Uses a `FootprintsCmd` subcommand enum (with both `List` and `Get` variants)
  rather than the shared `ListCmd`.
- `dry_run` for `List` calls `client.footprints_dry_run` (which builds an
  OData `$filter` parameter), not `client.list_dry_run`.
- The `Get` sub-arm calls `client.footprint(&id)` and has its own `dry_run`
  flag, which calls `client.footprint_dry_run`.

**Inference — why the five arms look like copy-paste.** The iLEAP standalone
endpoints were added one by one with no abstraction; each was the shortest
incremental change. The PACT footprints arm was first and carries OData
semantics (`$filter`), a distinct URL prefix (`/2/` vs `/v1/ileap/`), and a
`Get` subcommand that the standalone endpoints lack. These differences make
footprints ineligible for the same abstraction as the other five.

**Fact — `run_list` is already fully generic** and accepts an arbitrary
`F: Fn(u32) -> Fut` closure (`src/commands.rs:124-131`). The repetition
lives entirely in the five `run_cmd` match arms, not inside `run_list`.

**Fact — `list_dry_run` is already parameterised by path.** The single
`client.list_dry_run(path, limit, offset, filters)` method
(`src/client.rs:245-259`) handles all five iLEAP endpoints identically. The
`dry_run` path in each arm only differs in the literal string passed for `path`.

This ADR is the companion to ADR-0002. ADR-0002 deleted the equivalent 6×
duplication in `src/repl.rs` (which inline-repeated the pagination loop for
each resource). ADR-0003 addresses the same structural repetition in
`src/commands.rs`, which survived because the REPL removal did not touch it.

The question this ADR settles: how — or whether — to collapse the five
identical iLEAP arms into a single dispatch path.

## Decision

**Use a `macro_rules!` macro to expand the shared arm *body*, and leave
`Command::Footprints` explicit.**

Define an `ileap_list_arm!` macro inside `src/commands.rs` that takes the
client, output, the destructured `args`, a path literal, and a client method
name, and expands to the **body** of a list arm (the dry-run guard + the
`run_list` call). Each of the five `match` arms keeps its explicit pattern
(`Command::X { cmd: ListCmd::List(args) } =>`) but its ~8-line body collapses to
a single macro invocation. `Command::Footprints` stays unchanged.

**Why body-expansion and not arm-expansion:** a `macro_rules!` invocation
**cannot** produce a bare match arm (`PAT => BODY`) to be spliced among other
arms — that is not a valid macro expansion site in Rust. Macros expand in
expression/statement/item/pattern positions; the arm body is an expression, so
the macro must expand there. This is why the five `match` *patterns* remain
explicit while only their bodies are unified. (A macro generating the *entire*
`match cmd { … }` expression is possible but obscures all arms behind one
invocation and is rejected as less readable.)

We explicitly **reject a data-driven function-pointer table** (Option B) and
**reject the status quo** (Option C) — see Considered Options.

## Considered Options

- **A — `macro_rules!` body-expansion (chosen).** A macro parameterised by
  `($client, $output, $args, $path:literal, $method:ident)` expands to the arm
  *body* (dry-run guard + `run_list` call) with zero runtime cost, no heap
  allocation, no lifetime friction, and no change to `run_list`'s signature. The
  five `match` patterns remain explicit (Rust does not allow a macro to produce
  a bare match arm — see Decision); only the ~8-line bodies collapse to a single
  invocation each. The expansion is visible via `cargo expand` when debugging,
  and the macro lives next to its callsites. *Accepted.*
- **B — data-driven table: `(&str, fn(&Client, ...) -> Pin<Box<dyn Future>>)`.** 
  More idiomatic-looking at the call site, but async method pointers over
  `&self` require boxing: `fn(&Client, Option<u32>, u32, &[String]) ->
  Pin<Box<dyn Future<Output=Result<Value>> + '_>>`. Each call allocates a
  `Box`; the lifetime annotation on the `Box` is non-trivial; and each of the
  five thin client methods (`shipments`, `tocs`, …) would need a wrapper shim
  or must be rewritten to return `Pin<Box<…>>` rather than `impl Future`.
  For a stable, five-entry table in a CLI that issues one request at a time,
  the ergonomic and allocation cost buys nothing. *Rejected.*
- **C — status quo (explicit repetition).** Safe and readable when read in
  isolation; any one arm is immediately self-contained. But five identical
  ~10-line blocks create real maintenance surface: adding a new filter flag,
  changing the dry-run output shape, or adding request tracing must be applied
  five times. ADR-0002 already removed the equivalent repetition from the REPL;
  leaving `commands.rs` unaddressed is inconsistent. *Rejected.*

## Consequences

**Positive**
- Five ~12-line match arms shrink to a 2-line pattern + macro invocation each
  (~40 lines of body removed), with one ~12-line macro definition added.
- A new iLEAP resource (e.g. a future DT5 endpoint) requires one line, not
  another copy-paste arm.
- Dry-run logic, `run_list` call shape, and filter semantics have one canonical
  definition; a future change (e.g. adding a `--trace` flag) is applied once.
- Consistent with ADR-0002's direction: the codebase now has a single
  abstraction boundary for iLEAP list dispatch in both the former REPL path
  (now deleted) and the subcommand path.

**Negative / risks**
- `macro_rules!` is less familiar to some Rust engineers than a trait or
  function. The macro's expansion is invisible unless `cargo expand` is used;
  a reader who has not seen the macro definition will not immediately see what
  a one-line callsite expands to. Mitigated by keeping the macro and its
  callsites in the same file and adding a short doc comment.
- If the five endpoints diverge in the future (e.g. one gains a `--since` flag
  with distinct client semantics), the macro must either grow a parameter or
  that arm must be broken out explicitly. This is the correct action and not a
  regression — it is precisely how explicit arms re-emerge when warranted.

**Neutral**
- `Command::Footprints` is unchanged. OData filter semantics,
  `footprints_dry_run`, and the `Get` subcommand remain explicit and are
  unaffected by this change.
- No change to `run_list`, `merge_pages`, `Client`, or any test. The macro
  expands to the same code that exists today; behaviour is identical.
- No change to `src/cli.rs`: the `Command` enum and `ListCmd`/`ListArgs`
  structs are untouched.
- Composes with ADR-0005 (typed errors) at no extra cost **provided ADR-0004's
  generalized `run_list` error bound is in place**: the macro's
  `|off| $client.$method(...)` closure then returns `Result<_, CliError>`, which
  satisfies `E: Into<anyhow::Error>` directly — no `.map_err` shim in the macro.
  If ADR-0005 lands without ADR-0004's bound change, this macro (and the
  pre-existing closures) will not compile. Order: 0004 → 0005 → 0003.

## Changes (for coding agent)

All changes are confined to `src/commands.rs`.

### 1. Define the macro (add before `run_cmd`)

The macro expands to the arm **body** (a block expression), not to a match arm.
It captures `args` by name from the enclosing arm's pattern binding.

```rust
/// Expand the body of a uniform iLEAP list-resource arm.
///
/// Parameters:
///   $client — the &Client
///   $output — the &OutputFormat
///   $args   — the ListArgs bound by the arm's `ListCmd::List(args)` pattern
///   $path   — the API path literal (e.g. "/v1/ileap/shipments")
///   $method — the Client method name (e.g. shipments)
macro_rules! ileap_list_arm {
    ($client:expr, $output:expr, $args:expr, $path:literal, $method:ident) => {{
        if $args.dry_run {
            output::print_value(
                &$client.list_dry_run($path, $args.limit, 0, &$args.filter),
                $output,
            );
            return Ok(());
        }
        run_list($args.yes, $args.max_pages, $args.limit, $output, |off| {
            $client.$method($args.limit, off, &$args.filter)
        })
        .await?;
    }};
}
```

The outer `{{ … }}` makes the expansion a single block expression usable as an
arm body. `return` and `.await?` are valid because the expansion is lexically
inside the `async fn run_cmd`.

### 2. Replace the five arm bodies inside `run_cmd`

Keep each arm's explicit pattern; replace its body
(`src/commands.rs:36-114`) with a macro invocation:

```rust
Command::Shipments { cmd: ListCmd::List(args) } =>
    ileap_list_arm!(client, output, args, "/v1/ileap/shipments", shipments),
Command::Tocs { cmd: ListCmd::List(args) } =>
    ileap_list_arm!(client, output, args, "/v1/ileap/tocs", tocs),
Command::Hocs { cmd: ListCmd::List(args) } =>
    ileap_list_arm!(client, output, args, "/v1/ileap/hocs", hocs),
Command::Tad { cmd: ListCmd::List(args) } =>
    ileap_list_arm!(client, output, args, "/v1/ileap/tad", tad),
Command::Aed { cmd: ListCmd::List(args) } =>
    ileap_list_arm!(client, output, args, "/v1/ileap/aed", aed),
```

This compiles as written: each macro invocation sits in arm-body (expression)
position, which is a valid expansion site. The `Command::X { … }` patterns are
not generated by the macro — Rust does not permit a macro to expand to a bare
match arm.

### 3. Leave `Command::Footprints` and `Command::Auth` untouched

`src/commands.rs:13-34` (footprints, with OData filter and `Get` subcommand)
and `src/commands.rs:116-118` (auth unreachable arm) are not modified.

### 4. Verify

```
cargo build && cargo test && cargo clippy -- -D warnings
```

No test changes are expected: the macro expands to identical code; the
existing `merge_pages` unit tests are unaffected. `cargo clippy` will catch
any unused-import regressions introduced by the refactor.
