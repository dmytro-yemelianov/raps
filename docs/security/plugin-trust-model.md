# ASVS V10 Audit: Plugin Trust Model

**Audit Date:** 2026-02-28
**RAPS Version:** 4.14.0
**ASVS Version:** 4.0.3

## Scope

Audit of the RAPS plugin and extension system covering:
- Plugin discovery mechanism (`raps-cli/src/plugins.rs`)
- Plugin execution model
- Credential access and environment inheritance
- Hook system and command validation
- Current trust model and future plans

## Findings

### V10.1.1 - Plugin Discovery Mechanism

- **Status:** Partial
- **Evidence:** `raps-cli/src/plugins.rs:139-179`
- **Details:** Plugins are discovered by scanning the system `PATH` (plus the directory containing the `raps` executable) for executables matching the naming convention:
  - Unix: `raps-<name>`
  - Windows: `raps-<name>.exe`

  The discovery process:
  1. Splits the `PATH` environment variable by platform separator (`:` or `;`)
  2. Also checks the parent directory of the current executable
  3. Iterates all entries in each directory
  4. Checks filename against the `raps-*` pattern
  5. Deduplicates by plugin name (first match wins)

  **Security concern:** Any executable on the PATH matching `raps-<name>` is treated as a plugin. An attacker with write access to any directory in the user's PATH can inject malicious plugins. There is no integrity verification (hash, signature) of discovered plugins.

### V10.1.2 - Plugin Configuration

- **Status:** Met
- **Evidence:** `raps-cli/src/plugins.rs:88-116`
- **Details:** Plugin configuration is stored in `~/.config/raps/plugins.json` with the following structure:
  - **plugins:** Map of plugin name to `PluginEntry` (enabled, path, description)
  - **hooks:** Map of hook name to list of commands
  - **aliases:** Map of alias name to raps command string

  Plugins can be individually enabled/disabled via the `enabled` field. New discovered plugins default to enabled (`default_true()` at line 73).

### V10.1.3 - Plugin Execution Model

- **Status:** Gap (by design -- documented here)
- **Evidence:** `raps-cli/src/plugins.rs:233-239`
- **Details:** Plugins are executed using `std::process::Command::new(path).args(args).status()`, which:
  - Spawns a new OS process
  - Inherits the parent process's environment variables (including `APS_CLIENT_ID`, `APS_CLIENT_SECRET`, and any active tokens in environment)
  - Inherits stdin/stdout/stderr
  - Runs with the same OS user privileges as the parent process
  - Has no sandboxing, resource limits, or capability restrictions

  This is the standard model for CLI plugin systems (similar to `git-<name>`, `kubectl-<name>`, `gh-<name>`). The user is implicitly trusting any plugin they install.

### V10.1.4 - Credential Access

- **Status:** Gap (by design -- documented here)
- **Evidence:** `raps-cli/src/plugins.rs:233-239`
- **Details:** Plugins inherit the full environment of the parent process. This means:
  - `APS_CLIENT_ID` and `APS_CLIENT_SECRET` are accessible to plugins
  - Any environment-based secrets are accessible
  - If tokens are in environment variables, those are also accessible
  - Keychain access is theoretically available (same user context)
  - File-based token storage is readable (same user permissions)

  There is no mechanism to:
  - Restrict which environment variables a plugin can see
  - Provide scoped/limited credentials to plugins
  - Revoke plugin access to credentials

### V10.1.5 - Hook System Security

- **Status:** Met
- **Evidence:** `raps-cli/src/plugins.rs:255-363`
- **Details:** The hook system includes several security measures:

  **Command parsing (lines 284-324):**
  - Commands are parsed manually without shell interpretation
  - Supports quoted arguments with escape characters
  - Does not pass commands through `sh -c` or `cmd /c`
  - Prevents shell metacharacter injection

  **Command validation (lines 327-363):**
  - A hardcoded allowlist (`ALLOWED_COMMANDS`) restricts hook commands to:
    `echo`, `notify-send`, `curl`, `wget`, `git`, `npm`, `cargo`, `python`, `node`, `raps`
  - `raps-*` prefixed commands are also allowed
  - Absolute paths are allowed with a warning
  - Unlisted commands are rejected with an error

  **Concerns:**
  - The allowlist includes `curl` and `wget`, which can exfiltrate credentials
  - `python` and `node` provide arbitrary code execution
  - Absolute paths bypass the allowlist entirely
  - The allowlist is hardcoded and not user-configurable

### V10.1.6 - No Code Signing or Integrity Verification

- **Status:** Gap
- **Evidence:** Entire `raps-cli/src/plugins.rs` file
- **Details:** There is no mechanism to verify the integrity or authenticity of plugins:
  - No cryptographic signatures
  - No hash verification
  - No publisher/author validation
  - No trust-on-first-use (TOFU) mechanism
  - No audit trail of when plugins were discovered or modified

  A plugin executable could be replaced by malware without detection.

## Current Trust Model

The RAPS plugin system operates on an **implicit trust model**, where:

1. **Installation = Trust:** Any executable placed on the PATH matching `raps-<name>` is automatically discovered and available for execution.
2. **Execution = Full Access:** Running a plugin grants it full access to the user's credentials, environment, and system resources.
3. **Hook System = Partially Restricted:** Hooks are more restricted than plugins (allowlisted commands, no shell interpretation), but still allow exfiltration via `curl`/`wget` and arbitrary code execution via `python`/`node`.
4. **Configuration = Optional:** Plugins can be disabled in `plugins.json`, but this requires manual action after discovery.

This model is consistent with other CLI plugin ecosystems (git, kubectl, gh) and is appropriate for RAPS's use case as a developer tool where the user controls their own machine.

## Plugin Execution Flow

```
User runs: raps <plugin-name> [args]
    |
    v
PluginManager::execute_plugin()
    |
    +-- Check plugins.json for configured path
    |   (if found and enabled, use configured path)
    |
    +-- Otherwise: discover_plugins()
    |   (scan PATH for raps-<name> executables)
    |
    +-- run_plugin(path, args)
        |
        v
    std::process::Command::new(path)
        .args(args)
        .status()
        |
        v
    Plugin runs with full environment
    (inherits env vars, cwd, permissions)
```

## Future Plans

Based on the codebase audit and project documentation, the following security enhancements could be considered:

- **ed25519-dalek signature verification:** Digital signatures for plugin executables, allowing users to verify that plugins were published by trusted authors.
- **Plugin registry:** A curated list of known/verified plugins.
- **Scoped credentials:** Pass limited-scope tokens to plugins rather than full credentials.
- **Sandboxing:** OS-level sandboxing for plugin execution (e.g., seccomp, AppArmor, or macOS sandbox profiles).

## Summary

| Requirement | Status | Evidence |
|---|---|---|
| Plugin discovery mechanism | Partial | `plugins.rs:139-179` |
| Plugin configuration | Met | `plugins.rs:88-116` |
| Plugin execution sandboxing | Gap (by design) | `plugins.rs:233-239` |
| Credential access control | Gap (by design) | `plugins.rs:233-239` |
| Hook command validation | Met | `plugins.rs:284-363` |
| Code signing / integrity | Gap | `plugins.rs` (entire file) |

## Recommendations

### For Users

1. **Audit your PATH:** Ensure that only trusted directories are in your PATH. Remove world-writable directories.
2. **Review installed plugins:** Periodically run plugin discovery and verify that all found plugins are expected.
3. **Disable unused plugins:** Set `enabled: false` in `plugins.json` for plugins you do not actively use.
4. **Use explicit paths:** When possible, configure plugin paths explicitly in `plugins.json` rather than relying on PATH discovery.
5. **Restrict hook commands:** Review the hooks configured in `plugins.json` and limit them to necessary operations.

### For Development

1. **Implement plugin signing (High Priority):** Add ed25519 signature verification for plugin executables. Each plugin should ship with a detached signature, and RAPS should verify it against a set of trusted public keys before execution.
2. **Add trust-on-first-use (TOFU):** When a new plugin is discovered, record its hash. Warn if the hash changes on subsequent invocations.
3. **Credential scoping:** Instead of inheriting the full environment, create a subprocess environment with only the minimum required variables. Use short-lived, scoped tokens where possible.
4. **Default to disabled:** Consider defaulting newly discovered plugins to `enabled: false` and requiring explicit user opt-in.
5. **Remove overly permissive hook commands:** Consider removing `python`, `node`, `curl`, and `wget` from the default hook allowlist, or requiring explicit user approval for these commands.
6. **Log plugin execution:** Add audit logging for plugin and hook execution, including the plugin path, arguments, and exit code.
