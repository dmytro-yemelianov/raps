# Contract: Output Behavior

## Global Flag
`--output <FORMAT>`
Values: `json`, `yaml`, `table`, `csv`, `plain`

## Default Behaviors

| Scenario | `--output` Flag | TTY (Interactive) | Resulting Format |
|----------|-----------------|-------------------|------------------|
| Normal | (none) | Yes | `Table` (Human) |
| Pipe/CI | (none) | No | `Json` (Machine) |
| Explicit | `yaml` | Any | `Yaml` |
| Explicit | `json` | Any | `Json` |

## Error Output
All errors (logic, connection, serialization) MUST be printed to **STDERR**.
**STDOUT** must contain ONLY the requested data or be empty.

## JSON Schema Stability
For any command returning a resource (e.g., `Bucket`), the JSON structure MUST match the API response structure or the defined internal model, ensuring keys do not change between patch versions.
