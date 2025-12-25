---
layout: default
title: Feature Overview
---

# Feature Overview

This page provides a visual overview of RAPS CLI capabilities and how different components work together.

## Command Architecture

```mermaid
graph TB
    subgraph CLI["RAPS CLI"]
        direction TB
        Auth[🔐 auth]
        Bucket[📦 bucket]
        Object[📄 object]
        Translate[🔄 translate]
        Hub[🏢 hub]
        Project[📁 project]
        Folder[📂 folder]
        Item[📎 item]
        Issue[🔧 issue]
        Webhook[🔔 webhook]
        DA[⚙️ da]
        RC[📸 reality-capture]
        Pipeline[📋 pipeline]
        Config[⚙️ config]
    end

    subgraph APIs["APS APIs"]
        AuthAPI[Authentication API]
        OSSAPI[OSS API]
        MDAPI[Model Derivative API]
        DMAPI[Data Management API]
        IssuesAPI[Issues API]
        WebhooksAPI[Webhooks API]
        DAAPI[Design Automation API]
        RCAPI[Reality Capture API]
    end

    Auth --> AuthAPI
    Bucket --> OSSAPI
    Object --> OSSAPI
    Translate --> MDAPI
    Hub --> DMAPI
    Project --> DMAPI
    Folder --> DMAPI
    Item --> DMAPI
    Issue --> IssuesAPI
    Webhook --> WebhooksAPI
    DA --> DAAPI
    RC --> RCAPI
    Pipeline --> CLI
    Config --> CLI
```

## Authentication Flow

```mermaid
flowchart LR
    subgraph TwoLeg["2-Legged OAuth"]
        ClientCreds[Client ID + Secret] --> Token2L[Access Token]
        Token2L --> ServerOps[Server Operations]
    end

    subgraph ThreeLeg["3-Legged OAuth"]
        Browser[Browser Login] --> AuthCode[Authorization Code]
        AuthCode --> Token3L[Access + Refresh Token]
        Token3L --> UserOps[User Operations]
        Device[Device Code] --> Token3L
    end

    subgraph Storage["Token Storage"]
        Token2L --> FileStore[File Storage]
        Token3L --> FileStore
        Token3L --> Keychain[OS Keychain]
    end
```

## Feature Matrix

### Core Features

| Feature | Command | Auth Type | Status |
|---------|---------|-----------|--------|
| Bucket Management | `bucket` | 2-legged | ✅ Stable |
| Object Upload | `object upload` | 2-legged | ✅ Stable |
| Multipart Upload | `object upload` (auto) | 2-legged | ✅ Stable |
| Resumable Upload | `object upload --resume` | 2-legged | ✅ New |
| Batch Upload | `object upload --batch` | 2-legged | ✅ New |
| Object Download | `object download` | 2-legged | ✅ Stable |
| Signed URLs | `object signed-url` | 2-legged | ✅ Stable |

### Model Derivative

| Feature | Command | Auth Type | Status |
|---------|---------|-----------|--------|
| Start Translation | `translate start` | 2-legged | ✅ Stable |
| Check Status | `translate status` | 2-legged | ✅ Stable |
| View Manifest | `translate manifest` | 2-legged | ✅ Stable |
| Download Derivatives | `translate download` | 2-legged | ✅ New |
| Translation Presets | `translate preset` | Local | ✅ New |

### Data Management

| Feature | Command | Auth Type | Status |
|---------|---------|-----------|--------|
| List Hubs | `hub list` | 3-legged | ✅ Stable |
| List Projects | `project list` | 3-legged | ✅ Stable |
| List Folders | `folder list` | 3-legged | ✅ Stable |
| Create Folder | `folder create` | 3-legged | ✅ Stable |
| Item Versions | `item versions` | 3-legged | ✅ Stable |
| Bind OSS Object | `item bind` | 3-legged | ✅ New |

### ACC Issues

| Feature | Command | Auth Type | Status |
|---------|---------|-----------|--------|
| List Issues | `issue list` | 3-legged | ✅ Stable |
| Create Issue | `issue create` | 3-legged | ✅ Stable |
| Update Issue | `issue update` | 3-legged | ✅ Stable |
| Issue Types | `issue types` | 3-legged | ✅ Stable |
| Comments | `issue comment` | 3-legged | ✅ New |
| Attachments | `issue attachment` | 3-legged | ✅ New |
| State Transitions | `issue transition` | 3-legged | ✅ New |

### Design Automation

| Feature | Command | Auth Type | Status |
|---------|---------|-----------|--------|
| List Engines | `da engines` | 2-legged | ✅ Stable |
| List App Bundles | `da appbundles` | 2-legged | ✅ Stable |
| Create App Bundle | `da appbundle-create` | 2-legged | ✅ Stable |
| List Activities | `da activities` | 2-legged | ✅ Stable |
| Create Activity | `da activity create` | 2-legged | ✅ New |
| Run Work Item | `da workitem run` | 2-legged | ✅ New |
| Get Work Item | `da workitem get` | 2-legged | ✅ New |
| Work Item Status | `da status` | 2-legged | ✅ Stable |

### Webhooks

| Feature | Command | Auth Type | Status |
|---------|---------|-----------|--------|
| List Webhooks | `webhook list` | 2-legged | ✅ Stable |
| Create Webhook | `webhook create` | 2-legged | ✅ Stable |
| Delete Webhook | `webhook delete` | 2-legged | ✅ Stable |
| List Events | `webhook events` | Local | ✅ Stable |
| Test Endpoint | `webhook test` | None | ✅ New |

### Configuration & Automation

| Feature | Command | Auth Type | Status |
|---------|---------|-----------|--------|
| Profile Management | `config profile` | Local | ✅ Stable |
| Profile Import | `config profile import` | Local | ✅ New |
| Profile Export | `config profile export` | Local | ✅ New |
| Token Inspection | `auth inspect-token` | Local | ✅ New |
| Pipeline Execution | `pipeline run` | Various | ✅ New |
| Pipeline Validation | `pipeline validate` | Local | ✅ New |

## Data Flow Diagrams

### Upload and Translate Workflow

```mermaid
sequenceDiagram
    participant User
    participant RAPS
    participant OSS
    participant MD as Model Derivative

    User->>RAPS: raps object upload bucket file.dwg
    RAPS->>OSS: PUT /buckets/{bucket}/objects/{key}
    OSS-->>RAPS: Object URN
    RAPS-->>User: ✓ Upload complete (URN: xxx)

    User->>RAPS: raps translate start {urn} --format svf2
    RAPS->>MD: POST /designdata/v2/designdata/{urn}/jobs
    MD-->>RAPS: Job started
    RAPS-->>User: ✓ Translation started

    User->>RAPS: raps translate status {urn} --wait
    loop Check Status
        RAPS->>MD: GET /designdata/v2/designdata/{urn}/manifest
        MD-->>RAPS: Status: inprogress
    end
    MD-->>RAPS: Status: success
    RAPS-->>User: ✓ Translation complete
```

### Pipeline Execution

```mermaid
flowchart TD
    Start([Start]) --> Load[Load Pipeline File]
    Load --> Validate{Validate}
    Validate -->|Invalid| Error[Show Errors]
    Validate -->|Valid| Variables[Process Variables]
    Variables --> Step1[Execute Step 1]
    Step1 -->|Success| Step2[Execute Step 2]
    Step1 -->|Fail| Check1{continue_on_error?}
    Check1 -->|Yes| Step2
    Check1 -->|No| Fail[Pipeline Failed]
    Step2 -->|Success| StepN[Execute Step N]
    Step2 -->|Fail| Check2{continue_on_error?}
    StepN --> Complete([Pipeline Complete])
```

### Design Automation Workflow

```mermaid
sequenceDiagram
    participant User
    participant RAPS
    participant DA as Design Automation
    participant Engine

    User->>RAPS: raps da activity create
    RAPS->>DA: POST /activities
    DA-->>RAPS: Activity created

    User->>RAPS: raps da workitem run {activity}
    RAPS->>DA: POST /workitems
    DA->>Engine: Execute activity
    Engine-->>DA: Processing...
    DA-->>RAPS: Work item ID

    User->>RAPS: raps da workitem get {id} --wait
    loop Check Status
        RAPS->>DA: GET /workitems/{id}
        DA-->>RAPS: Status: inprogress
    end
    DA-->>RAPS: Status: success + report URL
    RAPS-->>User: ✓ Work item complete
```

## Version History

```mermaid
timeline
    title RAPS CLI Version History
    section v0.4.0
        Profile Management : Create, switch, delete profiles
        Config Commands : Get and set configuration values
    section v0.5.0
        Timeout & Concurrency : CLI flags for fine control
        OS Keychain : Secure token storage option
        Batch Processing : Parallel upload/download
    section v0.6.0
        SBOM Generation : CycloneDX format support
        Checksums : SHA256 verification for releases
        Code of Conduct : Community guidelines
    section v0.7.0
        Multipart Uploads : Resume interrupted uploads
        Derivative Downloads : Export translated models
        Translation Presets : Saved configurations
        Issues Enhancements : Comments, attachments
        Pipeline Execution : YAML/JSON automation
        Token Inspection : Scope and expiry analysis
        Webhook Testing : Endpoint validation
```

## Related Documentation

- [Getting Started](getting-started.md) - Quick start guide
- [Commands](commands/buckets.md) - Complete command reference
- [Configuration](configuration.md) - Setup and profiles
- [Pipelines](commands/pipeline.md) - Automation workflows
- [Exit Codes](cli/exit-codes.md) - Error handling for CI/CD

