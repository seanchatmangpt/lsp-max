# MCP Servers Integration Guide & Best Practices

## Overview

This guide provides practical integration patterns, complete code examples, and best practices for using MCP servers in Claude Code effectively and securely.

**Version:** 1.0
**Last Updated:** 2026-06-14

---

## Table of Contents

1. [Quick Start Patterns](#quick-start-patterns)
2. [Integration Recipes](#integration-recipes)
3. [Advanced Usage](#advanced-usage)
4. [Performance Optimization](#performance-optimization)
5. [Security Hardening](#security-hardening)
6. [Troubleshooting Guide](#troubleshooting-guide)

---

## Quick Start Patterns

### Pattern 1: Email Draft Management

**Use Case:** Create, review, and manage email drafts

**Example Workflow:**

```javascript
// 1. Create a draft
const draft = await gmail.createDraft({
  to: ["recipient@example.com"],
  subject: "Project Update",
  body: "Here's the latest project status...",
  cc: ["manager@example.com"]
});
console.log("Draft created:", draft.draftId);

// 2. List recent drafts
const drafts = await gmail.listDrafts({
  pageSize: 10
});
drafts.forEach(d => console.log(`${d.subject} (${d.draftId})`));

// 3. Create with attachments
const draftWithAttachment = await gmail.createDraft({
  to: ["recipient@example.com"],
  subject: "Report - Q2 2026",
  htmlBody: "<p>Attached is the Q2 report.</p>",
  attachments: [{
    filename: "Q2_Report.pdf",
    content: Buffer.from(pdfData).toString('base64'),
    mimeType: "application/pdf"
  }]
});
```

**Best Practices:**

- Always verify draft content before sending
- Use htmlBody for formatted emails
- Test with small attachments first
- Keep recipient lists manageable
- Review drafts before marking as sent

---

### Pattern 2: Calendar Event Management

**Use Case:** Create and manage calendar events

**Example Workflow:**

```javascript
// 1. Get available time slots
const slots = await calendar.suggestTime({
  attendeeEmails: ["john@example.com", "jane@example.com"],
  startTime: "2026-06-16T00:00:00Z",
  endTime: "2026-06-20T23:59:59Z",
  durationMinutes: 60,
  preferences: {
    startHour: "09:00",
    endHour: "17:00",
    excludeWeekends: true
  }
});

// 2. Create meeting for first available slot
if (slots.suggestedSlots.length > 0) {
  const slot = slots.suggestedSlots[0];
  
  const event = await calendar.createEvent({
    summary: "Team Sync",
    description: "Weekly team synchronization",
    startTime: slot.startTime,
    endTime: slot.endTime,
    location: "Zoom Room 123",
    attendees: [
      { email: "john@example.com" },
      { email: "jane@example.com", optionalAttendee: true }
    ],
    addGoogleMeetUrl: true,
    overrideReminders: [
      { method: "email", minutes: 15 },
      { method: "popup", minutes: 5 }
    ]
  });
  
  console.log("Event created:", event.id);
}

// 3. List upcoming events
const upcoming = await calendar.listEvents({
  startTime: new Date().toISOString(),
  endTime: new Date(Date.now() + 7*24*60*60*1000).toISOString(),
  pageSize: 10
});
```

**Best Practices:**

- Always use suggestTime before creating meetings
- Set appropriate reminders
- Use Google Meet for virtual meetings
- Specify timezone for consistency
- Handle recurring events carefully

---

### Pattern 3: GitHub Repository Management

**Use Case:** Manage repository files and PRs

**Example Workflow:**

```javascript
// 1. Create documentation files
const files = [
  {
    path: "README.md",
    content: "# My Project\n\nProject description..."
  },
  {
    path: "docs/API.md",
    content: "# API Documentation\n\n..."
  },
  {
    path: "docs/INSTALL.md",
    content: "# Installation Guide\n\n..."
  }
];

await github.pushFiles({
  owner: "myorg",
  repo: "myrepo",
  branch: "main",
  files: files,
  message: "docs: add comprehensive documentation"
});

// 2. Create feature branch
await github.createBranch({
  owner: "myorg",
  repo: "myrepo",
  branch: "feature/new-feature",
  from_branch: "main"
});

// 3. Create pull request
const pr = await github.createPullRequest({
  owner: "myorg",
  repo: "myrepo",
  title: "feat: add new feature",
  head: "feature/new-feature",
  base: "main",
  body: "## Description\n\nDetailed description of the feature..."
});

console.log("PR created:", pr.number);

// 4. Enable auto-merge
await github.enablePrAutoMerge({
  owner: "myorg",
  repo: "myrepo",
  pullNumber: pr.number,
  mergeMethod: "SQUASH"
});
```

**Best Practices:**

- Use semantic commit messages
- Keep PRs focused and small
- Enable auto-merge for routine changes
- Test before pushing
- Use branch protection rules

---

### Pattern 4: Diagram Generation

**Use Case:** Create and validate Mermaid diagrams

**Example Workflow:**

```javascript
// 1. Create flowchart diagram
const flowchart = `
flowchart LR
    Start([Start]) --> Input[Get User Input]
    Input --> Process{Valid?}
    Process -->|Yes| Execute[Execute Process]
    Process -->|No| Error[Show Error]
    Error --> Input
    Execute --> Output[Return Result]
    Output --> End([End])
`;

const diagram1 = await mermaid.validateAndRenderMermaidDiagram({
  diagramCode: flowchart,
  title: "Process Flowchart"
});

// 2. Create sequence diagram
const sequence = `
sequenceDiagram
    participant User
    participant Client
    participant Server
    participant Database
    
    User->>Client: Click Button
    Client->>Server: POST /api/data
    Server->>Database: Query
    Database-->>Server: Result
    Server-->>Client: JSON Response
    Client-->>User: Display Data
`;

const diagram2 = await mermaid.validateAndRenderMermaidDiagram({
  diagramCode: sequence,
  title: "API Request Flow"
});

// 3. Create state diagram
const state = `
stateDiagram-v2
    [*] --> Idle
    Idle --> Loading: Click Load
    Loading --> Ready: Data Loaded
    Ready --> Processing: User Action
    Processing --> Ready: Done
    Ready --> [*]: Exit
`;

const diagram3 = await mermaid.validateAndRenderMermaidDiagram({
  diagramCode: state,
  title: "Application States"
});
```

**Best Practices:**

- Test diagram syntax in Mermaid Live
- Use descriptive labels and titles
- Keep diagrams focused and simple
- Document diagram purpose
- Version diagrams with code

---

## Integration Recipes

### Recipe 1: Email-to-Calendar Workflow

**Use Case:** Parse email and create calendar event

**Implementation:**

```javascript
async function emailToCalendarWorkflow(emailQuery) {
  try {
    // 1. Search for emails matching query
    console.log(`Searching for: ${emailQuery}`);
    const threads = await gmail.searchThreads({
      query: emailQuery,
      pageSize: 5
    });
    
    if (threads.threads.length === 0) {
      console.log("No matching emails found");
      return;
    }
    
    // 2. Get first matching thread details
    const thread = await gmail.getThread({
      threadId: threads.threads[0].threadId,
      messageFormat: "FULL_CONTENT"
    });
    
    const message = thread.messages[0];
    
    // 3. Parse email for meeting details
    const emailBody = message.plaintext_body;
    const meetingDetails = parseMeetingInfo(emailBody);
    
    if (!meetingDetails) {
      console.log("Could not extract meeting details");
      return;
    }
    
    // 4. Create calendar event
    const event = await calendar.createEvent({
      summary: meetingDetails.title || message.subject,
      description: emailBody,
      startTime: meetingDetails.startTime.toISOString(),
      endTime: meetingDetails.endTime.toISOString(),
      location: meetingDetails.location || "TBD",
      attendees: extractAttendees(message),
      addGoogleMeetUrl: true
    });
    
    // 5. Label email with calendar reference
    await gmail.labelThread({
      threadId: thread.threadId,
      labelIds: ["CALENDAR_SYNCED"]
    });
    
    console.log(`Event created: ${event.id}`);
    return event;
    
  } catch (error) {
    console.error("Error in email-to-calendar workflow:", error);
    throw error;
  }
}

// Helper function to parse meeting info
function parseMeetingInfo(emailBody) {
  // Simple parsing - in production use NLP or structured data
  const dateMatch = emailBody.match(/\b(\d{1,2}[-\/]\d{1,2}[-\/]\d{4})\b/);
  const timeMatch = emailBody.match(/\b(\d{1,2}:\d{2}\s?(?:AM|PM|am|pm))\b/);
  
  if (!dateMatch || !timeMatch) return null;
  
  const startTime = new Date(`${dateMatch[1]} ${timeMatch[1]}`);
  const endTime = new Date(startTime.getTime() + 60 * 60 * 1000);
  
  return {
    title: "Meeting",
    startTime,
    endTime,
    location: "Meeting Room"
  };
}

// Helper function to extract attendees
function extractAttendees(message) {
  const attendees = [];
  
  if (message.from) {
    attendees.push({
      email: message.from,
      displayName: message.from
    });
  }
  
  return attendees;
}

// Usage
await emailToCalendarWorkflow("subject:Meeting");
```

---

### Recipe 2: GitHub PR Status Reporter

**Use Case:** Monitor and report on PR status

**Implementation:**

```javascript
async function generatePRStatusReport(owner, repo) {
  try {
    // 1. Get all open PRs
    const prs = await github.listPullRequests({
      owner: owner,
      repo: repo,
      state: "open",
      perPage: 100
    });
    
    if (prs.length === 0) {
      console.log("No open pull requests");
      return;
    }
    
    // 2. Gather PR details
    const prDetails = await Promise.all(
      prs.map(async (pr) => {
        const fullPr = await github.pullRequestRead({
          method: "get",
          owner: owner,
          repo: repo,
          pullNumber: pr.number
        });
        
        const status = await github.pullRequestRead({
          method: "get_status",
          owner: owner,
          repo: repo,
          pullNumber: pr.number
        });
        
        const checks = await github.pullRequestRead({
          method: "get_check_runs",
          owner: owner,
          repo: repo,
          pullNumber: pr.number
        });
        
        return {
          number: pr.number,
          title: pr.title,
          author: pr.user.login,
          createdAt: pr.created_at,
          status: status.state,
          checksStatus: checks.total_count > 0 ? "Running" : "None",
          reviewsCount: fullPr.reviews ? fullPr.reviews.length : 0
        };
      })
    );
    
    // 3. Create summary
    let report = `# PR Status Report\n\n`;
    report += `**Repository:** ${owner}/${repo}\n`;
    report += `**Generated:** ${new Date().toISOString()}\n\n`;
    report += `## Summary\n`;
    report += `- Total Open PRs: ${prDetails.length}\n`;
    report += `- Failing Checks: ${prDetails.filter(p => p.status === 'failure').length}\n`;
    report += `- Pending Review: ${prDetails.filter(p => p.reviewsCount === 0).length}\n\n`;
    
    report += `## Details\n\n`;
    prDetails.forEach(pr => {
      report += `### #${pr.number}: ${pr.title}\n`;
      report += `- Author: ${pr.author}\n`;
      report += `- Status: ${pr.status}\n`;
      report += `- Reviews: ${pr.reviewsCount}\n`;
      report += `- Checks: ${pr.checksStatus}\n\n`;
    });
    
    // 4. Send report via email
    const draft = await gmail.createDraft({
      to: ["team@example.com"],
      subject: `PR Status Report - ${owner}/${repo}`,
      htmlBody: report
    });
    
    console.log(`Report created: ${draft.draftId}`);
    return draft;
    
  } catch (error) {
    console.error("Error generating PR report:", error);
    throw error;
  }
}

// Usage
await generatePRStatusReport("myorg", "myrepo");
```

---

### Recipe 3: Automated Documentation Generator

**Use Case:** Generate and update repository documentation

**Implementation:**

```javascript
async function generateDocumentation(owner, repo) {
  try {
    // 1. Get repository information
    const files = await github.getFileContents({
      owner: owner,
      repo: repo,
      path: "/"
    });
    
    // 2. Extract project structure
    const structure = generateProjectStructure(files);
    
    // 3. Get README if exists
    let readmeContent = "";
    try {
      const readmeFile = await github.getFileContents({
        owner: owner,
        repo: repo,
        path: "README.md"
      });
      readmeContent = Buffer.from(readmeFile.content, 'base64').toString();
    } catch {
      // README doesn't exist
    }
    
    // 4. Generate documentation
    const docs = {
      overview: generateOverview(owner, repo),
      structure: structure,
      setup: generateSetupGuide(),
      contributing: generateContributingGuide(),
      troubleshooting: generateTroubleshootingGuide()
    };
    
    // 5. Create documentation files
    const docsFiles = [
      {
        path: "docs/PROJECT_STRUCTURE.md",
        content: docs.structure
      },
      {
        path: "docs/SETUP.md",
        content: docs.setup
      },
      {
        path: "docs/CONTRIBUTING.md",
        content: docs.contributing
      },
      {
        path: "docs/TROUBLESHOOTING.md",
        content: docs.troubleshooting
      }
    ];
    
    // 6. Push documentation
    await github.pushFiles({
      owner: owner,
      repo: repo,
      branch: "main",
      files: docsFiles,
      message: "docs: auto-generate documentation"
    });
    
    console.log("Documentation generated successfully");
    
  } catch (error) {
    console.error("Error generating documentation:", error);
    throw error;
  }
}

function generateProjectStructure(files) {
  return `# Project Structure\n\n\`\`\`\n${JSON.stringify(files, null, 2)}\n\`\`\``;
}

function generateOverview(owner, repo) {
  return `# ${repo}\n\nProject overview...`;
}

function generateSetupGuide() {
  return `# Setup Guide\n\n## Installation\n\n...`;
}

function generateContributingGuide() {
  return `# Contributing\n\n## Guidelines\n\n...`;
}

function generateTroubleshootingGuide() {
  return `# Troubleshooting\n\n## Common Issues\n\n...`;
}

// Usage
await generateDocumentation("myorg", "myrepo");
```

---

### Recipe 4: Daily Digest Generator

**Use Case:** Aggregate email, calendar, and GitHub activity into daily digest

**Implementation:**

```javascript
async function generateDailyDigest(userEmail) {
  try {
    // 1. Get today's unread emails
    const unreadEmails = await gmail.searchThreads({
      query: `is:unread newer_than:1d`,
      pageSize: 20
    });
    
    // 2. Get today's calendar events
    const today = new Date();
    const tomorrow = new Date(today.getTime() + 24 * 60 * 60 * 1000);
    
    const todayEvents = await calendar.listEvents({
      startTime: today.toISOString(),
      endTime: tomorrow.toISOString(),
      pageSize: 20
    });
    
    // 3. Get user's GitHub activity
    const user = await github.getMe();
    
    // 4. Build digest
    let digest = `# Daily Digest - ${today.toDateString()}\n\n`;
    
    digest += `## Email Summary\n`;
    digest += `**Unread Messages:** ${unreadEmails.threads.length}\n\n`;
    
    unreadEmails.threads.forEach(thread => {
      digest += `- ${thread.messages[0].subject}\n`;
    });
    
    digest += `\n## Today's Calendar\n`;
    digest += `**Scheduled Events:** ${todayEvents.events.length}\n\n`;
    
    todayEvents.events.forEach(event => {
      digest += `- ${event.summary} (${event.startTime} - ${event.endTime})\n`;
    });
    
    digest += `\n## GitHub Activity\n`;
    digest += `**User:** ${user.login}\n`;
    digest += `**Repositories:** ${user.public_repos}\n`;
    digest += `**Followers:** ${user.followers}\n`;
    
    // 5. Send digest
    const draft = await gmail.createDraft({
      to: [userEmail],
      subject: `Daily Digest - ${today.toDateString()}`,
      htmlBody: digest
    });
    
    console.log(`Digest created: ${draft.draftId}`);
    return draft;
    
  } catch (error) {
    console.error("Error generating daily digest:", error);
    throw error;
  }
}

// Usage
await generateDailyDigest("user@example.com");
```

---

## Advanced Usage

### Advanced Pattern 1: Conditional Workflow Automation

**Use Case:** Create complex conditional workflows

**Implementation:**

```javascript
async function conditionalWorkflow() {
  try {
    // 1. Check calendar availability
    const availability = await calendar.listEvents({
      startTime: new Date().toISOString(),
      endTime: new Date(Date.now() + 2 * 60 * 60 * 1000).toISOString()
    });
    
    const isBusy = availability.events.length > 0;
    
    if (isBusy) {
      // 2A. If busy, defer emails to draft
      const draft = await gmail.createDraft({
        to: ["pending@example.com"],
        subject: "Currently Busy - Will Review Later",
        body: "Deferring this email to after current meeting..."
      });
      console.log("Deferred to draft:", draft.draftId);
      
    } else {
      // 2B. If available, create meeting immediately
      const event = await calendar.createEvent({
        summary: "Follow-up Discussion",
        startTime: new Date().toISOString(),
        endTime: new Date(Date.now() + 60 * 60 * 1000).toISOString(),
        addGoogleMeetUrl: true
      });
      
      // And notify via email
      await gmail.createDraft({
        to: ["colleague@example.com"],
        subject: "Quick sync scheduled",
        body: "I've scheduled a meeting for now: [meeting link]"
      });
      
      console.log("Event created:", event.id);
    }
    
  } catch (error) {
    console.error("Error in conditional workflow:", error);
    throw error;
  }
}
```

---

### Advanced Pattern 2: Batch Processing with Error Recovery

**Use Case:** Process large batches with robust error handling

**Implementation:**

```javascript
async function batchProcessWithRecovery(items, processor, options = {}) {
  const {
    batchSize = 10,
    retryCount = 3,
    retryDelay = 1000,
    timeout = 30000
  } = options;
  
  const results = [];
  const errors = [];
  
  for (let i = 0; i < items.length; i += batchSize) {
    const batch = items.slice(i, i + batchSize);
    
    console.log(`Processing batch ${Math.floor(i / batchSize) + 1} of ${Math.ceil(items.length / batchSize)}`);
    
    const batchResults = await Promise.allSettled(
      batch.map(item => processWithRetry(item, processor, retryCount, retryDelay, timeout))
    );
    
    batchResults.forEach((result, index) => {
      if (result.status === 'fulfilled') {
        results.push(result.value);
      } else {
        errors.push({
          item: batch[index],
          error: result.reason
        });
      }
    });
    
    // Add delay between batches to avoid rate limiting
    if (i + batchSize < items.length) {
      await new Promise(resolve => setTimeout(resolve, 500));
    }
  }
  
  console.log(`Completed: ${results.length} succeeded, ${errors.length} failed`);
  
  return { results, errors };
}

async function processWithRetry(item, processor, retryCount, retryDelay, timeout) {
  let lastError;
  
  for (let attempt = 0; attempt < retryCount; attempt++) {
    try {
      return await Promise.race([
        processor(item),
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error("Timeout")), timeout)
        )
      ]);
    } catch (error) {
      lastError = error;
      
      if (attempt < retryCount - 1) {
        const delay = retryDelay * Math.pow(2, attempt);
        console.log(`Retry attempt ${attempt + 1} after ${delay}ms`);
        await new Promise(resolve => setTimeout(resolve, delay));
      }
    }
  }
  
  throw lastError;
}

// Usage
const emails = [
  { to: "user1@example.com", subject: "Message 1" },
  { to: "user2@example.com", subject: "Message 2" },
  // ... more emails
];

const { results, errors } = await batchProcessWithRecovery(
  emails,
  async (email) => await gmail.createDraft(email),
  { batchSize: 5, retryCount: 3 }
);

if (errors.length > 0) {
  console.log("Failed items:", errors.map(e => e.item));
}
```

---

### Advanced Pattern 3: Caching and Memoization

**Use Case:** Optimize performance with intelligent caching

**Implementation:**

```javascript
class MemoizedGitHubClient {
  constructor(client, cacheDuration = 5 * 60 * 1000) {
    this.client = client;
    this.cacheDuration = cacheDuration;
    this.cache = new Map();
  }
  
  getCacheKey(method, params) {
    return `${method}:${JSON.stringify(params)}`;
  }
  
  isExpired(timestamp) {
    return Date.now() - timestamp > this.cacheDuration;
  }
  
  async get(method, params) {
    const key = this.getCacheKey(method, params);
    const cached = this.cache.get(key);
    
    if (cached && !this.isExpired(cached.timestamp)) {
      console.log(`Cache hit: ${key}`);
      return cached.value;
    }
    
    console.log(`Cache miss: ${key}`);
    const value = await this.client[method](params);
    
    this.cache.set(key, {
      value,
      timestamp: Date.now()
    });
    
    return value;
  }
  
  clearCache(pattern) {
    if (!pattern) {
      this.cache.clear();
      return;
    }
    
    for (const key of this.cache.keys()) {
      if (key.includes(pattern)) {
        this.cache.delete(key);
      }
    }
  }
}

// Usage
const cachedGitHub = new MemoizedGitHubClient(github);

// First call - fetches from API
const user1 = await cachedGitHub.get('getMe', {});

// Second call - returns from cache
const user2 = await cachedGitHub.get('getMe', {});

// Clear cache for specific pattern
cachedGitHub.clearCache('getMe');
```

---

## Performance Optimization

### Rate Limit Management

```javascript
class RateLimitAwareClient {
  constructor(client, maxConcurrent = 1) {
    this.client = client;
    this.maxConcurrent = maxConcurrent;
    this.activeRequests = 0;
    this.queue = [];
    this.rateLimitRemaining = Infinity;
    this.rateLimitReset = 0;
  }
  
  async execute(fn) {
    // Check if rate limited
    if (this.rateLimitRemaining === 0) {
      const waitTime = Math.max(0, this.rateLimitReset - Date.now());
      console.log(`Rate limited. Waiting ${waitTime}ms...`);
      await new Promise(resolve => setTimeout(resolve, waitTime));
    }
    
    // Queue if at capacity
    if (this.activeRequests >= this.maxConcurrent) {
      await new Promise(resolve => this.queue.push(resolve));
    }
    
    this.activeRequests++;
    
    try {
      const response = await fn();
      
      // Update rate limit info from headers
      if (response.headers) {
        this.rateLimitRemaining = parseInt(
          response.headers['x-ratelimit-remaining'] || Infinity
        );
        this.rateLimitReset = parseInt(
          response.headers['x-ratelimit-reset'] || 0
        ) * 1000;
      }
      
      return response;
    } finally {
      this.activeRequests--;
      const resolve = this.queue.shift();
      if (resolve) resolve();
    }
  }
}

// Usage
const limitedGitHub = new RateLimitAwareClient(github);

// Requests are automatically throttled
await limitedGitHub.execute(() => github.getMe());
```

---

### Batch Operations Optimization

```javascript
// Instead of creating individual files
for (const file of files) {
  await github.createOrUpdateFile({
    owner, repo,
    path: file.path,
    content: file.content,
    message: `docs: update ${file.path}`
  });
}

// Use batch push
await github.pushFiles({
  owner, repo,
  branch: "main",
  files: files.map(f => ({
    path: f.path,
    content: f.content
  })),
  message: "docs: update multiple files"
});
```

---

## Security Hardening

### Credential Management

```javascript
// Bad: Hardcoded credentials
const github = new GitHubClient("ghp_xxxxxxxxxxxxxxxxxxxx");

// Good: Environment variables
const github = new GitHubClient(process.env.GITHUB_TOKEN);

// Better: Encrypted credential store
const credentials = await loadCredentialsFromSecureStore("github");
const github = new GitHubClient(credentials.token);

// Best: Temporary credentials with auto-refresh
class SecureGitHubClient {
  constructor() {
    this.token = null;
    this.tokenExpiry = null;
    this.refreshToken = process.env.GITHUB_REFRESH_TOKEN;
  }
  
  async getToken() {
    if (this.token && this.tokenExpiry > Date.now()) {
      return this.token;
    }
    
    this.token = await this.refreshToken();
    this.tokenExpiry = Date.now() + 3600000; // 1 hour
    
    return this.token;
  }
  
  async refreshToken() {
    // Implement token refresh logic
    const newToken = await getNewTokenFromGitHub(this.refreshToken);
    return newToken;
  }
}
```

---

### Input Validation

```javascript
function validateEmailAddress(email) {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  if (!emailRegex.test(email)) {
    throw new Error(`Invalid email address: ${email}`);
  }
}

function validateRepositoryName(name) {
  const validName = /^[a-zA-Z0-9._-]+$/.test(name);
  if (!validName) {
    throw new Error(`Invalid repository name: ${name}`);
  }
}

function validateISO8601DateTime(dateString) {
  try {
    const date = new Date(dateString);
    if (isNaN(date.getTime())) throw new Error();
    // Verify it's actually ISO 8601
    if (dateString !== date.toISOString().replace(/\.\d{3}Z/, 'Z')) {
      throw new Error();
    }
    return true;
  } catch {
    throw new Error(`Invalid ISO 8601 date-time: ${dateString}`);
  }
}

// Usage with validation
async function sendSecureEmail(to, subject, body) {
  validateEmailAddress(to);
  
  if (!subject || subject.length === 0) {
    throw new Error("Subject cannot be empty");
  }
  
  return await gmail.createDraft({
    to: [to],
    subject: subject.substring(0, 1000),
    body: body.substring(0, 100000)
  });
}
```

---

### Least Privilege Access

```javascript
// Bad: Request all scopes
const paToken = "ghp_with_repo,read:user,user:email,read:org,workflow";

// Good: Request only necessary scopes for the task
// For read-only operations
const readOnlyToken = "ghp_with_public_repo,read:user";

// For specific repository
const repoSpecificToken = "ghp_with_limited_repo_access";

// For automated workflows
const workflowToken = "ghp_with_workflow,read:repo_hook";
```

---

## Troubleshooting Guide

### Common Issues and Solutions

#### Issue: "UNAUTHENTICATED" Error

**Symptom:** 401 Unauthorized responses

**Diagnosis:**

```javascript
async function diagnoseAuthIssue() {
  try {
    const user = await github.getMe();
    console.log("Authentication OK");
  } catch (error) {
    if (error.status === 401) {
      console.log("Issue: Invalid or expired token");
      console.log("Solution: Generate new token or check expiration");
    }
  }
}
```

**Solutions:**

1. Verify token hasn't expired
2. Check token has required scopes
3. Regenerate token from source
4. Clear cached credentials

---

#### Issue: "RATE_LIMIT_EXCEEDED" Error

**Symptom:** 429 Too Many Requests

**Diagnosis & Solution:**

```javascript
async function handleRateLimit(error) {
  if (error.status === 429) {
    const resetTime = error.headers['x-ratelimit-reset'];
    const waitTime = new Date(resetTime * 1000) - new Date();
    
    console.log(`Rate limited. Resetting in ${waitTime}ms`);
    
    // Wait and retry
    await new Promise(resolve => setTimeout(resolve, waitTime));
    return await retryOperation();
  }
}
```

---

#### Issue: "NOT_FOUND" Error for Resources

**Symptom:** 404 errors for threads, events, or repositories

**Diagnosis:**

```javascript
async function diagnoseNotFound(owner, repo) {
  try {
    // Verify repository exists
    const files = await github.getFileContents({ owner, repo, path: "/" });
    console.log("Repository found");
  } catch (error) {
    if (error.status === 404) {
      // Repository doesn't exist or no access
      console.log("Issue: Repository not found or no access");
      console.log("Solutions:");
      console.log("1. Verify owner/repo names");
      console.log("2. Check token has access to repository");
      console.log("3. For private repos, verify token has 'repo' scope");
    }
  }
}
```

---

#### Issue: "INVALID_ARGUMENT" for Email Operations

**Symptom:** 400 Bad Request for email tools

**Common Causes & Solutions:**

```javascript
// Bad: Display name format
const email = "John Doe <john@example.com>";  // ✗

// Good: Plain email only
const email = "john@example.com";  // ✓

// Bad: Multiple recipients as string
await gmail.createDraft({
  to: "john@example.com,jane@example.com"  // ✗
});

// Good: Array of strings
await gmail.createDraft({
  to: ["john@example.com", "jane@example.com"]  // ✓
});

// Bad: Invalid time zone
const event = await calendar.createEvent({
  summary: "Meeting",
  startTime: "2026-06-15T14:00:00",  // No timezone
  endTime: "2026-06-15T15:00:00",
  timeZone: "Invalid/Zone"  // ✗
});

// Good: Valid IANA timezone
const event = await calendar.createEvent({
  summary: "Meeting",
  startTime: "2026-06-15T14:00:00Z",
  endTime: "2026-06-15T15:00:00Z",
  timeZone: "America/Los_Angeles"  // ✓
});
```

---

### Debug Logging

```javascript
class DebugClient {
  constructor(client, debugMode = true) {
    this.client = client;
    this.debugMode = debugMode;
    this.requestLog = [];
  }
  
  async call(method, params) {
    const requestId = Date.now();
    
    if (this.debugMode) {
      console.log(`[${requestId}] → ${method}`, params);
    }
    
    const startTime = Date.now();
    
    try {
      const result = await this.client[method](params);
      const duration = Date.now() - startTime;
      
      if (this.debugMode) {
        console.log(`[${requestId}] ← ${method} (${duration}ms)`, result);
      }
      
      this.requestLog.push({
        requestId,
        method,
        status: 'success',
        duration,
        timestamp: new Date()
      });
      
      return result;
    } catch (error) {
      const duration = Date.now() - startTime;
      
      if (this.debugMode) {
        console.error(`[${requestId}] ✗ ${method} (${duration}ms)`, error);
      }
      
      this.requestLog.push({
        requestId,
        method,
        status: 'error',
        error: error.message,
        duration,
        timestamp: new Date()
      });
      
      throw error;
    }
  }
  
  getLog() {
    return this.requestLog;
  }
  
  exportLog(format = 'json') {
    if (format === 'json') {
      return JSON.stringify(this.requestLog, null, 2);
    } else if (format === 'csv') {
      const header = 'RequestID,Method,Status,Duration(ms),Timestamp\n';
      const rows = this.requestLog.map(row =>
        `${row.requestId},${row.method},${row.status},${row.duration},${row.timestamp}`
      ).join('\n');
      return header + rows;
    }
  }
}

// Usage
const debugGitHub = new DebugClient(github, true);
await debugGitHub.call('getMe', {});
console.log(debugGitHub.exportLog('csv'));
```

---

## Conclusion

These integration patterns and best practices provide a foundation for building reliable, efficient, and secure automation using MCP servers. Start with simple patterns and gradually adopt more advanced techniques as your needs grow.

**Key Takeaways:**

1. Always validate input and handle errors gracefully
2. Respect rate limits and implement backoff strategies
3. Use batch operations when possible
4. Cache results appropriately
5. Follow security best practices
6. Log and monitor operations
7. Test thoroughly before deployment

---

**Document Version:** 1.0
**Last Updated:** 2026-06-14
**Maintainer:** Claude Code Documentation Team
