---
layout: default
title: Troubleshooting
---

# Troubleshooting

Common issues and solutions when using RAPS CLI.

## Authentication Issues

### "Authentication failed" Error

**Symptoms:**
```
Error: Authentication failed
```

**Solutions:**
1. Verify `APS_CLIENT_ID` and `APS_CLIENT_SECRET` are set correctly:
   ```bash
   # Check environment variables
   echo $APS_CLIENT_ID
   echo $APS_CLIENT_SECRET
   ```

2. Verify credentials in APS Developer Portal:
   - Go to [APS Developer Portal](https://aps.autodesk.com/myapps)
   - Check that your application is active
   - Verify Client ID and Secret match

3. Check for typos or extra spaces in credentials

4. Try logging out and testing again:
   ```bash
   raps auth logout
   raps auth test
   ```

### "Callback URL mismatch" Error

**Symptoms:**
```
Error: Callback URL mismatch
```

**Solutions:**
1. Verify `APS_CALLBACK_URL` matches your APS application settings:
   ```bash
   # Check current callback URL
   echo $APS_CALLBACK_URL
   ```

2. Default callback URL is `http://localhost:8080/callback`

3. Update APS application settings to match your callback URL

4. For production, use HTTPS:
   ```bash
   export APS_CALLBACK_URL="https://your-domain.com/callback"
   ```

### "Token expired" Error

**Symptoms:**
```
Error: Token expired
```

**Solutions:**
1. Tokens are automatically refreshed, but if you see this error:
   ```bash
   # Logout and login again
   raps auth logout
   raps auth login
   ```

2. Check your system clock is synchronized:
   ```bash
   # Windows
   w32tm /resync
   
   # macOS/Linux
   sudo ntpdate -s time.nist.gov
   ```

## Bucket and Object Issues

### "Bucket already exists" Error

**Symptoms:**
```
Error: Bucket already exists
```

**Solutions:**
1. Bucket keys must be globally unique across all APS applications
2. Use a unique prefix:
   ```bash
   raps bucket create --key "my-unique-prefix-$(date +%s)-bucket"
   ```

3. List existing buckets:
   ```bash
   raps bucket list
   ```

### "Bucket not found" Error

**Symptoms:**
```
Error: Bucket not found
```

**Solutions:**
1. Verify bucket key is correct:
   ```bash
   raps bucket list
   ```

2. Check bucket exists in the correct region:
   ```bash
   raps bucket info <bucket-key>
   ```

3. Ensure you're using the correct credentials (buckets are per-application)

### "File not found" Error

**Symptoms:**
```
Error: File not found: model.dwg
```

**Solutions:**
1. Verify file path is correct:
   ```bash
   ls -la model.dwg  # macOS/Linux
   dir model.dwg     # Windows
   ```

2. Use absolute path if relative path doesn't work:
   ```bash
   raps object upload my-bucket /full/path/to/model.dwg
   ```

3. Check file permissions:
   ```bash
   chmod 644 model.dwg  # macOS/Linux
   ```

## Translation Issues

### Translation Stuck in "pending" Status

**Symptoms:**
Translation status shows "pending" for a long time

**Solutions:**
1. Large files take longer to process (10+ minutes is normal)
2. Check status periodically:
   ```bash
   raps translate status <urn>
   ```

3. Use `--wait` flag to monitor:
   ```bash
   raps translate status <urn> --wait
   ```

4. If stuck for >30 minutes, the translation may have failed

### Translation Failed

**Symptoms:**
```
Status: failed
```

**Solutions:**
1. Verify file format is supported:
   - Supported: DWG, DXF, RVT, NWD, FBX, OBJ, STL, STEP, IGES, IFC

2. Check file isn't corrupted:
   ```bash
   # Try opening file in native application
   ```

3. Verify file was fully uploaded:
   ```bash
   raps object list <bucket>
   ```

4. Try a different output format:
   ```bash
   raps translate start <urn> --format obj
   ```

5. Check file size (max 5GB per file)

### "Invalid URN" Error

**Symptoms:**
```
Error: Invalid URN
```

**Solutions:**
1. URN must be base64-encoded
2. Get URN from upload output:
   ```bash
   raps object upload my-bucket model.dwg
   # URN is displayed in output
   ```

3. Verify URN format:
   ```
   urn:adsk.objects:os.object:bucket-key/object-key
   ```

## Data Management Issues

### "Authentication required" Error

**Symptoms:**
```
Error: Authentication required (3-legged OAuth)
```

**Solutions:**
1. Data Management commands require 3-legged OAuth:
   ```bash
   raps auth login
   ```

2. Ensure you selected the correct scopes:
   - `data:read` for listing/browsing
   - `data:write` for creating/updating

3. Check authentication status:
   ```bash
   raps auth status
   ```

### "Project not found" Error

**Symptoms:**
```
Error: Project not found
```

**Solutions:**
1. Verify project ID format:
   - For Data Management: Use `b.project123` format
   - For Issues: Use `project123` format (without "b." prefix)

2. List projects to get correct ID:
   ```bash
   raps project list <hub-id>
   ```

3. Ensure you have access to the project

### "Permission denied" Error

**Symptoms:**
```
Error: Permission denied
```

**Solutions:**
1. Check you have the required scopes:
   ```bash
   raps auth status
   ```

2. Login with appropriate scopes:
   ```bash
   raps auth login
   # Select data:write or data:create scopes
   ```

3. Verify you have permissions in BIM 360/ACC project

## Design Automation Issues

### "Nickname required" Error

**Symptoms:**
```
Error: Design Automation nickname required
```

**Solutions:**
1. Set `APS_DA_NICKNAME` environment variable:
   ```bash
   export APS_DA_NICKNAME="your-nickname"
   ```

2. Add to `.env` file:
   ```env
   APS_DA_NICKNAME=your-nickname
   ```

3. Nickname must be unique across all APS applications

### "Engine not found" Error

**Symptoms:**
```
Error: Engine not found
```

**Solutions:**
1. List available engines:
   ```bash
   raps da engines
   ```

2. Use exact engine ID format:
   ```
   Autodesk.AutoCAD+24
   Autodesk.Revit+2024
   ```

3. Check engine availability in your region

## Webhook Issues

### Webhook Not Receiving Events

**Symptoms:**
Webhook status shows "active" but no events received

**Solutions:**
1. Verify callback URL is publicly accessible:
   ```bash
   curl https://your-server.com/webhook
   ```

2. Check webhook status:
   ```bash
   raps webhook list
   ```

3. Ensure endpoint returns 200 OK:
   ```javascript
   app.post('/webhook', (req, res) => {
     // Process webhook
     res.status(200).send('OK');
   });
   ```

4. Check server logs for errors

5. Use HTTPS for production (required)

### Webhook Status Shows "inactive"

**Symptoms:**
Webhook status is "inactive"

**Solutions:**
1. Verify endpoint is responding:
   ```bash
   curl -X POST https://your-server.com/webhook
   ```

2. Check endpoint returns 200 OK quickly (< 5 seconds)

3. Ensure HTTPS is used (required for production)

4. Delete and recreate webhook:
   ```bash
   raps webhook delete <hook-id> --system data --event dm.version.added
   raps webhook create --url https://your-server.com/webhook --event dm.version.added
   ```

## General Issues

### Command Not Found

**Symptoms:**
```
raps: command not found
```

**Solutions:**
1. Verify RAPS is installed:
   ```bash
   which raps  # macOS/Linux
   where.exe raps  # Windows
   ```

2. Add to PATH:
   ```bash
   # macOS/Linux
   export PATH=$PATH:/path/to/raps
   
   # Windows PowerShell
   $env:PATH += ";C:\path\to\raps"
   ```

3. Reinstall if necessary:
   ```bash
   cargo install raps  # From crates.io
   ```

### Slow Performance

**Symptoms:**
Commands are slow or timing out

**Solutions:**
1. Check network connection
2. Verify APS service status
3. Large files take longer - be patient
4. Use `--wait` flags for long-running operations

### Environment Variables Not Loading

**Symptoms:**
Commands fail with "missing credentials" error

**Solutions:**
1. Verify variables are set:
   ```bash
   echo $APS_CLIENT_ID
   echo $APS_CLIENT_SECRET
   ```

2. Use `.env` file in current directory:
   ```env
   APS_CLIENT_ID=your_id
   APS_CLIENT_SECRET=your_secret
   ```

3. Reload shell after setting variables:
   ```bash
   source ~/.bashrc  # or ~/.zshrc
   ```

## Getting Help

If you're still experiencing issues:

1. **Check Documentation**: Review command-specific documentation
2. **Verify Setup**: Run `raps auth test` to verify authentication
3. **Check Logs**: Look for detailed error messages
4. **GitHub Issues**: Report issues at [GitHub Repository](https://github.com/dmytro-yemelianov/raps/issues)
5. **APS Support**: Contact APS support for API-related issues

## Related Documentation

- [Getting Started](getting-started.md) - Setup guide
- [Configuration](configuration.md) - Configuration options
- [Command Reference](commands/auth.md) - Complete command documentation

