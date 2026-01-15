# Quickstart: Global Output Format

## Usage

### Basic Usage
```bash
# Default (Table on TTY)
raps bucket list

# JSON Output
raps bucket list --output json

# YAML Output (New!)
raps bucket list --output yaml
```

### Automation / CI
```bash
# Automatically detects pipe -> JSON
raps bucket list | jq '.[0].bucketKey'

# Explicitly force YAML for file generation
raps da engines --output yaml > engines.yaml
```

### Configuration
You can set a default output format in your profile (future/existing feature interaction):
```bash
raps config set output yaml
```
