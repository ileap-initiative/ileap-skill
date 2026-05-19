# iLEAP Data Schemas

**Source of truth:** The OpenAPI spec at `https://ileap-preview.fly.dev/openapi.json` documents all resources, fields, enums, and nesting.

This page shows common patterns and how to explore the schema dynamically. For any field not listed below, use the jq recipes in the **Discovery Workflow** section to extract it from the OpenAPI spec.

## Filtering Overview

All iLEAP standalone endpoints use **dot notation for nested fields**: `parent.child.grandchild=value`

Examples:
- `origin.city=Berlin` — top-level nested field
- `energyCarriers.energyCarrier=Diesel` — field inside an array
- `energyCarriers.feedstocks.feedstock=Fossil` — field nested multiple levels deep

**Not documented here?** Use the discovery workflow below to extract the schema for any field directly from OpenAPI.json.

## ShipmentFootprints (Data Transaction 1)

**Endpoint:** `shipments list`
**Filter syntax:** `key=value` pairs with dot notation for nested fields
**Common fields:**

| Field | Type | Example |
|---|---|---|
| `shipmentId` | string (UUID-like) | `-f shipmentId=abc-123` |
| `status` | enum: `Active`, `Deprecated` | `-f status=Active` |
| `createdAt` | ISO 8601 datetime | `-f createdAt=gt:2024-01-01T00:00:00Z` |
| `referencePeriodStart` | ISO 8601 datetime | `-f referencePeriodStart=lt:2024-12-31T23:59:59Z` |
| `origin.city` | string | `-f origin.city=Berlin` |
| `origin.country` | 2-letter country code (ISO 3166) | `-f origin.country=DE` |
| `destination.city` | string | `-f destination.city=Amsterdam` |
| `destination.country` | 2-letter country code (ISO 3166) | `-f destination.country=NL` |
| `mass` | decimal | `-f mass=gt:1000` |
| `companyName` | string | `-f companyName=Acme` |

**Multi-filter example:**

```bash
# Shipments from Berlin to Amsterdam
ileap shipments list --yes \
  -f origin.city=Berlin \
  -f destination.city=Amsterdam
```

**Discovery tip:** Fetch one record to see the full structure:

```bash
ileap -o compact shipments list --yes --limit 1 | jq '.'
```

---

## TOCs (Data Transaction 2)

**Endpoint:** `tocs list`
**Filter syntax:** `key=value` pairs
**Common fields:**

| Field | Type | Example |
|---|---|---|
| `tocId` | string (UUID-like) | `-f tocId=xyz-789` |
| `status` | enum: `Active`, `Deprecated` | `-f status=Active` |
| `createdAt` | ISO 8601 datetime | `-f createdAt=gt:2024-01-01T00:00:00Z` |
| `mode` | enum: `Road`, `Rail`, `Air`, `Sea`, `InlandWaterway` | `-f mode=Road` |
| `companyName` | string | `-f companyName=Logistics Inc` |

---

## HOCs (Data Transaction 2)

**Endpoint:** `hocs list`
**Filter syntax:** `key=value` pairs
**Common fields:**

| Field | Type | Example |
|---|---|---|
| `hocId` | string (UUID-like) | `-f hocId=hub-456` |
| `status` | enum: `Active`, `Deprecated` | `-f status=Active` |
| `createdAt` | ISO 8601 datetime | `-f createdAt=gt:2024-01-01T00:00:00Z` |
| `hubType` | enum: `Transshipment`, `StorageAndTransshipment`, `Warehouse`, `LiquidBulkTerminal`, `MaritimeContainerTerminal` | `-f hubType=Warehouse` |
| `hubLocation.city` | string | `-f hubLocation.city=Rotterdam` |
| `hubLocation.country` | 2-letter country code (ISO 3166) | `-f hubLocation.country=NL` |
| `companyName` | string | `-f companyName=Hub Operator Ltd` |

---

## TAD (Data Transaction 3 — Transport Activity Data)

**Endpoint:** `tad list`
**Filter syntax:** `key=value` pairs
**Common fields:**

| Field | Type | Example |
|---|---|---|
| `activityId` | string | `-f activityId=activity-001` |
| `mode` | enum: `Road`, `Rail`, `Air`, `Sea`, `InlandWaterway` | `-f mode=Air` |
| `origin.city` | string | `-f origin.city=Berlin` |
| `origin.country` | 2-letter country code (ISO 3166) | `-f origin.country=DE` |
| `destination.city` | string | `-f destination.city=Singapore` |
| `destination.country` | 2-letter country code (ISO 3166) | `-f destination.country=SG` |
| `departureAt` | ISO 8601 datetime | `-f departureAt=gt:2024-01-01T00:00:00Z` |
| `distance.actual` | decimal (kilometers) | `-f distance.actual=gt:1000` |

---

## AED (Data Transaction 4 — Aggregated Emissions Data)

**Endpoint:** `aed list`
**Filter syntax:** `key=value` pairs
**Common fields:**

| Field | Type | Example |
|---|---|---|
| `reportId` | string | `-f reportId=report-2024-001` |
| `status` | enum: `Active`, `Deprecated` | `-f status=Active` |
| `createdAt` | ISO 8601 datetime | `-f createdAt=gt:2024-01-01T00:00:00Z` |
| `standardsUsed` | array of strings | `-f standardsUsed=ISO14083:2023` |

---

## PACT Footprints (Endpoints `/2/footprints` and `/2/footprints/<id>`)

**Endpoint:** `footprints list` and `footprints get <id>`
**Filter syntax:** OData v4 (different from iLEAP standalone)
**Common fields:**

| Field | Type | Example |
|---|---|---|
| `id` | UUID | `-f "id eq '550e8400-e29b-41d4-a716-446655440000'"` |
| `status` | enum: `Active`, `Deprecated` | `-f "status eq 'Active'"` |
| `created` | ISO 8601 datetime | `-f "created lt '2024-01-01T00:00:00Z'"` |
| `productNameCompany` | string | `-f "productNameCompany eq 'Widget Pro'"` |
| `companyName` | string | `-f "contains(companyName, 'Acme')"` |

**OData examples:**

```bash
# Footprints created before Jan 2024
ileap footprints list --yes -f "created lt '2024-01-01T00:00:00Z'"

# Footprints from companies containing "Acme"
ileap footprints list --yes -f "contains(companyName, 'Acme')"

# Deprecated footprints
ileap footprints list --yes -f "status eq 'Deprecated'"
```

See [OData v4 filter spec](http://docs.oasis-open.org/odata/odata/v4.0/errata03/os/complete/part2-url-conventions/odata-v4.0-errata03-os-part2-url-conventions.html) for full syntax.

---

## Discovery Workflow

For any field not in the tables above, extract the schema directly from the OpenAPI spec.

### Quick Start: Fetch and Explore Locally

1. **Download the OpenAPI spec:**
   ```bash
   curl -s https://ileap-preview.fly.dev/openapi.json > openapi.json
   ```

2. **Find the schema for a resource (e.g., ShipmentFootprint):**
   ```bash
   jq '.components.schemas.ShipmentFootprint' openapi.json | less
   ```

3. **Extract all field names and types from a schema:**
   ```bash
   jq '.components.schemas.ShipmentFootprint.properties | keys' openapi.json
   ```

4. **Find all enum values for a field:**
   ```bash
   # E.g., find all valid TransportMode values
   jq '.components.schemas.TransportMode.enum' openapi.json
   ```

5. **Explore nested objects (e.g., Location schema inside ShipmentFootprint):**
   ```bash
   jq '.components.schemas.Location.properties' openapi.json
   ```

### Resource Type Mapping

| Endpoint | OpenAPI Schema Name | Response Field |
|---|---|---|
| `shipments list` | `ShipmentListingResponseInner` | `data[].shipmentId` |
| `tocs list` | `TocListingResponseInner` | `data[].tocId` |
| `hocs list` | `HocListingResponseInner` | `data[].hocId` |
| `tad list` | `TadListingResponseInner` | `data[].activityId` |
| `aed list` | `AedListingResponseInner` | `data[].reportId` |
| `footprints list` | `PfListingResponseInner` | `data[].id` |

### Example: Discovering TAD Energy Carrier Fields

1. **Get the TAD schema:**
   ```bash
   jq '.components.schemas.TAD.properties | keys' openapi.json
   ```
   Output includes `energyCarriers` (array type).

2. **Find the nested EnergyCarrier schema:**
   ```bash
   jq '.components.schemas.EnergyCarrier.properties | keys' openapi.json
   ```
   Output includes `energyCarrier`, `feedstocks`, `relativeShare`, etc.

3. **Find Feedstock enum:**
   ```bash
   jq '.components.schemas.FeedstockType.enum' openapi.json
   ```
   Output: `["Fossil", "Natural gas", "Grid", "Renewable electricity", "Cooking oil"]`

4. **Now you can filter:**
   ```bash
   ileap tad list --yes -f energyCarriers.feedstocks.feedstock=Fossil
   ```

### Fetch and Query OpenAPI in Code (for Agents)

If implementing automated schema discovery, fetch and parse directly:

```bash
# Get all enum values from any schema
curl -s https://ileap-preview.fly.dev/openapi.json | jq '.components.schemas.<SchemaName>.enum'

# Find all properties of a schema
curl -s https://ileap-preview.fly.dev/openapi.json | jq '.components.schemas.<SchemaName>.properties | keys'

# Chain through $ref pointers to resolve nested types
curl -s https://ileap-preview.fly.dev/openapi.json | jq '.components.schemas.ShipmentFootprint.properties.origin | ."$ref"'
# Output: "#/components/schemas/Location"
# Then fetch: curl ... | jq '.components.schemas.Location.properties'
```
