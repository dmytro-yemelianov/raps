# RAPS Strategic Direction: Local Operations & Rust-Native Processing

> **Last updated:** 2026-03-01 (v5.0.0)
>
> **Current state:** RAPS v5.0.0 — 95+ commands, 107 MCP tools, 10 workspace crates, swarm orchestration kernel (circuit breaker, rate budget, response cache, metrics, checkpoint, audit), HTTP/2 multiplexing, ASVS L2 100% compliance. See also: `docs/architecture/distributed-orchestration.md` for the full swarm architecture.

## Document Purpose

This document covers three interconnected strategic directions for RAPS:

1. **Strategic positioning** — why replacing APS functions locally is symbiotic, not competitive (IBM COBOL analogy)
1. **Local operations migration** — optional path to perform APS operations locally where feasible
1. **Rust-native rewrites** — which external libraries can/should be rewritten in pure Rust

These are exploration directions, not build specs. Each section includes decision criteria, risks, and open questions.

-----

## Part 1: Strategic Positioning — The IBM COBOL Precedent

### The Analogy

IBM mainframes running COBOL handle trillions of transactions annually for banks, insurers, and governments. This code has been running since the 1960s. IBM earns billions from mainframe licenses, support, and the fact that replacing this infrastructure is too risky and too expensive.

AI tools like Claude do not attempt to replace mainframes. They do three things:

1. **Read and explain** what the mainframe does — translate COBOL into understandable language, document business logic, map dependencies.
1. **Translate where possible** — convert COBOL modules to Java/Python for new components, but leave the working core untouched.
1. **Orchestrate** — create an intermediate layer connecting legacy mainframe to modern systems, APIs, and microservices.

IBM not only tolerates this — they actively integrate AI into their own platform (watsonx for Code, IBM Consulting). Because AI does not take customers away from IBM. AI makes the IBM ecosystem more viable by addressing the biggest pain: shortage of COBOL developers and opacity of legacy code.

### Direct Mapping to RAPS vs Autodesk

|IBM / COBOL                                |Autodesk / APS                                       |
|-------------------------------------------|-----------------------------------------------------|
|Mainframe hardware                         |APS cloud infrastructure                             |
|COBOL code (trillions of LOC)              |Proprietary formats (RVT, NWC, DWG)                  |
|IBM licenses ($$$)                         |APS token pricing ($$$)                              |
|COBOL developers (scarce, expensive)       |APS developers (steep learning curve, complex API)   |
|Claude reads/explains COBOL                |RAPS diagnostics, error codes, troubleshooting tools |
|Claude translates COBOL→Java where possible|RAPS converts IFC/STEP/DXF locally where possible    |
|Claude does not touch the mainframe core   |RAPS does not touch RVT/NWC (proprietary, cloud-only)|
|Claude orchestrates legacy↔modern          |RAPS agent swarm orchestrates local↔cloud            |
|IBM earns from lock-in                     |Autodesk earns from subscriptions + ecosystem        |
|AI makes mainframe more accessible         |RAPS makes APS more accessible                       |
|IBM integrates AI itself (watsonx)         |Autodesk could integrate/recommend RAPS              |

### Why IBM Does Not Kill Claude

IBM does not sue Anthropic for Claude reading COBOL. Three reasons:

**Lock-in remains intact.** Even if Claude translates 100% of a bank’s COBOL to Java, the bank will not shut down the mainframe. The risk is too high. Decades of business rules, edge cases, regulatory requirements. Translation is not migration.

**Developer shortage is the real threat to IBM.** If COBOL developers disappear, mainframes become unmaintainable. AI that helps understand and maintain COBOL extends the mainframe’s useful life, not shortens it.

**New customers.** Companies that previously feared mainframe (too complex, no specialists available) can now consider it because AI lowers the entry barrier.

### Why Autodesk Should Not Fight RAPS

Identical logic applies:

**Lock-in remains intact.** Even if RAPS converts 100% of IFC files locally, enterprises will not abandon APS. RVT (Revit) drives 70%+ of BIM workflows. NWC (Navisworks) is the coordination standard. DWG is the industry standard. These formats are closed, and RAPS routes them to APS cloud. Converting IFC locally is not migrating away from Autodesk.

**API complexity is the real threat to Autodesk.** If developers cannot figure out APS (auth confusion, region mismatches, translation failures, undocumented behaviors), they either leave for other platforms or stop automating entirely. RAPS simplifying APS extends the APS ecosystem’s life and reach.

**New customers.** Small AEC firms that could not afford 500 tokens/month for trivial IFC conversions can now use APS for what truly requires it (Revit processing) and handle the rest locally with RAPS. More APS customers means more Revit subscriptions.

### The Abstraction Layer Insight

What Claude did for COBOL is create a new abstraction layer between legacy infrastructure and modern consumers. A business analyst can now understand what a mainframe program does without knowing COBOL. This is not replacement — it is an interface.

RAPS does the same for APS:

```
Without RAPS:
  Developer ← (must know OAuth, URN encoding, region routing,
                chunk sizing, manifest polling, error codes...) → APS APIs

With RAPS:
  Developer ← (raps translate model.rvt) → RAPS ← (handles all complexity) → APS APIs
  AI Agent  ← (raps.analyze_model MCP)   → RAPS ← (handles all complexity) → APS APIs
```

Claude did not replace the mainframe. Claude became the interface to the mainframe for people who do not know COBOL.

RAPS does not replace APS. RAPS becomes the interface to APS for developers who do not want to learn 15 different APIs, OAuth flows, and region routing rules.

### What Actually Happened to IBM

IBM saw that the AI layer was inevitable and made a strategic choice: be inside it rather than fight it. They created watsonx, integrated AI into their products, and positioned themselves as “AI for enterprise.” Mainframe revenue did not decline — it grew, because AI made mainframe more accessible for new use cases.

If Autodesk is strategic (and Cyrille Fauvel’s invitation to DevCon suggests they are), they will do something similar:

1. Recognize that CLI/developer tooling is not their core competency
1. Recognize that RAPS fills a real gap in their ecosystem
1. Either integrate (official recommendation / partnership) or tolerate (symbiotic ecosystem)

### Messaging Framework for the May 2026 Demo

Use this framing directly with Cyrille’s team:

> “RAPS does for APS what AI modernization tools do for COBOL mainframes. We don’t replace the platform — we make it accessible to a new generation of developers. The proprietary core stays in the cloud. We handle open formats locally where it makes sense, and route everything else to APS. The result: more developers can adopt APS, fewer give up in frustration, and Autodesk’s ecosystem grows.”

This framing works because the IBM/COBOL/AI analogy is currently in every enterprise boardroom. It is familiar, non-threatening, and positions RAPS as ecosystem infrastructure rather than competition.

### Positioning Rules

**RAPS should say:**

- “We optimize your APS token spend”
- “Local processing for open formats, cloud for proprietary”
- “Faster dev/test cycles — cloud for production”
- “Works offline when cloud is unavailable”
- “Reduces load on APS infrastructure for trivial conversions”
- “Makes APS accessible to smaller teams with limited token budgets”

**RAPS should never say:**

- “Replace Autodesk cloud”
- “Free alternative to APS”
- “You don’t need Autodesk”
- “APS is overpriced”

-----

## Part 2: Local Operations Migration Plan

### Overview

RAPS currently routes all operations through APS cloud APIs. This section describes an optional, incremental migration where certain operations can be performed locally when the file format allows it, while maintaining APS as the default and fallback for proprietary formats.

The guiding principle: **local where possible, cloud where necessary, user chooses when in doubt.**

### Format Router Architecture

```
raps translate <file>
       │
       ▼
┌─────────────────────────────────┐
│         Format Router            │
│                                  │
│  Inspect file extension/magic    │
│  Check local capability          │
│  Check user preference           │
│                                  │
│  Decision matrix:                │
│  ┌───────┬────────┬───────────┐ │
│  │Format │Local?  │Cloud?     │ │
│  ├───────┼────────┼───────────┤ │
│  │IFC    │✅ Yes  │✅ Fallback│ │
│  │STEP   │✅ Yes  │✅ Fallback│ │
│  │DXF    │✅ Yes  │✅ Fallback│ │
│  │OBJ    │✅ Yes  │✅ Fallback│ │
│  │STL    │✅ Yes  │✅ Fallback│ │
│  │glTF   │✅ Yes  │✅ Fallback│ │
│  │DWG    │⚠️ Partial│✅ Default│ │
│  │RVT    │❌ No   │✅ Only   │ │
│  │NWC    │❌ No   │✅ Only   │ │
│  │NWD    │❌ No   │✅ Only   │ │
│  │IPT    │❌ No   │✅ Only   │ │
│  │IAM    │❌ No   │✅ Only   │ │
│  └───────┴────────┴───────────┘ │
│                                  │
│  Overrides:                      │
│  --force-local   (fail if no)    │
│  --force-cloud   (always APS)    │
│  config: local.enabled = true    │
│  config: local.default = false   │
└─────────────────────────────────┘
```

### Local Operations by Category

#### Category 1: File Translation (Format Conversion)

|Source Format     |Target Format     |Local Tool                         |Quality vs APS                          |Status             |
|------------------|------------------|-----------------------------------|----------------------------------------|-------------------|
|IFC → glTF/GLB    |3D mesh           |IfcConvert (subprocess)            |~95% (tessellation differences possible)|Ready to prototype |
|IFC → SVG         |2D plans          |IfcConvert (subprocess)            |~90% (different rendering)              |Ready to prototype |
|STEP → glTF       |3D mesh           |OpenCascade (FFI or subprocess)    |~95%                                    |Needs evaluation   |
|DXF → SVG         |2D drawing        |Rust-native DXF parser + SVG writer|~98% (simpler format)                   |Ready to build     |
|OBJ → glTF        |Mesh conversion   |Rust-native (existing crates)      |~99% (trivial)                          |Ready now          |
|STL → glTF        |Mesh conversion   |Rust-native (existing crates)      |~99% (trivial)                          |Ready now          |
|PLY → glTF        |Point cloud / mesh|Rust-native                        |~99%                                    |Ready now          |
|DWG → SVG         |2D drawing        |LibreDWG subprocess or ODA         |~80-90% (format complexity)             |Partial, needs eval|
|RVT → anything    |N/A               |Not possible locally               |N/A                                     |Cloud only         |
|NWC/NWD → anything|N/A               |Not possible locally               |N/A                                     |Cloud only         |

#### Category 2: Property Extraction

|Format|Extractable Locally?|Tool                  |Data Available                                                                         |
|------|--------------------|----------------------|---------------------------------------------------------------------------------------|
|IFC   |✅ Full              |Rust-native IFC parser|Property sets, quantities, materials, classifications, spatial structure, relationships|
|DXF   |✅ Full              |Rust-native DXF parser|Block attributes, layer info, extended entity data                                     |
|STEP  |✅ Partial           |Rust STEP parser      |Product structure, geometric metadata                                                  |
|OBJ   |✅ Full              |Rust-native           |Material references, group names                                                       |
|STL   |⚠️ Minimal           |Rust-native           |Triangle count, bounding box, volume estimate                                          |
|DWG   |⚠️ Partial           |LibreDWG subprocess   |Layers, blocks, attributes (version-dependent)                                         |
|RVT   |❌ No                |N/A                   |Cloud only                                                                             |

#### Category 3: Validation & Analysis

These operations are zero-risk from Autodesk relationship perspective — they benefit everyone.

|Operation             |Description                                                                                                                                       |Implementation                                                 |
|----------------------|--------------------------------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------|
|`raps validate <file>`|Check file integrity, required fields, reference consistency                                                                                      |Rust-native parser per format                                  |
|`raps stats <file>`   |Object counts, type distribution, file size analysis, spatial structure summary                                                                   |Rust-native parser                                             |
|`raps diff <v1> <v2>` |Compare two file versions, report added/modified/removed entities                                                                                 |Rust-native parser + diff algorithm                            |
|`raps estimate <file>`|Predict APS translation time and token cost based on file size, type, complexity                                                                  |Rust-native analysis + historical data from Observability Agent|
|`raps check <file>`   |Pre-upload validation — detect issues that would cause APS translation failure (large coordinates, missing references, unsupported schema version)|Rust-native parser                                             |

#### Category 4: Viewing

|Mode                    |Description                                                                                           |Implementation                                              |
|------------------------|------------------------------------------------------------------------------------------------------|------------------------------------------------------------|
|`raps view <urn>`       |View via APS Viewer (existing, requires cloud translation)                                            |Current behavior, unchanged                                 |
|`raps view-local <file>`|View locally without cloud                                                                            |Convert to glTF locally → open in bundled/recommended viewer|
|Viewer options          |xeokit (AGPL, recommend separately), three.js (MIT, can bundle), model-viewer (Apache-2.0, can bundle)|Evaluate which viewer to recommend vs bundle                |

#### Category 5: Storage

|Backend         |Description                              |Use Case                                   |
|----------------|-----------------------------------------|-------------------------------------------|
|APS OSS         |Current default (Autodesk Object Storage)|Production, APS-integrated workflows       |
|S3/MinIO        |AWS S3 or S3-compatible                  |Enterprise with existing AWS infrastructure|
|Azure Blob      |Azure storage                            |Enterprise with Azure                      |
|GCS             |Google Cloud Storage                     |Enterprise with GCP                        |
|Local filesystem|Local directory                          |Development, testing, offline              |

```bash
raps storage use s3://my-bucket          # switch to S3
raps storage use oss://my-aps-bucket     # switch back to APS OSS
raps storage use local:///data/models    # local filesystem
raps upload model.ifc                    # goes to currently configured storage
```

### CLI Interface for Local Operations

```bash
# Automatic routing (default when local.enabled = true)
raps translate model.ifc              # → local, 3 seconds, 0 tokens
raps translate model.rvt              # → APS cloud, 12 minutes, 1.5 tokens

# Explicit override
raps translate model.ifc --local      # force local processing
raps translate model.ifc --cloud      # force APS cloud
raps translate model.ifc --compare    # run both, compare output quality

# Batch with mixed formats — auto-routes each file
raps bulk-translate ./models/
# Output:
# Routing: 120 IFC → local, 40 RVT → cloud, 40 DWG → local
# Local:  160 files completed in 47 seconds (0 tokens)
# Cloud:   40 files completed in 18 minutes (60 tokens)
# Saved:  100 tokens ($300) vs all-cloud

# Property extraction
raps props extract model.ifc              # auto-routes (local for IFC)
raps props extract model.rvt              # auto-routes (cloud for RVT)
raps props extract-local model.ifc        # explicit local

# Validation (always local, zero risk)
raps validate model.ifc
raps validate model.rvt                   # can validate structure even for RVT (partial)
raps diff model-v1.ifc model-v2.ifc
raps estimate model.rvt                   # predict cloud translation time/cost

# Viewing
raps view-local model.ifc                 # local viewer
raps view <urn>                           # APS viewer (existing)
```

### Configuration

```toml
# ~/.config/raps/config.toml

[local]
enabled = true              # enable local processing capability
default = false             # false = cloud by default, local on --local flag
                            # true = local by default for supported formats
fallback_to_cloud = true    # if local fails, automatically try cloud

[local.converters]
# RAPS discovers converters in PATH or configured locations
ifcconvert = "auto"         # auto-detect, or explicit path: "/usr/bin/IfcConvert"
opencascade = "auto"        # auto-detect OCCT tools
libredwg = "auto"           # auto-detect (GPL quarantined subprocess)

[local.quality]
tessellation_tolerance = 0.01   # mesh quality for local conversion
coordinate_precision = "double" # double or single precision

[local.viewer]
backend = "three.js"        # "three.js" (bundled), "xeokit" (external), "model-viewer"
auto_open = true            # automatically open browser

[local.storage]
default = "oss"             # "oss" (APS), "s3", "azure", "gcs", "local"
local_path = "~/.local/share/raps/storage"
```

### Migration Phases

#### Phase A: Validation & Analysis Only (Zero Risk)

Ship first. No competition concern whatsoever — Autodesk benefits from users not uploading broken files.

Deliverables:

- Rust-native IFC parser (data layer only, no geometry)
- Rust-native DXF parser (extend existing crate)
- `raps validate` command
- `raps stats` command
- `raps diff` command
- `raps estimate` command (uses historical metrics from Observability Agent)
- `raps check` command (pre-upload validation)

Timeline: 2-3 months
Risk: None
Autodesk impact: Positive (fewer failed translations, less wasted cloud resources)

#### Phase B: Local Property Extraction

Deliverables:

- `raps props extract-local` for IFC, DXF, STEP, OBJ
- Output format matches APS properties endpoint JSON structure
- Performance benchmarks vs APS cloud extraction

Timeline: 1-2 months after Phase A (parsers already built)
Risk: Very low
Autodesk impact: Minimal (properties extraction is a small part of APS token usage)

#### Phase C: Local Translation for Open Formats

Deliverables:

- Format Router in Coordination Agent
- IfcConvert integration (subprocess) for IFC → glTF
- Trivial format converters (OBJ/STL/PLY → glTF) via Rust crates
- DXF → SVG via Rust-native
- Quality comparison tooling (`raps translate --compare`)
- `raps view-local` with bundled three.js viewer

Timeline: 2-3 months after Phase B
Risk: Moderate (this is where Autodesk messaging matters)
Autodesk impact: Token revenue reduction for open formats, but offset by ecosystem growth

#### Phase D: Alternative Storage Backends

Deliverables:

- Storage abstraction layer
- S3/MinIO backend
- Azure Blob backend
- Local filesystem backend
- `raps storage use <backend>` switching

Timeline: 1-2 months (independent of Phases A-C)
Risk: Low-moderate (enterprise data sovereignty is a legitimate need)
Autodesk impact: Reduces OSS usage, but OSS is free anyway (not a revenue source)

#### Phase E: Optional Commercial Converters

Deliverables:

- Plugin architecture for commercial converter add-ons
- ODA SDK integration for high-quality DWG support (optional, paid)
- HOOPS Exchange integration for 30+ proprietary formats (optional, paid)
- CAD Exchanger CLI integration (optional, paid)

Timeline: 3-6 months
Risk: Low (this is a revenue opportunity for RAPS, not a threat to Autodesk)
Revenue model: RAPS Pro tier or per-converter licensing

### Integration with Swarm Orchestration (v5.0)

Local processing plugs into the swarm kernel modules shipped in v5.0 (see `docs/architecture/distributed-orchestration.md`):

- **Format Router** — new module that decides local vs cloud for each operation based on file format, user preference, and converter availability
- **Response Cache** (`raps-kernel/src/response_cache.rs`) — caches local conversion results identically to cloud results, deduplicates repeated conversions
- **Checkpoint Store** (`raps-kernel/src/checkpoint.rs`) — tracks progress for mixed local+cloud batch jobs, enables resume after interruption
- **Circuit Breaker** (`raps-kernel/src/circuit_breaker.rs`) — falls back to cloud if local conversion fails repeatedly
- **Rate Budget** (`raps-kernel/src/rate_budget.rs`) — routes open formats locally when APS rate limits are exhausted
- **Metrics Collector** (`raps-kernel/src/metrics.rs`) — tracks local vs cloud usage, cost savings, conversion times for `raps estimate` predictions
- **Audit Logger** (`raps-kernel/src/audit.rs`) — records all local/cloud routing decisions for compliance

### Financial Impact Model

APS token costs (current pricing, approximately $3 per token):

|Operation                          |Token Cost        |
|-----------------------------------|------------------|
|Model Derivative (Revit/Navisworks)|1.5 tokens        |
|Model Derivative (other formats)   |0.5 tokens        |
|Design Automation (per hour)       |6 tokens          |
|Viewer sessions                    |Free              |
|Data Management                    |Free              |
|OSS Storage                        |Free (with limits)|

Typical AEC project, 200 translations/month:

Without local processing:

```
200 translations × avg 0.8 tokens = 160 tokens = $480/month = $5,760/year
```

With RAPS hybrid routing (typical format mix: 60% IFC, 15% RVT, 15% DWG, 10% other open):

```
120 IFC   → local ($0)
30  RVT   → APS cloud (45 tokens = $135)
30  DWG   → local ($0)
20  other → local ($0)
Total: $135/month = $1,620/year (72% savings)
```

Enterprise scale, 1000 translations/month:

```
Without: ~800 tokens = $2,400/month = $28,800/year
With:    ~225 tokens = $675/month  = $8,100/year
Annual savings: $20,700
```

Speed improvement:

- Local IFC→glTF: 2-10 seconds (depending on file size)
- APS IFC translation: 3-30 minutes
- Improvement: 20-100x for open formats

-----

## Part 3: Rust-Native Rewrite Roadmap

### Rationale

External tools (IfcOpenShell, LibreDWG, OpenCascade) carry licensing constraints, distribution complexity, and cross-platform build challenges. Rewriting key components in pure Rust eliminates these problems and delivers native performance, zero external dependencies, and full license control.

However, not everything can or should be rewritten. Geometry kernels represent decades of PhD-level research. The strategy is: **rewrite data/parsing layers in Rust, keep geometry processing as subprocess calls to external tools.**

### Feasibility Assessment

#### Can and Should Rewrite in Rust

**DXF Parser/Writer**

DXF is a text-based format with a publicly documented specification (Autodesk publishes the full DXF Reference). The format is essentially structured key-value pairs organized in sections (HEADER, CLASSES, TABLES, BLOCKS, ENTITIES, OBJECTS).

Current state in Rust: the `dxf` crate on crates.io exists with basic reading/writing support. It can be extended or forked.

Scope: ezdxf (Python reference implementation) is approximately 50,000 lines of Python. A Rust implementation focused on reading + entity extraction + SVG export would be approximately 15,000-20,000 lines of Rust.

Effort: 2-3 months for one developer to reach production-ready read support.

License result: MIT or Apache-2.0, full control, no constraints.

Value for RAPS: native-speed DXF parsing for `raps validate`, `raps props extract-local`, `raps stats`, and DXF→SVG conversion. No Python dependency.

**IFC-SPF Parser (Data Layer Only, No Geometry)**

IFC files use STEP Physical File format (ISO 10303-21), which is a text-based format with a well-documented grammar. Parsing the file, extracting entities, properties, relationships, and spatial structure is complex but well-defined work that does not require a geometry kernel.

Current state in Rust: the `ifc-rs` crate exists on GitHub with basic IFC4 entity parsing. It is not mature but provides a starting point and confirms feasibility.

What this enables without geometry:

- `raps validate model.ifc` — structural validation, required field checks, reference integrity
- `raps props extract-local model.ifc` — all property sets, quantities, materials, classifications
- `raps diff v1.ifc v2.ifc` — entity-level comparison between versions
- `raps stats model.ifc` — object counts, types, spatial hierarchy, size analysis
- `raps query model.ifc "IfcWall WHERE Name LIKE 'Ext*'"` — IFC query language
- `raps ifc-to-json model.ifc` — full data export to JSON for downstream processing
- `raps check model.ifc` — pre-upload validation (detect large coordinates, missing refs, unsupported schemas)

Scope: the data layer of IfcOpenShell (excluding OpenCascade geometry engine) is approximately 100,000 lines of C++. A focused Rust implementation covering IFC2x3, IFC4, and IFC4x3 data schemas would be approximately 30,000-40,000 lines of Rust.

Effort: 4-6 months for one developer. The IFC schema is large (800+ entity types) but highly regular — much of it can be code-generated from the EXPRESS schema definitions.

License result: MIT or Apache-2.0, full control.

Value for RAPS: this is the single highest-value Rust rewrite. IFC is the dominant open format in AEC/BIM. Native Rust parsing would be 10-50x faster than Python IfcOpenShell for data extraction on large files. Zero external dependencies. Works on all platforms RAPS targets.

**STEP Parser (Data Layer Only)**

STEP Physical File (ISO 10303-21) shares its grammar with IFC-SPF. Building the IFC parser effectively gives you 80% of a STEP parser. The remaining 20% is schema-specific entity handling.

Effort: 1-2 months after IFC parser is complete (shared foundation).

License result: MIT or Apache-2.0.

**STL/OBJ/PLY/glTF Readers and Writers**

These formats are simple. Mature, well-maintained MIT-licensed Rust crates already exist:

- `gltf` crate — full glTF 2.0 read/write
- `obj-rs` — OBJ reader
- `stl_io` — STL read/write
- `ply-rs` — PLY reader

No rewrite needed. Use existing crates directly. They are already Rust-native and properly licensed.

Effort: 0 months (already available).

**Mesh Conversion Pipeline (between simple formats)**

Converting between mesh formats (OBJ→glTF, STL→glTF, PLY→glTF) is algorithmically trivial — it is data structure transformation with optional normal recalculation and index buffer optimization.

Effort: 2-4 weeks for a robust pipeline using existing crates.

**URN Encoding/Decoding, JWT Parsing, OAuth Flows**

Already implemented in RAPS. Included here for completeness — these are Rust-native and have no external dependencies.

#### Could Rewrite But Not Worth It

**STEP Geometry Evaluation**

STEP geometry uses NURBS surfaces, CSG operations, and BREP topology. Evaluating this into renderable meshes requires the same geometry kernel capabilities as IFC geometry. The parser (data extraction) is feasible in Rust, but geometry tessellation requires OpenCascade or equivalent.

Verdict: rewrite the parser, keep OpenCascade subprocess for geometry.

**DWG Parser**

DWG is a closed, undocumented binary format. LibreDWG (~200,000 lines of C) is built on decades of reverse engineering. Rewriting in Rust would mean repeating all that reverse engineering work — the format specification is not publicly available.

Additionally, there is legal risk. Autodesk has historically litigated to protect the DWG format (lawsuits against Open Design Alliance). A clean-room Rust implementation would need careful legal review.

Verdict: do not rewrite. Use LibreDWG as subprocess (GPL quarantined) or ODA SDK as optional commercial plugin. If DWG support becomes critical, invest in ODA membership ($5K/year) rather than a custom parser.

#### Cannot Rewrite — Use As External Tool

**OpenCascade (OCCT) — Geometry Kernel**

OpenCascade is 7+ million lines of C++ developed over 30+ years. It provides:

- BREP (Boundary Representation) modeling
- Boolean operations (union, intersection, subtraction)
- NURBS surface evaluation
- Sweep and loft operations
- Tessellation (BREP → triangle mesh)
- Tolerance handling for imprecise geometry

The Rust CAD ecosystem has early-stage alternatives:

- **Truck** — Japanese company, b-rep kernel in Rust. Has basic topology but experts on Hacker News note: “a reliable geometric kernel takes around a decade of work to develop, for a team of PhDs. Boolean operations, offset surfaces, lofted surfaces, blended surfaces — each of these can be a year-long research project in itself.”
- **Fornjot** — early-stage experimental b-rep kernel in Rust. Self-describes as having “so far unrealized” goals.
- **vcad** — parametric CAD in Rust, ~35K lines, has boolean operations and MCP server integration. Most promising for future evaluation but not production-ready for industrial use.

The CADmium project (another Rust CAD effort) honestly describes the landscape: “Each of the big four CAD companies has written their own [b-rep kernel], and it took them decades.” About OpenCascade specifically: “the Pontiac Aztek of b-rep kernels: It is ugly, barebones, and it might break down on you, but it is drivable and you can get one for free.”

Verdict: do not attempt to rewrite. Use OpenCascade or IfcConvert as subprocess. Monitor Truck and vcad for future viability (re-evaluate annually). When Rust geometry kernels mature (estimated 3-5 years), consider switching.

**IfcOpenShell Geometry Engine**

IfcOpenShell’s geometry processing is a bridge between IFC’s implicit geometry descriptions and OpenCascade’s kernel. It translates IFC geometry representations (extrusions, booleans, swept solids, clipping planes, mapped items) into OCCT operations.

This layer is approximately 200,000 lines of C++ tightly coupled to OpenCascade’s API.

Verdict: do not rewrite. Use IfcConvert as subprocess for IFC→glTF conversion. Use Rust-native IFC parser for everything that does not require geometry (which is most of what RAPS needs).

### Architecture: Three-Layer Model

```
┌─────────────────────────────────────────────────────────┐
│ Layer 1: Pure Rust (own code, MIT/Apache-2.0)           │
│                                                          │
│ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐     │
│ │ IFC Parser   │ │ DXF Parser   │ │ STEP Parser  │     │
│ │ (data only)  │ │ (read/write) │ │ (data only)  │     │
│ └──────────────┘ └──────────────┘ └──────────────┘     │
│ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐     │
│ │ glTF r/w     │ │ OBJ/STL/PLY  │ │ Mesh Pipeline│     │
│ │ (gltf crate) │ │ (existing)   │ │ (conversion) │     │
│ └──────────────┘ └──────────────┘ └──────────────┘     │
│ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐     │
│ │ Format Router│ │ Validation   │ │ Diff Engine  │     │
│ │              │ │ Engine       │ │              │     │
│ └──────────────┘ └──────────────┘ └──────────────┘     │
│                                                          │
│ → Zero external dependencies                             │
│ → Any license                                            │
│ → Native speed (10-50x vs Python)                        │
│ → Cross-platform (all RAPS targets)                      │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ Layer 2: Subprocess (external CLI tools, license-safe)   │
│                                                          │
│ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐     │
│ │ IfcConvert   │ │ LibreDWG     │ │ OCCT CLI     │     │
│ │ (LGPL, ok)   │ │ (GPL, quar.) │ │ (LGPL, ok)   │     │
│ └──────────────┘ └──────────────┘ └──────────────┘     │
│                                                          │
│ → License isolated via process boundary                  │
│ → User installs separately or RAPS auto-detects          │
│ → Graceful degradation if not available                   │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ Layer 3: Optional Commercial Plugins (paid add-ons)      │
│                                                          │
│ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐     │
│ │ ODA SDK      │ │ HOOPS        │ │ CAD Exchanger│     │
│ │ (DWG, $5K/y) │ │ Exchange     │ │ (30+ formats)│     │
│ │              │ │ (30+ fmt,    │ │              │     │
│ │              │ │  $10K+/y)    │ │              │     │
│ └──────────────┘ └──────────────┘ └──────────────┘     │
│                                                          │
│ → Optional, not required for core functionality           │
│ → Revenue opportunity: RAPS Pro tier                      │
│ → Enables proprietary format support locally              │
└─────────────────────────────────────────────────────────┘
```

### Converter Plugin Discovery

RAPS discovers available converters at startup and adjusts Format Router capabilities accordingly:

```rust
struct ConverterRegistry {
    /// Layer 1: always available (compiled into RAPS binary)
    native: Vec<NativeConverter>,

    /// Layer 2: discovered in PATH or configured locations
    external: Vec<ExternalConverter>,

    /// Layer 3: discovered via plugin directory
    plugins: Vec<PluginConverter>,
}

struct NativeConverter {
    name: String,                    // "raps-ifc-parser"
    input_formats: Vec<Format>,      // [IFC]
    output_formats: Vec<Format>,     // [JSON, CSV]
    capabilities: Vec<Capability>,   // [Validate, ExtractProps, Diff, Stats]
}

struct ExternalConverter {
    name: String,                    // "IfcConvert"
    path: PathBuf,                   // "/usr/bin/IfcConvert"
    license: License,                // LGPL
    input_formats: Vec<Format>,      // [IFC]
    output_formats: Vec<Format>,     // [glTF, OBJ, DAE, SVG]
    capabilities: Vec<Capability>,   // [Translate]
}

struct PluginConverter {
    name: String,                    // "raps-oda-dwg"
    path: PathBuf,                   // "~/.config/raps/plugins/raps-oda-dwg"
    license: License,                // Commercial
    input_formats: Vec<Format>,
    output_formats: Vec<Format>,
    capabilities: Vec<Capability>,
}
```

On `raps translate model.ifc`, the Format Router:

1. Checks native converters first (Layer 1) — can it validate? extract props?
1. Checks external converters (Layer 2) — can IfcConvert do IFC→glTF?
1. Checks plugins (Layer 3) — any commercial converter available?
1. Falls back to APS cloud if no local option exists
1. Respects user override flags (`--local`, `--cloud`)

```bash
# Show available converters
raps converters list
# Native (built-in):
#   IFC parser    → validate, props, diff, stats, query    [Rust-native]
#   DXF parser    → validate, props, convert to SVG        [Rust-native]
#   glTF/OBJ/STL  → convert between mesh formats           [Rust-native]
#
# External (detected):
#   IfcConvert 0.8.0  → IFC to glTF/OBJ/DAE/SVG           [LGPL, /usr/bin/IfcConvert]
#   LibreDWG 0.13     → DWG read                           [GPL, /usr/bin/dwgread]
#
# Plugins (installed):
#   (none)
#
# Cloud (always available):
#   APS Model Derivative → all supported formats            [requires tokens]

# Install a plugin
raps converters install oda-dwg --license-key <KEY>
```

### Rust Rewrite Priority Order

Based on value-to-effort ratio and strategic importance:

**Priority 1: IFC Data Parser** (4-6 months)

- Highest value: IFC is the dominant open BIM format
- Enables: validate, props, diff, stats, query, check, estimate — all without external deps
- Performance: 10-50x faster than IfcOpenShell Python for large files
- Strategic: demonstrates RAPS technical depth, unique capability no other APS tool has
- Code generation opportunity: IFC EXPRESS schemas can auto-generate entity structs

**Priority 2: DXF Parser** (2-3 months)

- High value: DXF/DWG are the most common CAD exchange formats
- Publicly documented specification
- Existing `dxf` crate as starting point
- Enables DXF→SVG conversion without external tools

**Priority 3: STEP Data Parser** (1-2 months after IFC)

- Shares ~80% of grammar/infrastructure with IFC parser
- Covers mechanical CAD data exchange (non-BIM use cases)
- Extends RAPS into manufacturing/mechanical engineering market

**Priority 4: Mesh Conversion Pipeline** (2-4 weeks)

- Trivial using existing crates
- Enables OBJ/STL/PLY→glTF conversion natively
- Completes the “open formats locally” story

**Priority 5: IFC Query Language** (2-4 weeks after IFC parser)

- Small additional effort on top of IFC parser
- Unique feature — no other CLI tool offers SQL-like queries over IFC files
- Powerful for scripting and automation

### Dependency Additions

```toml
# Workspace Cargo.toml additions for local processing
# Follow existing pattern: workspace.dependencies + member crate opt-in

[workspace.members]
# Add to existing members list:
# "raps-ifc", "raps-dxf", "raps-mesh", "raps-converters"

[workspace.dependencies]
# Layer 1: Internal crates
raps-ifc = { path = "raps-ifc", version = "5.0.0" }
raps-dxf = { path = "raps-dxf", version = "5.0.0" }
raps-mesh = { path = "raps-mesh", version = "5.0.0" }
raps-converters = { path = "raps-converters", version = "5.0.0" }

# Layer 1: Pure Rust parsers (MIT/Apache-2.0)
gltf = "1"
stl_io = "0.7"

# raps-cli/Cargo.toml — opt-in via feature flag
[features]
local = ["dep:raps-ifc", "dep:raps-dxf", "dep:raps-mesh", "dep:raps-converters"]

# No Layer 2/3 dependencies in Cargo.toml — they are external binaries
```

### Crate Structure

New crates follow the existing workspace layout (top-level members, not a `crates/` subdirectory):

```
raps/
├── raps-kernel/               ← existing: auth, http, security, swarm modules
├── raps-oss/                  ← existing: object storage + model derivative
├── raps-derivative/           ← existing: translation workflows
├── raps-dm/                   ← existing: data management
├── raps-da/                   ← existing: design automation
├── raps-acc/                  ← existing: construction cloud
├── raps-webhooks/             ← existing: webhooks
├── raps-reality/              ← existing: reality capture
├── raps-cli/                  ← existing: CLI + MCP server + TUI dashboard
│
├── raps-ifc/                  ← NEW: Rust-native IFC parser
│   ├── src/
│   │   ├── parser/            ← STEP Physical File parser
│   │   │   ├── lexer.rs
│   │   │   ├── parser.rs
│   │   │   └── spf.rs         ← ISO 10303-21 grammar
│   │   ├── schema/            ← IFC entity definitions
│   │   │   ├── ifc2x3.rs      ← generated from EXPRESS
│   │   │   ├── ifc4.rs        ← generated from EXPRESS
│   │   │   └── ifc4x3.rs      ← generated from EXPRESS
│   │   ├── model.rs           ← in-memory IFC model
│   │   ├── properties.rs      ← property set extraction
│   │   ├── spatial.rs         ← spatial structure navigation
│   │   ├── validate.rs        ← structural validation
│   │   ├── diff.rs            ← model comparison
│   │   ├── query.rs           ← IFC query language
│   │   └── json.rs            ← JSON export
│   ├── build.rs               ← optional: EXPRESS → Rust codegen
│   └── Cargo.toml
│
├── raps-dxf/                  ← NEW: Extended DXF support
│   ├── src/
│   │   ├── reader.rs
│   │   ├── entities.rs
│   │   ├── svg_export.rs      ← DXF → SVG conversion
│   │   └── properties.rs
│   └── Cargo.toml
│
├── raps-mesh/                 ← NEW: Mesh format pipeline
│   ├── src/
│   │   ├── convert.rs         ← OBJ/STL/PLY ↔ glTF
│   │   ├── optimize.rs        ← index buffer optimization
│   │   └── thumbnail.rs       ← headless render to image
│   └── Cargo.toml
│
├── raps-converters/           ← NEW: Converter registry + subprocess mgmt
│   ├── src/
│   │   ├── registry.rs        ← discover native/external/plugin converters
│   │   ├── subprocess.rs      ← run external tools safely
│   │   ├── router.rs          ← Format Router decision logic
│   │   └── plugin.rs          ← plugin loading
│   └── Cargo.toml
│
├── python-bindings/           ← existing: PyO3 bindings (separate from workspace)
└── Cargo.toml                 ← workspace root
```

-----

## Open Questions

### Technical

1. **IFC parser scope.** Should the Rust IFC parser aim to cover 100% of IFC entities (800+) or focus on the most common subset (IfcWall, IfcSlab, IfcBeam, IfcColumn, IfcWindow, IfcDoor — covers ~80% of typical models)? Full coverage is achievable via code generation from EXPRESS schemas but adds compilation time.
1. **EXPRESS codegen.** The IFC schema is defined in EXPRESS (ISO 10303-11). A build.rs script could parse EXPRESS and generate Rust structs automatically. This ensures schema completeness and simplifies updates when new IFC versions release. Worth the build complexity?
1. **Binary size.** Adding IFC parser + DXF parser + mesh pipeline to the RAPS binary — estimated 2-5MB additional. Acceptable? Or should these be separate binaries discovered via PATH?
1. **IfcConvert distribution.** IfcOpenShell provides pre-built binaries for Windows/macOS/Linux. Should `raps converters install ifcconvert` auto-download and configure it? Or require manual installation?
1. **Quality benchmarking.** Need a test suite comparing local conversion output vs APS output for identical input files. Geometry diff, property completeness, visual comparison. What threshold is “good enough”?

### Strategic

1. **Timing relative to Autodesk relationship.** Phase A (validation) is safe to ship anytime. Phase C (local translation) should wait for signals from the May demo. If Cyrille’s team is enthusiastic about RAPS, position local processing as “offline dev mode.” If they are cautious, hold it.
1. **Naming.** “Local processing” is neutral. “APS replacement” is threatening. “Offline mode” is safe. “Hybrid routing” is technical. Choose terminology carefully for public messaging.
1. **Open-source the parsers?** The Rust IFC parser could become a standalone open-source crate (`ifc-rs` or `raps-ifc`). This would build community, attract contributors, and establish RAPS as a technical authority. But it also helps competitors. Trade-off analysis needed.

### Business

1. **RAPS Pro tier.** Commercial converter plugins (ODA, HOOPS) create a natural premium tier. Pricing model: per-plugin license, or bundle as “RAPS Enterprise”?
1. **Value quantification.** The token savings calculator should be a marketing tool: “Enter your monthly translation volume → see how much RAPS saves.” This is concrete, measurable, and compelling for procurement decisions.