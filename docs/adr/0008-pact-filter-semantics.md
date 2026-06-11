# ADR-0008: PACT `$filter` — error on multiple `-f` expressions

## Status

Proposed (2026-06-11). Promotes backlog candidate **C3**.

## Context

**Fact — only the first `-f` is forwarded for PACT `footprints`
(`client.rs:281-285`, re-verified 2026-06-11).**

```rust
// PACT uses OData $filter; only a single expression is supported
if let Some(f) = filter.first() {
    params.push(("$filter".into(), f.clone()));
}
```

The dry-run twin mirrors this (`client.rs:250-252`). iLEAP-standalone
endpoints forward *all* `-f` values as key=value pairs (`get_kv_filters`,
`client.rs:218,227`).

**Fact — the drop is silent at runtime.** `ileap footprints list -f a -f b`
applies only `a`, returns a superset of what the user asked for, and exits 0.
Only `--dry-run` reveals it.

**Fact — the limitation is intentional** (in-code comment above) but reaches
neither the user at runtime nor the help text: `cli.rs:115-124` documents the
two filter syntaxes and says "repeatable" without scoping repeatability to the
iLEAP endpoints.

**Fact — the signature over-promises (`client.rs:278-281`).**
`footprints(..., filter: &[String])` accepts plural; the body uses singular.

## Decision

**Error when more than one `-f` is passed to a PACT endpoint** (option (c)
from C3), and make the signature say what the function does:

1. `footprints` list (and its dry-run) validates `filter.len() <= 1` and
   returns a usage-style error otherwise, naming the dropped expressions and
   suggesting a single combined OData expression
   (`-f "a and b"` — OData `and` is valid *user-side* syntax inside one
   expression; the user writes it, we don't synthesize it).
2. Change the `Client::footprints` / `footprints_dry_run` parameter from
   `&[String]` to `Option<&str>`; the (single) validation lives at the
   dispatch layer in `commands.rs`, where `ListArgs` is in scope.
3. Extend the `-f` help text (`cli.rs:115-124`): "repeatable for iLEAP
   endpoints; PACT `footprints` accepts at most one OData expression —
   combine conditions with `and`."

## Considered Options

- **(a) Document only.** Zero code risk, but the silent superset result
  remains — the failure mode is *wrong data with exit 0*, the worst kind for a
  tool whose output feeds analyses. **Rejected.**
- **(b) Join multiple `-f` with `" and "` server-side.** Convenient, but the
  CLI would synthesize OData syntax it cannot validate, and the in-code
  comment asserts the server supports only a single expression — joining may
  just move the silent failure server-side. **Rejected** (revisit if the
  server's OData support is confirmed broader).
- **(c) Error on multiple `-f` (chosen).** Loud, naive, and reversible toward
  (b) later without breaking anyone (erroring → accepting is non-breaking;
  the reverse is not).

## Consequences

**Positive**

- The silent-data-loss failure mode is gone; users get an actionable error.
- The `Option<&str>` signature makes the single-expression constraint visible
  at every call site and in the type.

**Negative / risks**

- Scripts that (unknowingly, incorrectly) passed multiple `-f` to
  `footprints` now fail loudly. That is the point, but it is a behavior
  change; note it in the changelog.

**Neutral**

- iLEAP key=value filtering (`get_kv_filters`) is untouched.
- Exit-code taxonomy: reuse `CliError::Other` (exit 1) or clap-level
  validation (exit 2) — implementer's choice; prefer the dispatch-layer
  `CliError` so `--dry-run` shares the same check.

## Changes (for coding agent)

1. **`src/commands.rs`** (footprints dispatch arm): before calling
   `client.footprints(...)` or `client.footprints_dry_run(...)`, validate:

   ```rust
   if args.filter.len() > 1 {
       return Err(CliError::Other(format!(
           "PACT footprints accepts at most one --filter; got {}. \
            Combine conditions in one OData expression, e.g. -f \"{} and {}\"",
           args.filter.len(), args.filter[0], args.filter[1]
       ))
       .into());
   }
   let filter = args.filter.first().map(String::as_str);
   ```

2. **`src/client.rs:248-252` and `:278-285`:** change `filter: &[String]` to
   `filter: Option<&str>`; body becomes `if let Some(f) = filter { ... }`.
   Keep the OData comment. The `Get`/`footprint` path is unaffected.
3. **`src/cli.rs:115-124`:** amend the `-f` doc comment per Decision §3.
4. **Tests:** unit test in `commands.rs` for the >1-filter error (message
   mentions both expressions); adjust any existing `client.rs` tests for the
   new signature; add a wiremock assertion that exactly one `$filter` param is
   sent.

### Verify

```
cargo build && cargo clippy --all-targets -- -D warnings && cargo test
```

Manually: `ileap footprints list -f "a eq 1" -f "b eq 2"` exits non-zero with
the combined-expression hint; single `-f` and `--dry-run` behave as before.
