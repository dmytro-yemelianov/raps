---
layout: default
title: Plugins
---

# Plugin System

RAPS CLI supports a plugin system for extending functionality with external commands, workflow hooks, and command aliases.

## Overview

The plugin system provides three extension mechanisms:

1. **External Command Plugins**: Executables that extend RAPS with new commands
2. **Workflow Hooks**: Pre/post command hooks for automation
3. **Command Aliases**: Shortcuts for frequently used command patterns

## External Command Plugins

### Plugin Discovery

RAPS automatically discovers plugins by searching your system PATH for executables matching the pattern:

- **Windows**: `raps-<name>.exe`
- **macOS/Linux**: `raps-<name>`

For example, if you have an executable named `raps-report` in your PATH, you can invoke it as:

```bash
raps report [arguments]
```

### Creating a Plugin

A plugin can be written in any language. It receives command-line arguments directly.

**Example: Simple Bash Plugin**

```bash
#!/bin/bash
# Save as: raps-hello (make executable with chmod +x)

echo "Hello from RAPS plugin!"
echo "Arguments: $@"
```

**Example: Python Plugin**

```python
#!/usr/bin/env python3
# Save as: raps-stats

import sys
import json

def main():
    print("RAPS Statistics Plugin")
    # Your plugin logic here
    
if __name__ == "__main__":
    main()
```

### Plugin Best Practices

1. **Exit Codes**: Use standard exit codes (0 for success, non-zero for errors)
2. **Output Format**: Support `--output json` for machine-readable output
3. **Help Text**: Implement `--help` for usage information
4. **Error Handling**: Print errors to stderr, not stdout

## Configuration File

Plugins and hooks are configured in `~/.config/raps/plugins.json` (Linux/macOS) or `%APPDATA%\raps\plugins.json` (Windows).

### Configuration Structure

```json
{
  "plugins": {
    "my-plugin": {
      "enabled": true,
      "path": "/path/to/raps-my-plugin",
      "description": "My custom plugin"
    }
  },
  "hooks": {
    "pre_upload": ["echo 'Starting upload...'"],
    "post_translate": ["notify-send 'Translation complete'"]
  },
  "aliases": {
    "quick-upload": "object upload --resume",
    "dev-bucket": "bucket list --output json"
  }
}
```

### Plugin Entry Fields

| Field | Type | Description |
|-------|------|-------------|
| `enabled` | boolean | Whether the plugin is active (default: true) |
| `path` | string | Optional explicit path to the plugin executable |
| `description` | string | Optional description for `raps plugin list` |

## Workflow Hooks

Hooks allow you to run commands before or after RAPS operations.

### Hook Names

Hooks follow the pattern `pre_<command>` or `post_<command>`:

| Hook | Triggered |
|------|-----------|
| `pre_upload` | Before any upload operation |
| `post_upload` | After successful upload |
| `pre_translate` | Before starting translation |
| `post_translate` | After translation completes |
| `pre_login` | Before authentication |
| `post_login` | After successful login |

### Hook Examples

**Notification on Translation Complete**

```json
{
  "hooks": {
    "post_translate": [
      "notify-send 'RAPS' 'Translation finished!'",
      "echo 'Translation completed at $(date)' >> ~/raps.log"
    ]
  }
}
```

**Validation Before Upload**

```json
{
  "hooks": {
    "pre_upload": [
      "echo 'Validating file...'",
      "/path/to/validate-script.sh"
    ]
  }
}
```

### Hook Behavior

- Hooks run in order defined in the array
- Hook failures are logged but don't stop the main command
- Hooks receive no arguments (use environment variables for context)
- On Windows, hooks run via `cmd /C`; on Unix, via `sh -c`

## Command Aliases

Aliases provide shortcuts for common command patterns.

### Defining Aliases

```json
{
  "aliases": {
    "up": "object upload --resume",
    "ls": "bucket list --output table",
    "translate-svf": "translate start --format svf2 --wait"
  }
}
```

### Using Aliases

```bash
# Instead of:
raps object upload --resume mybucket file.dwg

# Use:
raps up mybucket file.dwg
```

### Alias with Variables

Aliases support argument passing:

```bash
# Alias: "quick-translate": "translate start --format svf2"
raps quick-translate urn:adsk:... --wait
# Expands to: raps translate start --format svf2 urn:adsk:... --wait
```

## Plugin Management Commands

### List Plugins

```bash
# List all discovered and configured plugins
raps plugin list
```

Output:
```
Discovered Plugins:
  raps-report    /usr/local/bin/raps-report    ✓ enabled
  raps-stats     /usr/local/bin/raps-stats     ✓ enabled
  raps-export    /home/user/bin/raps-export    ✗ disabled
```

### Enable/Disable Plugins

```bash
# Disable a plugin
raps plugin disable export

# Enable a plugin
raps plugin enable export
```

### List Aliases

```bash
# Show all configured aliases
raps alias list
```

## Security Considerations

### Plugin Security

1. **Trust**: Only install plugins from trusted sources
2. **Permissions**: Plugins run with your user permissions
3. **PATH Security**: Ensure your PATH doesn't include untrusted directories
4. **Review Code**: Inspect plugin source before installation

### Hook Security

1. **Command Injection**: Be careful with dynamic content in hooks
2. **Sensitive Data**: Don't log credentials in hook scripts
3. **Error Handling**: Hook failures are logged; don't expose secrets in error messages

### Best Practices

```json
{
  "hooks": {
    "pre_upload": [
      "/path/to/vetted-script.sh"
    ]
  }
}
```

Prefer calling dedicated scripts over inline shell commands for better security auditing.

## Troubleshooting

### Plugin Not Found

1. Verify the plugin is in your PATH: `which raps-pluginname`
2. Check file permissions (must be executable)
3. On Windows, ensure `.exe` extension

### Hooks Not Running

1. Check hook names match the pattern `pre_<command>` or `post_<command>`
2. Verify `plugins.json` is valid JSON
3. Run with `--debug` to see hook execution logs

### Debug Mode

```bash
# See detailed plugin/hook execution
raps --debug plugin list
raps --debug object upload mybucket file.dwg
```

## Examples

### Complete Configuration

```json
{
  "plugins": {
    "report": {
      "enabled": true,
      "description": "Generate project reports"
    },
    "backup": {
      "enabled": true,
      "path": "/opt/raps-plugins/raps-backup",
      "description": "Backup bucket contents"
    }
  },
  "hooks": {
    "pre_upload": [
      "echo 'Upload starting...'"
    ],
    "post_upload": [
      "echo 'Upload complete!'",
      "/usr/local/bin/notify-team.sh"
    ],
    "post_translate": [
      "curl -X POST https://webhook.site/xxx -d 'Translation done'"
    ]
  },
  "aliases": {
    "up": "object upload --resume",
    "down": "object download",
    "trans": "translate start --format svf2",
    "status": "translate status --wait"
  }
}
```

### CI/CD Integration

```json
{
  "hooks": {
    "post_translate": [
      "echo '::set-output name=status::complete'"
    ]
  },
  "aliases": {
    "ci-upload": "object upload --non-interactive --output json",
    "ci-translate": "translate start --non-interactive --output json --wait"
  }
}
```

