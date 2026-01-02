---
layout: default
title: ACC Extended Modules
---

# ACC Extended Modules

Manage Assets, Submittals, and Checklists in Autodesk Construction Cloud (ACC) and BIM 360 projects.

## Assets

Manage project assets, including tracking markers, categories, and status.

### List Assets
```bash
raps acc asset list <project-id>
```

### Get Asset
```bash
raps acc asset get <project-id> <asset-id>
```

### Create Asset
```bash
raps acc asset create <project-id> --description "Description" --barcode "12345" --category-id <category-id>
```

### Update Asset
```bash
raps acc asset update <project-id> <asset-id> --description "New Description" --status-id <status-id>
```

## Submittals

Manage submittal items for project specifications and requirements.

### List Submittals
```bash
raps acc submittal list <project-id>
```

### Get Submittal
```bash
raps acc submittal get <project-id> <submittal-id>
```

### Create Submittal
```bash
raps acc submittal create <project-id> --title "My Submittal" --spec-section "01 00 00" --due-date "2023-12-31"
```

### Update Submittal
```bash
raps acc submittal update <project-id> <submittal-id> --status "open"
```

## Checklists

Manage field checklists and templates.

### List Checklists
```bash
raps acc checklist list <project-id>
```

### List Templates
```bash
raps acc checklist templates <project-id>
```

### Create Checklist
```bash
raps acc checklist create <project-id> --title "Daily Safety Check" --template-id <template-id>
```

### Update Checklist
```bash
raps acc checklist update <project-id> <checklist-id> --status "completed"
```

> [!NOTE]
> Project ID should **not** include the `b.` prefix used in Data Management API.
