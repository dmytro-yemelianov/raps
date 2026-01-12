# Test Fixtures

This directory contains sample API responses for testing RAPS API clients.

## Authentication

- `token_response.json` - Successful OAuth token response

## Object Storage Service (OSS)

- `buckets_list.json` - List of buckets
- `objects_list.json` - List of objects in a bucket

## Model Derivative

- `manifest_success.json` - Completed translation manifest
- `manifest_pending.json` - In-progress translation manifest

## Data Management

- `hubs_list.json` - List of BIM 360/ACC hubs
- `projects_list.json` - List of projects in a hub

## Design Automation

- `workitem_success.json` - Completed workitem

## Error Responses

- `error_401.json` - Unauthorized error
- `error_429.json` - Rate limit exceeded error

## Usage

Load fixtures in tests using:

```rust
let fixture = include_str!("../fixtures/buckets_list.json");
let buckets: serde_json::Value = serde_json::from_str(fixture).unwrap();
```

Or with wiremock:

```rust
use wiremock::{Mock, ResponseTemplate};

Mock::given(method("GET"))
    .and(path("/oss/v2/buckets"))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_string(include_str!("../fixtures/buckets_list.json"))
    )
    .mount(&server)
    .await;
```
