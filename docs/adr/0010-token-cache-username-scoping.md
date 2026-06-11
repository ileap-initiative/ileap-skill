# ADR-0010: Scope the token cache by username, not just base URL

## Status

Proposed (2026-06-11). Follow-up to
[ADR-0006](0006-token-cache-hardening.md), which hardened the token-cache
*filename* (scheme-aware, `0600`, no CWD fallback) but left it keyed by base URL
alone.

## Context

**Fact — the cache file is keyed by base URL only (`auth.rs:14-21`).**
`token_file(base_url)` sanitizes the full base URL into a single filename
(`token_https___api_example_com`). Username is not part of the key.

**Fact — `save_token`/`load_saved_token` never see a username (`auth.rs:23,48`;
`main.rs:70`).** Both take only `base_url`. The non-auth command path loads the
cache with `auth::load_saved_token(&cli.base_url)` and ignores `--username`
entirely.

**Inference — cross-user token reuse.** Two users (or one user with two
accounts) against the *same* server share one cache file. After user A runs
`ileap auth login`, user B running `ileap shipments --username B` (with A's
token still cached and unexpired) is silently served **A's** token — B's
`--username` is ignored. The request runs as A. This is a correctness and
least-surprise defect, not just posture: the supplied identity is silently
overridden by whoever logged in last.

**Inference — username is not a secret.** Unlike the password, the username is
routinely supplied on the command line / in `ILEAP_USERNAME`. Requiring it to
*read* the cache costs nothing in secrecy and is exactly what disambiguates
whose token is being loaded.

## Decision

**Key the cache file by base URL *and* username.** `token_file`, `save_token`,
and `load_saved_token` all take a `username: &str`; the filename becomes
`token_<enc(base-url)>__<enc(username)>`, where `enc` is **percent-encoding with
`NON_ALPHANUMERIC`** (every byte except `[A-Za-z0-9]` is `%XX`-escaped).

This replaces ADR-0006's `replace(['/', ':', '.', '-'], "_")`, which was
**lossy and non-injective** — distinct URLs could map to the same filename, and
ADR-0006 itself conceded those collisions "remain theoretically possible."
Worse, adding a username section behind a literal `__` separator created a *new*
collision class: with `-`/`.` mapping to `_`, a user-controlled username could
straddle the boundary (`url=…x-`,`user=y` and `url=…x`,`user=_y` both →
`…x___y`). Percent-encoding removes both: the encoding is reversible (injective),
filesystem-safe (`/` → `%2F` keeps a single path component, no `..` traversal),
preserves the scheme (so http/https never collide, per ADR-0006), and makes the
`__` separator unambiguous because `_` itself encodes to `%5F` — an encoded
section can never contain a raw underscore. The filename stays greppable
(`token_https%3A%2F%2Fapi.example.com__alice`), honouring ADR-0006's
plaintext-and-debuggable posture.

Username for the cache key is resolved as:

1. The explicit `--username` / `ILEAP_USERNAME` value when present.
2. For `auth login --token <t>` with no username: the JWT `sub` claim
   (`jwt_sub`, a new sibling of `jwt_exp`).
3. Otherwise: a clear `CliError::Auth` telling the user to provide `--username`.

For non-auth commands, **the cache is consulted only when a username is known.**
With no username, the CLI behaves as if the cache is empty (falls through to
credentials, or the existing credential error). This is the deliberate trade
that kills cross-user reuse: you must say who you are to use *your* cached token.

## Considered Options

- **A — Key by base URL + username (chosen).** Smallest change that removes the
  cross-user reuse. Username is non-secret, so requiring it for cache reads is
  free. Keeps the greppable plaintext-file design from ADR-0006.
- **B — Embed the username in the file *content* and validate on load.** Still
  needs the supplied username at load time to compare against; no simpler than
  keying the name, and complicates the file format. Rejected.
- **C — Derive the key solely from the JWT `sub` claim.** Works for *saving*
  (the token carries its subject) but not for *loading* — at load time there is
  no token yet to read a subject from. Used only as the fallback in option A's
  rule 2. Rejected as the primary key.
- **D — Status quo.** Leaves silent cross-user token reuse in place. Rejected.

### Filename encoding (sub-decision)

Given the key is `(base-url, username)`, how to render it as one safe filename:

- **E1 — Char-class `replace([...], "_")` (ADR-0006's scheme).** Greppable, zero
  deps, but lossy/non-injective and — once a username section is added — opens
  the cross-boundary collision above. **Rejected.**
- **E2 — Separate path components** (`…/ileap/<url>/<user>`). Zero deps, the FS
  `/` is an unambiguous boundary, but each component still needs sanitizing and
  the intra-component lossiness remains. Adequate but not injective.
- **E3 — Percent-encode each section with `NON_ALPHANUMERIC` (chosen).**
  Injective, filesystem- and traversal-safe, stays human-readable/greppable, and
  `percent-encoding` is already in the dependency tree (transitively via
  `reqwest`→`url`), so it adds no new compiled crate. **Chosen.**
- **E4 — Hash the canonical key (SHA-256 → hex/base32).** Fully injective and
  fixed-length, but **kills greppability** — you can no longer see whose token a
  file holds — which fights ADR-0006's explicit "keep the cache greppable"
  rationale. **Rejected.**
- **E5 — OS keyring (`keyring` crate).** Dissolves filename construction
  entirely, but was **explicitly rejected in ADR-0006 (option A)** as
  over-engineered for short-lived internal tokens; not reopened here.

## Consequences

**Positive**

- Switching `--username` against the same server now selects that user's token
  (or misses cleanly); a stale token from another user is never silently reused.
- The cache key is now **injective** — percent-encoding (E3) removes both
  ADR-0006's residual collisions and the new separator-straddle collision a
  literal `__` join would have introduced. File hardening (`0600`,
  no-CWD-fallback) from ADR-0006 is reused unchanged.

**Negative / risks**

- **One-time cache invalidation** (again): existing `token_<url>` files use both
  the old key (no username) *and* the old `replace`-based encoding, so they no
  longer match the new `token_<enc(url)>__<enc(user)>` scheme and are orphaned.
  Users re-run `ileap auth login` once. Old files expire into harmless garbage;
  not migrated.
- **New direct dependency** `percent-encoding` — already present transitively
  (`reqwest`→`url`), so no new compiled crate, only an explicit `Cargo.toml`
  entry.
- **UX change:** to *use* a cached token for a non-auth command you must supply a
  username (`--username`/`ILEAP_USERNAME`). The "log in interactively, then run
  flagless" flow now needs `ILEAP_USERNAME` set. Acceptable: username is not a
  secret and this is what makes the cache unambiguous.
- `auth status` with no username reports `authenticated: false` (it cannot
  identify a cache entry without one).

**Neutral**

- Token file content, expiry logic, and `0600`/no-CWD-fallback hardening from
  ADR-0006 are unchanged.
- Interaction with **ADR-0009** (Proposed): 0009 moves the credential chain into
  `auth::resolve_client`. If 0009 lands after this, that helper must thread the
  username into the cache probe; if this lands after 0009, the same edit applies
  inside `resolve_client`. No conflict in decision, only in final call site.

## Changes (for coding agent)

Code changes in `src/auth.rs`, the cache call site in `src/main.rs`, and the
`percent-encoding` entry in `Cargo.toml`.

1. **`token_file` (`auth.rs:14`):** add `username: &str`; percent-encode each
   section with `NON_ALPHANUMERIC` and join with `__` (E3).
2. **`save_token` / `load_saved_token`:** add `username: &str`, thread through.
3. **`jwt_sub` (new):** sibling of `jwt_exp`, returns the `sub` claim.
4. **`run_auth` `Login` arm:** resolve the cache username (explicit → `jwt_sub`
   for `--token` → error); save/load under it; interactive path prompts the
   username, then probes the cache for it before prompting the password.
5. **`run_auth` `Status` arm:** scope by username; no username → `authenticated:
   false`.
6. **`main.rs` command path:** consult `load_saved_token` only when
   `cli.username` is `Some`, keyed by it.
7. **Tests:** thread username through existing tests; add
   `token_file_distinguishes_usernames`, a username-scoping load test, and a
   `jwt_sub` test.

### Verify

```
cargo build && cargo clippy --all-targets -- -D warnings && cargo test
```

Manually: `ILEAP_USERNAME=a ileap auth login`, then
`ILEAP_USERNAME=b ileap auth status` reports `authenticated: false` while
`ILEAP_USERNAME=a ileap auth status` reports the cached token.

## Implementation

Implemented 2026-06-11 on branch `adr-0010-token-cache-username-scoping`
(branched from `origin/main` at `6a74133`) in `src/auth.rs`, `src/main.rs`, and
`Cargo.toml`. Verified: `cargo build` pass; `cargo clippy --all-targets -- -D
warnings` clean; `cargo test` pass (**42 unit + 10 integration, 0 failed**).

**Files:** `src/auth.rs` (signatures, `jwt_sub`, percent-encoded `token_file`,
`run_auth` Login/Status arms, tests), `src/main.rs` (username-scoped cache probe
in the command path), `Cargo.toml` (`percent-encoding = "2"`).

**Filename format** (example): base `https://api.example.com` + user `alice`
→ `token_https%3A%2F%2Fapi.example.com__alice`.

**New/changed tests:** `token_file_distinguishes_usernames`,
`token_file_no_collision_on_separator_chars` (proves injectivity vs the old
scheme), `load_saved_token_is_scoped_by_username`, `jwt_sub_*`,
`run_auth_login_token_flag_keys_by_jwt_sub_without_username`,
`run_auth_login_token_flag_no_username_no_sub_errors`,
`run_auth_status_without_username_reports_unauthenticated`; all existing
`save_token`/`load_saved_token`/`token_file` call sites threaded a username.

**Deviations from the Changes text:** none of substance. The `Status` arm uses
`username.and_then(|u| load_saved_token(base_url, u).transpose())` to keep error
propagation while treating "no username" as `authenticated: false`.
