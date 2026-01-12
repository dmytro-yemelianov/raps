# Code Coverage 80%+ Implementation Plan

## Current State

| Metric | Current | Target |
|--------|---------|--------|
| Line Coverage | 21.56% | 80%+ |
| Function Coverage | 25.92% | 80%+ |
| Region Coverage | 16.36% | 80%+ |
| Total Tests | 218 | ~600+ |

## Coverage by Module

### Already Meeting Target (>75%)
| File | Coverage | Status |
|------|----------|--------|
| interactive.rs | 98.71% | ✅ Done |
| progress.rs | 92.00% | ✅ Done |
| http.rs | 81.85% | ✅ Done |
| error.rs | 76.89% | ✅ Done |

### Near Target (50-75%)
| File | Coverage | Gap |
|------|----------|-----|
| output.rs | 62.15% | +18% |
| shell.rs | 59.00% | +21% |
| config.rs | 55.34% | +25% |
| derivative/lib.rs | 50.36% | +30% |

### Major Work Required (<50%)
| File | Lines | Coverage | Priority |
|------|-------|----------|----------|
| oss/lib.rs | 1577 | 18.39% | 🔴 Critical |
| auth.rs | 886 | 18.28% | 🔴 Critical |
| dm/lib.rs | 630 | 23.49% | 🔴 High |
| da/lib.rs | 600 | 36.67% | 🟡 Medium |
| reality/lib.rs | 446 | 30.94% | 🟡 Medium |
| webhooks/lib.rs | 297 | 38.05% | 🟡 Medium |
| storage.rs | 349 | 30.66% | 🟡 Medium |
| logging.rs | 86 | 17.44% | 🟢 Low |
| prompts.rs | 155 | 0.00% | 🟡 Medium |

### Zero Coverage (Commands)
| File | Lines | Priority |
|------|-------|----------|
| commands/acc.rs | 1060 | 🟡 Medium |
| commands/demo.rs | 1013 | 🟢 Low |
| commands/translate.rs | 995 | 🟡 Medium |
| commands/issue.rs | 782 | 🟢 Low |
| commands/object.rs | 778 | 🔴 High |
| commands/config.rs | 772 | 🟡 Medium |
| commands/da.rs | 740 | 🟡 Medium |
| mcp/server.rs | 737 | 🟡 Medium |
| commands/auth.rs | 623 | 🔴 High |
| commands/bucket.rs | 466 | 🔴 High |
| commands/webhook.rs | 504 | 🟢 Low |
| commands/rfi.rs | 499 | 🟢 Low |
| commands/reality.rs | 429 | 🟢 Low |

---

## Implementation Strategy

### Phase 1: Test Infrastructure (Week 1)

#### 1.1 Create Mock HTTP Server Framework

Create `raps-kernel/src/test_utils.rs`:

```rust
//! Test utilities for mocking API responses

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock response configuration
pub struct MockResponse {
    pub status: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
}

/// Mock HTTP server for testing
pub struct MockServer {
    responses: Arc<Mutex<Vec<MockResponse>>>,
    requests: Arc<Mutex<Vec<String>>>,
    port: u16,
}

impl MockServer {
    pub fn new() -> Self { /* ... */ }
    pub fn url(&self) -> String { /* ... */ }
    pub fn expect(&mut self, response: MockResponse) { /* ... */ }
    pub fn verify_request(&self, index: usize) -> Option<String> { /* ... */ }
}

/// Builder for creating test clients with mock server
pub struct TestClientBuilder<T> {
    mock_server: MockServer,
    _phantom: std::marker::PhantomData<T>,
}
```

#### 1.2 Add Test Dependencies

```toml
# Cargo.toml [workspace.dev-dependencies]
wiremock = "0.6"           # HTTP mocking
tokio-test = "0.4"         # Async test utilities
tempfile = "3.2"           # Temp directories
assert_fs = "1.1"          # File system assertions
predicates = "3.0"         # Assertion predicates
fake = "2.9"               # Fake data generation
proptest = "1.4"           # Property-based testing
```

#### 1.3 Create Test Fixtures

Create `tests/fixtures/` directory with sample API responses:
- `buckets_list.json`
- `objects_list.json`
- `manifest_success.json`
- `manifest_pending.json`
- `token_response.json`
- `error_401.json`
- `error_429.json`

---

### Phase 2: Core API Client Testing (Week 2-3)

#### 2.1 OSS Client (raps-oss) - Target: 80%

**Current: 18.39% → Target: 80% (+61.61%)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path, header};

    async fn setup_mock_server() -> (MockServer, OssClient) {
        let mock = MockServer::start().await;
        let config = Config {
            base_url: mock.uri(),
            // ...
        };
        (mock, OssClient::new(config, auth))
    }

    // Bucket Operations
    #[tokio::test]
    async fn test_create_bucket_success() { /* ... */ }

    #[tokio::test]
    async fn test_create_bucket_already_exists() { /* ... */ }

    #[tokio::test]
    async fn test_create_bucket_invalid_name() { /* ... */ }

    #[tokio::test]
    async fn test_list_buckets_empty() { /* ... */ }

    #[tokio::test]
    async fn test_list_buckets_paginated() { /* ... */ }

    #[tokio::test]
    async fn test_delete_bucket_success() { /* ... */ }

    #[tokio::test]
    async fn test_delete_bucket_not_empty() { /* ... */ }

    // Object Operations
    #[tokio::test]
    async fn test_upload_object_small() { /* ... */ }

    #[tokio::test]
    async fn test_upload_object_multipart() { /* ... */ }

    #[tokio::test]
    async fn test_upload_object_resume() { /* ... */ }

    #[tokio::test]
    async fn test_download_object_success() { /* ... */ }

    #[tokio::test]
    async fn test_download_object_not_found() { /* ... */ }

    #[tokio::test]
    async fn test_copy_object_success() { /* ... */ }

    #[tokio::test]
    async fn test_delete_object_success() { /* ... */ }

    // Error Handling
    #[tokio::test]
    async fn test_handle_401_unauthorized() { /* ... */ }

    #[tokio::test]
    async fn test_handle_429_rate_limit() { /* ... */ }

    #[tokio::test]
    async fn test_handle_5xx_server_error() { /* ... */ }

    // URN Generation
    #[test]
    fn test_get_urn_encoding() { /* ... */ }

    #[test]
    fn test_get_urn_special_characters() { /* ... */ }
}
```

**Estimated tests: 25-30**

#### 2.2 Auth Client (raps-kernel/auth.rs) - Target: 80%

**Current: 18.28% → Target: 80% (+61.72%)**

```rust
#[cfg(test)]
mod tests {
    // 2-Legged Auth
    #[tokio::test]
    async fn test_get_2legged_token_success() { /* ... */ }

    #[tokio::test]
    async fn test_get_2legged_token_invalid_credentials() { /* ... */ }

    #[tokio::test]
    async fn test_2legged_token_caching() { /* ... */ }

    #[tokio::test]
    async fn test_2legged_token_refresh() { /* ... */ }

    // 3-Legged Auth
    #[tokio::test]
    async fn test_generate_auth_url() { /* ... */ }

    #[tokio::test]
    async fn test_exchange_code_for_token() { /* ... */ }

    #[tokio::test]
    async fn test_refresh_3legged_token() { /* ... */ }

    // Device Code Flow
    #[tokio::test]
    async fn test_initiate_device_code() { /* ... */ }

    #[tokio::test]
    async fn test_poll_device_code_pending() { /* ... */ }

    #[tokio::test]
    async fn test_poll_device_code_success() { /* ... */ }

    // Token Validation
    #[test]
    fn test_token_is_expired() { /* ... */ }

    #[test]
    fn test_token_needs_refresh() { /* ... */ }

    #[test]
    fn test_parse_jwt_claims() { /* ... */ }

    // Scope Handling
    #[test]
    fn test_scope_parsing() { /* ... */ }

    #[test]
    fn test_scope_validation() { /* ... */ }
}
```

**Estimated tests: 20-25**

#### 2.3 Data Management Client (raps-dm) - Target: 80%

**Current: 23.49% → Target: 80% (+56.51%)**

```rust
#[cfg(test)]
mod tests {
    // Hub Operations
    #[tokio::test]
    async fn test_list_hubs_success() { /* ... */ }

    #[tokio::test]
    async fn test_list_hubs_empty() { /* ... */ }

    // Project Operations
    #[tokio::test]
    async fn test_list_projects_success() { /* ... */ }

    #[tokio::test]
    async fn test_get_project_details() { /* ... */ }

    // Folder Operations
    #[tokio::test]
    async fn test_get_top_folders() { /* ... */ }

    #[tokio::test]
    async fn test_list_folder_contents() { /* ... */ }

    #[tokio::test]
    async fn test_create_folder_success() { /* ... */ }

    #[tokio::test]
    async fn test_create_folder_duplicate() { /* ... */ }

    // Item/Version Operations
    #[tokio::test]
    async fn test_get_item_details() { /* ... */ }

    #[tokio::test]
    async fn test_list_item_versions() { /* ... */ }

    #[tokio::test]
    async fn test_get_version_details() { /* ... */ }
}
```

**Estimated tests: 15-20**

---

### Phase 3: Derivative & Other Clients (Week 3-4)

#### 3.1 Derivative Client - Target: 80%

**Current: 50.36% → Target: 80% (+29.64%)**

```rust
#[cfg(test)]
mod tests {
    // Translation
    #[tokio::test]
    async fn test_translate_svf2_success() { /* ... */ }

    #[tokio::test]
    async fn test_translate_with_root_filename() { /* ... */ }

    #[tokio::test]
    async fn test_translate_invalid_urn() { /* ... */ }

    // Manifest
    #[tokio::test]
    async fn test_get_manifest_success() { /* ... */ }

    #[tokio::test]
    async fn test_get_manifest_pending() { /* ... */ }

    #[tokio::test]
    async fn test_get_manifest_failed() { /* ... */ }

    // Download
    #[tokio::test]
    async fn test_download_derivative_success() { /* ... */ }

    #[tokio::test]
    async fn test_download_derivative_not_ready() { /* ... */ }

    // Metadata
    #[tokio::test]
    async fn test_get_metadata() { /* ... */ }

    #[tokio::test]
    async fn test_get_properties() { /* ... */ }
}
```

**Estimated tests: 15**

#### 3.2 Design Automation Client - Target: 80%

**Current: 36.67% → Target: 80% (+43.33%)**

```rust
#[cfg(test)]
mod tests {
    // AppBundle
    #[tokio::test]
    async fn test_create_appbundle() { /* ... */ }

    #[tokio::test]
    async fn test_list_appbundles() { /* ... */ }

    #[tokio::test]
    async fn test_upload_appbundle() { /* ... */ }

    // Activity
    #[tokio::test]
    async fn test_create_activity() { /* ... */ }

    #[tokio::test]
    async fn test_list_activities() { /* ... */ }

    // WorkItem
    #[tokio::test]
    async fn test_create_workitem() { /* ... */ }

    #[tokio::test]
    async fn test_get_workitem_status() { /* ... */ }

    #[tokio::test]
    async fn test_workitem_completed() { /* ... */ }

    #[tokio::test]
    async fn test_workitem_failed() { /* ... */ }

    // Engine
    #[tokio::test]
    async fn test_list_engines() { /* ... */ }
}
```

**Estimated tests: 15**

#### 3.3 Other Clients (Reality, Webhooks, ACC)

Similar pattern for each client:
- Reality: 10 tests
- Webhooks: 10 tests
- ACC: 15 tests

---

### Phase 4: Command Handler Testing (Week 4-5)

#### 4.1 Strategy: Functional Testing

Instead of unit testing each command, use integration-style tests:

```rust
// tests/commands/bucket_test.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_bucket_list_help() {
    Command::cargo_bin("raps")
        .unwrap()
        .args(["bucket", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List buckets"));
}

#[test]
fn test_bucket_list_no_auth() {
    Command::cargo_bin("raps")
        .unwrap()
        .args(["bucket", "list"])
        .env_remove("APS_CLIENT_ID")
        .assert()
        .failure()
        .stderr(predicate::str::contains("credentials"));
}

#[test]
fn test_bucket_create_requires_name() {
    Command::cargo_bin("raps")
        .unwrap()
        .args(["bucket", "create"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}
```

#### 4.2 Priority Commands (High Impact)

1. **bucket.rs** - 15 tests
2. **object.rs** - 15 tests
3. **auth.rs** - 10 tests
4. **translate.rs** - 10 tests
5. **config.rs** - 10 tests

#### 4.3 Medium Priority Commands

6. **da.rs** - 10 tests
7. **acc.rs** - 10 tests
8. **webhook.rs** - 5 tests
9. **mcp/server.rs** - 10 tests

---

### Phase 5: Utility Modules (Week 5)

#### 5.1 Storage Module - Target: 80%

**Current: 30.66% → Target: 80%**

```rust
#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    #[test]
    fn test_save_token() { /* ... */ }

    #[test]
    fn test_load_token() { /* ... */ }

    #[test]
    fn test_delete_token() { /* ... */ }

    #[test]
    fn test_token_expiry() { /* ... */ }

    #[test]
    fn test_keyring_fallback() { /* ... */ }
}
```

**Estimated tests: 10**

#### 5.2 Prompts Module - Target: 80%

**Current: 0% → Target: 80%**

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_confirm_yes_flag() {
        crate::interactive::init(false, true);
        assert!(confirm("Delete?").unwrap());
    }

    #[test]
    fn test_confirm_non_interactive() {
        crate::interactive::init(true, false);
        assert!(!confirm_destructive("Delete?").unwrap());
    }

    #[test]
    fn test_input_non_interactive_fails() {
        crate::interactive::init(true, false);
        assert!(input::<String>("Name").is_err());
    }
}
```

**Estimated tests: 8**

#### 5.3 Logging Module - Target: 80%

**Current: 17.44% → Target: 80%**

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_log_verbose_enabled() { /* ... */ }

    #[test]
    fn test_log_verbose_disabled() { /* ... */ }

    #[test]
    fn test_log_debug() { /* ... */ }

    #[test]
    fn test_init_logging() { /* ... */ }
}
```

**Estimated tests: 5**

---

### Phase 6: Shell & Plugins (Week 6)

#### 6.1 Shell Module - Target: 80%

**Current: 59% → Target: 80% (+21%)**

```rust
#[cfg(test)]
mod tests {
    // Completion
    #[test]
    fn test_complete_top_level_commands() { /* ... */ }

    #[test]
    fn test_complete_subcommands() { /* ... */ }

    #[test]
    fn test_complete_flags() { /* ... */ }

    #[test]
    fn test_complete_bucket_names() { /* ... */ }

    // Hints
    #[test]
    fn test_hint_formatting() { /* ... */ }

    #[test]
    fn test_hint_colors() { /* ... */ }

    // History
    #[test]
    fn test_history_save_load() { /* ... */ }
}
```

**Estimated tests: 10**

#### 6.2 Plugin System - Already Good

**Current: Has 14 tests, likely ~60-70%**

Add 5 more edge case tests.

---

## Test Count Summary

| Phase | Module | New Tests | Running Total |
|-------|--------|-----------|---------------|
| - | Current | 218 | 218 |
| 1 | Infrastructure | 0 | 218 |
| 2.1 | OSS Client | 30 | 248 |
| 2.2 | Auth Client | 25 | 273 |
| 2.3 | DM Client | 20 | 293 |
| 3.1 | Derivative | 15 | 308 |
| 3.2 | DA Client | 15 | 323 |
| 3.3 | Reality/Webhooks/ACC | 35 | 358 |
| 4 | Commands | 85 | 443 |
| 5 | Utilities | 23 | 466 |
| 6 | Shell/Plugins | 15 | 481 |
| - | Buffer/Edge Cases | ~50 | ~530 |

**Total: ~530 tests for 80%+ coverage**

---

## Implementation Order (Recommended)

### Sprint 1 (Week 1-2): Foundation
1. ✅ Set up test infrastructure (wiremock, fixtures)
2. ✅ OSS client mock tests (highest LOC, critical path)
3. ✅ Auth client mock tests (critical for all operations)

### Sprint 2 (Week 2-3): Core APIs
4. DM client tests
5. Derivative client tests
6. DA client tests

### Sprint 3 (Week 3-4): Secondary APIs
7. Reality client tests
8. Webhooks client tests
9. ACC client tests

### Sprint 4 (Week 4-5): Commands
10. Bucket/Object command tests
11. Auth/Config command tests
12. Other command tests

### Sprint 5 (Week 5-6): Polish
13. Storage/Prompts/Logging tests
14. Shell tests
15. Edge cases and property tests

---

## Coverage Milestones

| Week | Target Coverage | Tests |
|------|-----------------|-------|
| 1 | 30% | 270 |
| 2 | 40% | 320 |
| 3 | 50% | 380 |
| 4 | 60% | 430 |
| 5 | 70% | 480 |
| 6 | 80%+ | 530+ |

---

## CI/CD Integration

### Add Coverage Gate

```yaml
# .github/workflows/ci.yml
- name: Check coverage threshold
  run: |
    COVERAGE=$(cargo llvm-cov --workspace --summary-only | grep TOTAL | awk '{print $NF}' | tr -d '%')
    if (( $(echo "$COVERAGE < 80" | bc -l) )); then
      echo "Coverage $COVERAGE% is below 80% threshold"
      exit 1
    fi
```

### Coverage Badge

Add to README.md:
```markdown
[![codecov](https://codecov.io/gh/dmytro-yemelianov/raps/branch/main/graph/badge.svg)](https://codecov.io/gh/dmytro-yemelianov/raps)
```

---

## Success Criteria

1. **Line Coverage**: ≥80%
2. **Function Coverage**: ≥80%
3. **Branch Coverage**: ≥70%
4. **All Tests Pass**: 100%
5. **No Flaky Tests**: 0
6. **Test Runtime**: <5 minutes
