---
layout: default
title: Configuration
---

# Configuration

RAPS CLI supports multiple configuration methods: environment variables, profiles, and `.env` files. Configuration precedence is: **environment variables > active profile > defaults**.

## Required Environment Variables

### APS_CLIENT_ID

Your APS application Client ID from the [APS Developer Portal](https://aps.autodesk.com/myapps).

```powershell
# Windows PowerShell
$env:APS_CLIENT_ID = "your_client_id_here"
```

```bash
# macOS/Linux
export APS_CLIENT_ID="your_client_id_here"
```

### APS_CLIENT_SECRET

Your APS application Client Secret from the APS Developer Portal.

```powershell
# Windows PowerShell
$env:APS_CLIENT_SECRET = "your_client_secret_here"
```

```bash
# macOS/Linux
export APS_CLIENT_SECRET="your_client_secret_here"
```

## Optional Environment Variables

### APS_CALLBACK_URL

Callback URL for 3-legged OAuth. Defaults to `http://localhost:8080/callback` if not specified.

```powershell
# Windows PowerShell
$env:APS_CALLBACK_URL = "http://localhost:8080/callback"
```

```bash
# macOS/Linux
export APS_CALLBACK_URL="http://localhost:8080/callback"
```

### APS_DA_NICKNAME

Design Automation nickname (required only if using Design Automation features).

```powershell
# Windows PowerShell
$env:APS_DA_NICKNAME = "your_nickname"
```

```bash
# macOS/Linux
export APS_DA_NICKNAME="your_nickname"
```

## Profile Management (v0.4.0+)

Profiles allow you to manage multiple configurations for different environments (development, staging, production).

### Creating and Using Profiles

```bash
# Create a new profile
raps config profile create production

# Set configuration values for the active profile
raps config set client_id "your_production_client_id"
raps config set client_secret "your_production_client_secret"
raps config set base_url "https://developer.api.autodesk.com"

# Switch to a different profile
raps config profile use development

# List all profiles
raps config profile list

# Show current active profile
raps config profile current

# Delete a profile
raps config profile delete old-profile
```

### Profile Storage

Profiles are stored in:
- **Windows**: `%APPDATA%\raps\profiles.json`
- **macOS**: `~/Library/Application Support/raps/profiles.json`
- **Linux**: `~/.config/raps/profiles.json`

### Configuration Precedence

When RAPS loads configuration, it uses this order:

1. **Environment variables** (highest priority)
2. **Active profile** (if set)
3. **Defaults** (lowest priority)

This means environment variables always override profile settings, making it easy to override for specific commands.

### Example: Multi-Environment Setup

```bash
# Development profile
raps config profile create dev
raps config profile use dev
raps config set client_id "dev_client_id"
raps config set client_secret "dev_secret"

# Production profile
raps config profile create prod
raps config profile use prod
raps config set client_id "prod_client_id"
raps config set client_secret "prod_secret"

# Switch between environments
raps config profile use dev   # Use development credentials
raps config profile use prod # Use production credentials
```

## Using .env File

You can create a `.env` file in your working directory to store credentials:

```env
APS_CLIENT_ID=your_client_id_here
APS_CLIENT_SECRET=your_client_secret_here
APS_CALLBACK_URL=http://localhost:8080/callback
APS_DA_NICKNAME=your_nickname
```

RAPS will automatically load variables from `.env` files in the current directory and parent directories.

**Security Note:** Never commit `.env` files to version control. Add `.env` to your `.gitignore`.

## Making Environment Variables Permanent

### Windows PowerShell

Add to your PowerShell profile (`$PROFILE`):

```powershell
# Edit profile
notepad $PROFILE

# Add these lines:
$env:APS_CLIENT_ID = "your_client_id_here"
$env:APS_CLIENT_SECRET = "your_client_secret_here"
```

### Windows Command Prompt

Use System Properties → Environment Variables, or:

```cmd
setx APS_CLIENT_ID "your_client_id_here"
setx APS_CLIENT_SECRET "your_client_secret_here"
```

### macOS/Linux

Add to your shell configuration file (`~/.bashrc`, `~/.zshrc`, etc.):

```bash
# Edit your shell config
nano ~/.bashrc  # or ~/.zshrc

# Add these lines:
export APS_CLIENT_ID="your_client_id_here"
export APS_CLIENT_SECRET="your_client_secret_here"
```

Then reload your shell:

```bash
source ~/.bashrc  # or source ~/.zshrc
```

## Shell Completions

RAPS supports auto-completion for bash, zsh, fish, PowerShell, and elvish.

### PowerShell

```powershell
# Add to your PowerShell profile ($PROFILE)
raps completions powershell | Out-String | Invoke-Expression

# Or save to a file and source it
raps completions powershell > "$env:USERPROFILE\Documents\PowerShell\raps.ps1"
# Then add to $PROFILE: . "$env:USERPROFILE\Documents\PowerShell\raps.ps1"
```

### Bash

```bash
# Add to ~/.bashrc
eval "$(raps completions bash)"

# Or save to completions directory
raps completions bash > ~/.local/share/bash-completion/completions/raps
```

### Zsh

```zsh
# Add to ~/.zshrc (before compinit)
eval "$(raps completions zsh)"

# Or save to fpath directory
raps completions zsh > ~/.zfunc/_raps
# Add to ~/.zshrc: fpath=(~/.zfunc $fpath)
```

### Fish

```fish
# Save to completions directory
raps completions fish > ~/.config/fish/completions/raps.fish
```

### Elvish

```elvish
# Add to ~/.elvish/rc.elv
eval (raps completions elvish | slurp)
```

## Testing Configuration

After setting up your credentials, test the configuration:

```bash
# Test 2-legged authentication
raps auth test
```

If successful, you'll see a confirmation message. If not, check:

1. Environment variables are set correctly
2. Client ID and Secret are valid
3. Your APS application is active

## Token Storage

RAPS securely stores authentication tokens in platform-specific directories:

- **Windows**: `%APPDATA%\raps\` or `%LOCALAPPDATA%\raps\`
- **macOS**: `~/Library/Application Support/raps/`
- **Linux**: `~/.local/share/raps/` or `$XDG_DATA_HOME/raps/`

Tokens are automatically refreshed when they expire. You can clear stored tokens by logging out:

```bash
raps auth logout
```

## Next Steps

After configuration:

1. **[Test authentication]({{ '/commands/auth' | relative_url }})**
2. **[Create your first bucket]({{ '/commands/buckets' | relative_url }})**
3. **[Upload and translate a model]({{ '/examples' | relative_url }})**

