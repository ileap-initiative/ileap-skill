# Architecture Review & ADR Workflow — Operating Memory

Durable operating rules for working on this repo's **architecture review, ADRs,
and documentation ledgers**. Distilled from review sessions; meant to be read by
any engineer or coding agent before touching `docs/adr/` or `docs/DOC-DRIFT.md`.

These are *process* rules. For using the iLEAP CLI and its data, see the skill
files referenced in `AGENTS.md`.

---

## 1. Epistemic labeling is mandatory in ground-truth docs

Label every non-trivial claim:

- **Fact** — verified at source this session (cite `file:line`).
- **Inference** — reasoned from facts, not line-verified.
- **Claim** — from delegated/subagent output, spot-checked only.
- **Assumption** — needs human confirmation.

Anything that drives a decision (an ADR's Decision, a backlog promotion, a
Resolved status) must be **Fact**. Promote Claim/Inference → Fact by verifying at
source *before* it influences the outcome. Delegated output has been wrong before
(e.g. "`edition = "2024"` is a typo" — it is stable since Rust 1.85); always
spot-check it.

## 2. git is the oracle for implementation status

Never trust prose like "implemented", "pending commit", or "in the working tree".
Verify against git:

- `git status --short` / `git diff --stat` — is there actually uncommitted work?
- `git log --oneline` — was it committed/merged, and in which PR?
- file existence / `grep` — does the claimed code actually exist now?

When a doc's status disagrees with git, **git wins** and the doc gets corrected.

## 3. Single writer for shared ground-truth docs

`docs/adr/README.md` (index + candidate backlog) and `docs/DOC-DRIFT.md` are
edited by **one session at a time**. Other sessions propose changes; the owner
applies them. The editor's "file modified since read" guard is a safety net that
prevents data loss — it is **not** a coordination protocol. Concurrent edits to
these files have collided in practice; route through the single writer.

Individual ADR files (`NNNN-slug.md`) and the code are owned by the implementing
session.

## 4. Citations rot — re-verify after every ADR merge

When an ADR merges, code shifts: line numbers move, modules get renamed
(`tty.rs` → `prompt.rs`), error mappings relocate (`client.rs` `ExitCode` →
`error.rs` `CliError` in ADR-0005). The same commits that *resolve* some ledger
items *invalidate the line references* of others.

**Standing trigger:** after any ADR merges, re-verify the `file:line` citations of
every still-Open `DOC-DRIFT.md` item and live backlog candidate against the new
code. A drift ledger whose own evidence has drifted is worse than none.

## 5. Delegate broad reads; reserve the strong model for judgment

- Broad read-and-compare, codebase mapping, doc-vs-code sweeps → cheaper subagent
  (e.g. Sonnet via the Agent tool). Cheap, parallelizable.
- Synthesis, design decisions, the Fact/Claim distinction, and writing ADRs →
  keep on the strong model.
- Always spot-check delegated conclusions before they enter ground truth (rule 1).
- A couple of targeted `grep`s often beats spawning an agent for a handful of
  line-ref confirmations — pick the cheaper tool for the actual breadth.

## 6. Surface consequential side-effects; don't bury them

When a change has a non-obvious consequence, make it an explicit sub-decision, not
a silent default. (Example: removing the REPL in ADR-0002 deleted the *only*
interactive credential prompt → that became an explicit companion decision "A2",
not an assumption.) When a request's framing contradicts the code, **lead with the
correction** before answering as-framed.

## 7. Document conventions

- **ADRs** live at `docs/adr/NNNN-slug.md`. House style: epistemic labels in
  Context; **Considered Options**; **Consequences** split Positive / Negative-risks
  / Neutral; a **"Changes (for coding agent)"** section with concrete file-level
  edits. Status: Proposed · Accepted · Superseded · Deprecated · Rejected.
- **`docs/adr/README.md`** is the canonical index + candidate backlog. It *owns
  decisions*. Keep the index Status in sync with each ADR file's own Status (they
  have contradicted before).
- **`docs/DOC-DRIFT.md`** tracks documentation-vs-code mismatches. It **defers
  code-fixable items to ADRs and never restates a fix** — it points at the ADR
  that owns the decision. Items deferred to an ADR are fixed only when that ADR is
  implemented (don't "fix" a doc to describe behaviour that doesn't exist yet).

## 8. Cybernetic habit

Each session, note what improved the working loop and adjust. The rules above are
themselves outputs of that loop (collisions → single-writer; stale prose →
git-as-oracle; rotted citations → re-verify-after-merge). Add new rules here when
a failure mode recurs.
