# Quickstart: Align Mock Server and Examples with v4.13.0

**Branch**: `001-align-mock-examples` | **Date**: 2026-02-24

## Scenario 1: Model Derivative Metadata via Mock

```bash
# Start mock server
cd raps-mock && cargo run -- --mode stateful --port 3000

# In another terminal, configure RAPS to use mock
export APS_BASE_URL=http://localhost:3000
export APS_CLIENT_ID=mock-client
export APS_CLIENT_SECRET=mock-secret

# Create a translation (mock auto-completes it)
raps translate start dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6YnVja2V0L2ZpbGUucnZ0 --format svf2

# Get metadata (lists model views/viewables)
raps translate metadata dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6YnVja2V0L2ZpbGUucnZ0 --output json

# Get object tree for a specific view GUID
raps translate tree dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6YnVja2V0L2ZpbGUucnZ0 mock-guid-001 --output json

# Get all properties for a view
raps translate properties dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6YnVja2V0L2ZpbGUucnZ0 mock-guid-001 --output json

# Query specific object properties by ID
raps translate query-properties dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6YnVja2V0L2ZpbGUucnZ0 mock-guid-001 --filter "1,2,3" --output json

# With region header
raps translate metadata dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6YnVja2V0L2ZpbGUucnZ0 --region EMEA --output json
```

## Scenario 2: OSS Copy and Batch Operations via Mock

```bash
# Create source and destination buckets
raps bucket create -k src-bucket -p transient -r US
raps bucket create -k dest-bucket -p transient -r US

# Upload test objects
raps object upload src-bucket ./test-data/sample.ifc
raps object upload src-bucket ./test-data/sample.stp

# Copy single object
raps object copy --source-bucket src-bucket --source-object sample.ifc --dest-bucket dest-bucket

# Batch copy all objects from src to dest
raps object batch-copy src-bucket dest-bucket

# Batch rename objects (prefix change)
raps object batch-rename src-bucket --from "sample" --to "renamed"

# Verify copies
raps object list dest-bucket --output json
```

## Scenario 3: DA Appbundle Upload via Mock

```bash
# Create an appbundle first (required before upload)
raps da appbundle-create -i MyBundle -e "Autodesk.Revit+2024"

# Upload the zip archive to the bundle (mock returns upload parameters + accepts upload)
raps da appbundle-upload MyBundle --file ./test-data/bundle.zip --engine "Autodesk.Revit+2024"

# Verify the bundle was created
raps da appbundles
```

## Scenario 4: Running Example Tests Against Mock

```bash
# Start mock server
cd raps-mock && cargo run -- --mode stateful --port 3000 &

# Run all example tests in mock mode
cd raps-examples && pytest --mock --mock-port 3000

# Run only the new metadata tests
pytest --mock tests/test_05_model_derivative.py -k "metadata or tree or properties"

# Run only the new batch operation tests
pytest --mock tests/test_03_storage.py -k "copy or batch"

# Run only the new DA upload tests
pytest --mock tests/test_06_design_automation.py -k "appbundle_upload"
```
