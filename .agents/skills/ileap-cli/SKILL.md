---
name: ileap-cli
description: >-
  Use the iLEAP CLI to query iLEAP API resources and render an HTML dashboard.
  USE FOR: fetching shipments, footprints, tocs, hocs, tad, aed data from the iLEAP API;
  filtering and paginating results; checking auth status; generating a dashboard overview;
  exploring transport emissions data. TRIGGER PHRASES: "show ileap dashboard",
  "fetch ileap data", "list shipments", "list footprints", "query iLEAP", "ileap summary".
---

# iLEAP CLI Skill

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

If `authenticated` is `false`, run `ileap auth login` first (see Authentication section).

### 2. Fetch the relevant resources

Only run commands for the resources the user requested. Add `--base-url` if not using the default:

```bash
ileap --base-url <url> -o compact shipments list --yes --limit 50 --max-pages 1
ileap --base-url <url> -o compact tocs       list --yes --limit 50 --max-pages 1
# etc.
```

For endpoints that return a non-zero exit code, capture stderr and check `cli_error.type`:
- `auth_error` → re-authenticate before retrying
- `not_found` → endpoint not supported on this server; mark as unavailable in the dashboard
- `error` → show the message in the dashboard card

### 3. Generate and open the dashboard

Write an `ileap-dashboard.html` file and open it in the default browser.

**Content to include:**
- Header: base URL and generation timestamp
- Auth status badge
- One summary card per resource type (e.g. one card for Shipments, one for TOCs) showing the total record count returned, or an error message if the fetch failed
- Collapsible block per resource with the first 5 records as pretty-printed JSON

Keep styling clean and modern with inline CSS (no external dependencies).
