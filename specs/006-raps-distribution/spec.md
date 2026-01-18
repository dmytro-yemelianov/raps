# Feature Specification: Multi-Channel RAPS Distribution

**Feature Branch**: `006-raps-distribution`
**Created**: 2026-01-17
**Status**: Draft
**Input**: User description: "Multi-channel distribution for RAPS CLI via PyPI, npm, and install scripts"

## Clarifications

### Session 2026-01-17

- Q: Should all 4 distribution channels be implemented in a single release or phased? → A: Phased - Install scripts + PyPI CLI first, npm + Python bindings later
- Q: Should install scripts collect anonymous usage telemetry? → A: No telemetry - privacy-first, rely on user-reported issues

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Quick Install via Shell Script (Priority: P1)

A developer wants to quickly install RAPS on their Linux or macOS machine without needing any package manager. They should be able to run a single command that downloads and installs RAPS, making it immediately available in their terminal.

**Why this priority**: This provides the fastest path to installation with zero dependencies. Users on any Unix-like system can get started immediately, which is critical for adoption and first impressions.

**Independent Test**: Run the install script on a fresh system and verify `raps --version` works within 60 seconds of starting.

**Acceptance Scenarios**:

1. **Given** a fresh Linux/macOS system without RAPS, **When** user runs the install command, **Then** RAPS is installed and available in PATH without restart
2. **Given** RAPS install script running, **When** the download completes, **Then** user sees progress indicators and a success message with next steps
3. **Given** user wants a specific version, **When** they set version environment variable before running the script, **Then** that specific version is installed
4. **Given** user wants to uninstall, **When** they run the script with uninstall flag, **Then** RAPS is removed and user is notified about PATH cleanup

---

### User Story 2 - Quick Install via PowerShell Script (Priority: P1)

A developer on Windows wants to quickly install RAPS without needing Scoop, Chocolatey, or manual downloads. They should be able to run a single PowerShell command to install RAPS.

**Why this priority**: Windows users need an equally simple installation path. PowerShell is available by default on Windows 10/11.

**Independent Test**: Run the PowerShell script on a fresh Windows system and verify `raps --version` works.

**Acceptance Scenarios**:

1. **Given** a fresh Windows 10/11 system without RAPS, **When** user runs the PowerShell install command, **Then** RAPS is installed and available in new terminal windows
2. **Given** user wants a specific version, **When** they pass version parameter, **Then** that specific version is installed
3. **Given** user wants to uninstall, **When** they run the script with uninstall switch, **Then** RAPS is removed and PATH entry is cleaned up

---

### User Story 3 - Install via pip (Priority: P2)

A Python developer wants to install RAPS using pip, their familiar package manager. This allows them to manage RAPS alongside their other Python tools and potentially include it in their project's requirements.

**Why this priority**: Python is extremely popular in automation and scripting. Many AEC professionals already use Python for BIM automation.

**Independent Test**: Run `pip install raps` in a clean virtual environment and verify `raps --version` works.

**Acceptance Scenarios**:

1. **Given** a Python 3.8+ environment, **When** user runs `pip install raps`, **Then** the `raps` command becomes available in their terminal
2. **Given** RAPS installed via pip, **When** user runs any RAPS command, **Then** the command executes correctly (same as binary distribution)
3. **Given** user on any supported platform (Windows, macOS, Linux), **When** they install via pip, **Then** they get the correct platform-specific binary

---

### User Story 4 - Install via npm (Priority: P2)

A JavaScript/Node.js developer wants to install RAPS using npm, their familiar package manager. They may want to use npx for one-off commands or install globally.

**Why this priority**: npm is the most widely used package manager. Supporting it expands RAPS reach to the massive JavaScript ecosystem.

**Independent Test**: Run `npm install -g @raps/cli` and verify `raps --version` works.

**Acceptance Scenarios**:

1. **Given** Node.js 16+ installed, **When** user runs `npm install -g @raps/cli`, **Then** the `raps` command becomes globally available
2. **Given** user doesn't want global install, **When** they run `npx @raps/cli bucket list`, **Then** the command executes correctly
3. **Given** user on any supported platform, **When** they install via npm, **Then** npm automatically downloads the correct platform-specific binary package

---

### User Story 5 - Python Library Integration (Priority: P3)

A Python developer wants to use RAPS functionality programmatically in their scripts, Jupyter notebooks, or automation pipelines. They need a native Python library rather than shelling out to CLI commands.

**Why this priority**: While CLI is primary, programmatic access enables advanced automation use cases, especially in data science and BIM automation workflows.

**Independent Test**: Import `raps` in Python, authenticate, and list buckets using Python code.

**Acceptance Scenarios**:

1. **Given** Python 3.8+ environment, **When** user imports the raps library, **Then** they can use environment-based authentication
2. **Given** authenticated client, **When** user calls bucket list method, **Then** they receive a list of Bucket objects with proper Python types
3. **Given** a Bucket object, **When** user calls object upload method, **Then** file is uploaded and an Object is returned
4. **Given** any RAPS operation fails, **When** error occurs, **Then** appropriate Python exception is raised

---

### User Story 6 - Automated Release Distribution (Priority: P1)

A maintainer wants all distribution channels to be updated automatically when a new release is published. They should only need to create a GitHub release, and all packages should be published automatically.

**Why this priority**: Manual distribution is error-prone and time-consuming. Automation ensures consistent releases across all channels.

**Independent Test**: Create a test release tag and verify packages appear on PyPI and npm within 30 minutes.

**Acceptance Scenarios**:

1. **Given** a version tag is pushed, **When** release workflow runs, **Then** binaries are built for all 6 platform/arch combinations
2. **Given** binaries are built, **When** workflow continues, **Then** Python wheels are published to PyPI
3. **Given** binaries are built, **When** workflow continues, **Then** npm packages are published (platform packages first, then main)
4. **Given** release is published, **When** user runs install scripts, **Then** they get the newly released version

---

### Edge Cases

- What happens when user's platform/architecture is not supported?
  - Clear error message listing supported platforms and alternative installation methods
- What happens when network connection fails during install?
  - Script should fail gracefully with retry instructions
- What happens when install directory is not writable?
  - Script should detect this early and suggest running with appropriate permissions or using custom directory
- What happens when PATH modification fails?
  - Script should complete installation and provide manual PATH instructions
- What happens when PyPI or npm is temporarily unavailable?
  - Release workflow should retry with backoff, fail clearly if unavailable

## Requirements *(mandatory)*

### Functional Requirements

**Install Scripts (Bash)**
- **FR-001**: Install script MUST detect operating system (Linux, macOS) and architecture (x64, arm64)
- **FR-002**: Install script MUST download the correct binary from GitHub releases
- **FR-003**: Install script MUST install to user's home directory by default, with configurable install directory
- **FR-004**: Install script MUST add install directory to PATH by modifying shell configuration files
- **FR-005**: Install script MUST verify installation by running `raps --version`
- **FR-006**: Install script MUST support installing specific versions via environment variable
- **FR-007**: Install script MUST provide uninstall functionality via flag

**Install Scripts (PowerShell)**
- **FR-008**: PowerShell script MUST detect architecture (x64, arm64)
- **FR-009**: PowerShell script MUST download correct binary from GitHub releases
- **FR-010**: PowerShell script MUST install to user's home directory by default, with configurable directory
- **FR-011**: PowerShell script MUST add install directory to User PATH environment variable
- **FR-012**: PowerShell script MUST support version, no-path-update, and uninstall parameters
- **FR-013**: PowerShell script MUST be compatible with PowerShell 5.1+

**PyPI Distribution (CLI)**
- **FR-014**: Python package MUST install via `pip install raps`
- **FR-015**: After installation, `raps` command MUST be available in PATH
- **FR-016**: Package MUST support Python 3.8+
- **FR-017**: Package MUST support all target platforms (Windows, macOS, Linux on x64 and arm64)
- **FR-018**: Package version MUST match the main project version

**npm Distribution**
- **FR-019**: Main package MUST be published as scoped package
- **FR-020**: Platform-specific packages MUST be published separately for each OS/arch combination
- **FR-021**: Main package MUST detect platform and use correct binary
- **FR-022**: Package MUST provide clear error message if platform is unsupported
- **FR-023**: Package MUST work with npx without prior installation

**Python Bindings (PyO3)**
- **FR-024**: Python library MUST expose Client class for authentication
- **FR-025**: Library MUST support 2-legged auth via constructor with credentials
- **FR-026**: Library MUST support environment-based auth via factory method
- **FR-027**: Library MUST expose bucket operations: list, create, get, delete
- **FR-028**: Library MUST expose object operations: list, upload, download, delete, signed_url
- **FR-029**: Library MUST expose translation operations: translate, status, wait
- **FR-030**: Library MUST provide type hints via stub files
- **FR-031**: Library MUST convert Rust errors to appropriate Python exceptions

**GitHub Actions**
- **FR-032**: Release workflow MUST trigger on version tag push
- **FR-033**: Workflow MUST build binaries for all 6 platform/arch combinations
- **FR-034**: Workflow MUST publish to PyPI using trusted publishing
- **FR-035**: Workflow MUST publish to npm in correct order (platform packages, then main)
- **FR-036**: Workflow MUST test install scripts on all platforms

### Key Entities

- **Distribution Channel**: A method by which users can install RAPS (pip, npm, shell script, etc.)
- **Platform Package**: A package containing the binary for a specific OS/architecture combination
- **Release Artifact**: A binary or package produced during the release process
- **Client**: The main entry point for Python bindings, handling authentication state
- **Bucket/Object**: Representations of OSS resources exposed through Python bindings

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Users can install RAPS via shell script in under 60 seconds on a fresh system
- **SC-002**: Users can install RAPS via `pip install raps` in under 2 minutes
- **SC-003**: Users can install RAPS via npm in under 2 minutes
- **SC-004**: All 6 platform/architecture combinations are supported (Windows, macOS, Linux each on x64 and arm64)
- **SC-005**: Release automation publishes to all channels within 30 minutes of tag push
- **SC-006**: Python bindings allow users to complete common workflows (list buckets, upload file, translate) programmatically
- **SC-007**: Install scripts successfully complete on 95%+ of target systems without errors
- **SC-008**: All distribution methods result in identical CLI behavior when running `raps --version`

## Implementation Scope

**Phase 1 (This Release):**
- Install scripts (Bash for Linux/macOS, PowerShell for Windows) - User Stories 1, 2
- PyPI CLI distribution via maturin wheels - User Story 3
- Release automation for Phase 1 channels - User Story 6 (partial)

**Phase 2 (Future Release):**
- npm distribution with platform packages - User Story 4
- Python bindings via PyO3 - User Story 5
- Extended release automation for all channels - User Story 6 (complete)

## Assumptions

- Users have internet connectivity during installation
- GitHub releases remain the source of truth for release binaries
- PyPI and npm registries remain available for publishing
- Python 3.8+ is the minimum supported Python version (covers 95%+ of Python users)
- Node.js 16+ is the minimum supported Node version (LTS versions)
- PowerShell 5.1 is available on Windows 10/11 by default
- Users on Linux/macOS have bash, zsh, or fish shells
- No telemetry or usage data collection - installation issues tracked via GitHub issues
