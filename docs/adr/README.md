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
| [0006](0006-token-cache-hardening.md) | Token cache hardening | **Proposed** | Accept plaintext storage (resolves C4) but: scheme-aware cache filename, `0600` on Unix, error instead of CWD fallback |
| [0007](0007-pagination-loop-correctness.md) | Pagination loop — correctness and shape | **Proposed** | Reject `--limit 0` at clap; unify the two `run_list` loops; `merge_pages` envelope from first page (absorbs C5) |
| [0008](0008-pact-filter-semantics.md) | PACT `$filter` — error on multiple `-f` | **Proposed** | Error when >1 `-f` for `footprints`; signature `&[String]` → `Option<&str>`; help-text fix (resolves C3) |
| [0009](0009-straighten-dispatch-and-interactive-flow.md) | Straighten command dispatch and interactive flow | **Proposed** (implemented) | Credential chain `main.rs` → `auth::resolve_client`; `run_cmd` dispatches Auth too (no `unreachable!`); `auth login` prompts only for missing credentials; `pager.rs` dissolved into `commands.rs` (the next-page prompt was already gone — ADR-0007 removed interactive paging) |
| [0010](0010-ileap-skill-microsite.md) | Minimal Zola microsite for the iLEAP skill | **Proposed** | First non-code ADR. Zola SSG (search off) under `site/`, vendor-neutral, audience = decarb end-users, CTA = install; three demos (dashboard, scenario planner, TOC/HOC decarb table) pre-generated + committed in `site/static/demos/` with preview / SINE+SFC iLEAP-Initiative / may-vary disclaimers; brand styling (blue `#006c9e`, logo, system fonts) from ileap.global; all copy uses `ileap` (rename) |

**Implementation order** (0001+0002 accepted as baseline): **0004 → 0005**
(0003 is **Rejected**). 0004 generalizes `run_list`'s error bound; 0005's typed
`Client` errors depend on that bound to compile. 0004 and 0005 are otherwise
independent of the accepted baseline; only 0005 touches baseline code (the
`auth login` prompt path added by 0002, and `main`'s error block).

**For the 2026-06-11 batch:** 0006, 0007, 0008 are independent of each other
and can land in any order. **0009 lands last** — it is a structural refactor
over the same files (`main.rs`, `commands.rs`, `auth.rs`, `pager.rs`) and
shares `run_list` edits with 0007; sequencing it after the behavior fixes keeps
review diffs separable.

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
>
> **2026-06-11 batch: C3 promoted → [ADR-0008](0008-pact-filter-semantics.md);
> C4 promoted → [ADR-0006](0006-token-cache-hardening.md); C5 absorbed by
> [ADR-0007](0007-pagination-loop-correctness.md)** (accept-and-document the
> clones). The entries below are retained for their evidence trail; the ADRs
> own the decisions.

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

## Chores — no ADR needed

Small fixes with no decision content (2026-06-11 review batch). Per the
convention above ("record as a code comment, not an ADR"), these can be done
in any housekeeping PR:

| Chore | Evidence | Fix |
|-------|----------|-----|
| `build_client` swallows builder failure and silently drops the configured `--timeout` | **Fact** (`client.rs:25-31`): `builder.build().unwrap_or_default()` | Propagate the error instead of defaulting |
| Fractional JWT `exp` (valid per RFC 7519 NumericDate) bypasses the expiry check | **Fact** (`auth.rs:35-40`): `as_u64()` returns `None` for `1234.5`; `None` means "never expires" | Fall back to `as_f64()`; do **not** change the no-`exp` (opaque token) behaviour |
| Retry jitter via `subsec_millis` is opaque | **Claim** (`client.rs:157-165`) | Add a one-line comment (or use a named helper) explaining the jitter source |
| Long-form `std::result::Result<V, CliError>` repeated through `client.rs` | **Fact** (e.g. `client.rs:281`) | Local `type Result<T> = std::result::Result<T, CliError>;` alias in `client.rs` |

## Dropped / Won't-ADR candidates

| Candidate | Reason |
|-----------|--------|
| OIDC discovery response validation (`client.rs`) | Edge case; no real-world impact on the internal demo server. Would be relevant if this tool goes production. Record as a code comment, not an ADR. |
| ~~Replacing `anyhow` with `thiserror` for typed errors~~ | **Un-dropped → [ADR-0005](0005-typed-errors-with-thiserror.md)**. Reconsidered at the user's request; resolved as a *hybrid* (thiserror at the client/auth boundary, anyhow above) rather than a full migration. |
| Remove `pub(crate)` from `run_list` (2026-06-11 review) | Contradicts [ADR-0004](0004-client-trait-abstraction.md)'s explicit, Accepted decision. Not worth superseding an ADR for a visibility keyword. |
| Revisit the anyhow/thiserror hybrid (2026-06-11 review) | Decided in [ADR-0005](0005-typed-errors-with-thiserror.md) (Accepted, implemented). No new evidence; not reopened. |
| "Redundant `rt` tokio feature" (2026-06-11 review) | **False claim** from a delegated review: `#[tokio::main(flavor = "current_thread")]` requires the `rt` feature; the set `["macros", "rt", "time"]` was chosen deliberately in [ADR-0001](0001-minimize-tokio-runtime-footprint.md). |
