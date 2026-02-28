# ASVS V6/V9 Audit: Cryptography & Communications

**Audit Date:** 2026-02-28
**RAPS Version:** 4.14.0
**ASVS Version:** 4.0.3

## Scope

Audit of the RAPS cryptographic and communications layer covering:
- TLS configuration (`Cargo.toml` workspace dependency, `raps-kernel/src/http.rs`)
- Domain allowlisting (`raps-kernel/src/http.rs`)
- Cryptographic algorithm usage across the codebase

## Findings

### V9.1.1 - TLS Enforcement via rustls

- **Status:** Met
- **Evidence:** `Cargo.toml:49`
- **Details:** The workspace-level `reqwest` dependency is configured as:
  ```toml
  reqwest = { version = "0.11", features = ["json", "stream", "multipart", "rustls-tls"], default-features = false }
  ```
  The `default-features = false` combined with `rustls-tls` means the `native-tls` backend is explicitly excluded. All HTTP requests are handled through `rustls`, a memory-safe TLS implementation written in Rust. This eliminates an entire class of vulnerabilities associated with OpenSSL/native TLS libraries (buffer overflows, memory corruption). The `Cargo.lock` confirms `rustls`, `hyper-rustls`, `tokio-rustls`, and `rustls-webpki` are present as resolved dependencies. No `native-tls` or `openssl-sys` crates appear in the dependency tree.

### V9.1.2 - No Plaintext HTTP for API Communication

- **Status:** Met
- **Evidence:** `raps-kernel/src/http.rs:15-22` (allowed domains), Codebase-wide search
- **Details:** All API base URLs in the configuration default to `https://` schemes. The domain allowlist at `raps-kernel/src/http.rs:15-22` contains only Autodesk domains. The only plaintext HTTP usage is the local OAuth callback server (`http://localhost:{port}/callback` at `raps-kernel/src/auth/three_leg.rs:129`), which is bound exclusively to `127.0.0.1` (loopback interface) and is only active during the brief OAuth login flow. This is acceptable per ASVS since loopback traffic does not traverse the network.

### V9.1.3 - TLS Certificate Validation

- **Status:** Met
- **Evidence:** `raps-kernel/src/http.rs:76-82`
- **Details:** The `HttpClientConfig::create_client()` method uses `reqwest::Client::builder()` with default settings, which means TLS certificate validation is enabled (rustls validates certificates against the `webpki-roots` bundle). There are no calls to `danger_accept_invalid_certs(true)` or `danger_accept_invalid_hostnames(true)` anywhere in the codebase.

### V6.2.1 - Domain Allowlisting Mechanism

- **Status:** Met
- **Evidence:** `raps-kernel/src/http.rs:15-45`
- **Details:** The `ALLOWED_DOMAINS` constant defines a strict allowlist of Autodesk API domains:
  - `developer.api.autodesk.com`
  - `api.userprofile.autodesk.com`
  - `acc.autodesk.com`
  - `developer.autodesk.com`
  - `b360dm.autodesk.com`
  - `cdn.derivative.autodesk.io`

  The `is_allowed_url()` function parses the URL, extracts the host, and checks for exact domain match or valid subdomain match (ensuring the subdomain boundary is a dot character). This prevents attacks such as `developer.api.autodesk.com.evil.com`. The function correctly rejects:
  - External domains
  - Localhost URLs
  - Internal IP addresses
  - Lookalike domains (e.g., `developer.api.autodesk.com.evil.com`)

  Test coverage at lines 315-377 validates all of these cases.

### V6.2.2 - Subdomain Matching Safety

- **Status:** Met
- **Evidence:** `raps-kernel/src/http.rs:33-37`
- **Details:** The subdomain matching logic checks that the character immediately before the domain match is a dot (`b'.'`), preventing suffix-based domain spoofing. This is validated by the test at line 356-361 which confirms `developer.api.autodesk.com.evil.com` is rejected.

### V6.1.1 - Cryptographic Algorithm Strength

- **Status:** Met
- **Evidence:** See crypto inventory table below
- **Details:** All cryptographic algorithms in use are current, industry-standard, and of appropriate strength. No deprecated algorithms (MD5, SHA-1 for security, DES, RC4) are used for security purposes. SHA-1 appears only as a read-only hash from the Autodesk API for file integrity metadata (not security-critical).

## Crypto Inventory

| Algorithm | Purpose | Location | Library |
|---|---|---|---|
| SHA-256 | PKCE S256 code challenge (RFC 7636) | `raps-kernel/src/auth/device_code.rs:38` | `sha2` crate |
| Base64-URL (no pad) | PKCE challenge encoding | `raps-kernel/src/auth/device_code.rs:39` | `base64` crate |
| TLS 1.2/1.3 | All HTTPS communication | Workspace `Cargo.toml` | `rustls` via `reqwest` |
| CSPRNG | PKCE verifier generation | `raps-kernel/src/auth/device_code.rs:27` | `rand::thread_rng()` / `getrandom` |
| UUID v4 | CSRF state parameter generation | `raps-kernel/src/auth/device_code.rs:57`, `three_leg.rs:80` | `uuid` crate (uses `getrandom`) |
| HTTP Basic Auth | Client credential transmission | `device_code.rs:185`, `three_leg.rs:261`, `two_leg.rs:58` | `reqwest` (Base64 encoding) |
| OS Keychain | Token-at-rest encryption | `raps-kernel/src/storage.rs:225-241` | `keyring` crate (DPAPI/Keychain/SecretService) |
| SHA-1 | File integrity hash (read-only, from API) | `raps-oss/src/objects.rs` (API response) | Server-side (not computed locally) |

## Summary

| Requirement | Status | Evidence |
|---|---|---|
| TLS via rustls (no native-tls) | Met | `Cargo.toml:49` |
| No plaintext HTTP for APIs | Met | `http.rs:15-22`, codebase search |
| TLS certificate validation enabled | Met | `http.rs:76-82` |
| Domain allowlisting | Met | `http.rs:15-45` |
| Subdomain matching safety | Met | `http.rs:33-37` |
| Strong crypto algorithms only | Met | Crypto inventory table |

## Recommendations

1. **TLS version pinning:** Consider explicitly configuring `rustls` to require TLS 1.2 or higher (which is the default, but making it explicit in code documents the security intent). This can be done via `reqwest::Client::builder().min_tls_version(reqwest::tls::Version::TLS_1_2)`.

2. **Certificate pinning for APS endpoints:** For defense-in-depth against compromised CA scenarios, consider implementing certificate pinning or public key pinning for the core Autodesk API domains. This is a defense-in-depth measure and not strictly required by ASVS L2.

3. **Domain allowlist extensibility:** The current domain allowlist is hardcoded. If users need to work with custom Autodesk environments or proxies, consider making the allowlist configurable while keeping the default locked down. Document the security implications of extending it.
