# MCP Plugins Registry

## Overview

This registry provides a centralized index of all Model Context Protocol (MCP) servers and plugins available in the Claude Code ecosystem, organized by category and use case.

**Version:** 1.0
**Last Updated:** 2026-06-14

---

## Table of Contents

1. [Server Registry](#server-registry)
2. [Plugin Compatibility Matrix](#plugin-compatibility-matrix)
3. [Installation Guide](#installation-guide)
4. [Configuration Reference](#configuration-reference)
5. [Upgrade Guide](#upgrade-guide)

---

## Server Registry

### Master Registry

| Server | Category | Tools | Status | Min Version | Auth Type | Cost |
|--------|----------|-------|--------|-------------|-----------|------|
| **Gmail** | Email | 11 | Stable | 1.0 | OAuth 2.0 | Free (quota limits) |
| **Google Calendar** | Productivity | 8 | Stable | 1.0 | OAuth 2.0 | Free (quota limits) |
| **GitHub** | DevOps | 47 | Stable | 1.0 | PAT | Free (5k API calls/hr) |
| **Mermaid Chart** | Visualization | 1 | Stable | 1.0 | None | Free |

### Server Details

#### Gmail Server

**Canonical Name:** `mcp__Gmail`

**Full Tool List:**
1. create_draft
2. list_drafts
3. list_labels
4. create_label
5. update_label
6. delete_label
7. search_threads
8. get_thread
9. label_thread
10. unlabel_thread
11. label_message
12. unlabel_message

**Capabilities:**

```json
{
  "read": true,
  "write": true,
  "delete": true,
  "threads": true,
  "labels": true,
  "drafts": true,
  "attachments": true,
  "search": true,
  "organization": true
}
```

**Resource Limits:**

- Max email size: 25MB
- Max recipients per email: Unlimited
- Max labels per account: Unlimited
- Max query results: 50 per page
- Concurrent requests: 1 (sequential processing)

**Example Configuration:**

```json
{
  "mcpServers": {
    "gmail": {
      "command": "gmail-mcp",
      "args": [],
      "env": {
        "GOOGLE_APPLICATION_CREDENTIALS": "~/.config/claude/google-creds.json"
      },
      "disabled": false
    }
  }
}
```

---

#### Google Calendar Server

**Canonical Name:** `mcp__Google_Calendar`

**Full Tool List:**
1. list_calendars
2. list_events
3. get_event
4. create_event
5. update_event
6. delete_event
7. respond_to_event
8. suggest_time

**Capabilities:**

```json
{
  "read": true,
  "write": true,
  "delete": true,
  "sharing": true,
  "reminders": true,
  "attendees": true,
  "recurrence": true,
  "availability": true,
  "time_zones": true,
  "google_meet": true
}
```

**Resource Limits:**

- Max events per calendar: Unlimited
- Max attendees per event: Unlimited
- Max description length: 4000 characters
- Max attachment size: 25MB per event
- Concurrent requests: 1 (sequential processing)

**Example Configuration:**

```json
{
  "mcpServers": {
    "googleCalendar": {
      "command": "google-calendar-mcp",
      "args": [],
      "env": {
        "GOOGLE_APPLICATION_CREDENTIALS": "~/.config/claude/google-creds.json"
      },
      "disabled": false
    }
  }
}
```

---

#### GitHub Server

**Canonical Name:** `mcp__github`

**Tool Categories:**

| Category | Count | Tools |
|----------|-------|-------|
| Repository Management | 7 | create_repository, fork_repository, list_branches, create_branch, get_file_contents, create_or_update_file, delete_file |
| File Operations | 2 | push_files, (integrated with above) |
| Commit Operations | 4 | get_commit, list_commits, search_commits, get_tag |
| Issue & PR | 13 | issue_write, issue_read, add_issue_comment, list_issues, search_issues, create_pull_request, list_pull_requests, search_pull_requests, pull_request_read, pull_request_review_write, update_pull_request, merge_pull_request, update_pull_request_branch |
| Search | 5 | search_code, search_repositories, search_users, search_commits, search_issues, search_pull_requests |
| Release | 3 | list_releases, get_latest_release, get_release_by_tag |
| GitHub Actions | 4 | actions_list, actions_get, actions_run_trigger, get_job_logs |
| PR Auto-Merge | 2 | enable_pr_auto_merge, disable_pr_auto_merge |
| User & Teams | 4 | get_me, get_teams, get_team_members, list_repository_collaborators |
| Security | 2 | run_secret_scanning, request_copilot_review |
| Labels & Misc | 2 | get_label, list_issue_fields, list_issue_types |

**Capabilities:**

```json
{
  "repositories": {
    "create": true,
    "fork": true,
    "manage": true,
    "delete": false
  },
  "commits": {
    "read": true,
    "search": true,
    "create": false
  },
  "issues": {
    "create": true,
    "read": true,
    "update": true,
    "delete": false,
    "search": true
  },
  "pullRequests": {
    "create": true,
    "read": true,
    "review": true,
    "merge": true,
    "autoMerge": true
  },
  "workflows": {
    "list": true,
    "trigger": true,
    "cancel": true,
    "rerun": true
  },
  "security": {
    "secretScanning": true,
    "copilotReview": true
  }
}
```

**Resource Limits:**

- API Rate Limit: 5,000 requests/hour (authenticated)
- GraphQL Rate Limit: 5,000 points/hour
- Search Rate Limit: 30 queries/minute
- Max response size: 60MB
- Max file size: 100MB
- Concurrent requests: 1 (sequential processing)

**Example Configuration:**

```json
{
  "mcpServers": {
    "github": {
      "command": "github-mcp",
      "args": [],
      "env": {
        "GITHUB_TOKEN": "ghp_xxxxxxxxxxxxxxxxxxxx",
        "GITHUB_API_ENDPOINT": "https://api.github.com"
      },
      "disabled": false
    }
  }
}
```

---

#### Mermaid Chart Server

**Canonical Name:** `mcp__Mermaid_Chart`

**Tool List:**
1. validate_and_render_mermaid_diagram

**Supported Diagram Types:**

- Flowchart (LR, TB, BT, RL)
- Sequence Diagram
- Gantt Chart
- Class Diagram
- State Diagram
- Entity Relationship Diagram (ER)
- User Journey
- Pie Chart
- Bar Chart
- Git Graph

**Capabilities:**

```json
{
  "validation": true,
  "rendering": true,
  "export": true,
  "customization": true,
  "authentication": false,
  "rateLimiting": false
}
```

**Resource Limits:**

- Max diagram size: 100KB
- Max complexity: 1000 nodes/connections
- Rendering timeout: 10 seconds
- Concurrent requests: Unlimited

**Example Configuration:**

```json
{
  "mcpServers": {
    "mermaid": {
      "command": "mermaid-mcp",
      "args": [],
      "env": {},
      "disabled": false
    }
  }
}
```

---

## Plugin Compatibility Matrix

### Claude Code Version Compatibility

| Server | 1.0+ | 2.0+ | Notes |
|--------|------|------|-------|
| Gmail | ✓ | ✓ | OAuth 2.0 standard |
| Google Calendar | ✓ | ✓ | OAuth 2.0 standard |
| GitHub | ✓ | ✓ | PAT-based auth |
| Mermaid | ✓ | ✓ | No auth required |

### Operating System Compatibility

| Server | macOS | Linux | Windows | WSL |
|--------|-------|-------|---------|-----|
| Gmail | ✓ | ✓ | ✓ | ✓ |
| Google Calendar | ✓ | ✓ | ✓ | ✓ |
| GitHub | ✓ | ✓ | ✓ | ✓ |
| Mermaid | ✓ | ✓ | ✓ | ✓ |

### Tool Compatibility Table

| Server | Desktop | Web | Mobile | CLI |
|--------|---------|-----|--------|-----|
| Gmail | Full | Full | Partial | Full |
| Google Calendar | Full | Full | Partial | Full |
| GitHub | Full | Full | Partial | Full |
| Mermaid | Full | Full | Full | Full |

---

## Installation Guide

### Prerequisites

- Claude Code CLI v1.0 or later
- Node.js 16+ (for MCP server processes)
- Required authentication credentials

### Step-by-Step Installation

#### 1. Gmail Server

**Prerequisites:**

- Google account with Gmail
- Google Cloud Console access
- OAuth 2.0 credentials

**Installation:**

```bash
# Claude Code will handle installation automatically
# Configure in Claude Code settings:

1. Open Claude Code Settings
2. Navigate to MCP Servers
3. Click "Add Server"
4. Select "Gmail"
5. Click "Authorize with Google"
6. Complete OAuth flow
7. Grant calendar access
```

**Verification:**

```bash
# Test in Claude Code
/claude list_labels

# Expected output:
# [
#   {"id": "INBOX", "displayName": "INBOX", "type": "system"},
#   {"id": "Label_1", "displayName": "Work", "type": "user"},
#   ...
# ]
```

---

#### 2. Google Calendar Server

**Prerequisites:**

- Same Google account as Gmail (recommended)
- Google Cloud Console with Calendar API enabled

**Installation:**

```bash
# Usually shares OAuth credentials with Gmail
# Configure in Claude Code settings:

1. Open Claude Code Settings
2. Navigate to MCP Servers
3. Click "Add Server"
4. Select "Google Calendar"
5. Use existing Google authorization (or re-authorize)
6. Grant calendar access
```

**Verification:**

```bash
# Test in Claude Code
/claude list_calendars

# Expected output:
# {
#   "calendars": [
#     {
#       "id": "primary",
#       "summary": "Your Name's Calendar",
#       "timeZone": "America/Los_Angeles",
#       "primary": true
#     }
#   ]
# }
```

---

#### 3. GitHub Server

**Prerequisites:**

- GitHub account
- Personal Access Token (PAT) with scopes: repo, read:user, user:email, read:org, workflow

**Installation - Generate PAT:**

```bash
# Go to: https://github.com/settings/tokens

1. Click "Generate new token (classic)"
2. Give it a name: "Claude Code"
3. Select expiration: 90 days (for rotation schedule)
4. Select scopes:
   - [x] repo (full control)
   - [x] read:user
   - [x] user:email
   - [x] read:org
   - [x] workflow
5. Click "Generate token"
6. Copy token immediately (cannot retrieve later)
```

**Installation - Configure Server:**

```bash
# In Claude Code settings:

1. Open Claude Code Settings
2. Navigate to MCP Servers
3. Click "Add Server"
4. Select "GitHub"
5. Paste PAT into "Personal Access Token" field
6. Click "Verify Connection"
7. Save configuration
```

**Verification:**

```bash
# Test in Claude Code
/claude get_me

# Expected output:
# {
#   "login": "your_username",
#   "id": 12345,
#   "name": "Your Name",
#   "email": "your_email@example.com",
#   "company": "Your Company",
#   "public_repos": 15,
#   "followers": 100,
#   "following": 50
# }
```

---

#### 4. Mermaid Chart Server

**Installation:**

```bash
# Mermaid server is built-in (no configuration needed)

1. Open Claude Code Settings
2. Navigate to MCP Servers
3. Verify "Mermaid Chart" is listed and enabled
4. No authentication required
```

**Verification:**

```bash
# Test in Claude Code
/claude validate_and_render_mermaid_diagram --diagramCode="
flowchart LR
    A[Test] --> B[Diagram]
"

# Expected output: SVG diagram rendered
```

---

### Troubleshooting Installation

#### Gmail/Calendar OAuth Issues

**Problem:** "Authorization failed" or "Invalid credentials"

**Solutions:**

1. Clear cached credentials:
   ```bash
   # macOS/Linux
   rm -rf ~/.config/claude/credentials
   
   # Windows
   rmdir /s %USERPROFILE%\.config\claude\credentials
   ```

2. Re-authorize in settings
3. Check that Gmail API is enabled in Google Cloud Console
4. Verify OAuth consent screen is configured

**Problem:** "Permission denied" error

**Solutions:**

1. Check OAuth scopes include: gmail.modify, gmail.readonly
2. Verify Google account has Gmail enabled
3. Check account is not suspended or restricted

---

#### GitHub Authentication Issues

**Problem:** "Invalid token" or "Unauthorized"

**Solutions:**

1. Verify PAT has not expired
2. Check PAT has required scopes
3. Verify PAT is not revoked in GitHub settings
4. Generate new PAT if necessary
5. Paste exact token (no whitespace)

**Problem:** "Repository not found" or "Permission denied"

**Solutions:**

1. Verify PAT has `repo` scope
2. Check user has access to repository
3. For private repos, verify PAT has full `repo` scope
4. For org repos, verify PAT has `read:org` scope

---

#### Mermaid Chart Issues

**Problem:** "Invalid diagram syntax"

**Solutions:**

1. Validate syntax in Mermaid Live Editor: https://mermaid.live
2. Check for common syntax errors:
   - Missing colons in labels
   - Unmatched brackets
   - Invalid node IDs
3. Use simple diagram first to test

---

## Configuration Reference

### Global Settings

Located in: `~/.config/claude/settings.json`

```json
{
  "mcpServers": {
    "gmail": {
      "enabled": true,
      "timeout": 30000,
      "retries": 3,
      "cacheResults": true,
      "cacheDuration": 300
    },
    "googleCalendar": {
      "enabled": true,
      "timeout": 30000,
      "retries": 3,
      "defaultTimeZone": "America/Los_Angeles"
    },
    "github": {
      "enabled": true,
      "timeout": 30000,
      "retries": 3,
      "endpoint": "https://api.github.com",
      "graphqlEndpoint": "https://api.github.com/graphql"
    },
    "mermaid": {
      "enabled": true,
      "timeout": 10000,
      "renderingEngine": "svg",
      "theme": "default"
    }
  }
}
```

### Per-Server Environment Variables

#### Gmail

```bash
GOOGLE_APPLICATION_CREDENTIALS=~/.config/claude/google-creds.json
GMAIL_QUOTA_USER=user@example.com
GMAIL_MAX_BATCH_SIZE=10
```

#### Google Calendar

```bash
GOOGLE_APPLICATION_CREDENTIALS=~/.config/claude/google-creds.json
CALENDAR_TIMEZONE=America/Los_Angeles
CALENDAR_DEFAULT_CALENDAR=primary
```

#### GitHub

```bash
GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxx
GITHUB_API_ENDPOINT=https://api.github.com
GITHUB_GRAPHQL_ENDPOINT=https://api.github.com/graphql
GITHUB_MAX_RETRIES=3
GITHUB_RETRY_DELAY_MS=1000
```

#### Mermaid

```bash
MERMAID_THEME=default
MERMAID_MAX_DIAGRAM_SIZE=102400
MERMAID_RENDER_TIMEOUT=10000
```

---

## Upgrade Guide

### Version Management

Current stable versions:

```
Gmail: 1.0.0
Google Calendar: 1.0.0
GitHub: 1.0.0
Mermaid: 1.0.0
```

### Checking Installed Versions

```bash
# View all installed servers and versions
claude mcp list

# Check specific server version
claude mcp version gmail
```

### Upgrading Servers

#### Automatic Updates

By default, Claude Code checks for updates daily:

```json
{
  "autoUpdate": {
    "enabled": true,
    "checkInterval": 86400000,
    "installUpdates": false,
    "notifyOnUpdate": true
  }
}
```

#### Manual Updates

```bash
# Upgrade specific server
claude mcp upgrade gmail

# Upgrade all servers
claude mcp upgrade --all

# Upgrade to specific version
claude mcp upgrade github@1.1.0
```

### Breaking Changes

#### v1.0 → v2.0 (Future)

Anticipated breaking changes:

- Gmail: Thread IDs format change (planned)
- GitHub: Deprecated REST endpoints removed
- Google Calendar: Recurring event format changes

Migration path:

1. Test upgrade on non-production first
2. Update any custom scripts
3. Review deprecation warnings
4. Execute upgrade during maintenance window

### Rollback Procedure

```bash
# Downgrade to previous version
claude mcp downgrade gmail

# Downgrade to specific version
claude mcp install gmail@0.9.0

# Restore from backup
claude mcp restore gmail --backup-date=2026-06-01
```

---

## Maintenance

### Regular Maintenance Tasks

#### Daily

```bash
# Check server health
claude mcp health --verbose

# Monitor rate limit usage
claude mcp stats
```

#### Weekly

```bash
# Verify all credentials are valid
claude mcp verify-auth

# Check for available updates
claude mcp check-updates
```

#### Monthly

```bash
# Rotate personal access tokens (GitHub)
# Instructions: Generate new PAT, update settings, revoke old token

# Review and clean up unused labels (Gmail)
# Archive old drafts

# Review calendars and remove archived calendars
```

#### Quarterly

```bash
# Security audit of MCP permissions
claude mcp audit-permissions

# Review and rotate credentials
# Update OAuth consent screen (if needed)
```

---

## Support & Documentation

### Getting Help

- **Official Docs:** https://claude.ai/code/docs
- **GitHub Issues:** https://github.com/anthropic-ai/claude-code/issues
- **Community Forums:** https://community.anthropic.com
- **Email Support:** support@anthropic.com

### API Documentation

- **Gmail API:** https://developers.google.com/gmail/api
- **Google Calendar API:** https://developers.google.com/calendar
- **GitHub API:** https://docs.github.com/en/rest
- **Mermaid:** https://mermaid.js.org

### Contributing

Report bugs, request features, or contribute improvements:

```bash
# Report issue
claude mcp report-issue --server=gmail

# Request feature
claude mcp request-feature --description="Add feature X"

# Contribute
# See: https://github.com/anthropic-ai/claude-code/CONTRIBUTING.md
```

---

**Registry Version:** 1.0
**Last Updated:** 2026-06-14
**Next Review:** 2026-09-14
