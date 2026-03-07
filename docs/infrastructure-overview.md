# RAPS Infrastructure Overview

This document provides an overview of RAPS deployment and integration options.

## 🎯 Deployment Methods

### 1. **Standalone Binary** (Primary)
- **Install**: `cargo install raps`
- **Platforms**: Windows, macOS, Linux (x64/ARM64)
- **Use Case**: Development, local automation, manual workflows
- **Distribution**: GitHub Releases, crates.io, Homebrew, Scoop

### 2. **Docker Image** 🐳
- **Registry**: 
  - DockerHub: `dmytroyemelianov/raps`
  - GHCR: `ghcr.io/dmytro-yemelianov/raps`
- **Architectures**: linux/amd64, linux/arm64
- **Use Case**: CI/CD pipelines, containerized workflows, cloud deployments
- **Image**: Debian-based, minimal (ca-certificates, curl)

### 3. **GitHub Action** ⚡
- **Repository**: `dmytro-yemelianov/raps-action`
- **Usage**: Add to workflows for APS automation
- **Features**: Auto-version detection, multi-platform support
- **Use Case**: GitHub-based CI/CD for APS projects

### 4. **MCP Server** 🤖
- **Command**: `raps mcp`
- **Protocol**: Model Context Protocol (stdio)
- **Clients**: Claude Desktop, Cursor, other MCP-compatible AI tools
- **Use Case**: AI-assisted APS development and automation

---

## 🌼 OAuth Callback Port Fallback (v3.4.0+)

RAPS now includes intelligent port fallback for 3-legged OAuth:

### Port Priority
1. **8080** - Default (or from `APS_CALLBACK_URL`)
2. **12495** 🌼 - RAPS leet (R=12, A=4, P=9, S=5)
3. **7495** 🌼 - RAPS alternative
4. **9247** 🌼 - RAPS variation
5. **3000** - Common dev port
6. **5000** - Common dev port

### Configuration Required
Add ALL callback URLs to your APS app at https://aps.autodesk.com/myapps:
```
http://localhost:8080/callback
http://localhost:12495/callback
http://localhost:7495/callback
http://localhost:9247/callback
http://localhost:3000/callback
http://localhost:5000/callback
```

### Why Port Fallback?
- **Windows Compatibility**: Avoids permission issues with `0.0.0.0` binding
- **Port Conflicts**: Automatically finds available port
- **Developer Experience**: No manual port configuration needed
- **Fun Factor**: RAPS-themed leet-speak ports! 🌼

---

## 📦 Distribution Channels

### Package Managers
- **Cargo**: `cargo install raps`
- **Homebrew** (macOS/Linux): `brew install dmytro-yemelianov/tap/raps`
- **Scoop** (Windows): `scoop bucket add raps https://github.com/dmytro-yemelianov/scoop-bucket`

### Container Registries
- **DockerHub**: `docker pull dmytroyemelianov/raps:latest`
- **GHCR**: `docker pull ghcr.io/dmytro-yemelianov/raps:latest`

### Marketplace
- **GitHub Actions Marketplace**: Search "RAPS" or "Autodesk Platform Services"

---

## 🔧 Integration Examples

### Docker in GitHub Actions
```yaml
jobs:
  deploy:
    runs-on: ubuntu-latest
    container:
      image: dmytroyemelianov/raps:3.4.0
    steps:
      - name: Translate Model
        env:
          APS_CLIENT_ID: ${{ secrets.APS_CLIENT_ID }}
          APS_CLIENT_SECRET: ${{ secrets.APS_CLIENT_SECRET }}
        run: |
          raps oss objects upload my-bucket model.rvt
          raps md translate $(raps oss objects info my-bucket model.rvt --urn)
```

### RAPS GitHub Action
```yaml
- uses: dmytro-yemelianov/raps-action@v1
  with:
    command: 'oss buckets list'
    client-id: ${{ secrets.APS_CLIENT_ID }}
    client-secret: ${{ secrets.APS_CLIENT_SECRET }}
```

### MCP Server (Claude Desktop)
```json
{
  "mcpServers": {
    "raps": {
      "command": "raps",
      "args": ["mcp"],
      "env": {
        "APS_CLIENT_ID": "your_client_id",
        "APS_CLIENT_SECRET": "your_client_secret"
      }
    }
  }
}
```

### Docker Compose
```yaml
version: '3.8'
services:
  raps:
    image: dmytroyemelianov/raps:3.4.0
    environment:
      APS_CLIENT_ID: ${APS_CLIENT_ID}
      APS_CLIENT_SECRET: ${APS_CLIENT_SECRET}
    volumes:
      - ./data:/data
    command: ["oss", "buckets", "list"]
```

---

## 🚀 CI/CD Platform Support

| Platform | Method | Status |
|----------|--------|--------|
| **GitHub Actions** | Docker or native action | ✅ Fully Supported |
| **GitLab CI** | Docker image | ✅ Fully Supported |
| **Azure DevOps** | Docker task | ✅ Fully Supported |
| **Jenkins** | Docker or binary | ✅ Fully Supported |
| **CircleCI** | Docker executor | ✅ Fully Supported |
| **Bitbucket Pipelines** | Docker image | ✅ Fully Supported |

---

## 📊 Current Versions

- **Latest Release**: 3.4.0
- **Docker Image**: 3.4.0
- **GitHub Action**: v1
- **Rust Edition**: 2024 (MSRV: 1.88)

---

## 🔗 Resources

- **Website**: https://rapscli.xyz
- **Documentation**: https://rapscli.xyz/docs
- **GitHub**: https://github.com/dmytro-yemelianov/raps
- **DockerHub**: https://hub.docker.com/r/dmytroyemelianov/raps
- **Crates.io**: https://crates.io/crates/raps
- **OAuth Callback Ports**: [docs/oauth-callback-ports.md](../raps/docs/oauth-callback-ports.md)

---

## 🎯 Recommended Use Cases

| Use Case | Recommended Method | Reason |
|----------|-------------------|--------|
| Local Development | Binary install | Best performance, shell completions |
| CI/CD Pipeline | Docker image | Consistent environment, version control |
| GitHub Workflows | GitHub Action | Native integration, simple syntax |
| AI-Assisted Dev | MCP Server | Natural language interface |
| Multi-platform Dev | Docker | Cross-platform consistency |
| Quick Scripts | Binary install | Fast startup, no overhead |

---

## 🔐 Security Notes

### Secrets Management
- **GitHub Actions**: Use GitHub Secrets
- **Docker**: Use environment variables or Docker secrets
- **Local**: Use `.env` file (gitignored) or config profiles

### OAuth Best Practices
- Never commit credentials to version control
- Use 2-legged OAuth for server-to-server
- Use 3-legged OAuth for user data access
- Configure all callback URLs in APS app settings
- Use token-based auth for CI/CD (avoid browser flow)

---

## 📝 Changelog Highlights (v3.4.0)

- ✅ Fixed version display (now uses `CARGO_PKG_VERSION`)
- ✅ Added OAuth callback port fallback (8080, 12495🌼, 7495🌼, 9247🌼, 3000, 5000)
- ✅ Changed callback binding from `0.0.0.0` to `127.0.0.1` (Windows compatibility)
- ✅ Improved error messages for port conflicts
- ✅ Added comprehensive documentation for OAuth callback configuration

---

*Last Updated: January 2, 2026*
