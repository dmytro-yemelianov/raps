# ASVS V5/V12 Audit: Input Validation & File Handling

**Audit Date:** 2026-02-28
**RAPS Version:** 4.14.0
**ASVS Version:** 4.0.3

## Scope

Audit of input validation and file handling covering:
- Object download operations (`raps-cli/src/commands/object/download.rs`, `raps-oss/src/objects.rs`)
- Object upload operations (`raps-cli/src/commands/object/upload.rs`)
- URL validation and SSRF prevention (`raps-kernel/src/http.rs`)
- CSV input handling in admin module (`raps-cli/src/commands/admin/csv_ops.rs`)
- Pipeline execution safety (`raps-cli/src/commands/pipeline.rs`)

## Findings

### V5.1.1 - Path Traversal Protection in Download Operations

- **Status:** Gap
- **Evidence:** `raps-cli/src/commands/object/download.rs:66`, `raps-oss/src/objects.rs:135-188`
- **Details:** When no explicit output path is provided, the download command uses the object key as the filename:
  ```rust
  let output_path = output.unwrap_or_else(|| PathBuf::from(&object_key));
  ```
  The `object_key` comes from user input or API response and is used directly as a filesystem path. If a malicious object key contains path traversal sequences (e.g., `../../etc/shadow`), the file would be written to an arbitrary location. The underlying `download_object()` in `raps-oss/src/objects.rs:170` calls `File::create(output_path)` without sanitizing the path.

  The existing overwrite check at `download.rs:69-82` prompts before overwriting, which provides some protection, but it does not prevent creating files in unexpected directories.

  **Mitigating factors:** The object keys are typically controlled by the user who uploaded them, and the OSS API may sanitize object keys server-side. However, the client-side code does not enforce any path restrictions.

### V5.1.2 - URL Validation and SSRF Prevention

- **Status:** Met
- **Evidence:** `raps-kernel/src/http.rs:15-45`
- **Details:** The `is_allowed_url()` function provides SSRF protection for custom API calls by restricting URLs to a hardcoded list of Autodesk domains. The function:
  - Parses the URL using the `url` crate (RFC 3986 compliant)
  - Extracts and validates the host
  - Checks against the domain allowlist with proper subdomain boundary checks
  - Rejects localhost, internal IPs, and lookalike domains
  - Returns `false` for invalid or unparseable URLs

  This prevents credential leakage to external URLs when using the custom API call feature. Comprehensive tests cover edge cases including internal IPs, lookalike domains, and empty inputs.

### V5.1.3 - URL Encoding in API Calls

- **Status:** Met
- **Evidence:** `raps-oss/src/objects.rs:331`, `raps-oss/src/objects.rs:366`, `raps-oss/src/objects.rs:412-413`
- **Details:** Object keys are URL-encoded using `urlencoding::encode()` when constructing API URLs for object details, signed downloads, and signed uploads. This prevents injection of path separators or query parameters through object key names.

### V5.1.4 - CSV Input Validation in Admin Module

- **Status:** Met
- **Evidence:** `raps-cli/src/commands/admin/csv_ops.rs:72-116` (update), `raps-cli/src/commands/admin/csv_ops.rs:407-442` (import)
- **Details:** The CSV handling implements thorough validation:
  - **Parsing:** Uses the `csv` crate with `serde::Deserialize` for type-safe deserialization (lines 72-73, 407-408)
  - **Email validation:** Checks that email is non-empty and contains `@` (lines 83, 418)
  - **Required fields:** Validates that at least one updatable field is present (lines 88-95)
  - **Error aggregation:** Collects all validation errors with row numbers before aborting (lines 76-77, 105-116)
  - **Parse error handling:** Malformed CSV rows are caught and reported with row numbers (lines 98-101, 424-427)
  - **Empty file check:** Rejects empty CSV files (lines 118-120, 444-446)

  The email validation is basic (only checks for `@`). It does not validate email format per RFC 5322, but this is acceptable since the email is passed to the Autodesk API which performs its own validation.

### V5.1.5 - Pipeline Execution Safety

- **Status:** Partial
- **Evidence:** `raps-cli/src/commands/pipeline.rs:260-282`
- **Details:** Pipeline execution has both strengths and concerns:

  **Strengths:**
  - Commands are executed by invoking the `raps` binary itself (`std::env::current_exe()`) rather than a shell, preventing shell injection (line 268)
  - Command arguments are split by whitespace rather than interpreted by a shell (line 265)
  - Variable substitution is limited to simple `${}` or `$` patterns (lines 179-181)

  **Concerns:**
  - Pipeline files (YAML/JSON) are loaded from user-specified paths without any sandboxing or schema validation beyond deserialization (lines 92-122)
  - The variable substitution at lines 179-181 is unsanitized -- a malicious variable value could inject additional arguments since `command.replace()` operates on the full command string before `split_whitespace()`. For example, a variable value of `foo --yes --force` would be split into multiple arguments.
  - The `condition` field (line 75) has a trivial evaluator (line 284-288) that treats any non-empty, non-"false", non-"0" string as truthy. This is safe because conditions do not execute arbitrary code, but it limits utility.
  - Pipeline files can be read from stdin (`-`), which could be used to pipe untrusted content.

### V5.1.6 - Upload Input Validation

- **Status:** Met
- **Evidence:** `raps-cli/src/commands/object/upload.rs:39-60`
- **Details:** Upload operations validate:
  - File existence before attempting upload (line 56-58)
  - Stdin handling requires explicit `--key` to prevent unnamed uploads (line 47)
  - Resume and stdin are mutually exclusive (line 44)
  - Stdin is spooled to a temporary file before upload, preventing memory exhaustion (lines 49-54)
  - Batch uploads validate all files exist before starting (lines 257-261)

### V12.1.1 - Download File Size Handling

- **Status:** Met
- **Evidence:** `raps-oss/src/objects.rs:174-184`
- **Details:** Downloads use streaming (`bytes_stream()`) rather than buffering the entire response in memory. Chunks are written to the file as they arrive, preventing memory exhaustion for large files. Progress tracking is provided for user visibility.

### V12.1.2 - Object Listing Pagination Safety

- **Status:** Met
- **Evidence:** `raps-oss/src/objects.rs:242-243`
- **Details:** The `list_objects()` method implements pagination with a hard limit of 100 pages (`MAX_PAGES`), preventing infinite loops from malformed API responses. A warning is logged when the limit is reached.

### V5.2.1 - Filter Expression Parsing Safety

- **Status:** Met
- **Evidence:** `raps-admin/src/filter.rs:77-163`
- **Details:** The `ProjectFilter::from_expression()` parser uses strict key-value parsing:
  - Only recognized keys are accepted (`name`, `status`, `platform`, `created`, `region`)
  - Enum values are validated against known variants
  - Date parsing uses `chrono::NaiveDate::parse_from_str` with explicit format `%Y-%m-%d`
  - Unknown keys return an error
  - Invalid syntax (missing colon separator) returns an error
  - Name patterns use the `glob` crate for matching, which does not execute code

## Summary

| Requirement | Status | Evidence |
|---|---|---|
| Path traversal protection (download) | Gap | `download.rs:66`, `objects.rs:170` |
| URL validation / SSRF prevention | Met | `http.rs:15-45` |
| URL encoding in API calls | Met | `objects.rs:331,366,412` |
| CSV input validation | Met | `csv_ops.rs:72-116,407-442` |
| Pipeline execution safety | Partial | `pipeline.rs:260-282` |
| Upload input validation | Met | `upload.rs:39-60` |
| Download streaming (no memory exhaustion) | Met | `objects.rs:174-184` |
| Pagination safety | Met | `objects.rs:242-243` |
| Filter expression parsing safety | Met | `filter.rs:77-163` |

## Recommendations

1. **Path traversal protection (Critical):** Sanitize the output path derived from object keys in `download.rs`. At minimum:
   - Strip or replace `..` path components
   - Resolve the path and verify it remains within the current working directory
   - Consider using `Path::file_name()` to extract only the filename portion, discarding any directory components
   ```rust
   let safe_name = Path::new(&object_key)
       .file_name()
       .unwrap_or(OsStr::new("download"))
       .to_string_lossy()
       .to_string();
   let output_path = output.unwrap_or_else(|| PathBuf::from(&safe_name));
   ```

2. **Pipeline variable injection:** Sanitize variable values during substitution in `pipeline.rs`, or perform substitution after argument splitting to prevent variable values from injecting additional arguments.

3. **CSV size limits:** Consider adding a configurable maximum row count for CSV imports to prevent accidental processing of extremely large files that could overwhelm the API with requests.

4. **Pipeline file validation:** Add schema validation for pipeline files beyond basic deserialization. Consider validating that step commands only contain known raps subcommands.
