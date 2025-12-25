---
layout: default
title: Data Management Commands
---

# Data Management Commands

Browse and manage BIM 360/ACC hubs, projects, folders, and items. These commands require 3-legged OAuth authentication.

## Commands

### Hub Commands

#### `raps hub list`

List all accessible hubs.

**Usage:**
```bash
raps hub list
```

**Example:**
```bash
$ raps hub list
Fetching hubs (requires 3-legged auth)...

Hubs:
────────────────────────────────────────────────────────────────────────────────
Hub Name                                        Type           Region
────────────────────────────────────────────────────────────────────────────────
My Company Hub                                  BIM 360        US
  ID: b.abc123xyz
ACC Project Hub                                 ACC            US
  ID: b.def456uvw
────────────────────────────────────────────────────────────────────────────────
```

**Requirements:**
- 3-legged OAuth authentication (`raps auth login`)

#### `raps hub info`

Get detailed information about a hub.

**Usage:**
```bash
raps hub info <hub-id>
```

**Example:**
```bash
$ raps hub info b.abc123xyz
Fetching hub details...

Hub Details
────────────────────────────────────────────────────────────
  Name: My Company Hub
  ID: b.abc123xyz
  Type: hub
  Region: US
  Extension: BIM 360
────────────────────────────────────────────────────────────

Use 'raps project list <hub-id>' to see projects
```

**Requirements:**
- 3-legged OAuth authentication

### Project Commands

#### `raps project list`

List projects in a hub.

**Usage:**
```bash
raps project list <hub-id>
```

**Example:**
```bash
$ raps project list b.abc123xyz
Fetching projects...

Projects:
────────────────────────────────────────────────────────────────────────────────
Project Name                                    Type           Status
────────────────────────────────────────────────────────────────────────────────
Office Building Project                         ACC            active
  ID: b.project123
Warehouse Renovation                            BIM 360        active
  ID: b.project456
────────────────────────────────────────────────────────────────────────────────
```

**Requirements:**
- 3-legged OAuth authentication

### Folder Commands

#### `raps folder list`

List contents of a folder.

**Usage:**
```bash
raps folder list <project-id> <folder-id>
```

**Example:**
```bash
$ raps folder list b.project123 urn:adsk.wiprod:fs.folder:co.abc123xyz
Fetching folder contents...

Folder Contents:
────────────────────────────────────────────────────────────────────────────────
Name                                            Type           Size
────────────────────────────────────────────────────────────────────────────────
Models                                          folder         -
  ID: urn:adsk.wiprod:fs.folder:co.def456uvw
Documents                                       folder         -
  ID: urn:adsk.wiprod:fs.folder:co.ghi789rst
building.dwg                                    item           2.45 MB
  ID: urn:adsk.wiprod:fs.file:co.jkl012mno
────────────────────────────────────────────────────────────────────────────────
```

**Requirements:**
- 3-legged OAuth authentication

#### `raps folder create`

Create a new folder.

**Usage:**
```bash
raps folder create <project-id> <parent-folder-id> <folder-name>
```

**Example:**
```bash
$ raps folder create b.project123 urn:adsk.wiprod:fs.folder:co.abc123xyz "New Folder"
Creating folder...
✓ Folder created successfully!
  Name: New Folder
  ID: urn:adsk.wiprod:fs.folder:co.xyz789abc
```

**Requirements:**
- 3-legged OAuth authentication
- `data:write` scope

### Item Commands

#### `raps item list`

List items in a folder.

**Usage:**
```bash
raps item list <project-id> <folder-id>
```

**Example:**
```bash
$ raps item list b.project123 urn:adsk.wiprod:fs.folder:co.abc123xyz
Fetching items...

Items:
────────────────────────────────────────────────────────────────────────────────
Name                                            Type           Size        Version
────────────────────────────────────────────────────────────────────────────────
building.dwg                                    file           2.45 MB     v1
floorplan.pdf                                   file           1.23 MB     v2
────────────────────────────────────────────────────────────────────────────────
```

**Requirements:**
- 3-legged OAuth authentication

#### `raps item versions`

View versions of an item.

**Usage:**
```bash
raps item versions <project-id> <item-id>
```

**Example:**
```bash
$ raps item versions b.project123 urn:adsk.wiprod:fs.file:co.abc123xyz
Fetching versions...

Versions:
────────────────────────────────────────────────────────────────────────────────
Version  Date                    Modified By          Size
────────────────────────────────────────────────────────────────────────────────
v2       2024-01-15 10:30:00     john.doe@example.com  2.45 MB
v1       2024-01-10 14:20:00     jane.smith@example.com 2.12 MB
────────────────────────────────────────────────────────────────────────────────
```

**Requirements:**
- 3-legged OAuth authentication

## Common Workflows

### Browse a Project Structure

```bash
# 1. List hubs
raps hub list

# 2. List projects in a hub
raps project list b.abc123xyz

# 3. List folder contents
raps folder list b.project123 urn:adsk.wiprod:fs.folder:co.abc123xyz

# 4. View item versions
raps item versions b.project123 urn:adsk.wiprod:fs.file:co.def456uvw
```

### Create a Folder Structure

```bash
# 1. Get root folder ID (usually from project info)
# 2. Create a folder
raps folder create b.project123 urn:adsk.wiprod:fs.folder:co.root "Models"

# 3. Create subfolder
raps folder create b.project123 urn:adsk.wiprod:fs.folder:co.models "2024"
```

## Project ID Format

**Important:** When using Data Management commands, use the project ID **without** the "b." prefix that's used internally.

- **Correct**: `b.project123` (as shown in `raps project list`)
- **Incorrect**: `project123` (missing "b." prefix)

However, for Issues API commands, use the project ID **without** the "b." prefix:
- **Correct for Issues**: `project123`
- **Incorrect for Issues**: `b.project123`

## Hub Types

- **BIM 360** - Autodesk BIM 360 projects
- **ACC** - Autodesk Construction Cloud projects
- **A360** - Autodesk A360 projects
- **Fusion** - Autodesk Fusion projects

## Permissions

Different operations require different scopes:

- **Read operations** (`list`, `info`, `versions`): `data:read`
- **Write operations** (`create`): `data:write` or `data:create`

Ensure you have the appropriate scopes when logging in:
```bash
raps auth login
# Select the required scopes
```

## Related Commands

- [Authentication](commands/auth.md) - Set up 3-legged OAuth
- [Issues](commands/issues.md) - Manage ACC/BIM 360 issues
- [Translation](commands/translation.md) - Translate files from projects

