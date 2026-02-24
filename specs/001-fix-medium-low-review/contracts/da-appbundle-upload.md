# Contract: DA App Bundle Upload

## Upload App Bundle Archive

**CLI**: `raps da upload-appbundle <BUNDLE_ID> <FILE_PATH> [--engine <ENGINE>] [--description <DESC>]`
**Client**: `client.upload_appbundle(upload_params: &UploadParameters, file_path: &Path) -> Result<()>`

**Workflow**:
1. User creates app bundle: `raps da create-appbundle <ID> --engine <ENGINE>`
   - Returns `AppBundleDetails` with `upload_parameters`
2. User uploads archive: `raps da upload-appbundle <ID> <FILE_PATH>`
   - CLI calls `create_appbundle()` (or `create_appbundle_version()`) to get fresh upload params
   - Builds multipart form from `upload_parameters.form_data`
   - Appends file as last form part (field name: `file`)
   - POSTs to `upload_parameters.endpoint_url` (S3 pre-signed URL)

**APS endpoint**: S3 pre-signed URL (not a direct APS endpoint)
**Auth**: Pre-signed URL contains embedded auth (no bearer token needed for upload)
**Content-Type**: `multipart/form-data`

**Request structure**:
```
POST {endpoint_url}
Content-Type: multipart/form-data

-- form_data fields (key, policy, x-amz-credential, etc.) --
-- file: archive.zip --
```

**Response**: S3 returns 200/204 on success.

**Error cases**:
- File not found: Fail before upload attempt with clear path error
- Upload URL expired: Fail with suggestion to re-create the bundle version
- File too large: S3 rejects with 400 (practical limit ~500MB)
- Network error: reqwest error with retry (existing send_with_retry pattern)

**Validation**:
- File path must exist and be readable
- File extension should be .zip (warn if not, but proceed)
- `upload_parameters` must have non-null `endpoint_url`
