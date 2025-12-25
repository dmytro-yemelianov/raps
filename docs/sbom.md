---
layout: default
title: SBOM & Build Provenance
---

# SBOM & Build Provenance

RAPS CLI generates Software Bill of Materials (SBOM) and build provenance information for supply chain security and compliance.

## What is SBOM?

A Software Bill of Materials (SBOM) is a formal, machine-readable inventory of software components and dependencies. It helps organizations:

- **Track dependencies**: Understand what libraries and packages are included
- **Security compliance**: Identify vulnerabilities and security issues
- **License compliance**: Track license obligations
- **Supply chain security**: Verify software integrity and provenance

## Generating SBOM

### Prerequisites

Install a SBOM generation tool:

```bash
# Install cargo-cyclonedx (recommended for CycloneDX format)
cargo install cargo-cyclonedx

# Or install cargo-audit (for security audit)
cargo install cargo-audit
```

### Using Scripts

**Linux/macOS:**
```bash
# Generate CycloneDX SBOM (default)
./scripts/generate-sbom.sh

# Generate SPDX SBOM
./scripts/generate-sbom.sh spdx
```

**Windows PowerShell:**
```powershell
# Generate CycloneDX SBOM (default)
.\scripts\generate-sbom.ps1

# Generate SPDX SBOM
.\scripts\generate-sbom.ps1 -Format spdx
```

### Manual Generation

**Using cargo-cyclonedx:**
```bash
# Generate CycloneDX JSON
cargo cyclonedx --format json --output sbom/raps-sbom.json

# Generate CycloneDX XML
cargo cyclonedx --format xml --output sbom/raps-sbom.xml
```

**Using cargo-audit:**
```bash
# Generate audit report (JSON)
cargo audit --json > sbom/audit-report.json
```

## SBOM Formats

### CycloneDX

CycloneDX is a lightweight SBOM standard designed for application security contexts and supply chain component analysis.

**Features:**
- Component inventory
- Dependency relationships
- License information
- Vulnerability references
- Build metadata

**Example:**
```json
{
  "bomFormat": "CycloneDX",
  "specVersion": "1.4",
  "version": 1,
  "components": [
    {
      "type": "library",
      "name": "reqwest",
      "version": "0.11.27",
      "purl": "pkg:cargo/reqwest@0.11.27"
    }
  ]
}
```

### SPDX

SPDX (Software Package Data Exchange) is a standard format for communicating software bill of materials information.

**Features:**
- Package information
- File-level details
- License information
- Copyright information

## SBOM Contents

The generated SBOM includes:

1. **Application Information**
   - Name: `raps`
   - Version: Current release version
   - Description: Command-line interface for Autodesk Platform Services

2. **Dependencies**
   - All Rust crate dependencies
   - Transitive dependencies
   - Version information
   - Package URLs (PURLs)

3. **Metadata**
   - Build timestamp
   - Build tool information
   - Source repository information

## Using SBOM

### Security Scanning

Use SBOM with security scanning tools:

```bash
# Scan SBOM for vulnerabilities
grype sbom:sbom/raps-sbom.json

# Or use other tools
syft sbom:sbom/raps-sbom.json
```

### License Compliance

Check license compliance:

```bash
# Extract license information
cat sbom/raps-sbom.json | jq '.components[].licenses'
```

### Dependency Analysis

Analyze dependencies:

```bash
# Count total dependencies
cat sbom/raps-sbom.json | jq '.components | length'

# List all dependencies
cat sbom/raps-sbom.json | jq '.components[].name'
```

## Build Provenance

Build provenance provides information about how the software was built, including:

- Build environment
- Build tools and versions
- Source code location
- Build parameters

### Generating Build Provenance

Build provenance is typically generated during CI/CD builds:

```yaml
# Example GitHub Actions workflow
- name: Generate SBOM
  run: |
    cargo install cargo-cyclonedx
    cargo cyclonedx --format json --output sbom.json
    
- name: Upload SBOM
  uses: actions/upload-artifact@v3
  with:
    name: sbom
    path: sbom.json
```

## Release Integration

SBOM files are included with releases:

- **Location**: Attached as release artifacts
- **Format**: CycloneDX JSON (default)
- **Naming**: `raps-sbom-<version>.json`

### Downloading SBOM

Download SBOM from GitHub releases:

```bash
# Download SBOM for v0.4.0
wget https://github.com/dmytro-yemelianov/raps/releases/download/v0.4.0/raps-sbom-0.4.0.json
```

## Compliance & Security

### Enterprise Requirements

Many organizations require SBOM for:
- **Software supply chain security** (Executive Order 14028)
- **License compliance** tracking
- **Vulnerability management**
- **Risk assessment**

### Best Practices

1. **Regular Updates**: Generate SBOM for each release
2. **Version Control**: Track SBOM changes over time
3. **Verification**: Verify SBOM matches actual dependencies
4. **Distribution**: Include SBOM with releases
5. **Documentation**: Document SBOM generation process

## Tools & Resources

### SBOM Tools

- **cargo-cyclonedx**: CycloneDX format generator
- **cargo-audit**: Security audit tool
- **grype**: Vulnerability scanner
- **syft**: SBOM generator and scanner

### Standards

- **CycloneDX**: https://cyclonedx.org/
- **SPDX**: https://spdx.dev/
- **SLSA**: Supply-chain Levels for Software Artifacts

## Related Documentation

- [Release Process](RELEASE.md)
- [Checksums](cli/checksums.md)
- [Security Policy](../../SECURITY.md)

