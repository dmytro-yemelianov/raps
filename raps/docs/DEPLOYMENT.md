# Documentation Deployment Guide

This document explains how the RAPS CLI documentation is deployed to GitHub Pages.

## Setup

The documentation is built using Jekyll and automatically deployed to GitHub Pages via GitHub Actions.

## Repository Structure

```
aps-cli/
├── docs/                    # Documentation source files
│   ├── _config.yml         # Jekyll configuration
│   ├── Gemfile             # Ruby dependencies
│   ├── index.md            # Homepage
│   ├── getting-started.md  # Getting started guide
│   ├── installation.md     # Installation instructions
│   ├── configuration.md    # Configuration guide
│   ├── commands/           # Command reference
│   ├── examples.md         # Usage examples
│   └── troubleshooting.md  # Troubleshooting guide
└── .github/
    └── workflows/
        └── docs.yml        # GitHub Actions workflow for deployment
```

## Deployment Process

1. **Automatic Deployment**: When changes are pushed to the `main` branch in the `docs/` directory, GitHub Actions automatically:
   - Builds the Jekyll site
   - Deploys it to GitHub Pages

2. **Manual Deployment**: You can also trigger deployment manually via the "Actions" tab in GitHub.

## Local Development

To preview the documentation locally:

```bash
cd docs
bundle install
bundle exec jekyll serve
```

Then open http://localhost:4000 in your browser.

## GitHub Pages Settings

1. Go to repository Settings → Pages
2. Ensure "Source" is set to "GitHub Actions"
3. The site will be available at: `https://dmytro-yemelianov.github.io/raps/`

## Updating Documentation

1. Edit markdown files in the `docs/` directory
2. Commit and push changes
3. GitHub Actions will automatically build and deploy

## Theme

The documentation uses the `jekyll-theme-minimal` theme, which provides:
- Clean, minimal design
- Responsive layout
- Automatic navigation

## Customization

To customize the documentation:

- **Theme**: Edit `_config.yml` to change the theme
- **Navigation**: Update the `navigation` section in `_config.yml`
- **Styling**: Add custom CSS files if needed

## Troubleshooting

### Build Failures

If the GitHub Actions build fails:
1. Check the Actions tab for error messages
2. Verify Jekyll configuration in `_config.yml`
3. Test locally with `bundle exec jekyll build`

### Pages Not Updating

If changes aren't appearing:
1. Check GitHub Actions workflow completed successfully
2. Clear browser cache
3. Wait a few minutes for DNS propagation

