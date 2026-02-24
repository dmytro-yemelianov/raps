# Contract: Model Derivative Metadata Endpoints

## GET /metadata — List Model Views

**CLI**: `raps derivative metadata <URN> [--region <REGION>] [--output <FORMAT>]`
**Client**: `client.get_metadata(urn: &str) -> Result<ModelViews>`
**APS endpoint**: `GET {base}/modelderivative/v2/designdata/{urn}/metadata`
**Auth**: 2-legged or 3-legged, scope `data:read`
**Headers**: `x-ads-region` if region specified

**Response**: JSON with `data.metadata` array containing view objects with guid, name, role.

**Error cases**:
- 404: Model not found or not translated
- 401/403: Auth failure
- 409: Translation still in progress

---

## GET /metadata/{guid} — Fetch Object Tree

**CLI**: `raps derivative tree <URN> <GUID> [--region <REGION>] [--output <FORMAT>]`
**Client**: `client.get_object_tree(urn: &str, model_guid: &str) -> Result<ObjectTree>`
**APS endpoint**: `GET {base}/modelderivative/v2/designdata/{urn}/metadata/{modelGuid}`
**Auth**: 2-legged or 3-legged, scope `data:read`

**Query params**: `forceget` (boolean) — force fresh data even if cached

**Response**: JSON with `data.objects` array containing hierarchical tree nodes.

**Error cases**:
- 404: Model or GUID not found
- 413: Object tree too large (use pagination)

---

## GET /metadata/{guid}/properties — Fetch All Properties

**CLI**: `raps derivative properties <URN> <GUID> [--object-id <ID>] [--region <REGION>] [--output <FORMAT>]`
**Client**: `client.get_properties(urn: &str, model_guid: &str) -> Result<PropertiesResult>`
**APS endpoint**: `GET {base}/modelderivative/v2/designdata/{urn}/metadata/{modelGuid}/properties`
**Auth**: 2-legged or 3-legged, scope `data:read`

**Query params**: `objectid` (filter to specific object), `forceget`

**Response**: JSON with `data.collection` array of property objects.

**Error cases**:
- 404: GUID not found
- 413: Properties too large

---

## POST /metadata/{guid}/properties:query — Query Specific Properties

**CLI**: `raps derivative query-properties <URN> <GUID> --filter <OBJECT_IDS> [--fields <FIELDS>] [--region <REGION>] [--output <FORMAT>]`
**Client**: `client.query_properties(urn: &str, model_guid: &str, query: PropertyQuery) -> Result<PropertiesResult>`
**APS endpoint**: `POST {base}/modelderivative/v2/designdata/{urn}/metadata/{modelGuid}/properties:query`
**Auth**: 2-legged or 3-legged, scope `data:read`

**Request body**: JSON with `query.filter` (object IDs), optional `fields`, optional `pagination`.

**Response**: Same as GET properties but filtered.

**Error cases**:
- 400: Invalid query filter
- 404: GUID not found
