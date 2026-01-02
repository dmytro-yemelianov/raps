# OAuth Callback Ports Configuration

## RAPS Callback Port Fallback System

RAPS uses an intelligent port fallback system for the OAuth callback server. If the default port (8080) is unavailable, it will automatically try alternative ports.

### Port Priority Order

1. **8080** - Default port (or custom port from `APS_CALLBACK_URL`)
2. **12495** 🌼 - RAPS in leet speak (R=12, A=4, P=9, S=5)
3. **7495** 🌼 - RAPS alternative (7 looks like backwards R)
4. **9247** 🌼 - RAPS variation
5. **3000** - Common development port
6. **5000** - Common development port

### Configuring Your APS Application

To ensure OAuth authentication works regardless of which port is selected, you should add **all** fallback ports to your APS application's Callback URLs:

1. Go to https://aps.autodesk.com/myapps
2. Select your application
3. Add these URLs to **Callback URLs**:
   ```
   http://localhost:8080/callback
   http://localhost:12495/callback
   http://localhost:7495/callback
   http://localhost:9247/callback
   http://localhost:3000/callback
   http://localhost:5000/callback
   ```

### Custom Port

If you want to use a specific port, set the environment variable:

```powershell
# PowerShell
$env:APS_CALLBACK_URL = "http://localhost:9999/callback"
```

```bash
# Bash/Zsh
export APS_CALLBACK_URL="http://localhost:9999/callback"
```

Then add `http://localhost:9999/callback` to your APS app's Callback URLs.

### Troubleshooting

If authentication fails with port errors:

1. **Check reserved ports**:
   ```powershell
   netsh interface ipv4 show excludedportrange protocol=tcp
   ```

2. **Check what's using a port**:
   ```powershell
   netstat -ano | findstr :8080
   ```

3. **Allow through firewall** (run as Administrator):
   ```powershell
   New-NetFirewallRule -DisplayName "RAPS OAuth" -Direction Inbound -LocalPort 8080,12495,7495,9247,3000,5000 -Protocol TCP -Action Allow
   ```

### Why These Ports?

- **12495**: RAPS in leet speak (R=12, A=4, P=9, S=5) 🌼
- **7495**: Alternative with 7 for R (looks like backwards R)
- **9247**: Creative variation keeping the "APS" pattern
- **3000, 5000**: Industry-standard development ports

These uncommon ports (12495, 7495, 9247) are unlikely to conflict with other applications while being memorable for RAPS users!
