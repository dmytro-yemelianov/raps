# Security Policy

## Supported Versions

We release patches for security vulnerabilities. Which versions are eligible for receiving such patches depends on the CVSS v3.0 Rating:

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| < 0.2.0 | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability, please **do not** open a public issue. Instead, please report it via one of the following methods:

### Preferred Method: GitHub Security Advisories

1. Go to the [Security tab](https://github.com/dmytro-yemelianov/raps/security) in the repository
2. Click "Report a vulnerability"
3. Fill out the security advisory form with details about the vulnerability

### Alternative: Email

If you prefer not to use GitHub Security Advisories, you can email security concerns to:
- **Email**: [security@autodesk.com](mailto:security@autodesk.com)
- **Subject**: `[APS CLI Security]` followed by a brief description

## What to Include

When reporting a vulnerability, please include:

- **Description**: A clear description of the vulnerability
- **Impact**: The potential impact of the vulnerability
- **Steps to Reproduce**: Detailed steps to reproduce the issue
- **Affected Versions**: Which versions are affected
- **Suggested Fix**: If you have a suggestion for fixing the issue (optional)

## Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Resolution**: Depends on severity and complexity

## Security Best Practices

When using APS CLI:

1. **Never commit credentials**: Keep your `APS_CLIENT_ID` and `APS_CLIENT_SECRET` secure
2. **Use environment variables**: Prefer `.env` files or environment variables over hardcoding
3. **Rotate credentials**: Regularly rotate your APS application credentials
4. **Review permissions**: Only grant necessary scopes/permissions to your APS application
5. **Keep updated**: Use the latest version of APS CLI
6. **Secure token storage**: The CLI stores tokens securely in platform-specific directories, but be aware of file permissions

## Scope

This security policy applies to:

- The APS CLI codebase
- Dependencies managed by this project
- GitHub Actions workflows

## Out of Scope

The following are considered out of scope:

- Issues related to Autodesk Platform Services APIs themselves (report to [APS Support](https://aps.autodesk.com/support))
- Social engineering attacks
- Denial of service attacks
- Issues requiring physical access to the user's machine

## Recognition

We appreciate responsible disclosure. With your permission, we would like to recognize security researchers who help keep APS CLI secure. Please let us know if you'd like to be credited in security advisories.

Thank you for helping keep APS CLI and its users safe!

