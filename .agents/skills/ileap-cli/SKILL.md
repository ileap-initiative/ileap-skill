---
name: ileap-cli
description: >-
  Use the iLEAP CLI to query iLEAP API resources and render an HTML dashboard.
  REQUIRED for all iLEAP work — use this skill before reading source code, fetching data,
  or generating any iLEAP output. Prevents common mistakes: wrong terminology (TOC = Transport
  Operation Category, HOC = Hub Operation Category — not "Characteristics"), incorrect field
  names, and redundant reimplementation. USE FOR: fetching shipments, footprints, tocs, hocs,
  tad, aed; filtering and paginating; checking auth; building, creating, or showing a dashboard;
  exploring transport emissions data. TRIGGER PHRASES: "show ileap", "fetch ileap",
  "list shipments", "list footprints", "query iLEAP", "ileap summary", "build a dashboard",
  "create a dashboard", "generate a dashboard", "show a dashboard", "ileap dashboard".
---

# iLEAP CLI Skill

## Step 0 — Read schemas before doing anything else

**Read `.agents/skills/ileap-cli/SCHEMAS.md` now, before reading further or taking any action.**

It contains the authoritative terminology definitions and field schemas for all iLEAP resources. Without it you will make mistakes — for example, TOC stands for Transport Operation **Category** (not Characteristics), and field names differ from what general knowledge would suggest.

## DO NOT

- **DO NOT** read the CLI source code to understand the API or data shapes — use SCHEMAS.md instead.
- **DO NOT** implement your own data-fetching logic — use the CLI commands documented here.
- **DO NOT** assume what any iLEAP term means — check the Terminology table in SCHEMAS.md first.
- **DO NOT** write a new CLI command when the user asks for a dashboard — run the Dashboard Procedure below.

## Prerequisites

Verify the CLI is installed before proceeding:

```bash
which ileap || echo "not installed"
```

If missing, install it:

```bash
cargo install ileap-cli
```

Requires Rust. If `cargo` is not available, install it first:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**If `ileap` is not in PATH but you are inside the project repo**, use `cargo run --` instead of installing — it compiles and runs the local build:

```bash
cargo run -- -o compact auth status
# equivalent to: ileap -o compact auth status
```

Throughout this skill, replace `ileap` with `cargo run --` whenever the global binary is absent.

## Permissions

To avoid Claude asking for permission on every CLI invocation, add the commands to the project allowlist once in `.claude/settings.json`:

```json
{
  "permissions": {
    "allow": [
      "Bash(ileap *)",
      "Bash(cargo run -- *)",
      "Bash(open /tmp/ileap-dashboard.html)"
    ]
  }
}
```

You can apply this by running `/update-config`.

## Overview

`ileap` is a CLI tool for the iLEAP API. It supports listing all iLEAP resource types,
filtering, pagination, and structured JSON output suited for agent consumption.

## Authentication

Always verify auth before fetching data:

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
- `ILEAP_BASE_URL` (default: `https://ileap-preview.fly.dev`)

### Demo Server

A public demo server is available for exploration and presentations:

| Setting | Value |
|---|---|
| Base URL | `https://ileap-preview.fly.dev` |
| Username | `hello` |
| Password | `pathfinder` |

```bash
ileap --base-url https://ileap-preview.fly.dev --username hello --password pathfinder auth login
```

Once logged in, the token is cached and subsequent commands can omit credentials:

```bash
ileap --base-url https://ileap-preview.fly.dev shipments list --yes
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

For common filterable fields and how to discover deeper nested fields from the OpenAPI spec, see [SCHEMAS.md](./SCHEMAS.md).

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

Dashboards can show **all resources** or be **scoped to specific ones** based on the user's request.
Only fetch and display the resources the user asked about.

Examples:
- "show me a dashboard" → fetch all 6 resource types
- "show me a shipments dashboard" → fetch only `shipments`
- "show me TOCs and HOCs" → fetch only `tocs` and `hocs`

### 1. Check auth

```bash
ileap -o compact auth status
```

If `authenticated` is `false`, do not prompt the user interactively. Instead, try the demo server credentials first:

```bash
ileap --base-url https://ileap-preview.fly.dev --username hello --password pathfinder auth login
```

Only ask the user for credentials if the demo server login also fails.

### 2. Fetch the relevant resources in parallel

Run all required resource commands in a single message as parallel tool calls — do not fetch sequentially. Add `--base-url` if not using the default:

```bash
ileap --base-url <url> -o compact shipments list --yes --limit 50 --max-pages 1
ileap --base-url <url> -o compact tocs       list --yes --limit 50 --max-pages 1
# etc. — all issued at the same time
```

**Capture stderr separately** to detect errors without swallowing output:

```bash
ileap -o compact shipments list --yes 2>/tmp/ileap-shipments-err.json
# then check exit code; if non-zero, read /tmp/ileap-shipments-err.json
```

On non-zero exit, read `cli_error.type` from stderr:
- `auth_error` → re-authenticate before retrying
- `not_found` → endpoint not supported on this server; mark as unavailable in the dashboard
- `error` → show the message in the dashboard card

**Useful jq snippets** once you have the JSON response:

```bash
# Count records
echo "$SHIPMENTS" | jq '.data | length'

# Extract the data array for iteration
echo "$SHIPMENTS" | jq '.data[]'

# Pluck a single field from all records
echo "$TOCS" | jq '[.data[].tocId]'
```

### 3. Generate and open the dashboard

Write the dashboard to `/tmp/ileap-dashboard.html` and open it in the default browser:

```bash
open /tmp/ileap-dashboard.html
```

**Content to include:**
- Header: iLEAP logo (use an absolute `file://` path to `ileap-logo.png` in the repo root if it exists, so the image resolves from `/tmp`), base URL, and generation timestamp
- Auth status badge
- One summary card per resource type showing the total record count returned, or an error message if the fetch failed
- One expanded row/card per record showing **all mandatory fields** (see table below), plus any other fields present in the data
- Collapsible raw JSON block per record
- **Cross-references:** wherever a record contains a `tocId` or `hocId` (e.g. inside a shipment's TCEs), resolve it against the fetched TOC/HOC data and display the linked record's key fields inline (mode, emission intensity). This makes the relationship between shipments and their transport operations visible without requiring the user to look up IDs manually.

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

**Styling constraint:** Always use a light background (white or light grey). Do not use dark mode — the iLEAP logo brand guidelines require a light background context.
