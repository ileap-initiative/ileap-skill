# ADR-0007: Pagination loop — correctness and shape

## Status

Proposed (2026-06-11). Absorbs backlog candidate **C5** (`merge_pages` clones).
Respects ADR-0004 (Accepted): the generic `run_list` bound stays.

## Context

`commands::run_list` (`commands.rs:124-173`) owns pagination. It contains one
genuine bug, one latent shape bug, and a structural duplication that makes both
harder to see.

**Fact — `--limit 0` causes an infinite request loop (`commands.rs:142-153`,
`cli.rs:112-113`).** `--limit` is `Option<u32>` with no range restriction, so
`0` is accepted. In the non-interactive arm:

```rust
let at_boundary = limit.is_some_and(|l| item_count(&value) == l as usize);
```

With `limit = Some(0)` and an empty page, `item_count == 0 == limit`, so
`at_boundary` is `true` and the loop does not break; `offset += 0` never
advances. `ileap shipments list -l 0 -y` (without `--max-pages`) re-fetches the
same page forever. The interactive arm (`pager.rs:22`) has the same
`at_boundary` arithmetic but a human in the loop.

**Fact — `merge_pages` merges unlike shapes silently (`commands.rs:175-198`).**
The envelope of the merged output is decided by *any* page being an object
(`is_object = true` is sticky). If pages mix `{"data": [...]}` and bare-array
shapes (only reachable via a misbehaving server), array items are silently
re-wrapped in an envelope they never had. Additionally (**C5, Fact**) every
record is deep-cloned via `data.iter().cloned()` — fine at current scales.

**Fact — two near-identical loops (`commands.rs:138-171`).** The
non-interactive and interactive arms duplicate the fetch / `page_num` /
`max_pages` / `offset += l` machinery, differing only in what happens to each
page (accumulate vs. print-and-prompt). The `--limit 0` bug exists in one copy
of logic that *looks* like it exists once.

**Inference — why this matters beyond the bug.** Duplicate control flow is
where copies drift; the fix for any future pagination defect must be applied
twice or it is applied once and wrong.

## Decision

1. **Reject `--limit 0` at the CLI boundary.** Add a clap range
   `value_parser(clap::value_parser!(u32).range(1..))` on `limit`
   (`cli.rs:112-113`). The flag means "page size"; zero is meaningless, and
   rejecting it at parse time (clap exit code 2, standard usage error) is the
   naive fix — no loop logic needs to reason about zero.
2. **Unify the two loops into one.** A single loop fetches, then branches on
   `non_interactive` for the per-page action (push vs. `print_page`), and
   computes `continue?` in one place. The ADR-0004 generic bound on `fetch` is
   unchanged.
3. **Decide the merged envelope from the first page only.** `merge_pages`
   takes its output shape from `pages[0]`; later pages of a different shape
   still contribute their items (server is already misbehaving; we stay
   lossless) — but the envelope is deterministic. Keep the deep clones
   (resolving **C5**: accept and document; revisit only if bulk export becomes
   a requirement).

## Considered Options

- **Treat `limit == 0` as "no limit" in the loop.** Adds a special case to
  every boundary computation; clap rejection is strictly simpler. **Rejected.**
- **Keep two loops, fix the bug in both.** Preserves today's reading order but
  leaves the drift hazard that produced a single-copy fix risk in the first
  place. **Rejected.**
- **Restructure `merge_pages` to error on mixed shapes.** More honest, but
  turns a never-observed server bug into a hard CLI failure mid-paging;
  deterministic best-effort output is friendlier for a read-only tool.
  **Rejected** (revisit if shape mixing is ever observed).

## Consequences

**Positive**

- The infinite-loop class is eliminated at the type/parse boundary, not
  patched inside the loop.
- One pagination loop to read, test, and fix.
- `merge_pages` output shape becomes deterministic.

**Negative / risks**

- `--limit 0` changes from "hangs" to "usage error (exit 2)" — technically a
  behavior change; no legitimate use existed.
- Unifying the loops touches code that ADR-0004's pagination unit tests cover;
  tests must keep passing unchanged (they pin behavior, not structure).

**Neutral**

- Interactive paging UX, JSON output for well-behaved servers, and exit codes
  are unchanged.

## Changes (for coding agent)

1. **`src/cli.rs:112-113`:** add to the `limit` arg:
   `#[arg(long, short = 'l', value_parser = clap::value_parser!(u32).range(1..))]`.
2. **`src/commands.rs:124-173`:** merge the two arms of `run_list` into one
   loop. Sketch:

   ```rust
   let non_interactive = yes || !std::io::stdin().is_terminal();
   let mut pages: Vec<Value> = vec![];
   let mut offset = 0u32;
   let mut page_num = 0u32;
   loop {
       let value = fetch(offset).await.map_err(Into::into)?;
       page_num += 1;
       let more = if non_interactive {
           let at_boundary = limit.is_some_and(|l| item_count(&value) == l as usize);
           pages.push(value);
           at_boundary
       } else {
           print_page(&value, limit, output)?
       };
       let at_max = max_pages.is_some_and(|mp| page_num >= mp);
       let Some(l) = limit else { break };
       if !more || at_max {
           break;
       }
       offset += l;
   }
   if non_interactive {
       output::print_value(&merge_pages(pages), output);
   }
   ```

   Preserve the existing semantics for `limit = None` (single fetch) and
   `max_pages` (inclusive cap) — the in-file unit tests and ADR-0004's
   pagination tests are the oracle.
3. **`src/commands.rs:175-198` (`merge_pages`):** replace the sticky
   `is_object` flag with `let is_object = matches!(pages.first(), Some(Value::Object(_)));`
   computed before the accumulation loop. Add a unit test for mixed shapes
   pinning the first-page-wins envelope.
4. **Tests:** add an integration or unit test that `-l 0` is rejected by clap
   (exit code 2, error mentions the valid range).

### Verify

```
cargo build && cargo clippy --all-targets -- -D warnings && cargo test
```

Manually: `ileap shipments list -l 0 -y` exits immediately with a usage error;
`-l 2 -y -m 3` fetches at most 3 pages (confirm via `--dry-run`/server logs or
wiremock test).
