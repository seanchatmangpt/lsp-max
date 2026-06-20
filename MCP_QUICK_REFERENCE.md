# MCP Servers - Quick Reference

A concise reference guide for Claude Code MCP servers. Print this page or bookmark for quick lookup.

**Version:** 1.0
**Last Updated:** 2026-06-14

---

## Server Summary

| Server | Tools | Auth | Status | Key Use Cases |
|--------|-------|------|--------|---------------|
| **Gmail** | 11 | OAuth 2.0 | ✓ Stable | Email drafts, labels, threads, search |
| **Google Calendar** | 8 | OAuth 2.0 | ✓ Stable | Events, scheduling, reminders, availability |
| **GitHub** | 47 | PAT | ✓ Stable | Repos, PRs, issues, commits, workflows |
| **Mermaid** | 1 | None | ✓ Stable | Diagrams, flowcharts, visualizations |

---

## Gmail Tools at a Glance

| Tool | Input | Output | Max Size |
|------|-------|--------|----------|
| `create_draft` | to, subject, body | draftId | 25MB |
| `list_drafts` | query, pageSize | drafts[] | 50/page |
| `list_labels` | pageSize | labels[] | Unlimited |
| `create_label` | displayName, color | labelId | - |
| `update_label` | labelId, displayName | labelId | - |
| `delete_label` | labelId | - | - |
| `search_threads` | query, pageSize | threads[] | 50/page |
| `get_thread` | threadId, format | messages[] | Full content |
| `label_thread` | threadId, labelIds | - | - |
| `unlabel_thread` | threadId, labelIds | - | - |
| `label_message` | messageId, labelIds | - | - |
| `unlabel_message` | messageId, labelIds | - | - |

---

## Google Calendar Tools at a Glance

| Tool | Input | Output | Key Feature |
|------|-------|--------|-------------|
| `list_calendars` | pageSize | calendars[] | Get all calendars |
| `list_events` | startTime, endTime | events[] | List by date range |
| `get_event` | eventId | event | Get details |
| `create_event` | summary, start, end | eventId | Create new event |
| `update_event` | eventId, fields | eventId | Modify event |
| `delete_event` | eventId | - | Remove event |
| `respond_to_event` | eventId, response | - | Accept/decline |
| `suggest_time` | attendees, date range | slots[] | Find free time |

---

## GitHub Tools Summary

### Repositories
- `create_repository` - Create new repo
- `fork_repository` - Fork existing repo
- `list_branches` - List all branches
- `create_branch` - Create new branch
- `get_file_contents` - Read file/dir
- `create_or_update_file` - Create/edit file (needs SHA for update)
- `delete_file` - Delete file
- `push_files` - Batch push multiple files

### Commits & Tags
- `get_commit` - Get commit details
- `list_commits` - List commits
- `search_commits` - Search commits
- `get_tag` - Get tag details
- `list_tags` - List all tags
- `get_latest_release` - Latest release
- `get_release_by_tag` - Get specific release
- `list_releases` - List all releases

### Issues & PRs
- `issue_write` - Create/update issue
- `issue_read` - Get issue details
- `add_issue_comment` - Comment on issue
- `list_issues` - List issues
- `search_issues` - Search issues
- `create_pull_request` - Create PR
- `list_pull_requests` - List PRs
- `search_pull_requests` - Search PRs
- `pull_request_read` - Get PR details
- `pull_request_review_write` - Create/submit review
- `update_pull_request` - Update PR metadata
- `merge_pull_request` - Merge PR
- `update_pull_request_branch` - Update with base

### Actions
- `actions_list` - List workflows/runs
- `actions_get` - Get workflow details
- `actions_run_trigger` - Run/cancel workflow
- `get_job_logs` - Get job logs

### Search & More
- `search_code` - Search code
- `search_repositories` - Find repos
- `search_users` - Find users
- `get_me` - Get auth user
- `get_teams` - Get user teams
- `get_team_members` - List team members
- `list_repository_collaborators` - List collaborators
- `enable_pr_auto_merge` - Enable auto-merge
- `disable_pr_auto_merge` - Disable auto-merge
- `run_secret_scanning` - Scan for secrets
- `request_copilot_review` - Request Copilot review
- `get_label` - Get label details
- `list_issue_fields` - List custom fields
- `list_issue_types` - List issue types

---

## Common Parameters

### Email Address
```
Type: string
Format: "user@example.com"
Note: Plain email only, no display names
```

### ISO 8601 DateTime
```
Type: string
Format: "2026-06-15T14:00:00Z"
Example: "2026-06-15T14:00:00-07:00"
```

### Label ID
```
Type: string
System: "INBOX", "TRASH", "SPAM", "STARRED", "UNREAD", "IMPORTANT"
User: Get from list_labels response
```

### Page Token
```
Type: string
Use: Pagination for list operations
Pattern: Copy from previous response's pageToken
```

### Query String (Gmail)
```
Syntax: from:user to:user subject:text is:unread newer_than:7d
Combine: Multiple conditions with AND (default) or OR
```

---

## Authentication Quick Setup

### Gmail/Calendar
```bash
1. Settings > MCP Servers > Gmail
2. Click "Authorize with Google"
3. Sign in and grant permissions
4. Credentials auto-saved
```

**Required Scopes:**
- gmail.modify, gmail.readonly
- calendar, calendar.readonly

### GitHub
```bash
1. https://github.com/settings/tokens
2. Generate new token (classic)
3. Name: "Claude Code"
4. Expiration: 90 days
5. Scopes: repo, read:user, user:email, read:org, workflow
6. Copy token
7. Settings > MCP Servers > GitHub > Paste token
```

### Mermaid
```
No authentication needed
Built into Claude Code
Test immediately
```

---

## Rate Limits

| Service | Limit | Window |
|---------|-------|--------|
| Gmail | Quota | Daily |
| Calendar | 10,000 | Per minute |
| GitHub REST | 5,000 | Per hour |
| GitHub GraphQL | 5,000 pts | Per hour |
| Mermaid | Unlimited | - |

**Headers to Check:**
- `X-RateLimit-Remaining` - Requests left
- `X-RateLimit-Reset` - When limit resets (unix timestamp)
- `X-RateLimit-Limit` - Total limit

---

## Error Codes

| Code | Cause | Solution |
|------|-------|----------|
| 400 | Bad request | Check parameter format, emails, dates |
| 401 | Unauthorized | Re-authenticate, check token |
| 403 | Forbidden | Check scopes, verify access |
| 404 | Not found | Verify ID/name, check access |
| 429 | Rate limited | Wait for reset, implement backoff |
| 500 | Server error | Retry after delay |

---

## Diagram Types (Mermaid)

```
Supported:
- flowchart (LR, TB, BT, RL)
- sequenceDiagram
- gantt
- classDiagram
- stateDiagram-v2
- erDiagram
- pie / barChart
- gitGraph
- userJourney
```

---

## Code Snippets

### Create Email Draft
```javascript
await gmail.createDraft({
  to: ["recipient@example.com"],
  subject: "Subject Line",
  body: "Email body text"
});
```

### Create Calendar Event
```javascript
await calendar.createEvent({
  summary: "Meeting Title",
  startTime: "2026-06-15T14:00:00Z",
  endTime: "2026-06-15T15:00:00Z",
  attendees: [{ email: "john@example.com" }],
  addGoogleMeetUrl: true
});
```

### Create GitHub Branch & Push Files
```javascript
await github.createBranch({
  owner: "org", repo: "repo",
  branch: "feature-name"
});

await github.pushFiles({
  owner: "org", repo: "repo",
  branch: "feature-name",
  files: [
    { path: "README.md", content: "# Title" }
  ],
  message: "feat: description"
});
```

### Create Mermaid Diagram
```javascript
await mermaid.validateAndRenderMermaidDiagram({
  diagramCode: `
    flowchart LR
      A[Start] --> B[Process] --> C[End]
  `,
  title: "Flow"
});
```

---

## Keyboard Shortcuts (Claude Code)

| Action | Shortcut |
|--------|----------|
| Open settings | Ctrl/Cmd + , |
| Quick search | Ctrl/Cmd + K |
| Open MCP Servers | Ctrl/Cmd + Shift + P then "MCP" |
| Execute command | Ctrl/Cmd + Enter |
| Clear context | Ctrl/Cmd + L |

---

## Common Workflows

### Email → Calendar
1. Search email with `search_threads()`
2. Get details with `get_thread()`
3. Parse event info
4. Create event with `create_event()`
5. Label email

### GitHub Release Notes
1. Get commits with `list_commits()`
2. Get PRs with `search_pull_requests()`
3. Compile into notes
4. Create email draft

### Team Status Digest
1. Get emails with `search_threads()`
2. Get calendar with `list_events()`
3. Get GitHub activity with `search_pull_requests()`
4. Combine into digest
5. Send with `create_draft()`

### PR Monitoring
1. List open PRs with `list_pull_requests()`
2. Get status with `pull_request_read(method: "get_status")`
3. Get reviews with `pull_request_read(method: "get_reviews")`
4. Report issues

---

## Troubleshooting Quick Guide

**Can't authorize Gmail?**
- Check Gmail API is enabled in Google Cloud Console
- Clear browser cache
- Re-authorize from scratch

**GitHub rate limited?**
- Check remaining in response headers
- Wait for X-RateLimit-Reset time
- Use batch operations
- Cache results

**Email validation error?**
- Remove display names (use email only)
- Check for spaces or special characters
- Use array notation for multiple

**Event time issues?**
- Use ISO 8601 format: `2026-06-15T14:00:00Z`
- Specify timezone: `America/Los_Angeles`
- Check for timezone conflicts

**Diagram won't render?**
- Validate in Mermaid Live Editor
- Check for syntax errors
- Simplify diagram
- Use basic shapes first

---

## Configuration Files

### Settings Location
```
macOS/Linux: ~/.config/claude/settings.json
Windows: %USERPROFILE%\.config\claude\settings.json
```

### Sample Config
```json
{
  "mcpServers": {
    "gmail": {
      "enabled": true,
      "timeout": 30000
    },
    "github": {
      "enabled": true,
      "endpoint": "https://api.github.com"
    }
  }
}
```

---

## Resources

### Documentation Files
- `MCP_SERVERS_DOCUMENTATION.md` - Complete reference (25K words)
- `MCP_PLUGINS_REGISTRY.md` - Server inventory (12K words)
- `MCP_INTEGRATION_GUIDE.md` - Patterns & recipes (18K words)
- `MCP_DOCUMENTATION_INDEX.md` - Navigation guide (5K words)

### External Links
- Gmail API Docs: https://developers.google.com/gmail/api
- Calendar API Docs: https://developers.google.com/calendar
- GitHub API Docs: https://docs.github.com/rest
- Mermaid Docs: https://mermaid.js.org

### Support
- GitHub Issues: https://github.com/anthropic-ai/claude-code/issues
- Community: https://community.anthropic.com
- Email: support@anthropic.com

---

## Tips & Tricks

**Batch Operations**
- Use `github.pushFiles()` instead of individual file calls
- 10x faster for multiple files

**Caching Results**
- Cache label lists (Gmail)
- Cache calendar list
- Cache frequently searched repos

**Error Recovery**
- Always retry on 429 (rate limit)
- Wait exponentially longer each retry
- Log errors for debugging

**Parameter Validation**
- Validate emails before sending
- Check dates in ISO 8601
- Verify IDs match expected format

**Performance**
- Use pageSize wisely (balance detail vs speed)
- Batch related operations
- Implement caching
- Use search filters to reduce results

---

## Common Issues Reference

| Issue | Check | Fix |
|-------|-------|-----|
| Auth fails | Token expired? | Regenerate token |
| No results | Query wrong? | Try simpler query |
| Rate limited | Used too many? | Wait and retry |
| Bad format | Email/date format? | Use correct format |
| Not found | ID correct? | Verify with list |
| Timeout | Network slow? | Increase timeout |

---

## Token Rotation Schedule

**Gmail/Calendar:** Automatic (refresh token)
**GitHub:** Every 90 days (set reminder)

**GitHub Token Rotation:**
1. Generate new token
2. Update in Claude Code settings
3. Verify connection works
4. Go to GitHub settings
5. Delete old token

---

## Document Information

- **Total Tools:** 67
- **Pages:** ~5 pages (this quick ref)
- **Main Docs:** 4 files, ~60,000 words
- **Code Examples:** 360+
- **Last Updated:** 2026-06-14

---

**Quick Reference v1.0 — Print-friendly MCP Server Guide**
