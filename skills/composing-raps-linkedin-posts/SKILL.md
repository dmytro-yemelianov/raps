---
name: composing-raps-linkedin-posts
version: "1.0"
description: Use when new RAPS features ship and LinkedIn content is needed — adds posts to the raps-smm post bank following established tone, format, and hashtag conventions.
---

# Composing RAPS LinkedIn Posts

Add LinkedIn posts to the raps-smm post bank for new features and releases.

**Repo:** `/root/github/raps/raps-smm`
**File:** `content/linkedin/post-bank.md`

## Post Format

Posts are numbered sequentially and wrapped in fenced code blocks:

```markdown
### Post N: Title

` ` `
Post body here.

Lines starting with -> are bullet points.
-> First point
-> Second point

Question at the end for engagement?

#Tag1 #Tag2 #Tag3 #Tag4
` ` `

**firstComment:** github.com/dmytro-yemelianov/raps
```

The `firstComment` line (outside the code block) is optional — use for link-heavy posts to avoid LinkedIn's algorithm penalty.

## Sections

Posts are organized under themed H2 headers:

| Section | Content |
|---------|---------|
| Launch & Announcements | Major version launches |
| Educational Content | OAuth, CI/CD, DA tutorials |
| Quick Tips & Engagement | Short tips, polls |
| Tutorials & How-To | Step-by-step guides |
| MCP & AI Integration | MCP tools, AI workflows |
| Data-Driven Posts | Research-backed insights |
| New Feature Posts | Per-version feature highlights |

## Tone & Style

- Professional but conversational. Technical but accessible.
- Open with a hook — a number, a contrast, or a question.
- Use -> for bullet points (not `-` or `*`).
- End with an engagement question or call to action.
- 3-5 hashtags from the reference table below.
- No emojis in post body. Emojis only in section headers.
- Keep posts under 1300 characters (LinkedIn optimal length).

## Hashtag Reference

| Category | Tags |
|----------|------|
| Primary | #AutodeskPlatformServices #APS #Automation |
| DevOps | #CICD #DevOps #GitHubActions #GitLabCI |
| AEC | #BIM #AEC #DigitalConstruction |
| Tech | #Rust #CLI #OpenSource |
| AI/MCP | #AI #MCP #ClaudeAI #CursorIDE #VibeCoding |
| Personal | #BuildInPublic #SoftwareDevelopment |

Always use 3-5 total. Mix primary + specific category.

## Source of Truth Line

Update the version reference at the top of the file when stats change:

```markdown
> **Source of truth:** Feature counts reference [`raps/MANIFEST.md`](...) (v4.16.0: 95+ commands, 101 MCP tools, 10 crates, 7 modes).
```

## Checklist

1. Determine next post number (read last post number in file)
2. Update version range in New Feature Posts section header
3. Write 1-3 posts per major feature (hook -> detail -> engagement question)
4. Update source of truth version reference if stats changed
5. Commit and push
