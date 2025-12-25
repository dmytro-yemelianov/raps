---
layout: default
title: Plugin Management
---

# Plugin Management

Extend RAPS CLI capability with external plugins and command aliases.

## Aliases

Create shortcuts for frequently used commands.

### List Aliases
```bash
raps plugin alias list
```

### Add Alias
```bash
raps plugin alias add <name> "<command>"
```

**Example:**
```bash
# Create an alias 'up' for uploading with resume
raps plugin alias add up "object upload --resume"

# Use the alias
raps up my-bucket large-file.zip
```

### Remove Alias
```bash
raps plugin alias remove <name>
```

## External Plugins

RAPS CLI supports external plugins. Any executable named `raps-<name>` in your system PATH can be invoked as `raps <name>`.

### List Plugins
List all discovered plugins on your system.
```bash
raps plugin list
```

### Enable/Disable Plugins
Control which plugins are active.

```bash
raps plugin enable <name>
raps plugin disable <name>
```
