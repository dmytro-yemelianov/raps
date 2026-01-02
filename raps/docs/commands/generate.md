---
layout: default
title: Generate Synthetic Data
---

# Generate Synthetic Data

Generate synthetic engineering files and datasets for testing and demonstration purposes. This is useful for testing validation pipelines, bulk processing, or performance testing without needing proprietary data.

## Generate Files

Generate a set of synthetic files (OBJ, DXF, STL, IFC, JSON, XYZ).

```bash
raps generate files --count <N> --output <DIR> [--complexity <LEVEL>]
```

**Options:**
- `--count, -c`: Number of files of *each type* to generate (default: 3)
- `--output, -o`: Output directory (default: `./generated-files`)
- `--complexity`: Complexity level: `simple`, `medium`, `complex` (default: `medium`)

**Example:**
```bash
# Generate 5 files of each type in ./test-data
raps generate files --count 5 --output ./test-data --complexity simple
```

**Generated File Types:**
- **OBJ**: 3D mesh geometry (compatible with SVF translation)
- **DXF**: 2D CAD drawings (compatible with SVF translation)
- **STL**: 3D printing meshes (compatible with SVF translation)
- **IFC**: BIM models (Industry Foundation Classes)
- **JSON**: Metadata sidecar files
- **XYZ**: Point cloud data

## Complexity Levels

| Level | Vertices (OBJ) | Elements (IFC) | Points (XYZ) |
|-------|----------------|----------------|--------------|
| simple | 8 | 50 | 1,000 |
| medium | 50 | 200 | 10,000 |
| complex | 200 | 1,000 | 100,000 |
