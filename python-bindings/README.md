# RAPS Python Bindings

Native Python bindings for [RAPS](https://github.com/dmytro-yemelianov/raps) - programmatic access to Autodesk Platform Services.

## Installation

```bash
pip install raps-bindings
```

## Quick Start

```python
from raps import Client

# Create client from environment variables
client = Client.from_env()

# Or with explicit credentials
client = Client(client_id="your_id", client_secret="your_secret")

# Test authentication
client.test_auth()

# List buckets
for bucket in client.buckets.list():
    print(f"{bucket.key}: {bucket.policy}")

# Create a bucket
bucket = client.buckets.create("my-bucket", policy="transient", region="US")

# Upload a file
obj = client.objects("my-bucket").upload("model.rvt")
print(f"Uploaded: {obj.urn}")

# Start translation
job = client.translate(obj.urn, output_format="svf2")
job = job.wait()
print(f"Translation {job.status}: {job.progress}")
```

## Features

### Authentication
- 2-legged OAuth (Client Credentials)
- Environment variable support (`APS_CLIENT_ID`, `APS_CLIENT_SECRET`)

### Object Storage (OSS)
- List, create, get, delete buckets
- Upload, download, list, delete objects
- Signed URL generation
- Multipart upload for large files

### Model Derivative
- Start translation jobs
- Check translation status
- Wait for completion with polling

### Data Management (requires 3-legged auth)
- List hubs (requires prior `raps auth login` via CLI)

## API Reference

### Client

```python
# Create from environment
client = Client.from_env()

# Create with explicit credentials
client = Client(client_id, client_secret, base_url=None)

# Test authentication
client.test_auth() -> bool

# Access managers
client.buckets -> BucketsManager
client.hubs -> HubsManager  # requires 3-legged auth
client.objects(bucket_key) -> ObjectsManager

# Translation
client.translate(urn, output_format="svf2", force=False) -> TranslationJob
client.get_translation_status(urn) -> TranslationJob
client.get_urn(bucket_key, object_key) -> str
```

### BucketsManager

```python
buckets.list(region=None, limit=None) -> List[Bucket]
buckets.create(key, policy="transient", region="US") -> Bucket
buckets.get(key) -> Bucket
buckets.delete(key) -> None
```

### ObjectsManager

```python
objects.list(limit=None) -> List[Object]
objects.upload(path, object_key=None) -> Object
objects.download(object_key, path) -> str
objects.delete(object_key) -> None
objects.signed_url(object_key, minutes=2) -> str
```

### TranslationJob

```python
job.urn: str
job.status: str  # pending, inprogress, success, failed, timeout
job.progress: str
job.wait(timeout=600, poll_interval=5) -> TranslationJob
```

## Exceptions

- `RapsError` - Base exception
- `AuthenticationError` - Authentication failures
- `NotFoundError` - Resource not found
- `RateLimitError` - API rate limit exceeded
- `ValidationError` - Invalid parameters

## Requirements

- Python 3.8+
- APS application credentials from [APS Developer Portal](https://aps.autodesk.com/myapps)

## Environment Variables

```bash
export APS_CLIENT_ID="your_client_id"
export APS_CLIENT_SECRET="your_client_secret"
```

## License

Apache 2.0

## Links

- [RAPS CLI Documentation](https://rapscli.xyz)
- [RAPS GitHub](https://github.com/dmytro-yemelianov/raps)
- [APS Developer Portal](https://aps.autodesk.com)
