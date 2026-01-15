# Data Model: Global Output Format

## Enums

### `OutputFormat`
Represents the user-selected or auto-detected output format.

| Variant | Description | Serialization |
|---------|-------------|---------------|
| `Json`  | Standard JSON output | `serde_json::to_string_pretty` |
| `Yaml`  | YAML output | `serde_yaml::to_string` |
| `Table` | ASCII table (human readable) | `comfy_table` or similar (existing) |
| `Csv`   | Comma-separated values | `csv` crate |
| `Plain` | Simple text/string output | `Display` implementation |

## Logic Flow

1. **Parse Args**: `clap` extracts `--output`.
2. **Detect TTY**: Check `std::io::stdout().is_terminal()`.
3. **Determine Format**:
   - If `--output` is set -> Use it.
   - Else if `!is_terminal()` -> Default to `Json`.
   - Else -> Default to `Table` (or command-specific default).
4. **Execute Command**: Run domain logic, get `Result<T>`.
5. **Format & Print**: Pass `T` to `OutputFormatter`.

## Interfaces

```rust
pub enum OutputFormat {
    Json,
    Yaml,
    Table,
    Csv,
    Plain,
}

pub trait Printable: Serialize {
    // Optional: for specialized table rendering
    fn headers(&self) -> Vec<String>;
    fn row(&self) -> Vec<String>;
}
```
