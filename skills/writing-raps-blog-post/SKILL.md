---
name: writing-raps-blog-post
version: "1.0"
description: Use when writing a technical blog post for the RAPS website — covers MDX frontmatter, Astro component imports, series linking, i18n stub creation, and common escaping pitfalls.
---

# Writing a RAPS Blog Post

Create technical blog posts for the RAPS documentation website.

**Repo:** `/root/github/raps/raps-website`
**Location:** `src/content/blog/en/<slug>.mdx`

## Frontmatter

```yaml
---
title: "Post Title"
description: "SEO description (150-160 chars)"
pubDate: 2026-03-01
author: "Dmytro Yemelianov"
tags: ["ci-cd", "automation", "pipelines"]
featured: true               # Shows on homepage
series: "DevOps for Design"  # Optional — groups related posts
seriesOrder: 4               # Optional — order within series
draft: false
---
```

## Content Structure

1. **Hook** — Problem statement or attention-grabbing scenario
2. **Problem** — What's painful, with specifics (numbers, steps, time wasted)
3. **Solution** — How RAPS solves it, with code examples
4. **Code blocks** — Real `raps` commands, YAML pipelines, or GitHub Actions workflows
5. **Results** — Before/after comparison, metrics if possible
6. **Call to action** — Link to docs, GitHub, or related posts

## MDX Escaping Rules

| Pattern | Problem | Solution |
|---------|---------|----------|
| `{variable}` in prose | Parsed as JSX expression | Use `&#123;variable&#125;` |
| `${{ secrets.X }}` | GitHub Actions syntax | Use `{'${{ secrets.X }}'}` |
| `<100MB` | Looks like JSX tag | Write "under 100MB" |
| `{...}` outside code blocks | JSX expression | Use HTML entities |

Inside fenced code blocks (` ``` `) these are safe — no escaping needed.

## Component Imports

```mdx
import PerformanceChart from '@/components/PerformanceChart.astro';
```

Use `className` not `class` in JSX elements. Custom divs for callouts:

```mdx
<div className="not-prose mb-6 p-4 bg-green-50 dark:bg-green-900/20 rounded-lg border border-green-200 dark:border-green-800">
  <p className="text-sm text-green-800 dark:text-green-200">
    <strong>Note:</strong> Content here
  </p>
</div>
```

## i18n

English is primary. Ukrainian stubs go in `src/content/blog/uk/<same-slug>.mdx` with translated frontmatter. Content can remain in English as fallback.

## Tags Used

`ci-cd`, `automation`, `pipelines`, `github-actions`, `design-automation`, `revit`, `devops`, `mcp`, `ai`, `claude`, `cli`, `tutorial`, `installation`, `beginners`, `devcon`, `series`, `security`, `testing`

## Checklist

1. Create `src/content/blog/en/<slug>.mdx` with frontmatter
2. Write content with proper MDX escaping
3. Optionally create Ukrainian stub in `uk/`
4. Run `npm run build` to verify no MDX/Astro errors
5. Commit and push (auto-deploys via GitHub Actions)
