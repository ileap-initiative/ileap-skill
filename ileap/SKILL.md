---
name: ileap
description: >-
  Use the iLEAP CLI to query, filter, and explore iLEAP API resources (shipments, footprints,
  tocs, hocs, tad, aed). REQUIRED for all iLEAP work — use this skill before reading source
  code, fetching data, or generating any iLEAP output. Prevents common mistakes: wrong
  terminology (TOC = Transport Operation Category, HOC = Hub Operation Category — not
  "Characteristics"), incorrect field names, and redundant reimplementation. USE FOR: fetching
  shipments, footprints, tocs, hocs, tad, aed; filtering and paginating; checking auth;
  exploring transport emissions data; rendering an HTML dashboard of iLEAP data. TRIGGER
  PHRASES: "show ileap", "fetch ileap", "query iLEAP", "ileap summary", "list shipments",
  "list footprints", "show tocs", "show hocs", "show tad", "show aed", "ileap dashboard",
  and dashboard requests scoped to iLEAP or transport-emissions data (e.g. "build a
  dashboard of shipments, footprints, or emissions"). NOT FOR: dashboards or data unrelated
  to iLEAP.
license: MIT
compatibility: >-
  Designed for Claude Code, Claude.ai, and similar agent environments. Requires network
  access to an iLEAP API endpoint and permission to execute a bundled binary. Prebuilt
  binaries cover Linux x86_64, Linux aarch64, and macOS arm64 (Apple Silicon); other
  platforms are unsupported. No Rust toolchain is needed at runtime.
metadata:
  cli_version: "0.1.0"
---

# iLEAP CLI Skill

## Step 0 — Read schemas before doing anything else

**Read [SCHEMAS.md](./references/SCHEMAS.md) — located in the ./references directory — now, before reading further or taking any action.**

It contains the authoritative terminology definitions and field schemas for all iLEAP resources. Without it you will make mistakes — for example, TOC stands for Transport Operation **Category** (not Characteristics), and field names differ from what general knowledge would suggest.

Throughout this document, "the skill directory" means the directory containing this SKILL.md file.

If [SCHEMAS.md](./references/SCHEMAS.md) does not exist in the skill directory, stop and inform the user: "SCHEMAS.md is missing from the skill directory. Cannot proceed safely without authoritative field definitions. Please reinstall the skill." Do not attempt to infer field names from general knowledge.

If the file exists but cannot be read or is empty, stop and inform the user: "SCHEMAS.md in the skill directory is unreadable or empty. Cannot proceed safely. Please reinstall the skill."

## DO NOT

- **DO NOT** read the CLI source code to understand the API or data shapes — use [SCHEMAS.md](./references/SCHEMAS.md) instead.
- **DO NOT** implement your own data-fetching logic — use the CLI commands documented here.
- **DO NOT** assume what any iLEAP term means — check the Terminology table in [SCHEMAS.md](./references/SCHEMAS.md) first.
- **DO NOT** write a new CLI command when the user asks for a dashboard — run the Dashboard Procedure below.
- **DO NOT** violate the dashboard emoji policy. See Styling constraints: No emojis.

## Prerequisites

This skill uses **prebuilt `ileap` binaries only** — never compile the CLI at runtime (no `cargo install`, no `cargo run`, no `cargo build`). The binaries are built from the ileap-skill repository and shipped inside the skill bundle, so the skill works in any environment, including sandboxed ones (e.g. Claude.ai) without a Rust toolchain. Obtain a working binary by trying these steps **in order** — stop at the first one that succeeds:

**Step 1 — Already on PATH?**

```bash
which ileap || echo "not installed"
```

If found, use it and skip the rest of this section.

**Step 2 — Bundled binary.** The skill directory contains prebuilt binaries at `bin/ileap-<OS>-<ARCH>` (e.g. `bin/ileap-Linux-x86_64`, `bin/ileap-Linux-aarch64`, `bin/ileap-Darwin-arm64`). Use the one matching the current platform:

```bash
SKILL_DIR=<absolute path of the directory containing this SKILL.md>
CANDIDATE="$SKILL_DIR/bin/ileap-$(uname -s)-$(uname -m)"
if [ -f "$CANDIDATE" ]; then
  mkdir -p /tmp/ileap-bin && cp "$CANDIDATE" /tmp/ileap-bin/ileap && chmod +x /tmp/ileap-bin/ileap
  export PATH="/tmp/ileap-bin:$PATH"
  ileap --version
fi
```

If `ileap --version` succeeds, use it and skip the rest of this section. Remember to prepend `/tmp/ileap-bin` to PATH (or use the absolute path `/tmp/ileap-bin/ileap`) in every subsequent shell invocation, since environment changes do not persist between commands.

**Step 3 — No binary available.** Report to the user which platform was detected (`uname -s` / `uname -m`) and which binaries exist in the skill's `bin/` directory, then stop — do not attempt to fetch data or generate a dashboard without a working CLI binary, and do not compile one. Inform the user: "The skill bundle has no `ileap` binary for this platform. Rebuild the bundle from the ileap-skill repository using `scripts/build-skill-binaries.sh` and `scripts/package-skill.sh`."

## Overview

`ileap` is a CLI tool for the iLEAP API. It supports listing all iLEAP resource types,
filtering, pagination, and structured JSON output suited for agent consumption.

Default behavior: use standard query/fetch flows for data requests. Use the Dashboard Procedure only when the user explicitly asks for a dashboard or visual HTML output.

## Authentication

> Note: This section is for non-dashboard flows. For dashboard flows, use the self-contained auth decision block in the Dashboard Procedure (step 1) instead of this section.

For non-dashboard flows, verify auth before fetching data:

```bash
ileap auth status
```

To authenticate interactively:

```bash
ileap auth login
```

Or supply credentials directly:

```bash
ileap --username "$ILEAP_USERNAME" --password "$ILEAP_PASSWORD" auth login
```

Or use a token:

```bash
ileap --token "$ILEAP_TOKEN" auth login
```

Credentials can also be set via environment variables:
- `ILEAP_TOKEN`
- `ILEAP_USERNAME` / `ILEAP_PASSWORD`
- `ILEAP_BASE_URL` (default: `https://api.preview.ileap.dev`)

### Demo Server

A public demo server is available for exploration and presentations:

| Setting | Value |
|---|---|
| Base URL | `https://api.preview.ileap.dev` |
| Username | `hello` |
| Password | `pathfinder` |

```bash
ileap --base-url https://api.preview.ileap.dev --username hello --password pathfinder auth login
```

Once logged in, the token is cached and subsequent commands can omit credentials:

```bash
ileap --base-url https://api.preview.ileap.dev shipments list --yes
```

## Resource Types

| Command | Endpoint | Data Type |
|---|---|---|
| `footprints list` | `/2/footprints` | PACT DT1/DT2 combined |
| `footprints get <id>` | `/2/footprints/<id>` | Single PACT footprint by UUID |
| `shipments list` | `/v1/ileap/shipments` | iLEAP ShipmentFootprints (DT1) |
| `tocs list` | `/v1/ileap/tocs` | iLEAP TOCs (DT2) |
| `hocs list` | `/v1/ileap/hocs` | iLEAP HOCs (DT2) |
| `tad list` | `/v1/ileap/tad` | Transport Activity Data (DT3) |
| `aed list` | `/v1/ileap/aed` | Aggregated Emissions Data (DT4) |

## Listing Resources

```bash
# Machine-readable compact JSON (preferred for agents)
ileap -o compact shipments list --yes

# With a page limit
ileap -o compact shipments list --limit 100 --yes

# Cap at 5 pages max
ileap -o compact shipments list --limit 50 --max-pages 5 --yes
```

Always pass `--yes` (or `-y`) in non-interactive contexts to skip paging prompts.

## Filtering

**iLEAP standalone endpoints** (shipments, tocs, hocs, tad, aed) use `key=value` pairs:

```bash
# Filter by transport mode
ileap -o compact shipments list --yes -f mode=road

# Filter by ID
ileap -o compact shipments list --yes -f id=abc-123

# Nested attribute (dot notation)
ileap -o compact shipments list --yes -f origin.city=Berlin

# Interval filter
ileap -o compact shipments list --yes -f "created=gt:2024-01-01T00:00:00Z"
```

For common filterable fields and how to discover deeper nested fields from the OpenAPI spec, see [SCHEMAS.md](./references/SCHEMAS.md).

**PACT footprints** use OData syntax:

```bash
ileap -o compact footprints list --yes -f "created lt '2024-01-01T00:00:00Z'"
```

## Getting a Single Footprint

`footprints get` is only available for PACT footprints, not for iLEAP standalone resources.
Use `-f id=<uuid>` to look up individual records on iLEAP standalone endpoints.

```bash
ileap -o compact footprints get <uuid>
```

Returns exit code `3` (not found) if the UUID does not exist.

## Dry Run

Preview the request without executing it:

```bash
ileap -o compact shipments list --dry-run
ileap -o compact footprints get <uuid> --dry-run
```

## Output Format

All output is JSON on stdout. Errors are JSON on stderr:

```json
{ "cli_error": { "type": "auth_error", "message": "..." } }
```

Exit codes: `0` = success, `1` = general error, `3` = not found, `4` = auth error.

## Dashboard Procedure

Use this section only for dashboard requests. For normal data retrieval, summaries, filtering, and exploration tasks, use the non-dashboard command flows above.

Dashboards can show **all resources** or be **scoped to specific ones** based on the user's request.
Only fetch and display the resources the user asked about.

Examples:
- "show me a dashboard" → fetch all 6 resource types
- "show me a shipments dashboard" → fetch only `shipments`
- "show me TOCs and HOCs" → fetch only `tocs` and `hocs`

### 1. Check auth

Auth Check (for dashboard flows only):

- (a) Run `ileap -o compact auth status`.
- (b) If the command exits with a non-zero code and the error is not auth-related (for example, binary missing), stop and report the error to the user.
- (c) If the command exits with a non-zero code, or `authenticated` is `false`, attempt demo server login:
   ```bash
   ileap --base-url https://api.preview.ileap.dev --username hello --password pathfinder auth login
   ```
- (d) If demo login fails (exit code `4`), ask the user for credentials and stop. Do not attempt to fetch data until authenticated.
- (e) If `authenticated` is `true` or demo login succeeds, continue to fetch.

### 2. Fetch the relevant resources in parallel

Run all required resource commands in a single message as parallel tool calls — do not fetch sequentially.

Issue each resource command as a separate parallel Bash tool call, each with its own stderr redirect: `ileap -o compact shipments list --yes --limit 50 --max-pages 1 2>/tmp/ileap-shipments-err.json`. Store the exit code from each call independently.

Use a single explicit target base URL for all fetch commands in this step: if demo login was used, set it to `https://api.preview.ileap.dev`; if the user supplied a custom base URL, use that URL.

If all resource fetches return non-zero exit codes, do not generate a dashboard. Instead, report the errors to the user and ask them to verify connectivity and re-run.

If some but not all fetches fail, generate the dashboard showing available data and displaying the per-resource error state for failed resources. Only abort dashboard generation when every requested resource fetch has failed.

```bash
ileap --base-url <url> -o compact shipments list --yes --limit 50 --max-pages 1
ileap --base-url <url> -o compact tocs       list --yes --limit 50 --max-pages 1
# etc. — all issued at the same time
```

**Capture stderr separately** to detect errors without swallowing output:

```bash
SHIPMENTS=$(ileap -o compact shipments list --yes --limit 50 --max-pages 1 2>/tmp/ileap-shipments-err.json); SHIPMENTS_EXIT=$?
# stdout is captured in $SHIPMENTS; stderr is captured in /tmp/ileap-shipments-err.json
```

On non-zero exit, read `cli_error.type` from stderr:
- `auth_error` → re-authenticate before retrying
- `not_found` → endpoint not supported on this server; mark as unavailable in the dashboard
- `error` → show the message in the dashboard card

**Useful jq snippets** once you have the JSON response (if `jq` is unavailable in the environment, use `python3 -c` with the `json` module instead):

```bash
# Count records
echo "$SHIPMENTS" | jq '.data | length'

# Extract the data array for iteration
echo "$SHIPMENTS" | jq '.data[]'

# Pluck a single field from all records
echo "$TOCS" | jq '[.data[].tocId]'
```

### 3. Generate and open the dashboard

**Step 3a — Resolve timestamp:** Before writing, determine the current UTC timestamp:

```bash
TS=$(date -u +%Y%m%d-%H%M%S)
```

If `date -u` returns a non-zero exit code, use the literal fallback filename `/tmp/ileap-dashboard-unknown-ts.html` and note the timestamp could not be resolved.

Use this value in the filename (e.g. `/tmp/ileap-dashboard-20240315-142300.html`). Do not write a file named literally `ileap-dashboard-YYYYMMDD-HHMMSS.html`.

**Step 3b — Write and open:** Before writing HTML, resolve the logo from the skill directory and encode it as a base64 data URI so the HTML is self-contained (it must render correctly even when downloaded to another machine — never reference the logo by filesystem path):

```bash
SKILL_DIR=<absolute path of the directory containing this SKILL.md>
if [ -f "$SKILL_DIR/assets/ileap-logo.png" ]; then base64 < "$SKILL_DIR/assets/ileap-logo.png" | tr -d '\n' > /tmp/ileap-logo-b64.txt; echo "logo available"; else echo "no logo"; fi
```

Then write the HTML using the **Write tool**, then open the file:

```
Write tool → /tmp/ileap-dashboard-$TS.html
```

If the Write tool returns an error, report the error to the user and provide the full HTML content as a code block in the chat instead of attempting to open a file.

How to deliver the file depends on the environment:

- **Sandboxed/headless environment (e.g. Claude.ai, remote sessions, CI):** there is no browser to open. Provide the HTML file to the user as a downloadable output file, and state its absolute path. Do not attempt `open`/`xdg-open`.
- **Local desktop environment:** open the file with the OS-appropriate command:

```bash
os=$(uname -s)
if [ "$os" = "Darwin" ]; then open /tmp/ileap-dashboard-$TS.html
elif [ "$os" = "Linux" ]; then xdg-open /tmp/ileap-dashboard-$TS.html
else start /tmp/ileap-dashboard-$TS.html
fi
```

If the open command fails, fall back to the headless behavior: provide the file and print the absolute path so the user can open it manually.

**Before calling the Write tool, confirm each of the following 7 items is present in the HTML you are about to write:**

1. **Header** — iLEAP logo and metadata: If `assets/ileap-logo.png` exists in the skill directory (see Step 3b), embed it inline as `<img src="data:image/png;base64,<contents of /tmp/ileap-logo-b64.txt>">` so the HTML is fully self-contained and renders on any machine; if the logo is unavailable, omit the `<img>` element entirely and render the text "iLEAP" as a plain `<h1>` instead. Include the base URL and generation timestamp alongside the logo/heading.
2. **Auth status badge** — shows whether the session is authenticated.
3. **Summary cards** — one card per resource type showing the total record count, or an error message if the fetch failed.
4. **Expanded record cards** — one card per record showing all mandatory fields (see table below); if a mandatory field is absent from a record, show it explicitly as `—`. Plus any additional top-level scalar or object fields present in the record, rendered as a key-value table; nested arrays (e.g. TCEs inside a shipment) should each be rendered as a sub-table.
5. **Collapsible raw JSON** — a collapsible block per record showing the full raw JSON.
6. **Cross-references** — see Cross-Reference Resolution Rules below.
7. **Styling** — light background (white or light grey), inline CSS only, no external dependencies. See Styling constraints: No emojis.

### Cross-Reference Resolution Rules

| Condition | What to display |
|---|---|
| TOC/HOC data was not fetched for this dashboard scope, or the TOC/HOC fetch failed | Show raw `tocId`/`hocId` with note `(TOC/HOC data not loaded)` |
| TOC/HOC data was fetched, but the data array is empty | Show raw `tocId`/`hocId` with note `(record not found in fetched data)` |
| TOC/HOC data was fetched, but the specific `tocId`/`hocId` is not found in fetched records | Show raw `tocId`/`hocId` with note `(record not found in fetched data)` |

**Mandatory fields — always show these in the structured view:**

| Resource | Mandatory fields |
|---|---|
| ShipmentFootprint | `shipmentId`, `mass`, `tces` |
| TCE (inside each shipment) | `tceId`, `shipmentId`, `mass`, `co2eTTW`, `co2eWTW`, `distance`, `transportActivity` |
| TOC | `tocId`, `mode`, `co2eIntensityTTW`, `co2eIntensityWTW`, `energyCarriers`, `transportActivityUnit` |
| HOC | `hocId`, `hubType`, `co2eIntensityTTW`, `co2eIntensityWTW`, `energyCarriers`, `transportActivityUnit` |
| TAD | `activityId`, `mode`, `distance`, `mass`, `transportActivity` |
| AED | `reportId`, `status`, `referencePeriodStart`, `referencePeriodEnd` |

These required fields come from the OpenAPI spec (`required` arrays on each schema). If a mandatory field is absent from a record, show it explicitly as `—` (not missing silently).

Keep styling clean and modern with inline CSS (no external dependencies).

**Styling constraints:**
- Always use a light background (white or light grey). Do not use dark mode — the iLEAP logo brand guidelines require a light background context.
- **No emojis.** Do not use emoji characters anywhere in the generated HTML — not in headings, labels, badges, buttons, table cells, or descriptive text. This means no raw emoji, no `&#128xxx;` or `&#x1Fxxx;` numeric character references, and no emoji-adjacent codepoints (U+1F000 and above). Use plain text only.
