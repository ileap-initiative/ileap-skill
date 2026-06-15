# ADR-0002: Remove the interactive REPL; bare `ileap` prints help

## Status

Accepted — **implemented** in the working tree on `martin/adrs` (uncommitted as
of 2026-06-11). See **Implementation** at the end of this document.

## Context

Invoking `ileap` with **no subcommand** drops into an interactive, menu-driven
REPL (`src/repl.rs`, 143 lines), dispatched from the `None` arm of `run()` in
`src/main.rs:59-86`.

**Fact — it is TTY-only, never scriptable.** The `None` arm bails immediately
unless stdin is a terminal (`src/main.rs:60`):

```rust
if !std::io::stdin().is_terminal() {
    anyhow::bail!("no command provided and stdin is not a terminal — use a subcommand ...");
}
```

So the feature cannot be piped or scripted. It is purely a human-at-a-terminal
affordance. For every automated or programmatic use case (dashboards, reporting,
CI), it is already unreachable and contributes nothing.

**Fact — it duplicates the subcommand path, and the copies have already
drifted.** The REPL reimplements the pagination loop inline six times
(`src/repl.rs:58-135`) instead of reusing `commands::run_list`
(`src/commands.rs:124`). The two implementations are *not* equivalent:
`run_list` honours `--max-pages`; the REPL loop does not. Two implementations of
"the same" paging guarantee future divergence.

**Fact — it carries a latent panic.** Each REPL branch does
`offset += limit.unwrap()` (`src/repl.rs:66, 85, 97, 109, 121, 133`). `limit` is
`Option<u32>` and is `None` when the user presses Enter at the limit prompt
(`prompt_limit`, `src/repl.rs:9-16`). The program is only saved from panicking by
an incidental control-flow detail: `print_page`'s boundary check is
`limit.is_some_and(...)`, which is `false` for `None`, so the loop breaks before
reaching the `unwrap`. This is safety-by-accident, not by design.

**Fact — it is effectively untested.** No unit or integration test exercises the
REPL (verified: no test references `repl`, `is_terminal`, or the no-command
path). Interactive TTY loops require a PTY harness to test, which the project
does not have.

**Fact — it is the *only* interactive credential prompt in the whole tool.**
This is the consequential, non-obvious part. `ileap auth login` does **not**
prompt: with no `--token`, no cached token, and no `--username/--password`, it
returns `credential_error` (`src/auth.rs:111-121`). The only place the tool ever
prompts for `Username:`/`Password:` is the REPL entry arm
(`src/main.rs:68-69, 77-78`). Removing the REPL therefore removes interactive
credential entry from the product unless we add it back elsewhere. (Note: the
`ileap` skill doc claims "To authenticate interactively: `ileap auth login`"
— that is inaccurate against the current code.)

**Inference — why the REPL exists.** Discovery and onboarding: the menu
(`src/repl.rs:44-51`) teaches the resource taxonomy (PACT footprints vs. iLEAP
standalone TOC/HOC/TAD/AED) to a newcomer who has not read `--help`.

This ADR settles whether the REPL earns its place in an automation-first
internal tool, and what bare `ileap` should do instead.

## Decision

**Remove the interactive REPL. Make bare `ileap` (no subcommand) print the clap
help text and exit 0, matching the convention of tools like `gh`.**

1. Delete `src/repl.rs` and its `mod repl;` declaration.
2. Replace the `None` arm of `run()` so that, regardless of whether stdin is a
   terminal, it prints long help to stdout and returns success — no auth, no
   REPL. This also removes the `stdin().is_terminal()` branch entirely.

### Companion decision — interactive auth (decided: A2)

Because removal deletes the tool's only interactive credential prompt, we must
choose how humans authenticate without canned credentials. **Decision: A2 —
preserve interactive auth by moving the prompt into `auth login`** (the
`gh auth login` pattern): when `auth login` has no token/cache/flags **and**
stdin is a TTY, prompt for username/password via `tty::prompt` /
`tty::prompt_password` instead of erroring. This keeps `tty::prompt_password`
alive and restores the lost capability in its semantically correct home (~10
lines). The rejected alternative (A1) was to accept the regression and require
`--username/--password`, env vars, or `--token` — simpler, but a real UX loss
for ad-hoc human use.

## Considered Options

- **A — Remove the REPL; bare `ileap` prints help (chosen).** Deletes 143 lines
  of duplicated, untested, drift-prone code and a latent panic; collapses the
  tool to a single, composable, Unix-style execution model. Behavioural break
  for anyone typing bare `ileap` expecting a menu (low impact for an internal
  tool). Requires the companion auth decision above.
- **B — Keep the REPL but make it delegate to `commands::run_cmd`/`run_list`.**
  Removes the duplication, the `--max-pages` divergence, and the `unwrap` panic
  while preserving the discovery affordance and interactive auth. **Rejected as
  the primary path** because it retains a second execution mode and an
  untestable TTY surface for marginal benefit in an automation-first tool — but
  it is the right fallback if the team values the onboarding menu.
- **C — Status quo.** Rejected: keeps duplication, the latent panic, the
  untested surface, and a TTY-only mode that the documented use cases never
  touch.

## Consequences

**Positive**
- Removes 143 lines, a latent `unwrap` panic, and a parallel pagination
  implementation that has already drifted from `run_list`.
- One execution model (stateless subcommands) — easier to document, test, and
  reason about; composes cleanly in pipelines and the dashboard/reporting flows.
- Raises the tested fraction of the codebase without writing a test, by deleting
  the only untested module.
- Bare `ileap` behaviour matches user expectation set by `gh`, `cargo`, etc.

**Negative / risks**
- Interactive credential entry moves from bare `ileap` to `ileap auth login`
  (companion decision A2). Net capability is preserved, but the entry point
  changes — the skill doc and README must be updated to reflect it (and the
  skill's existing "interactive `auth login`" claim becomes correct).
- Loss of the onboarding/discovery menu. *Inference:* substitutable by `--help`,
  `--dry-run`, and a few example invocations in the README/skill.
- Behavioural break: bare `ileap` on a TTY no longer launches a menu.

**Neutral**
- `src/tty.rs` and `src/pager.rs` are **not** removed: `tty::prompt` is still
  used by `pager::print_page` (`src/pager.rs:24`), and `print_page` is shared by
  the non-REPL paging in `commands::run_list` (`src/commands.rs:162`).
- Relates to ADR-0001, whose Context cites "blocking on user input in the REPL"
  as evidence the runtime is sequential. That argument still holds via the
  remaining interactive auth prompt and `print_page`; no contradiction.

## Changes (for coding agent)

1. **Delete** `src/repl.rs`.
2. **`src/main.rs`:** remove the `mod repl;` declaration (`src/main.rs:7`).
3. **`src/main.rs`:** replace the entire `None` arm (`src/main.rs:59-86`) with a
   help-and-exit path. Add `use clap::CommandFactory;` and implement:
   ```rust
   None => {
       Cli::command().print_help()?;
       println!();
   }
   ```
   This removes the `stdin().is_terminal()` check and the interactive
   login-and-REPL block.
4. **`src/main.rs`:** remove now-unused imports flagged by the compiler — at
   minimum the `IsTerminal` import and the `crate::tty` import (the latter only
   if no other `main.rs` reference remains).
5. **Companion auth decision (A2):** in `auth::run_auth`, `AuthCmd::Login`
   (`src/auth.rs:111-121`), change the final `(u, p) => return
   Err(credential_error(u, p))` branch so that, when both are absent **and**
   `std::io::stdin().is_terminal()` is true, it prompts via `tty::prompt` /
   `tty::prompt_password`, then calls `Client::authenticate` and `save_token`
   exactly as the existing `(Some(u), Some(p))` branch does. When stdin is not a
   TTY, retain the `credential_error` path (so scripts still fail fast rather
   than hang). Keep `tty::prompt_password` in `src/tty.rs`.
6. **Docs:** update the `ileap` skill doc and README so the interactive auth
   entry point is `ileap auth login` (now accurate), not bare `ileap`.
7. **Verify** with `cargo build`, `cargo clippy` (catches unused imports), and
   `cargo test`. No existing test asserts the old no-command behaviour, so none
   should break; add integration tests asserting (a) bare `ileap` prints help and
   exits 0, and (b) `auth login` with no creds and non-TTY stdin still returns
   the auth error (exit 4).

## Implementation

Implemented on branch `martin/adrs` (working tree, **uncommitted** as of
2026-06-11). Verified: `cargo build` pass; `cargo test` pass (28 unit + 10
integration, 0 failed). `cargo clippy` introduced no new findings — the single
remaining error (`client.rs:152`, `absurd_extreme_comparisons`) is pre-existing
and unrelated to this change.

**What landed (mapped to Changes above):**
- §1–2: `src/repl.rs` deleted; `mod repl;` removed from `main.rs`.
- §3: `main.rs` `None` arm is now `Cli::command().print_help()?; println!();`
  (added `use clap::CommandFactory;`).
- §4: removed the now-unused `use std::io::IsTerminal;` from `main.rs`.
- §5 (A2): `auth::run_auth` / `AuthCmd::Login` prompts for username + password
  when stdin is a TTY, and returns `credential_error` (exit 4) when it is not.
- §6: `README.md` updated. `SKILL.md` needed **no** change — its "interactive
  `auth login`" claim is now accurate. DOC-DRIFT **D1 and D7 → Resolved**.
- §7: added integration tests `bare_ileap_prints_help_and_exits_0` and
  `auth_login_no_creds_non_tty_stdin_exits_4`.

**Deviation from §4–§5 (post-implementation cleanup):** the prompt helpers were
subsequently renamed `tty` → `prompt` (file `src/tty.rs` → `src/prompt.rs`),
following the "is a dedicated `tty` module necessary?" review (decision: keep the
module, rename for honesty). Consequently the implemented call sites use
`crate::prompt::{prompt, prompt_password}` from `src/prompt.rs`, **not** the
`tty::*` / `src/tty.rs` names written in the Changes section above.
`prompt_password` was retained as §5 requires.
