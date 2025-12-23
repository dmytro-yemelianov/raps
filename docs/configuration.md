---
layout: default
title: Configuration
---

# Configuration

RAPS CLI uses environment variables for configuration. You can set them in your shell session or use a `.env` file.

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

