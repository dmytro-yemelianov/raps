---
name: updating-devcon-materials
version: "1.0"
description: Use when DevCon 2026 session content needs updating — covers the 3 session files in raps-marketing, LinkedIn countdown posts in raps-smm, and forum announcements. DevCon is April 15-16 in Amsterdam.
---

# Updating DevCon 2026 Materials

Update session content, social posts, and promotional materials for DevCon 2026.

**Event:** April 15-16, 2026 — Amsterdam

## Session Files

**Location:** `/root/github/raps/raps-marketing/devcon/`

| File | Session | Focus |
|------|---------|-------|
| `1257-ai-pair-assistant.md` | AI Pair-Assistant for APS | MCP server, 101 tools, AI workflows |
| `1258-zero-to-production.md` | Zero to Production Speedrun | 30-min rapid deployment, CI/CD |
| `1259-acc-enterprise-scale.md` | ACC at Enterprise Scale | 8 automation patterns, bulk ops |

### Session File Structure (plain markdown, NOT frontmatter)

```markdown
# Session [ID]: Title

## Session Details
| Field | Value |
|-------|-------|
| **Session ID** | 1257 |
| **Title** | ... |
| **Speaker(s)** | Dmytro Yemelianov |
| **Session Type** | 30-minute deep dive |

## AI Pillars
- [x] **Automate** — description
- [ ] **Assist** — description

## Themes
- [x] **Digital Transformation**

## Target Audience
- [x] **Developers/Architects**

## Learning Objectives
1. Objective one

## Abstract
[Long paragraph]

## [Content Sections with code examples]

## Key Takeaways
1. Takeaway one

## Resources
- [RAPS GitHub](https://github.com/dmytro-yemelianov/raps)
```

When updating: add new features as talking points, update code examples, adjust key takeaways count.

## Social Media (raps-smm)

**LinkedIn countdown posts:** `/root/github/raps/raps-smm/devcon-2026/linkedin-posts/`
- `cfp-announcement.md` — CFP open/submitted
- `speaker-announcement.md` — if accepted
- `countdown-series.md` — 4-week countdown posts

**Forum posts:** `/root/github/raps/raps-smm/devcon-2026/forum-posts/`
- `devcon-announcement.md` — Autodesk Forums
- `mcp-announcement.md` — MCP-specific forums

### Post Timing

| Weeks Before | Content |
|--------------|---------|
| 8 weeks | Session preview teasers |
| 4 weeks | Countdown series starts |
| 2 weeks | Demo preview videos |
| 1 week | Final "see you there" post |
| Event day | Live updates |
| 1 week after | Recap + recording links |

## Campaign Strategy

**File:** `/root/github/raps/raps-marketing/src/content/campaigns/devcon-2026/campaign-strategy.md`

Contains the 8-month pre-event + 2-month post-event strategy. Update Phase tracking when milestones are reached.

## Checklist

1. Update session files with new features/capabilities
2. Update code examples to use latest RAPS version
3. Update key takeaways if new material warrants it
4. Refresh LinkedIn countdown posts with current stats
5. Update campaign strategy phase tracking
6. Commit to respective repos (raps-marketing + raps-smm)
