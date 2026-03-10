---
name: raps-security-audit
version: "1.0"
description: Use before releases or after dependency updates to run security checks — cargo-audit, ASVS compliance matrix review, clippy security lints, redaction pattern verification.
---

# RAPS Security Audit

Run security checks and update compliance documentation.

**Repo:** `/root/github/raps/raps`
**Docs:** `docs/security/`

## Audit Steps

### 1. Dependency Audit

```bash
cargo audit                    # Known vulnerabilities
cargo deny check               # License + advisory check
```

### 2. Static Analysis

```bash
cargo clippy --workspace --all-features -- -D warnings
```

Key clippy lints to watch for:
- `clippy::unwrap_used` — potential panics
- `clippy::expect_used` — same
- Security-relevant: command injection, path traversal patterns

### 3. Redaction Verification

```bash
# Verify log redaction patterns work
cargo test -p raps-kernel logging
cargo test -p raps-kernel security
```

Redaction covers: Bearer tokens, Basic auth, Cookie headers, X-API-Key, URL `?access_token=` / `?apikey=` params.

### 4. Path Traversal Protection

```bash
cargo test -p raps-kernel security
```

Tests: `../../etc/passwd` -> `passwd`, empty string -> Err, `..` alone -> Err, NUL bytes stripped, unicode preserved, validate_path_within rejects escape.

### 5. ASVS Compliance Matrix

**File:** `docs/security/asvs-l2-compliance-matrix.md`

Format:
```markdown
| ID | Requirement | Status | Evidence | Notes |
|----|-------------|--------|----------|-------|
| V2.1 | OAuth uses PKCE | Met | `auth/device_code.rs:37` | RFC 7636 compliant |
```

Statuses: `Met`, `Partial`, `Gap`, `N/A`

Update when:
- New security feature added -> change Gap/Partial to Met
- New code path added -> verify existing controls cover it
- Dependency updated -> re-verify affected controls

### 6. Full Security Docs

| File | Scope |
|------|-------|
| `asvs-l2-compliance-matrix.md` | ASVS 4.0.3 L2 controls |
| `asvs-v2-auth-audit.md` | Authentication (PKCE, CSRF, token storage) |
| `asvs-v5-v12-input-files-audit.md` | Input validation, file handling |
| `asvs-v6-v9-crypto-comms-audit.md` | Cryptography, communications |
| `asvs-v7-logging-audit.md` | Error handling, logging |
| `plugin-trust-model.md` | Plugin security |
| `gdpr-privacy-statement.md` | GDPR compliance |
| `nist-ssdf-mapping.md` | NIST SSDF mapping |
| `soc2-tsc-self-assessment.md` | SOC 2 Type II |

## Checklist

1. Run `cargo audit` and `cargo deny check`
2. Run `cargo clippy --workspace --all-features -- -D warnings`
3. Run security-related tests (`cargo test -p raps-kernel security logging`)
4. Review ASVS matrix for any newly applicable controls
5. Update compliance statuses and evidence references
6. Commit updates to `docs/security/`
