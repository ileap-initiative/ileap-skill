# ADR-0009: Straighten command dispatch and interactive flow

## Status

Proposed (2026-06-11) — **implemented** (2026-06-15, branch
`adr-0009-straighten-dispatch-and-interactive-flow`). See **Implementation** at
the end of this document. Bundles four review findings that share one theme:
**any single user action should read top-to-bottom in one place** (the
project's "almost naive" style goal). Respects ADR-0003 (Rejected — the 5×
resource dispatch arms stay) and ADR-0004/0005 (Accepted — generic `run_list`
bound and `CliError` hybrid are unchanged).

## Context

**Fact — the credential-resolution chain lives in `main.rs:68-80`, not
`auth.rs`.** The most security-relevant decision in the program — which
credential source wins (`--token` → cached token → username/password →
error) — is written inline in the dispatcher. `auth.rs`, the module named for
this concern, only supplies the pieces.

**Fact — `run_cmd` carries an `unreachable!` arm (`commands.rs:116-118`).**
`Command::Auth` is handled in `main.rs:55-66` before `run_cmd` is called, so
`run_cmd` ends with `unreachable!("auth command is handled before run_cmd")`.
The type (`Command`) permits a state the code forbids; the invariant lives in
a comment, enforced at runtime by a panic.

**Fact — interactive `auth login` discards partial credentials
(`auth.rs:114-137`).** With a TTY, `ileap auth login --username alice` falls
into the wildcard `(u, p)` arm, which prompts for **both** username and
password — the `--username` the user just passed is shadowed and silently
ignored.

**Fact — `print_page` hides a blocking prompt (`pager.rs:20-29`).** A
function named "print" also reads stdin ("Next page? [y/N]") and returns the
user's continue/stop choice. The pagination *decision* is made in a module
named for output.

**Inference — common thread.** Each finding is a responsibility sitting one
module away from its name, so following "what happens when I run X" requires
a file hop or trusting a comment.

## Decision

1. **Move the credential chain into `auth.rs`.** New
   `pub async fn resolve_client(base_url, token, username, password, timeout)
   -> Result<Client, ...>` containing exactly the chain now at
   `main.rs:68-80`. `main.rs` calls it in one line.
2. **Eliminate the `unreachable!` by widening `run_cmd`'s job, not by
   splitting the enum.** `run_cmd` becomes the single dispatcher for *all*
   commands: its `Command::Auth` arm calls `auth::run_auth`, and the client is
   resolved lazily — only the non-auth arms call `auth::resolve_client`.
   `main.rs` shrinks to parse → `run_cmd` → error formatting. (Splitting
   `Command` into `Auth` + `ApiCommand` enums was considered; it buys
   compile-time enforcement at the cost of a clap-derive restructuring — see
   Considered Options.)
3. **Interactive `auth login` prompts only for what is missing.** Provided
   `--username` (or `ILEAP_USERNAME`) is used; only the password is prompted,
   and vice versa.
4. **Move the next-page prompt out of `pager.rs` into the `run_list` loop.**
   `print_page` reduces to printing; the loop asks
   `prompt("Next page? [y/N] ")` itself. With `item_count` already used by
   `run_list`, `pager.rs` shrinks to (or is absorbed by) the pagination code —
   implementer may dissolve the module if nothing remains. Coordinates with
   ADR-0007's loop unification; implement the two together.

## Considered Options

- **Split `Command` into two enums (compile-time fix for the
  `unreachable!`).** Strongest guarantee, but requires restructuring the clap
  derive (nested/flattened subcommands) for one match arm, and the panic-free
  alternative (Decision §2) achieves the same reader-facing result with less
  type machinery. **Rejected** — revisit if a second "handled elsewhere" arm
  ever appears.
- **Leave the chain in `main.rs`, add comments.** Comments don't relocate
  responsibility; `auth.rs` remains a parts bin. **Rejected.**
- **Keep `print_page` but rename to `print_page_and_prompt`.** Honest name,
  same split brain: pagination decisions would still straddle two modules.
  **Rejected.**

## Consequences

**Positive**

- `main.rs` approaches the ideal shape: parse, delegate, format errors.
- The auth story (resolution chain + login + cache) is readable in one file.
- No `unreachable!`; no function whose name undersells its side effects.
- `auth login --username alice` does what it says.

**Negative / risks**

- This is a cross-cutting refactor touching `main.rs`, `commands.rs`,
  `auth.rs`, `pager.rs` — sequence it **after** ADR-0006/0007/0008 land, so
  behavior fixes aren't entangled with structure moves in review.
- `run_cmd`'s signature changes (takes `Cli` parts instead of a pre-built
  `&Client`); the in-file unit tests for dispatch need updating.
- §2 trades compile-time enforcement for runtime simplicity; the invariant
  "auth needs no client" is now expressed by *where* `resolve_client` is
  called.

**Neutral**

- No user-visible behavior change except §3 (partial credentials honored —
  strictly less surprising) and the removal of a panic that never fired.
- Exit codes, JSON output, and the ADR-0005 error boundary are unchanged.

## Changes (for coding agent)

Implement together with or after ADR-0007 (shared `run_list` edits).

1. **`src/auth.rs`:** add `resolve_client(...)` containing the chain from
   `main.rs:68-80` verbatim (token → `load_saved_token` → username/password →
   `credential_error`). In `run_auth`'s login arm (`auth.rs:114-137`), replace
   the wildcard arm body: `let u = match username { Some(u) => u.to_string(),
   None => prompt("Username: ")? };` and likewise for the password with
   `prompt_password`; keep the non-TTY `credential_error` path for whichever
   value is still missing.
2. **`src/commands.rs`:** change `run_cmd` to accept the pieces it needs
   (`base_url`, `token`, `username`, `password`, `timeout`, `cmd`, `output`)
   or a small struct of them; add a real `Command::Auth` arm calling
   `auth::run_auth`; non-auth arms start with
   `let client = auth::resolve_client(...).await?;`. Delete the
   `unreachable!` (`commands.rs:116-118`).
3. **`src/main.rs`:** collapse the `match` at `main.rs:55-83` to a single
   `commands::run_cmd(...)` call. Error-formatting block stays as-is
   (ADR-0005).
4. **`src/pager.rs` / `run_list`:** remove the prompt from `print_page`
   (`pager.rs:20-29`) — it returns `()`; the unified `run_list` loop
   (ADR-0007) calls `prompt("Next page? [y/N] ")` and interprets `y`/`yes`.
   If `pager.rs` is left holding only `item_count` + tests, move both into
   `commands.rs` and delete the module (update `mod` decls in `main.rs`).
5. **Tests:** existing integration tests (exit codes 0/3/4, JSON shapes) are
   the behavioral oracle and must pass unchanged, except: add a test that
   `auth login --username u` with piped stdin fails mentioning the missing
   *password* only (and, if feasible with a PTY harness, that the username is
   not re-prompted — otherwise cover via a unit test on the new prompt-fill
   logic).

### Verify

```
cargo build && cargo clippy --all-targets -- -D warnings && cargo test
```

Manually: `ileap auth login --username alice` (TTY) prompts only
`Password:`; `ileap shipments list -l 2` pages interactively as before;
`grep -rn "unreachable!" src/` is empty.

## Implementation

Implemented on branch `adr-0009-straighten-dispatch-and-interactive-flow`
(2026-06-15), based on `main` at `3c46d08`. Verified: `cargo build` pass;
`cargo clippy --all-targets -- -D warnings` clean; `cargo test` pass
(**36 unit + 12 integration, 0 failed**), with the prior behavioral oracles
(exit codes 0/2/4, JSON shapes, `auto_mode_merges_pages`,
`max_pages_caps_pagination`) passing unchanged.

**Files:** `src/auth.rs` (§1, §3), `src/commands.rs` (§2, §4), `src/main.rs`
(§3 of Changes), `tests/integration.rs` (§5); `src/pager.rs` deleted (§4).

**Material deviation — §4 was already done by ADR-0007.** This ADR was written
against a `pager.rs` that still held the `print_page` "Next page? [y/N]"
prompt. By the time ADR-0007 actually merged ([PR #14], plus commit `c81b7af`
"remove superfluous `yes` command line arg"), interactive paging had been
removed entirely: `run_list` now auto-paginates with no prompt, and `pager.rs`
held only `item_count`. So there was **no prompt to relocate**. §4 reduced to
its fallback clause: `item_count` (+ its tests) moved into `commands.rs` and the
`pager.rs` module was deleted, with `mod pager;` dropped from `main.rs`. No
`prompt("Next page? …")` call was added anywhere — doing so would have
*re-introduced* the interactivity ADR-0007 deliberately removed.

**Other deviations:**
- **§2 — single match, no second function.** `run_cmd(cli: Cli)` matches
  `cli.command` once: `None` prints help, `Auth` calls `run_auth`, and each of
  the six resource arms opens with `let client = auth::resolve_client(...)`.
  This is the literal "non-auth arms call `resolve_client`" design; it keeps the
  ADR-0003 five-arm dispatch intact and needs no `unreachable!` (the only one
  left in `src/` is the legitimate `(Some, Some)` guard inside
  `credential_error`). The duplicated one-line resolve-call per arm is the
  accepted cost of not splitting `Command` (Considered Options).
- **`resolve_client` takes `Option<&str>`, not owned values,** so the six arms
  can each call it without moving `cli`'s fields — and it matches `run_auth`'s
  existing signature shape.
- **§5 test assertion.** The piped-stdin test asserts stderr contains
  `"--password is missing"`. The original draft's stricter "must not mention
  `--username`" was dropped as incorrect: `credential_error(Some, None)`
  deliberately names the username as *present* to explain that the **password**
  is what's missing ("--username provided but --password is missing"). The
  no-re-prompt guarantee for the username is TTY-only and not observable through
  a piped harness; it is covered by the prompt-fill `match` logic in §3.

**New test:** `auth_login_username_only_non_tty_reports_missing_password`
(integration).

[PR #14]: https://github.com/sine-fdn/ileap-cli/pull/14
