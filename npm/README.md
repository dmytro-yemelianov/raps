# @anthropic-ai/raps-cli

RAPS (rapeseed) - Rust CLI for Autodesk Platform Services

## Installation

```bash
npm install -g @anthropic-ai/raps-cli
```

Or run without installing:

```bash
npx @anthropic-ai/raps-cli --version
```

## Usage

```bash
# Test authentication
raps auth test

# List buckets
raps bucket list

# Upload a file
raps object upload my-bucket model.rvt

# Translate to SVF2
raps translate start urn:... --format svf2
```

## Documentation

Full documentation available at [rapscli.xyz](https://rapscli.xyz)

## Alternative Installation Methods

- **pip**: `pip install raps`
- **Homebrew**: `brew install dmytro-yemelianov/raps/raps`
- **Scoop**: `scoop install raps`
- **Shell script**: `curl -fsSL https://raw.githubusercontent.com/dmytro-yemelianov/raps/main/install.sh | bash`

## Supported Platforms

- Windows x64
- macOS x64 (Intel)
- macOS ARM64 (Apple Silicon)
- Linux x64
- Linux ARM64

## License

Apache-2.0
