# iLEAP CLI

This repository contains a simple CLI tool to consume iLEAP data. Running `ileap` with no subcommand prints help and exits. To authenticate interactively, run `ileap auth login` on a TTY.

## For agents

An iLEAP CLI skill is defined at `.agents/skills/ileap-cli/`. Read both files before working with iLEAP data:

- `.agents/skills/ileap-cli/SKILL.md` — auth, commands, dashboard procedure, output format, exit codes
- `.agents/skills/ileap-cli/SCHEMAS.md` — field reference, filter syntax, and schema discovery workflow for all resource types

The skill is standalone and portable: it bootstraps the `ileap` binary from a bundled build, falling back to `cargo install --git https://github.com/sine-fdn/ileap-cli-test`. To deploy it elsewhere (including as a Claude.ai skill):

1. `scripts/build-skill-binaries.sh` — build static Linux binaries into the skill's `bin/` directory (requires Docker; needed for sandboxed environments without a Rust toolchain)
2. `scripts/package-skill.sh` — produce `dist/ileap-cli-skill.zip`, ready to upload at claude.ai (Settings → Capabilities → Skills) or to unpack into any agent's skills directory
