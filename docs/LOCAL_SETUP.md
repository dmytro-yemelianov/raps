# Local Documentation Setup (Windows)

This guide explains how to set up the documentation locally on Windows for testing before deployment.

## Option 1: Skip Local Testing (Recommended)

You don't need to install Ruby locally! GitHub Actions will automatically build and deploy your documentation when you push changes. Simply:

1. Edit markdown files in the `docs/` directory
2. Commit and push to GitHub
3. GitHub Actions will build and deploy automatically

## Option 2: Install Ruby for Local Testing

If you want to preview the documentation locally before pushing:

### Install Ruby on Windows

1. **Download RubyInstaller:**
   - Visit: https://rubyinstaller.org/
   - Download Ruby+Devkit 3.1.x (recommended)
   - Run the installer

2. **Verify Installation:**
   ```powershell
   ruby --version
   gem --version
   ```

3. **Install Bundler:**
   ```powershell
   gem install bundler
   ```

4. **Install Jekyll Dependencies:**
   ```powershell
   cd docs
   bundle install
   ```

5. **Run Local Server:**
   ```powershell
   bundle exec jekyll serve
   ```

6. **View Documentation:**
   - Open http://localhost:4000 in your browser

### Troubleshooting

**If `bundle install` fails:**
- Make sure you installed Ruby+Devkit (not just Ruby)
- Run the MSYS2 installer that comes with Ruby+Devkit
- Restart PowerShell after installation

**If Jekyll fails to start:**
- Ensure all dependencies installed: `bundle install`
- Check Ruby version: `ruby --version` (should be 3.1.x or compatible)

## Alternative: Use Docker

If you prefer not to install Ruby directly:

```powershell
# Using Docker (if you have Docker Desktop installed)
docker run --rm -it -v ${PWD}:/srv/jekyll -p 4000:4000 jekyll/jekyll:latest jekyll serve
```

Then open http://localhost:4000

## Recommended Workflow

For most users, we recommend:

1. **Edit documentation** in your editor
2. **Commit and push** to GitHub
3. **Let GitHub Actions** build and deploy
4. **View the live site** at: `https://dmytro-yemelianov.github.io/raps/`

This avoids local setup complexity while still allowing you to edit and deploy documentation easily.

