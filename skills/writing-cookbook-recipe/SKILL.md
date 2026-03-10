---
name: writing-cookbook-recipe
version: "1.0"
description: Use when writing a new cookbook recipe for the RAPS website — covers the MDX frontmatter format, section structure, category taxonomy, and Tailwind styling patterns for workflow visualizations.
---

# Writing a Cookbook Recipe

Create step-by-step workflow recipes for the RAPS documentation website.

**Repo:** `/root/github/raps/raps-website`
**Location:** `src/content/cookbook/en/<slug>.mdx`

## Frontmatter

```yaml
---
title: "Recipe Title"
description: "What this recipe automates"
category: "construction"      # aec | construction | manufacturing | media
order: 40                     # sort order within category
icon: "📋"
---
```

## Categories

| Category | Scope | Examples |
|----------|-------|---------|
| `aec` | Architecture, Engineering, Construction | IFC workflows, coordination, metadata |
| `construction` | ACC/BIM 360 | Admin, issues, RFIs, checklists, submittals |
| `manufacturing` | Manufacturing Cloud | BOMs, drawings, catalogs |
| `media` | Media & Entertainment | Animation, materials, photogrammetry |

## Content Structure

1. **H1 Title** — same as frontmatter title
2. **Intro paragraph** — what the recipe does and why
3. **Workflow Overview** — visual diagram using Tailwind divs
4. **CLI Approach** — bash code blocks with real `raps` commands
5. **Pipeline Automation** — YAML pipeline example
6. **CI/CD Integration** — GitHub Actions workflow example
7. **Troubleshooting** — common errors and fixes
8. **Related** — links to other recipes and docs

## Workflow Overview Visual Pattern

```mdx
<div className="not-prose mb-8">
  <div className="flex flex-col sm:flex-row gap-4 justify-center items-center">
    <div className="bg-blue-900/20 border border-blue-700/50 rounded-lg p-4 text-center">
      <div className="text-2xl mb-2">Upload</div>
      <div className="font-semibold">Upload</div>
      <div className="text-sm text-gray-400">Send models to OSS</div>
    </div>
    <div className="text-2xl">-></div>
    <div className="bg-purple-900/20 border border-purple-700/50 rounded-lg p-4 text-center">
      <div className="text-2xl mb-2">Translate</div>
      <div className="font-semibold">Translate</div>
      <div className="text-sm text-gray-400">Convert to SVF2</div>
    </div>
  </div>
</div>
```

## Code Block Pattern

````mdx
```bash
# Upload model to OSS
raps object upload my-bucket model.rvt

# Start translation
URN=$(raps object urn my-bucket model.rvt)
raps translate start "$URN" --format svf2 --wait
```
````

## i18n

Create parallel file in `src/content/cookbook/uk/<same-slug>.mdx` with translated frontmatter. Content can remain in English as fallback.

## Checklist

1. Create `src/content/cookbook/en/<slug>.mdx` with frontmatter
2. Write sections: overview, CLI, pipeline, CI/CD, troubleshooting, related
3. Use `className` not `class` in JSX divs
4. Create Ukrainian stub in `uk/`
5. Run `npm run build` to verify
6. Commit and push
