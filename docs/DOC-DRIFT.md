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

Doc paths use `SKILL.md` to mean `ileap/SKILL.md` (the
`.claude/skills/ileap/SKILL.md` path is a **symlink** to it — see D9).

---

## Open / Deferred

| # | Doc | Claim vs. reality | Severity | Status |
|---|-----|-------------------|----------|--------|
| D1 | `SKILL.md` | "authenticate interactively: `ileap auth login`" — but `auth login` never prompts; with no token/cache/creds it returns `credential_error` (exit 4) | HIGH | Resolved (ADR-0002, merged PR #7) |
| D7 | `README.md` / `SKILL.md` | Bare-`ileap` behaviour (TTY→REPL, non-TTY→exit-1 error) is undocumented/misleading; the "interactive" path points at the wrong command | MED | Resolved (ADR-0002, merged PR #7) |
| D2 | `SKILL.md` env list | `ILEAP_TIMEOUT` env var (`cli.rs:27-28`) is real but undocumented | MED | Open |
| D3 | `SKILL.md` | Short flags `-t`/`-u`/`-p` for `--token`/`--username`/`--password` (`cli.rs:11,15,19`) undocumented | LOW | Open |
| D4 | `SKILL.md` | Short flag `-m` for `--max-pages` (`cli.rs:135`) undocumented | LOW | Open |
| D5 | `SKILL.md` | Short flag `-n` for `--dry-run` (`cli.rs:104,127`) undocumented | LOW | Open |
| D6 | `SKILL.md` | Short flag `-l` for `--limit` (`cli.rs:112`) undocumented | LOW | Open |
| D8 | `SKILL.md` | `auth login` idempotent silent-success on a cached token (`auth.rs:107-110`) is not surfaced; compounds D1 confusion | LOW | Open |
| D9 | skill layout | `.claude/skills/ileap/SKILL.md` is a symlink to `ileap/SKILL.md`; tools that don't follow symlinks miss it (informational) | LOW | Open |
| D10 | `SKILL.md` | PACT `footprints` honours only the **first** `-f` filter; extra `-f` flags are silently dropped at runtime (only `--dry-run` reveals it). The repeatable-`-f` docs don't note this limitation | MED | Open |
| D11 | `ileap-dashboard.html` | Root demo dashboard uses the wrong term "Transport Operation **Characteristics**" (should be **Categories**) and contains emojis — violating SKILL.md terminology and the dashboard no-emoji policy. The site ships a sanitized copy; the source is still wrong | MED | Deferred→ADR-0010 |
| D12 | `site/` (microsite) | The site's truth-sensitive strings (skill name, `ileap-skill.zip` name, demo URL, example prompts) must track the skill or they will drift — first risk is the `ileap-cli`→`ileap` rename | MED | Deferred→ADR-0010 |

---

## Detail

### D1 — `auth login` does not prompt interactively (HIGH, Resolved — ADR-0002, PR #7)
- **Doc:** `SKILL.md` — "To authenticate interactively: `ileap auth login`".
- **Code:** `auth.rs:111-121` — `AuthCmd::Login` with no `--token`, no cached
  token, and no `--username/--password` returns `credential_error` → exit 4. No
  prompt path exists in `auth login`. *(Verified directly during review.)*
- **Resolution:** ADR-0002 decision **A2** adds an interactive prompt to
  `auth login` when stdin is a TTY, which makes this doc claim *correct*. Fix the
  doc as part of that implementation. **Do not fix the doc in isolation** — it
  would describe behaviour that doesn't exist yet.
- **✓ Resolved (committed, merged PR #7):** A2 implemented in `auth.rs`
  (`run_auth`, `AuthCmd::Login`, `auth.rs:124-126`): on a TTY it now prompts for
  username/password; non-TTY still returns `credential_error` (exit 4). The
  SKILL.md claim is now accurate — **no doc edit was required.** Note: the prompt
  helpers moved from `tty.rs` to a renamed `prompt.rs` module during
  implementation (`auth.rs:12` → `use crate::prompt::...`).

### D7 — Bare-`ileap` behaviour undocumented/misleading (MED, Resolved — ADR-0002, PR #7)
- **Doc:** `README.md` — "The CLI tool has both a REPL flow and non-interactive
  commands" is the only mention; `SKILL.md` is silent on no-subcommand behaviour.
- **Code:** `main.rs:59-86` — non-TTY → exit-1 error; TTY → interactive
  credential prompt + REPL.
- **Resolution:** ADR-0002 removes the REPL and makes bare `ileap` print help.
  Update README/SKILL.md when that lands. Same change set as D1.
- **✓ Resolved (committed, merged PR #7):** `main.rs` `None` arm now prints clap
  help and exits 0 (`main.rs:51`, via `clap::CommandFactory`); `repl.rs` deleted.
  README updated to describe the no-subcommand and `auth login` behaviour.
  SKILL.md needed no change.

### D2 — `ILEAP_TIMEOUT` undocumented (MED, Open)
- **Doc:** `SKILL.md` env list names only `ILEAP_TOKEN`, `ILEAP_USERNAME`,
  `ILEAP_PASSWORD`, `ILEAP_BASE_URL`.
- **Code:** `cli.rs:27-28` — `--timeout` is bound to `env = "ILEAP_TIMEOUT"`.
- **Fix:** add `ILEAP_TIMEOUT` to the env-var list in `SKILL.md` (and README if
  it lists env vars). Pure doc fix.

### D3–D6 — Undocumented short flags (LOW, Open)
- **Code:** `-t/-u/-p` (`cli.rs:11,15,19`), `-m` (`cli.rs:135`), `-n`
  (`cli.rs:104,127`), `-l` (`cli.rs:112`).
- **Fix:** optional — short flags are discoverable via `--help`. Document only if
  the team wants `SKILL.md` to be the complete reference. Pure doc fix.

### D8 — `auth login` cached-token silent success undocumented (LOW, Open)
- **Doc:** `SKILL.md` doesn't mention that `auth login` returns
  `{"authenticated": true, "token_source": "cache"}` without acting when a valid
  token is cached.
- **Code:** `auth.rs:107-110`. The behaviour is documented in the `cli.rs:83`
  doc-comment but not in user-facing docs.
- **Fix:** note the idempotent behaviour in `SKILL.md`, ideally alongside the D1
  fix so the full `auth login` decision tree (token → cache → creds → prompt) is
  described in one place.

### D9 — Skill file is a symlink (LOW, informational)
- `.claude/skills/ileap/SKILL.md` → `../../../ileap/SKILL.md`.
  No drift between the two (same inode), but symlink-unaware tooling may not see
  the `.claude/` copy. No action unless that becomes a problem.

### D10 — `footprints` single-filter limitation not user-documented (MED, Open)
- **Doc:** `SKILL.md` Filtering section documents `-f` as repeatable but does not
  note that for PACT `footprints` only the first `-f` is sent.
- **Code:** `client.rs:284-285` (live) and `client.rs:250-251` (dry-run) use
  `filter.first()` only — intentional and code-commented (`client.rs:283`).
  iLEAP-standalone endpoints honour all filters (`get_kv_filters`,
  `client.rs:218,227`). *(Re-verified against merged code 2026-06-11.)*
- **Cross-ref:** this is the *user-doc gap*; the *code-side* decision (accept /
  join with ` and ` / error on multiple) is backlog candidate **C3** in
  `adr/README.md`. Fix the doc to match whatever C3 decides — if C3 chooses
  "error on multiple `-f`", document that instead of the silent-drop behaviour.

### D11 — `ileap-dashboard.html` uses wrong terminology + emojis (MED, Deferred→ADR-0010)
- **Artifact:** `ileap-dashboard.html` (repo root) — a demo dashboard.
- **Reality:** contains "Transport Operation **Characteristics** (TOCs)" (should be
  **Categories** — the exact mistake `SKILL.md` warns against) and emoji glyphs
  (📊 🌱 ✅), which the SKILL.md dashboard styling policy forbids. *(Verified
  2026-06-16: `grep` for "Characteristics" and the emoji set both hit.)*
- **Mitigation in place:** the microsite ships a **sanitized** copy at
  `site/static/demos/dashboard.html` (terms fixed, emojis stripped, verified clean).
- **Fix:** correct or regenerate the *source* `ileap-dashboard.html`. Tracked as an
  open item in ADR-0010; do not let the bad source propagate to new copies.

### D12 — microsite truth-sensitive strings must track the skill (MED, Deferred→ADR-0010)
- **Artifact:** `site/` (the iLEAP skill microsite, ADR-0010).
- **Risk:** the site hard-references the skill name, the `ileap-skill.zip` download,
  the demo URL (`api.preview.ileap.dev`), and example prompts. These drift if the
  skill changes — the **first** instance being the `ileap-cli`→`ileap` rename
  (download name, all copy). The site is built to use `ileap` already.
- **Fix:** when the rename or any skill change lands, re-verify these strings.
  `grep -rn "ileap-cli" site/` must stay empty; the download must resolve to the
  current release asset. Owned by ADR-0010.

---

## Checked — no drift (recorded so they aren't re-investigated)

- **Default base URL** `https://api.preview.ileap.dev` — matches `cli.rs:10`.
- **Exit codes** 0/1/3/4 — match `error.rs` (`CliError::exit_code`, lines 22-26)
  and `main.rs` (downcast mapping, lines 24-26). Note: the mapping moved from
  `client.rs`'s old `ExitCode` to `error.rs` in ADR-0005; still 0/1/3/4.
- **Resource endpoints** (footprints/shipments/tocs/hocs/tad/aed paths) — match
  `client.rs`.
- **`--yes`/`-y`, `-o compact`** — match `cli.rs`.
- **`edition = "2024"` in `Cargo.toml`** — *not* drift. Flagged early in review as
  a possible typo by a delegated agent; in fact edition 2024 is stable (since Rust
  1.85, Feb 2025) and the value is correct. Recorded to prevent re-raising.

## Unverifiable from source

- Demo-server credentials (`hello` / `pathfinder`) and the demo URL's liveness
  depend on live server state, not the codebase. **Confirmed live 2026-06-16:**
  login to `https://api.preview.ileap.dev` with `hello`/`pathfinder` succeeded and
  all six resources fetched (exit 0). Re-confirm out-of-band over time — server
  state can change.
