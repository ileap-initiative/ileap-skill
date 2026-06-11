# ADR Index & Candidate Backlog

This file is the canonical tally of all ADRs — decided and candidate.
Candidates become ADRs once a decision is ready to record. Update this file
whenever an ADR is added, its status changes, or a candidate is promoted,
merged, or dropped.

**Status values:** Proposed · Accepted · Superseded · Deprecated · Rejected

**Related:** [`../DOC-DRIFT.md`](../DOC-DRIFT.md) tracks documentation-vs-code
mismatches. That ledger defers code-fixable items here (e.g. D1/D7 → ADR-0002)
and never re-states a fix; this index owns the decisions.

---

## Decided ADRs

| # | Title | Status | Key decision |
|---|-------|--------|--------------|
| [0001](0001-minimize-tokio-runtime-footprint.md) | Keep async, minimize tokio runtime footprint | **Accepted** | Replace `features = ["full"]` with `["macros", "rt", "time"]`; use `current_thread` flavour |
| [0002](0002-remove-interactive-repl.md) | Remove the interactive REPL; bare `ileap` prints help | **Accepted** | Delete `repl.rs`; bare `ileap` prints clap help (exit 0); move interactive credential prompt into `auth login` (TTY only) |
| [0003](0003-deduplicate-resource-dispatch.md) | Deduplicate the 5× iLEAP resource dispatch | **Rejected** | Not adopted; the 5 explicit dispatch arms stay. (was C1) |
| [0004](0004-client-trait-abstraction.md) | Do *not* add an `ApiClient` trait; extract pure logic | **Accepted** (implemented) | No trait seam; make `run_list` `pub(crate)` + generalize error bound + add in-process pagination unit tests; retry stays under wiremock (was C2) |
| [0005](0005-typed-errors-with-thiserror.md) | Typed errors with `thiserror` at the client/auth boundary | **Accepted** (implemented) | Hybrid: `CliError` enum (thiserror) in `client`/`auth`; `anyhow` above; `main` maps exit codes via typed match |

**Implementation order** (0001+0002 accepted as baseline): **0004 → 0005**
(0003 is **Rejected**). 0004 generalizes `run_list`'s error bound; 0005's typed
`Client` errors depend on that bound to compile. 0004 and 0005 are otherwise
independent of the accepted baseline; only 0005 touches baseline code (the
`auth login` prompt path added by 0002, and `main`'s error block).

---

## Candidate Backlog

Candidates are not yet ADRs — they are areas where a decision *may be needed*,
ranked by estimated impact. Each entry states what was observed and why it might
warrant a decision record.

Evidence labels: **Fact** = verified at source · **Inference** = reasoned,
not line-verified · **Claim** = delegated output, spot-checked.

---

> **C1 → [ADR-0003](0003-deduplicate-resource-dispatch.md): Rejected** — the
> dispatch dedup was not pursued; the 5 explicit arms stay. **C2 promoted →
> [ADR-0004](0004-client-trait-abstraction.md)** (decision: *do not* add a trait;
> extract pure logic instead).

### C3 — OData filter: only the first `-f` filter is forwarded for PACT `footprints`
**Estimated impact:** Possible silent data-loss bug; low-effort fix.

**Fact (`client.rs:284-285`, re-verified 2026-06-11):** The PACT `footprints`
endpoint does `if let Some(f) = filter.first()` → forwards only the first `-f` to
the OData `$filter` param, silently dropping additional `-f` flags. The dry-run
path mirrors this (`client.rs:250-251`). iLEAP-standalone endpoints loop over all
filters (`get_kv_filters`, `client.rs:218,227`) and handle multiples correctly.

**Fact:** The behaviour is *intentional* and carries an in-code comment
("PACT uses OData $filter; only a single expression is supported",
`client.rs:283`). It is also visible via `--dry-run`, which shows only the first
filter in the request preview.

**Inference:** Intentional, but still silent at runtime — a user passing
`-f a -f b` gets no warning that `b` was dropped; only `--dry-run` reveals it. The
limitation is documented in code but not in user-facing docs.

**Decision needed:** Clarify the intended behaviour; either (a) accept the
single-filter limitation and document it prominently, (b) join multiple `-f`
expressions with ` and ` for OData, or (c) error if more than one `-f` is passed
for `footprints`.

---

### C4 — Tokens are stored in plaintext on disk
**Estimated impact:** Security posture decision; effort varies by platform.

**Fact (`auth.rs`):** `save_token` writes the bearer token to a file under
`dirs::config_dir()` (e.g. `~/.config/ileap/token_<sanitised-url>`). No
encryption or platform keychain is used. File permissions are whatever
`std::fs::write` sets (typically 0o644 on Linux; 0o600 not enforced).

**Inference:** For an internal demo/preview tool this is probably acceptable, but
the decision should be recorded. Options range from explicit acceptance ("plaintext
is fine for this context") to using the OS keychain (`keyring` crate on
macOS/Linux/Windows).

**Decision needed:** Explicitly accept plaintext token storage (with a scope
statement — internal tool, short-lived tokens), or adopt a keychain crate.
Recording the explicit acceptance is valuable even if the decision is "do
nothing."

---

### C5 — `merge_pages` copies all collected JSON values
**Estimated impact:** Low; only matters for large paginated result sets.

**Fact (`commands.rs:175-189`, verified 2026-06-11):** `merge_pages` collects each
page's `data` array into one `Vec<Value>` via `all_data.extend(data.iter().cloned())`
(`commands.rs:186, 189`) — i.e. it deep-clones every record. For a tool fetching at
most a few hundred records this is not a problem; for a future bulk-export feature
it could accumulate significant heap.

**Inference:** Low priority for the current use cases. Only worth an ADR if bulk
export or streaming is planned.

**Decision needed:** Likely "accept and document the limit"; escalate to a
streaming approach only if a bulk-export requirement is added.

---

## Dropped / Won't-ADR candidates

| Candidate | Reason |
|-----------|--------|
| OIDC discovery response validation (`client.rs`) | Edge case; no real-world impact on the internal demo server. Would be relevant if this tool goes production. Record as a code comment, not an ADR. |
| ~~Replacing `anyhow` with `thiserror` for typed errors~~ | **Un-dropped → [ADR-0005](0005-typed-errors-with-thiserror.md)**. Reconsidered at the user's request; resolved as a *hybrid* (thiserror at the client/auth boundary, anyhow above) rather than a full migration. |
