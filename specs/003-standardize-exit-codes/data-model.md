# Data Model: Exit Codes

## Enums

### `ExitCode`

| Variant | Code | Description |
|---------|------|-------------|
| `Success` | 0 | Success |
| `Usage` | 2 | Invalid arguments |
| `Auth` | 3 | Authentication/Authorization failure |
| `NotFound` | 4 | Resource not found |
| `Remote` | 5 | API/Network error |
| `Internal` | 6 | Internal error |

## Interfaces

```rust
pub enum ExitCode {
    Success = 0,
    Usage = 2,
    Auth = 3,
    NotFound = 4,
    Remote = 5,
    Internal = 6,
}

impl ExitCode {
    pub fn from_error(err: &anyhow::Error) -> Self { ... }
    pub fn exit(self) -> ! { ... }
}
```
