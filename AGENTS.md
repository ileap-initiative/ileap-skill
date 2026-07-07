# iLEAP CLI — Agent Instructions

## Skills

This repo ships an iLEAP CLI skill at `ileap/`.

**Read both files in full before working with iLEAP data** — fetching shipments, TOCs, footprints, HOCs, TAD, AED, building dashboards, filtering, or any other API interaction:

- `ileap/SKILL.md` — auth, commands, dashboard procedure, output format, exit codes
- `ileap/references/SCHEMAS.md` — field reference, filter syntax, and schema discovery workflow for all resource types

These two are intentionally **not** imported into memory: they are large and the
`ileap` skill already surfaces them on demand when an iLEAP task is triggered.
Importing them would bloat every session for no benefit.

## Architecture review & ADRs

The operating memory for architecture review, ADRs (`docs/adr/`), and the
documentation drift ledger (`docs/DOC-DRIFT.md`) is imported below so it loads
automatically. It covers epistemic labeling, git-as-oracle for implementation
status, the single-writer rule for shared ledgers, re-verifying citations after
merges, delegation strategy, and ADR/ledger conventions.

@docs/adr/PROCESS.md
