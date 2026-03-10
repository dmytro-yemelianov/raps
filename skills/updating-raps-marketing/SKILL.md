---
name: updating-raps-marketing
version: "1.0"
description: Use when new RAPS features or releases need reflecting in raps-marketing — press releases, campaign updates, DevCon session content, feature lists.
---

# Updating raps-marketing

Update marketing materials after releases or major features.

**Repo:** `/root/github/raps/raps-marketing` (Astro content collections)
**Schema:** `src/content/config.ts` defines all collection schemas.

## Content Collections

| Collection | Path | When to Update |
|-----------|------|---------------|
| `press/press-releases/` | Major releases | New press release |
| `campaigns/` | Feature launches | Campaign strategy updates |
| `calendar/` | Any release | Master calendar entries |
| `articles/` | Pain-point features | Cross-platform articles |
| DevCon sessions | `devcon/` (root) | Session content changes |
| Upwork portfolio | `upwork/portfolio/` | Major capability additions |

## Press Release Frontmatter

```yaml
---
title: "RAPS 4.16: Pipeline v2 Engine with Retry, Parallelism, and Conditionals"
description: "One-line SEO description"
type: "release"              # release | announcement | template
publishDate: 2026-03-01
status: "draft"              # draft | approved | published
---
```

File: `src/content/press/press-releases/<slug>.md`

### Press Release Structure

```markdown
# FOR IMMEDIATE RELEASE

## Headline

*Subheadline with key details*

---

**Version Information**
- **Current Version**: X.Y.Z
- **APS API Coverage**: list
- **Architecture**: 10 Rust workspace crates
- **License**: Apache-2.0

---

Body paragraphs...

### What's New
Feature descriptions with code examples...

### Availability
Download/install instructions...

### About RAPS
Boilerplate...
```

## Campaign Update Frontmatter

```yaml
---
title: "Campaign Title"
description: "Description"
campaign: "campaign-slug"
status: "active"              # planning | active | completed
startDate: 2026-01-01
endDate: 2026-12-31
targetAudience: ["APS developers", "DevOps engineers"]
---
```

## Calendar Entry Frontmatter

```yaml
---
title: "Calendar Title"
description: "Description"
type: "master"                # master | campaign | theme | integrated
quarter: "Q1 2026"
priority: "high"              # low | medium | high
lastUpdated: 2026-03-01
---
```

## Key Files to Update

| File | Content |
|------|---------|
| `src/content/press/press-releases/raps-4-0-launch.md` | Main press release (update version, features) |
| `src/content/campaigns/devcon-2026/campaign-strategy.md` | DevCon talking points |
| `src/content/calendar/master-calendar-2026.md` | Release timeline |
| `devcon/1257-ai-pair-assistant.md` | MCP/AI session content |
| `devcon/1258-zero-to-production.md` | Deployment session |
| `devcon/1259-acc-enterprise-scale.md` | Enterprise patterns |
| `upwork/profile/overview.md` | Freelance profile capabilities |

## Checklist

1. Update main press release with current version and new features
2. Add new press release if warranted (major feature or milestone)
3. Update DevCon session content if relevant features added
4. Update master calendar with release entry
5. Update Upwork profile if new capability category added
6. Commit and push
