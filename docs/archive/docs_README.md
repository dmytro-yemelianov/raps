# RAPS CLI Documentation

This directory contains the user documentation for RAPS CLI, deployed via GitHub Pages.

## Local Development

**Note:** Local testing is optional. GitHub Actions will automatically build and deploy your documentation when you push changes.

### Quick Start (No Local Setup Required)

1. Edit markdown files in the `docs/` directory
2. Commit and push to GitHub
3. GitHub Actions will build and deploy automatically
4. View at: `https://dmytro-yemelianov.github.io/raps/`

### Local Testing (Optional)

If you want to preview locally before pushing:

**Windows:**
- See `LOCAL_SETUP.md` for Ruby installation instructions
- Or skip local testing and use GitHub Actions

**macOS/Linux:**
```bash
cd docs
bundle install
bundle exec jekyll serve
```

Then open http://localhost:4000 in your browser.

## Structure

- `index.md` - Homepage
- `getting-started.md` - Getting started guide
- `installation.md` - Installation instructions
- `configuration.md` - Configuration guide
- `commands/` - Command reference documentation
- `examples.md` - Usage examples
- `troubleshooting.md` - Troubleshooting guide

## Deployment

Documentation is automatically deployed to GitHub Pages when changes are pushed to the `main` branch in the `docs/` directory.

The GitHub Actions workflow (`.github/workflows/docs.yml`) handles building and deploying the site.

