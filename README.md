# iLEAP CLI

This repository contains a simple CLI tool to consume iLEAP data. Running `ileap` with no subcommand prints help and exits. To authenticate interactively, run `ileap auth login` on a TTY.

## For agents

An iLEAP CLI skill is defined at `.agents/skills/ileap-cli/`. Read both files before working with iLEAP data:

- `.agents/skills/ileap-cli/SKILL.md` — auth, commands, dashboard procedure, output format, exit codes
- `.agents/skills/ileap-cli/SCHEMAS.md` — field reference, filter syntax, and schema discovery workflow for all resource types
