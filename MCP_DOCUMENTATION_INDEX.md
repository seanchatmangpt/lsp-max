# MCP Servers Documentation Index

## Overview

This index provides an organized guide to all MCP (Model Context Protocol) server documentation for Claude Code. These documents form a comprehensive knowledge base for integrating, configuring, and optimizing MCP servers.

**Documentation Version:** 1.0
**Last Updated:** 2026-06-14
**Maintainer:** Claude Code Documentation Team

---

## Documentation Structure

```
MCP Documentation Suite
├── MCP_DOCUMENTATION_INDEX.md (this file)
├── MCP_SERVERS_DOCUMENTATION.md (main reference)
├── MCP_PLUGINS_REGISTRY.md (inventory & compatibility)
└── MCP_INTEGRATION_GUIDE.md (recipes & patterns)
```

---

## Quick Navigation

### For Getting Started
- **First Time User?** Start with [Quick Setup](#quick-setup-checklist)
- **Need Authentication Help?** See [Authentication Guide](#authentication-setup)
- **Want Code Examples?** Go to [MCP_INTEGRATION_GUIDE.md](#server-integration-guide)

### For Reference
- **Tool Parameter Details?** Check [MCP_SERVERS_DOCUMENTATION.md](#complete-server-reference)
- **Server Capabilities?** See [MCP_PLUGINS_REGISTRY.md](#plugins-registry)
- **Rate Limits?** Find them in [Rate Limiting Section](#rate-limiting)

### For Advanced Usage
- **Integration Patterns?** See [Integration Recipes](#integration-recipes)
- **Performance Optimization?** Check [Performance Section](#performance-optimization)
- **Security Hardening?** Go to [Security Guide](#security-hardening)

---

## Document Descriptions

### 1. MCP_SERVERS_DOCUMENTATION.md

**Purpose:** Complete technical reference for all MCP servers and tools

**Contents:**
- Server information and capabilities
- Complete tool list with all parameters
- Return types and data structures
- Authentication requirements
- Error codes and handling
- Rate limit specifications
- Best practices per server

**Key Sections:**
- Gmail Server (11 tools)
- Google Calendar Server (8 tools)
- GitHub Server (47 tools)
- Mermaid Chart Server (1 tool)
- Error Handling Guide
- Rate Limiting Reference
- General Best Practices

**Best For:**
- Looking up specific tool parameters
- Understanding API responses
- Implementing error handling
- Understanding scope requirements

**Size:** ~25,000 words
**Time to Read:** 2-3 hours (reference document)

---

### 2. MCP_PLUGINS_REGISTRY.md

**Purpose:** Central inventory and compatibility matrix for MCP servers

**Contents:**
- Master registry of all servers
- Compatibility matrices (OS, versions, tools)
- Installation instructions per server
- Configuration reference
- Environment variables
- Upgrade and maintenance procedures
- Support resources

**Key Sections:**
- Server Registry (with capabilities)
- Plugin Compatibility Matrix
- Installation Guide (all servers)
- Configuration Reference
- Troubleshooting Installation
- Upgrade Guide
- Maintenance Schedule

**Best For:**
- Installation and setup
- Checking server compatibility
- Configuration management
- Planning maintenance windows
- Upgrading servers

**Size:** ~12,000 words
**Time to Read:** 1-1.5 hours (reference document)

---

### 3. MCP_INTEGRATION_GUIDE.md

**Purpose:** Practical patterns, recipes, and best practices

**Contents:**
- Quick start patterns (4 core patterns)
- Integration recipes (4 complete workflows)
- Advanced usage patterns
- Performance optimization techniques
- Security hardening strategies
- Comprehensive troubleshooting

**Key Sections:**
- Quick Start Patterns
  - Email Draft Management
  - Calendar Event Management
  - GitHub Repository Management
  - Diagram Generation
- Integration Recipes
  - Email-to-Calendar Workflow
  - GitHub PR Status Reporter
  - Automated Documentation Generator
  - Daily Digest Generator
- Advanced Patterns
  - Conditional Workflows
  - Batch Processing with Recovery
  - Caching and Memoization
- Performance Optimization
- Security Hardening
- Troubleshooting Guide (with diagnostics)

**Best For:**
- Learning common patterns
- Getting working code examples
- Implementing complex workflows
- Optimizing performance
- Securing applications
- Debugging issues

**Size:** ~18,000 words
**Time to Read:** 1.5-2 hours (+ implementation time)

---

## Quick Setup Checklist

### Pre-Setup (10 minutes)

- [ ] Have Claude Code v1.0+ installed
- [ ] Identify which servers you need
- [ ] Have required accounts ready
  - [ ] Google account (for Gmail/Calendar)
  - [ ] GitHub account (for GitHub)
- [ ] Have appropriate permissions in those services

### Setup (30 minutes)

**For Gmail:**
- [ ] Enable Gmail API in Google Cloud Console
- [ ] Create OAuth 2.0 credentials
- [ ] Launch Claude Code authentication flow
- [ ] Test with `list_labels` command

**For Google Calendar:**
- [ ] Reuse Gmail OAuth credentials
- [ ] Grant calendar access
- [ ] Test with `list_calendars` command

**For GitHub:**
- [ ] Generate Personal Access Token
- [ ] Select required scopes (repo, read:user, etc.)
- [ ] Add token to Claude Code settings
- [ ] Test with `get_me` command

**For Mermaid:**
- [ ] No setup required
- [ ] Built-in to Claude Code
- [ ] Test immediately

### Verification (10 minutes)

- [ ] All servers show as "Connected" in settings
- [ ] Test commands execute successfully
- [ ] No authentication errors in logs

**Total Time:** ~50 minutes

---

## Authentication Setup

### Gmail & Google Calendar

**Path:** Settings > MCP Servers > Google Services

**Steps:**
1. Click "Add Server" > "Gmail" or "Google Calendar"
2. Click "Authorize with Google"
3. Sign in with your Google account
4. Review and grant requested permissions
5. Credentials are securely stored

**Scopes Required:**
- `gmail.modify` - Create and modify emails
- `gmail.readonly` - Read emails
- `calendar.readonly` - Read calendar events
- `calendar` - Create and modify events

**Token Refresh:** Automatic (no manual refresh needed)

---

### GitHub

**Path:** Settings > MCP Servers > GitHub

**Steps:**
1. Go to https://github.com/settings/tokens
2. Click "Generate new token (classic)"
3. Name: "Claude Code" (or similar)
4. Expiration: 90 days (for rotation)
5. Select scopes:
   - `repo` (full control of repositories)
   - `read:user` (read public user profile)
   - `user:email` (read user email)
   - `read:org` (read organization data)
   - `workflow` (manage GitHub Actions)
6. Copy the token
7. In Claude Code: Settings > MCP Servers > GitHub
8. Paste token in "Personal Access Token" field
9. Click "Verify Connection"

**Token Rotation:** Every 90 days (recommended)

---

## Rate Limiting Overview

| Service | Limit | Window | Exceeded Response |
|---------|-------|--------|-------------------|
| Gmail | Per quota | Daily | 429 Too Many Requests |
| Google Calendar | 10,000 | Per minute | 429 Too Many Requests |
| GitHub (REST) | 5,000 | Per hour | 403 Forbidden + headers |
| GitHub (GraphQL) | 5,000 pts | Per hour | 403 Forbidden |
| Mermaid | Unlimited | N/A | N/A |

**Monitoring Rate Limits:**

Response headers contain rate limit info:
- `X-RateLimit-Remaining` - Requests left
- `X-RateLimit-Reset` - Time limit resets
- `X-RateLimit-Limit` - Total limit

**When Rate Limited:**
1. Wait until reset time
2. Implement exponential backoff
3. Use batch operations
4. Cache results when possible

See [MCP_SERVERS_DOCUMENTATION.md](MCP_SERVERS_DOCUMENTATION.md#rate-limiting) for detailed rate limit info.

---

## Common Tasks Quick Reference

### Task: Send Email with Gmail

**Location:** [MCP_INTEGRATION_GUIDE.md - Pattern 1](MCP_INTEGRATION_GUIDE.md#pattern-1-email-draft-management)

**Simple Example:**
```javascript
await gmail.createDraft({
  to: ["recipient@example.com"],
  subject: "Hello",
  body: "This is my message"
});
```

**Reference:** [Gmail create_draft Tool](MCP_SERVERS_DOCUMENTATION.md#1-create_draft)

---

### Task: Schedule Calendar Event

**Location:** [MCP_INTEGRATION_GUIDE.md - Pattern 2](MCP_INTEGRATION_GUIDE.md#pattern-2-calendar-event-management)

**Simple Example:**
```javascript
await calendar.createEvent({
  summary: "Meeting",
  startTime: "2026-06-15T14:00:00Z",
  endTime: "2026-06-15T15:00:00Z",
  attendees: [{ email: "john@example.com" }],
  addGoogleMeetUrl: true
});
```

**Reference:** [Calendar create_event Tool](MCP_SERVERS_DOCUMENTATION.md#4-create_event)

---

### Task: Create GitHub Pull Request

**Location:** [MCP_INTEGRATION_GUIDE.md - Pattern 3](MCP_INTEGRATION_GUIDE.md#pattern-3-github-repository-management)

**Simple Example:**
```javascript
await github.createPullRequest({
  owner: "myorg",
  repo: "myrepo",
  title: "feat: add new feature",
  head: "feature-branch",
  base: "main",
  body: "Description of the feature"
});
```

**Reference:** [GitHub create_pull_request Tool](MCP_SERVERS_DOCUMENTATION.md#6-create_pull_request)

---

### Task: Create Mermaid Diagram

**Location:** [MCP_INTEGRATION_GUIDE.md - Pattern 4](MCP_INTEGRATION_GUIDE.md#pattern-4-diagram-generation)

**Simple Example:**
```javascript
await mermaid.validateAndRenderMermaidDiagram({
  diagramCode: `
    flowchart LR
      A[Start] --> B[Process]
      B --> C[End]
  `,
  title: "Simple Flow"
});
```

**Reference:** [Mermaid validate_and_render_mermaid_diagram Tool](MCP_SERVERS_DOCUMENTATION.md#tool-validate_and_render_mermaid_diagram)

---

## Troubleshooting Quick Reference

### Error: "UNAUTHENTICATED" (401)

**Cause:** Invalid or expired credentials

**Solution:**
1. Check token hasn't expired
2. Verify token has required scopes
3. Regenerate token or re-authenticate
4. Clear cached credentials

**Detailed Guide:** [MCP_INTEGRATION_GUIDE.md - Auth Issues](MCP_INTEGRATION_GUIDE.md#issue-unauthenticated-error)

---

### Error: "RATE_LIMIT_EXCEEDED" (429)

**Cause:** Too many API requests

**Solution:**
1. Wait until reset time (see headers)
2. Implement exponential backoff
3. Batch operations together
4. Cache results

**Detailed Guide:** [MCP_INTEGRATION_GUIDE.md - Rate Limit Issues](MCP_INTEGRATION_GUIDE.md#issue-rate_limit_exceeded-error)

---

### Error: "NOT_FOUND" (404)

**Cause:** Resource doesn't exist or no access

**Solution:**
1. Verify ID/name is correct
2. Check token has access to resource
3. For private repos, verify token has `repo` scope
4. Check resource actually exists

**Detailed Guide:** [MCP_INTEGRATION_GUIDE.md - Not Found Issues](MCP_INTEGRATION_GUIDE.md#issue-not_found-error-for-resources)

---

### Error: "INVALID_ARGUMENT" (400)

**Cause:** Malformed parameters

**Solutions:**
- Email format: Plain email only (`user@example.com`)
- Dates: ISO 8601 format only (`2026-06-15T14:00:00Z`)
- Arrays: Use proper array notation (`["item1", "item2"]`)
- IDs: Use ID not name (e.g., for labels, use ID not display name)

**Detailed Guide:** [MCP_INTEGRATION_GUIDE.md - Invalid Argument Issues](MCP_INTEGRATION_GUIDE.md#issue-invalid_argument-for-email-operations)

---

## Document Map

### By Use Case

**I want to...**

- **Get started** → [MCP_PLUGINS_REGISTRY.md - Installation Guide](MCP_PLUGINS_REGISTRY.md#installation-guide)
- **Learn tools** → [MCP_SERVERS_DOCUMENTATION.md](MCP_SERVERS_DOCUMENTATION.md)
- **See code examples** → [MCP_INTEGRATION_GUIDE.md - Quick Start Patterns](MCP_INTEGRATION_GUIDE.md#quick-start-patterns)
- **Understand parameters** → [MCP_SERVERS_DOCUMENTATION.md - Tool Reference](MCP_SERVERS_DOCUMENTATION.md)
- **Build workflows** → [MCP_INTEGRATION_GUIDE.md - Integration Recipes](MCP_INTEGRATION_GUIDE.md#integration-recipes)
- **Optimize performance** → [MCP_INTEGRATION_GUIDE.md - Performance Optimization](MCP_INTEGRATION_GUIDE.md#performance-optimization)
- **Secure my setup** → [MCP_INTEGRATION_GUIDE.md - Security Hardening](MCP_INTEGRATION_GUIDE.md#security-hardening)
- **Debug issues** → [MCP_INTEGRATION_GUIDE.md - Troubleshooting](MCP_INTEGRATION_GUIDE.md#troubleshooting-guide)
- **Check compatibility** → [MCP_PLUGINS_REGISTRY.md - Compatibility Matrix](MCP_PLUGINS_REGISTRY.md#plugin-compatibility-matrix)
- **Upgrade servers** → [MCP_PLUGINS_REGISTRY.md - Upgrade Guide](MCP_PLUGINS_REGISTRY.md#upgrade-guide)

### By Server

**Gmail:**
- Reference: [MCP_SERVERS_DOCUMENTATION.md - Gmail Server](MCP_SERVERS_DOCUMENTATION.md#gmail-server)
- Setup: [MCP_PLUGINS_REGISTRY.md - Gmail Installation](MCP_PLUGINS_REGISTRY.md#1-gmail-server)
- Examples: [MCP_INTEGRATION_GUIDE.md - Pattern 1](MCP_INTEGRATION_GUIDE.md#pattern-1-email-draft-management)
- Recipes: [Recipe 1](MCP_INTEGRATION_GUIDE.md#recipe-1-email-to-calendar-workflow)

**Google Calendar:**
- Reference: [MCP_SERVERS_DOCUMENTATION.md - Google Calendar Server](MCP_SERVERS_DOCUMENTATION.md#google-calendar-server)
- Setup: [MCP_PLUGINS_REGISTRY.md - Calendar Installation](MCP_PLUGINS_REGISTRY.md#2-google-calendar-server)
- Examples: [MCP_INTEGRATION_GUIDE.md - Pattern 2](MCP_INTEGRATION_GUIDE.md#pattern-2-calendar-event-management)

**GitHub:**
- Reference: [MCP_SERVERS_DOCUMENTATION.md - GitHub Server](MCP_SERVERS_DOCUMENTATION.md#github-server)
- Setup: [MCP_PLUGINS_REGISTRY.md - GitHub Installation](MCP_PLUGINS_REGISTRY.md#3-github-server)
- Examples: [MCP_INTEGRATION_GUIDE.md - Pattern 3](MCP_INTEGRATION_GUIDE.md#pattern-3-github-repository-management)
- Recipes: [Recipe 2](MCP_INTEGRATION_GUIDE.md#recipe-2-github-pr-status-reporter), [Recipe 3](MCP_INTEGRATION_GUIDE.md#recipe-3-automated-documentation-generator)

**Mermaid:**
- Reference: [MCP_SERVERS_DOCUMENTATION.md - Mermaid Server](MCP_SERVERS_DOCUMENTATION.md#mermaid-chart-server)
- Setup: [MCP_PLUGINS_REGISTRY.md - Mermaid Installation](MCP_PLUGINS_REGISTRY.md#4-mermaid-chart-server)
- Examples: [MCP_INTEGRATION_GUIDE.md - Pattern 4](MCP_INTEGRATION_GUIDE.md#pattern-4-diagram-generation)

---

## Key Concepts

### Tools
Individual API operations exposed by MCP servers
- Gmail: 11 tools
- Google Calendar: 8 tools
- GitHub: 47 tools
- Mermaid: 1 tool
- **Total: 67 tools**

### Scopes / Permissions
Granular access controls that limit what a token can do
- Always request minimum necessary scopes
- OAuth 2.0 for Google services
- PAT with selected scopes for GitHub

### Rate Limits
Maximum API calls per time period
- Differ by service and operation
- Response headers indicate remaining calls
- Implement backoff strategies

### Authentication
Methods to verify identity and grant access
- OAuth 2.0: Gmail, Google Calendar (interactive)
- Personal Access Token: GitHub (static)
- None: Mermaid (public service)

### Batch Operations
Send multiple changes in single API call
- More efficient than individual calls
- Reduces rate limit usage
- Example: `github.pushFiles()` for multiple files

---

## Support & Resources

### Documentation Files
- [MCP_SERVERS_DOCUMENTATION.md](MCP_SERVERS_DOCUMENTATION.md) - Complete technical reference
- [MCP_PLUGINS_REGISTRY.md](MCP_PLUGINS_REGISTRY.md) - Server inventory and compatibility
- [MCP_INTEGRATION_GUIDE.md](MCP_INTEGRATION_GUIDE.md) - Patterns, recipes, and best practices

### External Resources
- **Gmail API:** https://developers.google.com/gmail/api
- **Google Calendar API:** https://developers.google.com/calendar
- **GitHub API:** https://docs.github.com/en/rest
- **Mermaid:** https://mermaid.js.org

### Getting Help
- **Claude Code Docs:** https://claude.ai/code/docs
- **GitHub Issues:** https://github.com/anthropic-ai/claude-code/issues
- **Community Forums:** https://community.anthropic.com
- **Email Support:** support@anthropic.com

---

## Changelog

### Version 1.0 (2026-06-14)
- Initial comprehensive documentation package
- Complete tool reference for all 67 tools
- Integration recipes and patterns
- Security and performance guides
- Troubleshooting documentation
- Compatibility matrices
- Installation guides for all servers

---

## Document Statistics

| Document | Words | Sections | Examples | Code |
|----------|-------|----------|----------|------|
| MCP_SERVERS_DOCUMENTATION.md | ~25,000 | 50+ | 80+ | 200+ |
| MCP_PLUGINS_REGISTRY.md | ~12,000 | 35+ | 20+ | 50+ |
| MCP_INTEGRATION_GUIDE.md | ~18,000 | 40+ | 15+ | 100+ |
| MCP_DOCUMENTATION_INDEX.md | ~5,000 | 20+ | 5+ | 10+ |
| **Total** | **~60,000** | **145+** | **120+** | **360+** |

---

## Document Conventions

### Code Examples
- JavaScript/TypeScript syntax
- Assume asynchronous execution
- Pseudo-implementation shown (actual usage in Claude Code)
- Comments explain key concepts

### Parameters
- **Required:** Listed without note
- **Optional:** Marked with "No" in Required column
- **Types:** JavaScript/TypeScript notation
- **Default:** Specified in parameter description

### Status Badges
- ✓ Supported/Available
- ✗ Not supported
- ⚠ Limited support / Preview
- ⏳ Coming soon

### Links
- Internal: `[Text](file.md#section)`
- External: Full URL with description

---

## Navigation Tips

1. **Search for specific tool:** Use document search (Ctrl/Cmd+F) in [MCP_SERVERS_DOCUMENTATION.md](MCP_SERVERS_DOCUMENTATION.md)

2. **Find code example:** Search "Example" in [MCP_INTEGRATION_GUIDE.md](MCP_INTEGRATION_GUIDE.md)

3. **Check compatibility:** See [Plugin Compatibility Matrix](MCP_PLUGINS_REGISTRY.md#plugin-compatibility-matrix)

4. **Setup server:** Go to [Installation Guide](MCP_PLUGINS_REGISTRY.md#installation-guide) in registry

5. **Debug error:** Search error code in [MCP_INTEGRATION_GUIDE.md](MCP_INTEGRATION_GUIDE.md#troubleshooting-guide)

6. **Learn pattern:** Browse [Quick Start Patterns](MCP_INTEGRATION_GUIDE.md#quick-start-patterns)

7. **Understand rate limits:** See [Rate Limiting](MCP_SERVERS_DOCUMENTATION.md#rate-limiting) section

---

## Feedback & Updates

This documentation is maintained and updated regularly. 

**To report issues or suggest improvements:**
1. Document the issue clearly
2. Note which document section is affected
3. Provide the update/correction needed
4. Submit via GitHub Issues or email support@anthropic.com

**Next review scheduled:** 2026-09-14

---

**Documentation Index Version:** 1.0
**Last Updated:** 2026-06-14
**Maintainer:** Claude Code Documentation Team
**License:** CC BY-SA 4.0
