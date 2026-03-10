---
name: updating-raps-website
version: "1.0"
description: Use when a new RAPS version is released or a major feature ships and the raps-website repo needs updating — release entries, docs pages, blog posts, stats.
---

# Updating raps-website

Update the RAPS documentation website after releases or major features.

**Repo:** `/root/github/raps/raps-website` (Astro + MDX + Tailwind)

## What to Update

| File | When | Purpose |
|------|------|---------|
| `src/data/releases.js` | Every release | Changelog, homepage badge, version refs |
| `src/content/docs/en/*.mdx` | New/changed features | CLI documentation |
| `src/content/blog/en/*.mdx` | Major features | Technical blog posts |
| `src/pages/index.astro:57-62` | Stats change | Homepage stats (APIs, commands, modes) |

## releases.js Format

Latest version MUST be first in the array. Fields:

```js
{
  version: '4.16.0',
  date: '2026-03-01',           // YYYY-MM-DD
  highlights: ['Feature1', 'Feature2', 'Feature3', 'Feature4'],  // exactly 4
  description: 'One-line summary of the release',
  type: 'minor',                // 'major' | 'minor' | 'patch'
  changes: {
    added: ['Feature description'],
    changed: ['Change description'],
    fixed: ['Fix description'],
    security: ['Security fix'],  // optional
  },
}
```

## Docs Page Frontmatter (MDX)

```yaml
---
title: "Page Title"
description: "SEO description"
section: "guides"        # getting-started | modes | commands | guides | cookbook
order: 2                 # sort order within section
icon: "📋"               # optional emoji
---
```

File location: `src/content/docs/en/<slug>.mdx` (+ `uk/` for Ukrainian translation).

## Blog Post Frontmatter (MDX)

```yaml
---
title: "Post Title"
description: "SEO description"
pubDate: 2026-03-01
author: "RAPS Team"
tags: ["ci-cd", "automation", "pipelines"]
featured: true
draft: false
---
```

File location: `src/content/blog/en/<slug>.mdx`.

Supports Astro component imports. Use `className` not `class` in JSX.

## Homepage Stats (index.astro:57-62)

```js
const stats = [
  { value: '15+', label: t('home.stats.apis') },
  { value: '95+', label: t('home.stats.commands') },
  { value: '7', label: t('home.stats.modes') },
  { value: '0', label: t('home.stats.deps') },
];
```

Update values when command count, API count, or mode count changes.

## Checklist

1. Add release entry to `src/data/releases.js` (first position)
2. Update docs pages if features changed (e.g., `pipelines.mdx` for Pipeline v2)
3. Add blog post if major feature warrants it
4. Update homepage stats if counts changed
5. Run `npm run build` to verify no Astro/MDX errors
6. Commit and push to main (auto-deploys via GitHub Actions)

## Common Mistakes

- Forgetting to escape `{` and `<` in MDX (use `&#123;` or `{'${{ secrets.X }}'}` for GitHub Actions syntax)
- Putting new release entry at end instead of beginning of array
- Using `class` instead of `className` in MDX JSX elements
