# ADR-0006: Token cache hardening

## Status

Proposed (2026-06-11) — **implemented** in
[PR #13](https://github.com/ileap-initiative/ileap-skill/pull/13)
(branch `adr-0006-token-cache-hardening`). Promotes backlog candidate **C4**
and widens it with two related defects found in review. See
**Implementation** at the end of this document.

## Context

The CLI caches bearer tokens on disk between invocations. Three independent
weaknesses share one decision frame: *how much do we trust the token cache,
given this is an internal tool?*

**Fact — cache-path collisions, including across schemes (`auth.rs:14-23`).**
`token_file` strips `https://` / `http://` and maps all of `/ : . -` to `_`:

```rust
let name = base_url
    .trim_start_matches("https://")
    .trim_start_matches("http://")
    .replace(['/', ':', '.', '-'], "_");
```

Consequences: `http://api.example.com`, `https://api.example.com`, and
`https://api-example-com` all resolve to `token_api_example_com`. A token
obtained over plain HTTP is silently presented to the HTTPS endpoint of the
same host (and vice versa).

**Fact — token file written with umask permissions (`auth.rs:31`).**
`save_token` uses `std::fs::write(&path, token)`; no
`set_permissions`/`PermissionsExt` exists anywhere in `src/` (verified by grep
2026-06-11). On a typical umask the file is `0644` — readable by every local
user.

**Fact — cache can land in the current working directory (`auth.rs:19-21`).**
When `dirs::config_dir()` returns `None` (misconfigured `$HOME`, some
containers), the fallback is `PathBuf::from(".")`, i.e. `./ileap/token_*` —
potentially inside a git repository, where it can be committed.

**Fact — storage is plaintext (C4, `auth.rs:25-33`).** The raw token string is
the entire file content; no encryption, no OS keychain.

**Inference — threat model.** The tool targets an internal preview server
(default `https://ileap-preview.fly.dev`, `cli.rs:7`) with short-lived JWTs
(expiry is checked on load, `auth.rs:42-66`). The realistic risks are the
mundane ones: another local user reading a `0644` file, a token committed from
the CWD fallback, and the scheme collision weakening the value of HTTPS.

## Decision

**Accept plaintext file storage** (resolving C4: no keychain, no encryption —
scope-limited to an internal tool with short-lived tokens), **but fix the three
defects around it:**

1. **Include the scheme in the cache-file name.** Stop stripping the scheme;
   sanitize the full base URL. `https://api.example.com` →
   `token_https___api_example_com`. Collisions between *different* URLs remain
   theoretically possible with `_`-substitution but no longer across schemes or
   common separator confusions of real-world URLs.
2. **Create the token file with `0600` on Unix.** Use `OpenOptions` with
   `.mode(0o600)` at creation rather than `fs::write` + `set_permissions`
   (avoids the brief world-readable window).
3. **Error instead of falling back to the CWD.** If `dirs::config_dir()` is
   `None`, return an error telling the user to set `$HOME`/`$XDG_CONFIG_HOME`.
   A surprising file in the repo is worse than a clean failure.

## Considered Options

- **A — Keychain storage (`keyring` crate).** Best at-rest security; adds a
  platform-dependent native dependency, complicates headless/CI use (the main
  non-interactive consumer passes `--token`/`ILEAP_TOKEN` anyway), and is
  over-engineered for short-lived internal tokens. **Rejected.**
- **B — Plaintext file, hardened (chosen).** Smallest change that removes the
  actual defects; keeps the cache greppable/debuggable, fitting the project's
  "almost naive" style.
- **C — Status quo with documented acceptance.** C4 contemplated this, but two
  of the three findings (collision, CWD fallback) are bugs rather than posture;
  accepting them buys nothing.

## Consequences

**Positive**

- Tokens are no longer readable by other local users, no longer shared across
  `http`/`https`, and can no longer materialize inside a repository.
- C4 is resolved with an explicit, recorded scope statement.

**Negative / risks**

- **One-time cache invalidation:** the new file-naming scheme orphans existing
  cached tokens. Users re-run `ileap auth login` once. Old `token_*` files are
  not migrated or deleted (they expire into harmless garbage; document this).
- `0600` enforcement is Unix-only (`#[cfg(unix)]`); on Windows the file
  inherits default ACLs, which are per-user anyway.

**Neutral**

- Token file content and load/expiry logic (`load_saved_token`, `jwt_exp`)
  are unchanged.

## Changes (for coding agent)

All changes in `src/auth.rs`.

1. **`token_file` (`auth.rs:14-23`):** delete the two `trim_start_matches`
   calls; sanitize the full URL. Return `Result<PathBuf, CliError>` (or
   `anyhow::Result`, matching the file's current style above the `CliError`
   boundary) and replace the `unwrap_or_else(|| PathBuf::from("."))` with an
   error: `"cannot determine config directory; set HOME or XDG_CONFIG_HOME"`.
   Callers (`save_token`, `load_saved_token`) propagate with `?`.
2. **`save_token` (`auth.rs:25-33`):** replace `std::fs::write` with:

   ```rust
   let mut opts = std::fs::OpenOptions::new();
   opts.write(true).create(true).truncate(true);
   #[cfg(unix)]
   {
       use std::os::unix::fs::OpenOptionsExt;
       opts.mode(0o600);
   }
   opts.open(&path)?.write_all(token.as_bytes())?;
   ```

   Keep the existing `.with_context(...)` messages.
3. **Tests:** extend the existing `auth.rs` test module: (a) `token_file`
   yields *different* paths for `http://x` vs `https://x`; (b) on Unix, a
   saved token file has mode `0o600` (gate with `#[cfg(unix)]`).
4. **Docs:** mention the one-time re-login in the changelog/README if one
   exists; do not write migration code.

### Verify

```
cargo build && cargo clippy --all-targets -- -D warnings && cargo test
```

Manually: `ileap auth login` then `ls -l ~/.config/ileap/` shows `-rw-------`
and a scheme-prefixed filename.

## Implementation

Implemented on branch `adr-0006-token-cache-hardening`
([PR #13](https://github.com/ileap-initiative/ileap-skill/pull/13), 2026-06-11), based on
`main` at `42897cf`. Verified: `cargo build` pass; `cargo clippy --all-targets
-- -D warnings` clean; `cargo test` pass (**34 unit + 10 integration, 0
failed**), including the two new tests from §3: `token_file_distinguishes_schemes`
and `save_token_sets_owner_only_permissions` (`#[cfg(unix)]`).

**Files:** `src/auth.rs` only, as planned.

**Deviations from the Changes text:** none of substance.
- §1: `token_file` returns `anyhow::Result<PathBuf>` (the file's style above
  the `CliError` boundary, as the ADR allowed); test call sites gained
  `.unwrap()`.
- §2: implemented with `opts.open(&path)` +
  `std::io::Write::write_all` exactly as sketched; the existing
  `.with_context(...)` message is preserved.
- §4 (docs): no changelog/README-for-users file exists in the repo, so the
  one-time re-login note lives in the commit message and PR body only.

**Resulting filename format** (example): `https://api.example.com` →
`token_https___api_example_com`.
