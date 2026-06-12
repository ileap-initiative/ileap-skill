# iLEAP CLI

This repository contains a simple CLI tool to consume iLEAP data. Running `ileap` with no subcommand prints help and exits. To authenticate interactively, run `ileap auth login` on a TTY.

## For agents

An iLEAP CLI skill is defined at `.agents/skills/ileap-cli/`. Read both files before working with iLEAP data:

- `.agents/skills/ileap-cli/SKILL.md` — auth, commands, dashboard procedure, output format, exit codes
- `.agents/skills/ileap-cli/SCHEMAS.md` — field reference, filter syntax, and schema discovery workflow for all resource types

The skill is standalone and portable: it uses prebuilt `ileap` binaries built from this repository and bundled in the skill's `bin/` directory — it never compiles at runtime, so no Rust toolchain is needed where the skill runs. To deploy it (including as a Claude.ai skill):

1. `scripts/build-skill-binaries.sh` — build static Linux binaries (and a native host binary) from the local source into the skill's `bin/` directory (requires Docker)
2. `scripts/package-skill.sh` — produce `dist/ileap-cli-skill.zip`, ready to upload at claude.ai (Settings → Capabilities → Skills) or to unpack into any agent's skills directory
