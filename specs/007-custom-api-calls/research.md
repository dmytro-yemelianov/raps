# Research: Custom API Calls

**Feature**: 007-custom-api-calls
**Date**: 2026-01-22

## Research Summary

This document captures design decisions and research findings for implementing custom API calls in RAPS.

---

## 1. Domain Validation Strategy

**Decision**: Allowlist-based URL validation with compile-time domain constants

**Rationale**:
- Security requirement FR-005 mandates restricting requests to APS domains only
- Prevents credential leakage to external URLs (SSRF-like attack vector)
- Simple allowlist is sufficient given the known set of Autodesk API hosts

**Alternatives Considered**:
| Alternative | Reason Rejected |
|------------|-----------------|
| Regex pattern matching | Overly complex, prone to bypass via subdomain tricks |
| Denylist (block known bad domains) | Insecure - can't anticipate all malicious domains |
| No validation (trust user) | Violates security requirement, enables credential leakage |

**Implementation**:
```rust
const ALLOWED_DOMAINS: &[&str] = &[
    "developer.api.autodesk.com",
    "api.userprofile.autodesk.com",
    "acc.autodesk.com",
    "developer.autodesk.com",
];

fn is_allowed_url(url: &str) -> bool {
    // Parse URL, extract host, check against allowlist
}
```

---

## 2. CLI Subcommand Architecture

**Decision**: Method-based subcommands (`raps api get`, `raps api post`, etc.)

**Rationale**:
- Matches clarification decision from spec (Session 2026-01-22)
- Follows established pattern in RAPS (e.g., `bucket create`, `object delete`)
- More discoverable via `raps api --help`
- Enables method-specific argument validation (e.g., GET doesn't need `--data`)

**Alternatives Considered**:
| Alternative | Reason Rejected |
|------------|-----------------|
| Single command with `--method` flag | Less ergonomic, requires flag for every call |
| HTTP verb aliases (`raps get`, `raps post`) | Pollutes top-level namespace |

**Implementation**: Clap enum with 5 variants:
```rust
#[derive(Subcommand)]
pub enum ApiCommands {
    Get { endpoint: String, ... },
    Post { endpoint: String, ... },
    Put { endpoint: String, ... },
    Patch { endpoint: String, ... },
    Delete { endpoint: String, ... },
}
```

---

## 3. Request Body Handling

**Decision**: Support both inline JSON (`--data`) and file input (`--data-file`) with mutual exclusion

**Rationale**:
- Inline JSON is convenient for small payloads and scripting
- File input handles large/complex payloads and avoids shell escaping issues
- Mutual exclusion prevents ambiguity (which takes precedence?)
- Matches curl pattern (`-d` vs `@file`)

**Alternatives Considered**:
| Alternative | Reason Rejected |
|------------|-----------------|
| Only inline JSON | Large payloads become unwieldy, shell escaping problems |
| Only file input | Inconvenient for simple requests, requires temp files |
| Allow both (merge) | Complex semantics, potential for confusion |
| Stdin support | Adds complexity; file input covers same use case |

**Implementation**:
```rust
#[arg(short, long, conflicts_with = "data_file")]
data: Option<String>,

#[arg(short = 'f', long, conflicts_with = "data")]
data_file: Option<PathBuf>,
```

---

## 4. Output Handling

**Decision**: Reuse existing OutputFormat system with content-type detection

**Rationale**:
- Maintains consistency with other RAPS commands
- JSON responses format naturally with existing table/yaml/csv converters
- Non-JSON responses need special handling (text pass-through, binary to file)

**Alternatives Considered**:
| Alternative | Reason Rejected |
|------------|-----------------|
| Always output raw response | Loses formatting benefits, inconsistent with other commands |
| Force JSON-only responses | Some APS endpoints return XML or binary data |

**Implementation**:
1. Check response Content-Type header
2. If JSON: deserialize and pass through OutputFormat formatter
3. If text (XML, HTML, plain): display as-is
4. If binary: require `--output` flag, write to file

---

## 5. MCP Tool Design

**Decision**: Single `api_request` tool with method parameter

**Rationale**:
- MCP tools are invoked programmatically, not typed by humans
- Single tool with method enum is cleaner than 5 separate tools
- Easier for AI assistants to discover (one tool to learn)
- JSON schema clearly defines available methods

**Alternatives Considered**:
| Alternative | Reason Rejected |
|------------|-----------------|
| Separate tools (`api_get`, `api_post`, etc.) | Clutters tool list, redundant parameter definitions |
| Match CLI structure exactly | MCP ergonomics differ from CLI; single tool is idiomatic |

**Implementation**:
```rust
// Tool parameters
{
    "method": "GET" | "POST" | "PUT" | "PATCH" | "DELETE",
    "endpoint": "/path/to/resource",
    "body": { ... },  // optional, for POST/PUT/PATCH
    "headers": { ... },  // optional
    "query": { ... }  // optional
}
```

---

## 6. Error Handling Strategy

**Decision**: Map HTTP status codes to appropriate exit codes and error messages

**Rationale**:
- CLI scripts need reliable exit codes for flow control
- Users need actionable error messages, not raw HTTP status

**Exit Code Mapping**:
| HTTP Status | Exit Code | Meaning |
|-------------|-----------|---------|
| 2xx | 0 | Success |
| 401, 403 | 10 | Authentication error |
| 400, 422 | 2 | Client/validation error |
| 404 | 1 | Not found |
| 429 | 1 | Rate limited (with retry info) |
| 5xx | 1 | Server error |

---

## 7. Query Parameter Handling

**Decision**: Support `--query key=value` flag (repeatable) with URL encoding

**Rationale**:
- Clean separation from endpoint path
- Automatic URL encoding prevents encoding errors
- Repeatable flag for multiple parameters
- Matches common CLI patterns (curl `-d`, httpie `==`)

**Alternatives Considered**:
| Alternative | Reason Rejected |
|------------|-----------------|
| Require params in URL string | Error-prone encoding, harder to construct dynamically |
| JSON object for params | Overly verbose for simple key-value pairs |

**Implementation**:
```rust
#[arg(short, long = "query", value_parser = parse_key_value)]
query: Vec<(String, String)>,
```

---

## 8. Authentication Flow

**Decision**: Reuse existing auth mechanism; require prior authentication

**Rationale**:
- `raps auth login` already handles 2-legged, 3-legged, and device code flows
- Token stored securely in keyring, refreshed automatically
- No need to duplicate auth logic in custom API command

**Implementation**:
- Get token via `auth_client.get_token()` or `auth_client.get_3leg_token()`
- If no token available, display error with `raps auth login` guidance
- Token added automatically as `Authorization: Bearer {token}` header

---

## Dependencies

No new dependencies required. All functionality can be implemented with existing workspace crates:
- `raps-kernel`: AuthClient, HttpClientConfig, Config
- `reqwest`: Direct HTTP requests (already a dependency)
- `clap`: CLI parsing (already a dependency)
- `serde_json`: JSON parsing/validation (already a dependency)

---

## Open Items

None. All technical decisions resolved.
