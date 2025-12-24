# Verifying Release Checksums

RAPS CLI releases include SHA256 checksums to verify download integrity and detect tampering.

## Downloading Checksums

Checksums are published as `checksums.txt` alongside release artifacts on GitHub.

## Verifying on Windows (PowerShell)

```powershell
# Download the checksum file
Invoke-WebRequest -Uri "https://github.com/dmytro-yemelianov/raps/releases/download/v0.4.0/checksums.txt" -OutFile "checksums.txt"

# Calculate hash of downloaded file
$hash = Get-FileHash -Path "raps-x86_64-pc-windows-msvc.zip" -Algorithm SHA256

# Compare with checksums.txt
Select-String -Path "checksums.txt" -Pattern $hash.Hash
```

## Verifying on macOS/Linux

```bash
# Download the checksum file
wget https://github.com/dmytro-yemelianov/raps/releases/download/v0.4.0/checksums.txt

# Verify checksums
sha256sum -c checksums.txt
```

Or using `shasum` on macOS:

```bash
# Verify checksums
shasum -a 256 -c checksums.txt
```

## What to Check

1. **File Integrity**: The checksum verifies the file wasn't corrupted during download
2. **Authenticity**: Compare the checksum with the one published on GitHub releases
3. **Tampering Detection**: If checksums don't match, the file may have been modified

## Security Best Practices

- Always verify checksums before installing binaries
- Download checksums from the official GitHub releases page
- Use HTTPS when downloading files
- Store checksums securely for audit purposes

