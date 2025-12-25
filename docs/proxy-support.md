---
layout: default
title: Proxy Support
---

# Proxy Support

RAPS CLI supports HTTP/HTTPS proxy configuration through standard environment variables. This is essential for corporate networks and environments behind firewalls.

## Quick Start

Set the proxy environment variables before running RAPS CLI:

**Windows PowerShell:**
```powershell
$env:HTTP_PROXY = "http://proxy.company.com:8080"
$env:HTTPS_PROXY = "http://proxy.company.com:8080"
$env:NO_PROXY = "localhost,127.0.0.1,.local"
```

**macOS/Linux:**
```bash
export HTTP_PROXY="http://proxy.company.com:8080"
export HTTPS_PROXY="http://proxy.company.com:8080"
export NO_PROXY="localhost,127.0.0.1,.local"
```

## Environment Variables

RAPS CLI uses the standard proxy environment variables that are automatically detected by the underlying HTTP client (reqwest):

### HTTP_PROXY

HTTP proxy server URL for non-secure connections.

**Format:** `http://[username:password@]host[:port]`

**Examples:**
```bash
# Basic proxy
export HTTP_PROXY="http://proxy.company.com:8080"

# Proxy with authentication
export HTTP_PROXY="http://user:pass@proxy.company.com:8080"

# SOCKS5 proxy (if supported)
export HTTP_PROXY="socks5://proxy.company.com:1080"
```

### HTTPS_PROXY

HTTPS proxy server URL for secure connections. If not set, `HTTP_PROXY` will be used for HTTPS connections as well.

**Format:** `http://[username:password@]host[:port]`

**Examples:**
```bash
# HTTPS proxy
export HTTPS_PROXY="http://proxy.company.com:8080"

# Different proxy for HTTPS
export HTTPS_PROXY="http://secure-proxy.company.com:8443"
```

### NO_PROXY

Comma-separated list of hostnames or IP addresses that should bypass the proxy.

**Format:** `host1,host2,host3` or `*.domain.com`

**Examples:**
```bash
# Bypass localhost and internal domains
export NO_PROXY="localhost,127.0.0.1,.local,.company.internal"

# Bypass specific hosts
export NO_PROXY="api.internal.com,192.168.1.0/24"
```

**Common patterns:**
- `localhost` - Local machine
- `127.0.0.1` - Loopback address
- `.local` - All `.local` domains
- `.company.internal` - All internal company domains
- `192.168.0.0/16` - Private network ranges

## Configuration Methods

### Method 1: Environment Variables (Recommended)

Set variables in your shell session:

**PowerShell:**
```powershell
$env:HTTP_PROXY = "http://proxy.company.com:8080"
$env:HTTPS_PROXY = "http://proxy.company.com:8080"
$env:NO_PROXY = "localhost,127.0.0.1"
```

**Bash/Zsh:**
```bash
export HTTP_PROXY="http://proxy.company.com:8080"
export HTTPS_PROXY="http://proxy.company.com:8080"
export NO_PROXY="localhost,127.0.0.1"
```

### Method 2: .env File

Add to your `.env` file:

```env
HTTP_PROXY=http://proxy.company.com:8080
HTTPS_PROXY=http://proxy.company.com:8080
NO_PROXY=localhost,127.0.0.1,.local
```

### Method 3: System-Wide Configuration

**Windows:**
```cmd
setx HTTP_PROXY "http://proxy.company.com:8080"
setx HTTPS_PROXY "http://proxy.company.com:8080"
setx NO_PROXY "localhost,127.0.0.1"
```

**macOS/Linux:**
Add to `~/.bashrc` or `~/.zshrc`:
```bash
export HTTP_PROXY="http://proxy.company.com:8080"
export HTTPS_PROXY="http://proxy.company.com:8080"
export NO_PROXY="localhost,127.0.0.1,.local"
```

## Testing Proxy Configuration

Test if proxy is working:

```bash
# Test authentication (will use proxy if configured)
raps auth test

# List buckets (will use proxy for API calls)
raps bucket list
```

If proxy is configured correctly, requests will go through the proxy. If not, you'll see connection errors.

## Troubleshooting

### "Connection refused" or "Network error"

1. **Verify proxy URL is correct:**
   ```bash
   # Test proxy connectivity
   curl -x http://proxy.company.com:8080 https://developer.api.autodesk.com
   ```

2. **Check proxy requires authentication:**
   ```bash
   # Add credentials to proxy URL
   export HTTP_PROXY="http://username:password@proxy.company.com:8080"
   ```

3. **Verify NO_PROXY settings:**
   ```bash
   # Check if target domain is in NO_PROXY
   echo $NO_PROXY
   ```

### TLS/SSL Certificate Errors

Corporate proxies often intercept HTTPS traffic, which can cause certificate validation errors.

**Option 1: Add proxy CA certificate to system trust store**

**Windows:**
```powershell
# Import certificate
Import-Certificate -FilePath proxy-ca.crt -CertStoreLocation Cert:\LocalMachine\Root
```

**macOS:**
```bash
# Import certificate
sudo security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain proxy-ca.crt
```

**Linux:**
```bash
# Copy certificate to system trust store
sudo cp proxy-ca.crt /usr/local/share/ca-certificates/
sudo update-ca-certificates
```

**Option 2: Disable certificate verification (NOT RECOMMENDED for production)**

⚠️ **Security Warning:** Only use this for testing in controlled environments.

Set environment variable (reqwest respects this):
```bash
export RUSTLS_UNVERIFIED_CLIENT="1"  # Not recommended
```

### Proxy Authentication Issues

If your proxy requires authentication:

1. **Include credentials in proxy URL:**
   ```bash
   export HTTP_PROXY="http://username:password@proxy.company.com:8080"
   ```

2. **Use URL encoding for special characters:**
   ```bash
   # Password with special characters
   export HTTP_PROXY="http://user:p%40ssw0rd@proxy.company.com:8080"
   ```

3. **Check if proxy supports NTLM/Kerberos:**
   Some proxies require NTLM or Kerberos authentication, which may need additional configuration.

### Proxy Not Being Used

If requests aren't going through the proxy:

1. **Verify environment variables are set:**
   ```bash
   # Check variables
   echo $HTTP_PROXY
   echo $HTTPS_PROXY
   ```

2. **Check NO_PROXY doesn't exclude target:**
   ```bash
   # Target domain should NOT be in NO_PROXY
   echo $NO_PROXY
   ```

3. **Restart terminal/shell:**
   Environment variables are only available in the current shell session.

## Corporate Network Considerations

### Firewall Rules

Ensure these domains are allowed through your corporate firewall:
- `developer.api.autodesk.com` - APS API endpoints
- `api.userprofile.autodesk.com` - User profile API
- `oss.autodesk.com` - Object Storage Service
- `modelderivative.autodesk.com` - Model Derivative API
- `webhooks.autodesk.com` - Webhooks API
- `developer.api.autodesk.com/da/us-east/v3` - Design Automation API

### Proxy Authentication

Many corporate proxies require authentication. Use one of these methods:

1. **Username/Password in URL:**
   ```bash
   export HTTP_PROXY="http://DOMAIN\\username:password@proxy.company.com:8080"
   ```

2. **NTLM Authentication:**
   May require additional configuration depending on your proxy setup.

### Certificate Pinning

Some corporate proxies use certificate pinning. You may need to:
1. Export the proxy's CA certificate
2. Add it to your system's trust store
3. Ensure RAPS CLI uses the system trust store (default behavior)

## Examples

### Basic Corporate Proxy

```bash
export HTTP_PROXY="http://proxy.company.com:8080"
export HTTPS_PROXY="http://proxy.company.com:8080"
export NO_PROXY="localhost,127.0.0.1,.local,.company.internal"

raps auth test
```

### Proxy with Authentication

```bash
export HTTP_PROXY="http://user:pass@proxy.company.com:8080"
export HTTPS_PROXY="http://user:pass@proxy.company.com:8080"

raps bucket list
```

### Bypass Proxy for Internal APIs

```bash
export HTTP_PROXY="http://proxy.company.com:8080"
export NO_PROXY="localhost,127.0.0.1,*.internal.company.com,192.168.0.0/16"

raps hub list
```

## CI/CD Integration

### GitHub Actions

```yaml
env:
  HTTP_PROXY: http://proxy.company.com:8080
  HTTPS_PROXY: http://proxy.company.com:8080
  NO_PROXY: localhost,127.0.0.1

steps:
  - name: List buckets
    run: raps bucket list
```

### Azure DevOps

```yaml
variables:
  HTTP_PROXY: 'http://proxy.company.com:8080'
  HTTPS_PROXY: 'http://proxy.company.com:8080'
  NO_PROXY: 'localhost,127.0.0.1'

steps:
  - script: raps bucket list
```

### GitLab CI

```yaml
variables:
  HTTP_PROXY: "http://proxy.company.com:8080"
  HTTPS_PROXY: "http://proxy.company.com:8080"
  NO_PROXY: "localhost,127.0.0.1"

test:
  script:
    - raps bucket list
```

## Notes

- RAPS CLI uses the `reqwest` HTTP client, which automatically detects and uses standard proxy environment variables
- Proxy settings apply to all HTTP/HTTPS requests made by RAPS CLI
- Authentication tokens and credentials are never sent to the proxy (only HTTP headers)
- Proxy configuration does not affect local file operations or token storage

## Related Documentation

- [Configuration Guide](configuration)
- [Troubleshooting](troubleshooting)

