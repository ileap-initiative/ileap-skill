# Doc Drift Ledger

A running record of places where **documentation disagrees with the code**, found
during the architecture review. Code is treated as ground truth; each entry cites
evidence on both sides so it can be fixed (or the doc decision can be made)
without re-investigating.

This is a living document. When an item is fixed, move it to **Resolved** with the
commit/PR or ADR that closed it. Some items are intentionally deferred to an ADR
because the *right* fix is to change the code, not the doc.

**Severity:** HIGH = will mislead a user/agent into wrong behaviour · MED =
inaccurate, low harm · LOW = cosmetic/incomplete.

**Status:** Open · Deferred→ADR-NNNN (fix lands with that ADR) · Resolved.

Doc paths use `SKILL.md` to mean `.agents/skills/ileap-cli/SKILL.md` (the
`.claude/skills/ileap-cli/SKILL.md` path is a **symlink** to it — see D9).

---

## Open / Deferred

| # | Doc | Claim vs. reality | Severity | Status |
|---|-----|-------------------|----------|--------|
| D1 | `SKILL.md` | "authenticate interactively: `ileap auth login`" — but `auth login` never prompts; with no token/cache/creds it returns `credential_error` (exit 4) | HIGH | Deferred→ADR-0002 |
| D7 | `README.md` / `SKILL.md` | Bare-`ileap` behaviour (TTY→REPL, non-TTY→exit-1 error) is undocumented/misleading; the "interactive" path points at the wrong command | MED | Deferred→ADR-0002 |
| D2 | `SKILL.md` env list | `ILEAP_TIMEOUT` env var (`cli.rs:27-28`) is real but undocumented | MED | Open |
| D3 | `SKILL.md` | Short flags `-t`/`-u`/`-p` for `--token`/`--username`/`--password` (`cli.rs:11,15,19`) undocumented | LOW | Open |
| D4 | `SKILL.md` | Short flag `-m` for `--max-pages` (`cli.rs:135`) undocumented | LOW | Open |
| D5 | `SKILL.md` | Short flag `-n` for `--dry-run` (`cli.rs:128,105`) undocumented | LOW | Open |
| D6 | `SKILL.md` | Short flag `-l` for `--limit` (`cli.rs:113`) undocumented | LOW | Open |
| D8 | `SKILL.md` | `auth login` idempotent silent-success on a cached token (`auth.rs:104-110`) is not surfaced; compounds D1 confusion | LOW | Open |
| D9 | skill layout | `.claude/skills/ileap-cli/SKILL.md` is a symlink to `.agents/...`; tools that don't follow symlinks miss it (informational) | LOW | Open |
| D10 | `SKILL.md` | PACT `footprints` honours only the **first** `-f` filter; extra `-f` flags are silently dropped at runtime (only `--dry-run` reveals it). The repeatable-`-f` docs don't note this limitation | MED | Open |

---

## Detail

### D1 — `auth login` does not prompt interactively (HIGH, Deferred→ADR-0002)
- **Doc:** `SKILL.md` — "To authenticate interactively: `ileap auth login`".
- **Code:** `auth.rs:111-121` — `AuthCmd::Login` with no `--token`, no cached
  token, and no `--username/--password` returns `credential_error` → exit 4. No
  prompt path exists in `auth login`. *(Verified directly during review.)*
- **Resolution:** ADR-0002 decision **A2** adds an interactive prompt to
  `auth login` when stdin is a TTY, which makes this doc claim *correct*. Fix the
  doc as part of that implementation. **Do not fix the doc in isolation** — it
  would describe behaviour that doesn't exist yet.

### D7 — Bare-`ileap` behaviour undocumented/misleading (MED, Deferred→ADR-0002)
- **Doc:** `README.md` — "The CLI tool has both a REPL flow and non-interactive
  commands" is the only mention; `SKILL.md` is silent on no-subcommand behaviour.
- **Code:** `main.rs:59-86` — non-TTY → exit-1 error; TTY → interactive
  credential prompt + REPL.
- **Resolution:** ADR-0002 removes the REPL and makes bare `ileap` print help.
  Update README/SKILL.md when that lands. Same change set as D1.

### D2 — `ILEAP_TIMEOUT` undocumented (MED, Open)
- **Doc:** `SKILL.md` env list names only `ILEAP_TOKEN`, `ILEAP_USERNAME`,
  `ILEAP_PASSWORD`, `ILEAP_BASE_URL`.
- **Code:** `cli.rs:27-28` — `--timeout` is bound to `env = "ILEAP_TIMEOUT"`.
- **Fix:** add `ILEAP_TIMEOUT` to the env-var list in `SKILL.md` (and README if
  it lists env vars). Pure doc fix.

### D3–D6 — Undocumented short flags (LOW, Open)
- **Code:** `-t/-u/-p` (`cli.rs:11,15,19`), `-m` (`cli.rs:135`), `-n`
  (`cli.rs:128,105`), `-l` (`cli.rs:113`).
- **Fix:** optional — short flags are discoverable via `--help`. Document only if
  the team wants `SKILL.md` to be the complete reference. Pure doc fix.

### D8 — `auth login` cached-token silent success undocumented (LOW, Open)
- **Doc:** `SKILL.md` doesn't mention that `auth login` returns
  `{"authenticated": true, "token_source": "cache"}` without acting when a valid
  token is cached.
- **Code:** `auth.rs:104-110`. The behaviour is documented in the `cli.rs:83`
  doc-comment but not in user-facing docs.
- **Fix:** note the idempotent behaviour in `SKILL.md`, ideally alongside the D1
  fix so the full `auth login` decision tree (token → cache → creds → prompt) is
  described in one place.

### D9 — Skill file is a symlink (LOW, informational)
- `.claude/skills/ileap-cli/SKILL.md` → `../../../.agents/skills/ileap-cli/SKILL.md`.
  No drift between the two (same inode), but symlink-unaware tooling may not see
  the `.claude/` copy. No action unless that becomes a problem.

### D10 — `footprints` single-filter limitation not user-documented (MED, Open)
- **Doc:** `SKILL.md` Filtering section documents `-f` as repeatable but does not
  note that for PACT `footprints` only the first `-f` is sent.
- **Code:** `client.rs:269-271` (live) and `client.rs:235-237` (dry-run) use
  `filter.first()` only — intentional and code-commented (`client.rs:268`).
  iLEAP-standalone endpoints honour all filters (`get_kv_filters`,
  `client.rs:211-215`). *(Verified during review.)*
- **Cross-ref:** this is the *user-doc gap*; the *code-side* decision (accept /
  join with ` and ` / error on multiple) is backlog candidate **C3** in
  `adr/README.md`. Fix the doc to match whatever C3 decides — if C3 chooses
  "error on multiple `-f`", document that instead of the silent-drop behaviour.

---

## Checked — no drift (recorded so they aren't re-investigated)

- **Default base URL** `https://ileap-preview.fly.dev` — matches `cli.rs:7`.
- **Exit codes** 0/1/3/4 — match `main.rs` and `client.rs`.
- **Resource endpoints** (footprints/shipments/tocs/hocs/tad/aed paths) — match
  `client.rs`.
- **`--yes`/`-y`, `-o compact`** — match `cli.rs`.
- **`edition = "2024"` in `Cargo.toml`** — *not* drift. Flagged early in review as
  a possible typo by a delegated agent; in fact edition 2024 is stable (since Rust
  1.85, Feb 2025) and the value is correct. Recorded to prevent re-raising.

## Unverifiable from source

- Demo-server credentials (`hello` / `pathfinder`) and the demo URL's liveness
  depend on live server state, not the codebase. Confirm out-of-band.
