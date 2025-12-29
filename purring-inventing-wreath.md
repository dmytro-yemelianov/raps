# Plan: Add Cookbook Section to RAPS Website

## Overview

Add a new "Cookbook" documentation section containing industry-specific recipes for AEC/BIM, Manufacturing (MFG), Construction (ACC), and Media & Entertainment workflows.

## Implementation Summary

### Files to Modify

1. **`src/layouts/DocsLayout.astro`** (lines 24-53)
   - Add cookbook to `navSections` array

2. **`src/pages/docs/index.astro`** (lines 8-33)
   - Add cookbook to `sections` object

### Files to Create

3. **`src/content/docs/cookbook-aec.mdx`** - AEC/BIM industry recipes
4. **`src/content/docs/cookbook-manufacturing.mdx`** - Manufacturing workflows
5. **`src/content/docs/cookbook-construction.mdx`** - ACC/Construction recipes
6. **`src/content/docs/cookbook-media.mdx`** - Media & Entertainment workflows

## Detailed Changes

### 1. DocsLayout.astro Navigation

Add after the "Reference" section (around line 52):

```typescript
{
  title: 'Cookbook',
  icon: '🍳',
  items: allDocs
    .filter(doc => doc.data.section === 'cookbook')
    .sort((a, b) => a.data.order - b.data.order)
},
```

### 2. docs/index.astro Sections

Add to `sections` object (around line 32):

```typescript
'cookbook': {
  title: 'Cookbook',
  icon: '🍳',
  description: 'Industry-specific recipes and real-world workflows',
  docs: allDocs.filter(d => d.data.section === 'cookbook').sort((a, b) => a.data.order - b.data.order),
},
```

### 3. Cookbook Recipe Files

Each recipe file follows this frontmatter pattern:

```yaml
---
title: "Cookbook: AEC & BIM"
description: "Recipes for architectural, engineering, and construction workflows"
section: "cookbook"
order: 1
icon: "🏗️"
---
```

## Recipe Content Structure

### AEC & BIM (`cookbook-aec.mdx`, order: 1, icon: 🏗️)
- Model coordination pipelines (upload Revit/IFC, translate, extract data)
- Clash detection workflows
- Design review automation
- Model derivative extraction (thumbnails, metadata, properties)
- Multi-discipline model management

### Manufacturing (`cookbook-manufacturing.mdx`, order: 2, icon: ⚙️)
- CAD file processing (Inventor, STEP, IGES)
- Bill of materials extraction
- Drawing generation pipelines
- Design iteration tracking
- Supplier collaboration workflows

### Construction/ACC (`cookbook-construction.mdx`, order: 3, icon: 🚧)
- Checklist automation and export
- Issues/RFI bulk operations
- Asset management workflows
- Submittal tracking
- Quality control automation
- Site documentation pipelines

### Media & Entertainment (`cookbook-media.mdx`, order: 4, icon: 🎬)
- 3D asset processing (FBX, OBJ)
- Texture and material workflows
- Animation file management
- Reality Capture photogrammetry pipelines
- Asset library management

## Recipe Format Pattern

Each recipe follows the examples.mdx pattern:
1. Clear heading describing the use case
2. Brief context/problem statement
3. Step-by-step bash commands
4. Expected output or result
5. Variations or advanced options
6. Links to related docs

## No Schema Changes Required

The existing content schema in `src/content/config.ts` already supports:
- `section: z.string().optional()` - accepts "cookbook"
- `order: z.number().default(999)` - for ordering recipes
- `icon: z.string().optional()` - for navigation icons

## Navigation Order

The cookbook section will appear last in the sidebar:
1. Getting Started
2. Commands
3. Guides
4. Reference
5. **Cookbook** (new)

This makes sense as cookbook recipes are advanced use cases that build on the fundamentals.
