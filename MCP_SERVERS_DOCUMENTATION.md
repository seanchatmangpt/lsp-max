# Claude Code MCP Servers - Comprehensive Documentation

## Overview

This document provides complete documentation for all Model Context Protocol (MCP) servers integrated with Claude Code. MCP servers extend Claude's capabilities by providing access to external services and APIs through a standardized protocol.

**Current Version:** 1.0
**Last Updated:** 2026-06-14

---

## Table of Contents

1. [MCP Servers Registry](#mcp-servers-registry)
2. [Gmail Server](#gmail-server)
3. [Google Calendar Server](#google-calendar-server)
4. [GitHub Server](#github-server)
5. [Mermaid Chart Server](#mermaid-chart-server)
6. [Integration Guide](#integration-guide)
7. [Authentication & Security](#authentication--security)
8. [Error Handling](#error-handling)
9. [Rate Limiting](#rate-limiting)
10. [Best Practices](#best-practices)

---

## MCP Servers Registry

### Available Servers

| Server | Tools | Authentication | Status |
|--------|-------|-----------------|--------|
| **Gmail** | 11 tools | OAuth 2.0 | Active |
| **Google Calendar** | 8 tools | OAuth 2.0 | Active |
| **GitHub** | 47 tools | Personal Access Token (PAT) | Active |
| **Mermaid Chart** | 1 tool | None | Active |

**Total Tools Available:** 67

---

## Gmail Server

### Server Information

- **Name:** `mcp__Gmail`
- **Total Tools:** 11
- **Authentication:** OAuth 2.0 (Google Workspace)
- **API Base:** Google Gmail API v1

### Tools

#### 1. `create_draft`

Create a new draft email in the authenticated user's Gmail account.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `to` | string[] | Yes | Primary recipients (email addresses only, no "Name <email>" format) |
| `subject` | string | No | Email subject line |
| `body` | string | No | Plain-text email body |
| `htmlBody` | string | No | HTML-formatted email body (overrides plain text) |
| `cc` | string[] | No | Carbon copy recipients |
| `bcc` | string[] | No | Blind carbon copy recipients |
| `replyToMessageId` | string | No | ID of message to reply to (appends to original) |
| `attachments` | Attachment[] | No | File attachments (max 25MB total) |

**Attachment Object:**

```json
{
  "filename": "string",
  "content": "base64-encoded string",
  "mimeType": "string (default: application/octet-stream)",
  "inline": "boolean (default: false)"
}
```

**Returns:**

```json
{
  "draftId": "string"
}
```

**Example:**

```json
{
  "to": ["user@example.com"],
  "subject": "Project Update",
  "body": "Here's the latest project update.",
  "cc": ["manager@example.com"]
}
```

**Limitations:**
- Attachments must be under 25MB combined
- Recipient format is email-only (no display names)
- No inline attachments support yet

---

#### 2. `list_drafts`

List draft emails from the authenticated user's Gmail account.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | No | Gmail search syntax query |
| `pageSize` | integer | No | Max results per page (1-50, default: 20) |
| `pageToken` | string | No | Token for pagination |

**Supported Query Operators:**

- `subject:text` - Search subject line
- `from:email@example.com` - Filter by sender
- `to:email@example.com` - Filter by recipient
- `newer_than:7d` - Date range (7d, 1m, 1y)
- `older_than:7d` - Date range
- `has:attachment` - Filter with attachments
- `is:unread` - Unread status

**Returns:**

```json
{
  "drafts": [
    {
      "draftId": "string",
      "subject": "string",
      "snippet": "string (preview)"
    }
  ],
  "pageToken": "string (if more results available)"
}
```

**Example Query:**

```
subject:OneMCP Update from:gduser1@workspacesamples.dev newer_than:7d
```

---

#### 3. `list_labels`

List all user-defined labels in the Gmail account.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `pageSize` | integer | No | Max labels per page |
| `pageToken` | string | No | Pagination token |

**Built-in System Labels:**

```
INBOX, TRASH, SPAM, STARRED, UNREAD, IMPORTANT, CHAT, DRAFT, SENT
```

**Returns:**

```json
{
  "labels": [
    {
      "id": "string",
      "displayName": "string",
      "type": "user|system"
    }
  ],
  "pageToken": "string"
}
```

---

#### 4. `create_label`

Create a new label in Gmail.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `displayName` | string | Yes | Label name (supports nested with `/`, e.g., "Projects/Alpha") |
| `autoCreateParentLabels` | boolean | No | Auto-create parent labels (default: true) |
| `color` | LabelColor | No | Label color |

**Color Object:**

```json
{
  "backgroundColor": "hex string (#ffffff)",
  "textColor": "hex string (#000000)"
}
```

**Valid Color Values:**

```
#000000, #434343, #666666, #999999, #cccccc, #efefef, #f3f3f3, #ffffff,
#fb4c2f, #ffad47, #fad165, #16a766, #43d692, #4a86e8, #a479e2, #f691b3,
#f6c5be, #ffe6c7, #fef1d1, #b9e4d0, #c6f3de, #c9daf8, #e4d7f5, #fcdee8,
#efa093, #ffd6a2, #fce8b3, #89d3b2, #a0eac9, #a4c2f4, #d0bcf1, #fbc8d9,
#e66550, #ffbc6b, #fcda83, #44b984, #68dfa9, #6d9eeb, #b694e8, #f7a7c0,
#cc3a21, #eaa041, #f2c960, #149e60, #3dc789, #3c78d8, #8e63ce, #e07798,
#ac2b16, #cf8933, #d5ae49, #0b804b, #2a9c68, #285bac, #653e9b, #b65775,
#822111, #a46a21, #aa8831, #076239, #1a764d, #1c4587, #41236d, #83334c,
#464646, #e7e7e7, (and 30+ more)
```

**Returns:**

```json
{
  "labelId": "string"
}
```

**Example:**

```json
{
  "displayName": "Projects/Client-A/Sprint-1",
  "autoCreateParentLabels": true,
  "color": {
    "backgroundColor": "#4a86e8",
    "textColor": "#ffffff"
  }
}
```

---

#### 5. `update_label`

Modify an existing label's name and color.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `labelId` | string | Yes | Label ID to modify |
| `displayName` | string | No | New display name |
| `color` | LabelColor | No | New color |

**Returns:**

```json
{
  "labelId": "string",
  "displayName": "string"
}
```

---

#### 6. `delete_label`

Delete a label from Gmail.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `labelId` | string | Yes | Label ID to delete |

**Returns:** Empty success response

---

#### 7. `search_threads`

List and filter email threads from Gmail.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | No | Gmail search syntax |
| `pageSize` | integer | No | Max threads (1-50, default: 20) |
| `pageToken` | string | No | Pagination token |
| `includeTrash` | boolean | No | Include trash (default: false) |

**Query Syntax Examples:**

```
from:user@example.com                          # By sender
to:user@example.com                            # By recipient
subject:"meeting"                              # By subject
has:attachment                                 # With attachments
is:unread                                      # Unread only
newer_than:7d                                  # Last 7 days
label:ProjectX                                 # Specific label (use ID)
category:primary|social|promotions|updates     # By category
```

**Returns:**

```json
{
  "threads": [
    {
      "threadId": "string",
      "messages": [
        {
          "messageId": "string",
          "subject": "string",
          "from": "string",
          "date": "ISO8601",
          "snippet": "string"
        }
      ]
    }
  ],
  "pageToken": "string"
}
```

---

#### 8. `get_thread`

Retrieve full details of a specific email thread.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `threadId` | string | Yes | Thread ID to retrieve |
| `messageFormat` | string | No | MINIMAL or FULL_CONTENT (default: FULL_CONTENT) |

**Message Format Options:**

- `MINIMAL` - Subject, from, to, cc, date headers only
- `FULL_CONTENT` - Includes message body and attachment info

**Returns:**

```json
{
  "threadId": "string",
  "messages": [
    {
      "messageId": "string",
      "subject": "string",
      "from": "string",
      "to": "string[]",
      "cc": "string[]",
      "date": "ISO8601",
      "plaintext_body": "string",
      "html_body": "string",
      "attachment_ids": "string[]"
    }
  ]
}
```

---

#### 9. `label_thread`

Add labels to an entire email thread.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `threadId` | string | Yes | Thread ID |
| `labelIds` | string[] | Yes | Label IDs to add (user-defined or system) |

**System Label IDs:**

```
INBOX, TRASH, SPAM, STARRED, UNREAD, IMPORTANT
```

**Returns:** Success response (empty)

---

#### 10. `unlabel_thread`

Remove labels from an entire email thread.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `threadId` | string | Yes | Thread ID |
| `labelIds` | string[] | Yes | Label IDs to remove |

**Returns:** Success response (empty)

---

#### 11. `label_message` / `unlabel_message`

Add or remove labels from a specific message.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `messageId` | string | Yes | Message ID |
| `labelIds` | string[] | Yes | Label IDs |

**Returns:** Success response (empty)

---

### Gmail Authentication

**Required Scopes:**

```
https://www.googleapis.com/auth/gmail.modify
https://www.googleapis.com/auth/gmail.readonly
```

**Setup:**

1. Visit Google Cloud Console
2. Create OAuth 2.0 credentials
3. Authorize Claude Code application
4. Credentials stored securely in Claude's auth system

---

### Gmail Error Codes

| Error | Cause | Resolution |
|-------|-------|-----------|
| `UNAUTHENTICATED` | Missing/invalid OAuth token | Re-authenticate with Google |
| `INVALID_ARGUMENT` | Malformed email address | Use plain email format only |
| `NOT_FOUND` | Thread/message doesn't exist | Verify ID with search_threads |
| `PERMISSION_DENIED` | Insufficient OAuth scopes | Check scopes, re-authorize |
| `QUOTA_EXCEEDED` | Rate limit hit | Wait before retrying |

---

## Google Calendar Server

### Server Information

- **Name:** `mcp__Google_Calendar`
- **Total Tools:** 8
- **Authentication:** OAuth 2.0 (Google Workspace)
- **API Base:** Google Calendar API v3

### Tools

#### 1. `list_calendars`

List all calendars the user has access to.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `pageSize` | integer | No | Max results (1-250, default: 100) |
| `pageToken` | string | No | Pagination token |

**Returns:**

```json
{
  "calendars": [
    {
      "id": "string (primary or email)",
      "summary": "string (name)",
      "description": "string",
      "timeZone": "IANA string (e.g., America/Los_Angeles)",
      "primary": "boolean"
    }
  ],
  "pageToken": "string"
}
```

---

#### 2. `list_events`

List calendar events with filtering and pagination.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `calendarId` | string | No | Calendar ID (default: primary) |
| `startTime` | string | No | ISO 8601 start (e.g., 2026-06-03T10:00:00) |
| `endTime` | string | No | ISO 8601 end (exclusive) |
| `pageSize` | integer | No | Max events (1-250, default: 100) |
| `pageToken` | string | No | Pagination token |
| `fullText` | string | No | Free-text search (title, description, location) |
| `orderBy` | string | No | `default`, `startTime`, `startTimeDesc`, `lastModified` |
| `eventType` | string[] | No | Filter by type (DEFAULT, OUT_OF_OFFICE, FOCUS_TIME, WORKING_LOCATION, BIRTHDAY, FROM_GMAIL) |
| `timeZone` | string | No | IANA time zone for response |

**Returns:**

```json
{
  "events": [
    {
      "id": "string",
      "summary": "string",
      "description": "string",
      "startTime": "ISO8601",
      "endTime": "ISO8601",
      "location": "string",
      "attendees": [
        {
          "email": "string",
          "displayName": "string",
          "responseStatus": "needsAction|declined|tentative|accepted"
        }
      ],
      "organizer": {
        "email": "string"
      }
    }
  ],
  "pageToken": "string"
}
```

---

#### 3. `get_event`

Retrieve details of a specific event.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `eventId` | string | Yes | Event ID |
| `calendarId` | string | No | Calendar ID (default: primary) |

**Returns:**

```json
{
  "id": "string",
  "summary": "string",
  "description": "string",
  "startTime": "ISO8601",
  "endTime": "ISO8601",
  "location": "string",
  "visibility": "default|public|private",
  "status": "confirmed|tentative|cancelled",
  "attendees": [
    {
      "email": "string",
      "displayName": "string",
      "responseStatus": "needsAction|declined|tentative|accepted",
      "organizer": "boolean",
      "optionalAttendee": "boolean"
    }
  ]
}
```

---

#### 4. `create_event`

Create a new calendar event.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `summary` | string | Yes | Event title |
| `startTime` | string | Yes | ISO 8601 start time |
| `endTime` | string | Yes | ISO 8601 end time |
| `calendarId` | string | No | Calendar ID (default: primary) |
| `description` | string | No | Event description (HTML supported) |
| `location` | string | No | Geographic location |
| `attendees` | Attendee[] | No | Event attendees |
| `allDay` | boolean | No | All-day event (default: false) |
| `visibility` | string | No | `default`, `public`, `private` |
| `eventType` | string | No | DEFAULT, OUT_OF_OFFICE, FOCUS_TIME, WORKING_LOCATION, BIRTHDAY |
| `addGoogleMeetUrl` | boolean | No | Attach Google Meet (default: false) |
| `googleMeetUrl` | string | No | Attach existing Google Meet URL |
| `recurrenceData` | string[] | No | RRULE/RDATE/EXDATE per RFC 5545 |
| `availability` | string | No | AVAILABILITY_BUSY or AVAILABILITY_FREE |
| `overrideReminders` | Reminder[] | No | Custom reminders |
| `colorId` | string | No | Event color (1-11) |
| `attachments` | Attachment[] | No | File attachments |
| `guestPermissions` | GuestPermissions | No | Guest capabilities |

**Attendee Object:**

```json
{
  "email": "string",
  "displayName": "string",
  "optionalAttendee": "boolean",
  "additionalGuests": "integer"
}
```

**Reminder Object:**

```json
{
  "method": "email|popup",
  "minutes": "integer (minutes before)"
}
```

**GuestPermissions Object:**

```json
{
  "guestsCanInviteOthers": "boolean",
  "guestsCanModify": "boolean",
  "guestsCanSeeGuests": "boolean"
}
```

**Color IDs:**

```
1: Lavender, 2: Sage, 3: Grape, 4: Flamingo, 5: Banana, 6: Tangerine,
7: Peacock, 8: Graphite, 9: Blueberry, 10: Basil, 11: Tomato
```

**Example:**

```json
{
  "summary": "Team Meeting",
  "startTime": "2026-06-15T14:00:00Z",
  "endTime": "2026-06-15T15:00:00Z",
  "attendees": [
    {"email": "john@example.com"},
    {"email": "jane@example.com", "optionalAttendee": true}
  ],
  "addGoogleMeetUrl": true,
  "overrideReminders": [
    {"method": "email", "minutes": 15}
  ]
}
```

---

#### 5. `update_event`

Modify an existing calendar event.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `eventId` | string | Yes | Event ID to update |
| `calendarId` | string | No | Calendar ID (default: primary) |
| `summary` | string | No | New title |
| `startTime` | string | No | New start time |
| `endTime` | string | No | New end time |
| `description` | string | No | New description |
| `location` | string | No | New location |
| `addedAttendees` | Attendee[] | No | Attendees to add |
| `removedAttendeeEmails` | string[] | No | Attendee emails to remove |
| `allDay` | boolean | No | Convert to/from all-day |
| `visibility` | string | No | New visibility |
| `availability` | string | No | New availability status |
| `overrideReminders` | Reminder[] | No | Replace all reminders |
| `colorId` | string | No | New color |
| `addedAttachments` | Attachment[] | No | Attachments to add |
| `removedAttachmentFileUrls` | string[] | No | File URLs to remove |

**Note:** Only provided fields are updated; omitted fields remain unchanged.

**Returns:**

```json
{
  "id": "string",
  "updated": "ISO8601"
}
```

---

#### 6. `delete_event`

Delete a calendar event.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `eventId` | string | Yes | Event ID |
| `calendarId` | string | No | Calendar ID (default: primary) |
| `notificationLevel` | string | No | NONE, EXTERNAL_ONLY, ALL (default: EXTERNAL_ONLY) |

**Returns:** Empty success response

---

#### 7. `respond_to_event`

Accept, decline, or tentatively accept an event.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `eventId` | string | Yes | Event ID |
| `responseStatus` | string | Yes | `accepted`, `declined`, `tentative` |
| `calendarId` | string | No | Calendar ID (default: primary) |
| `responseComment` | string | No | Optional comment |
| `notificationLevel` | string | No | Notification control |

**Returns:** Updated event object

---

#### 8. `suggest_time`

Find available time slots across multiple calendars.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `attendeeEmails` | string[] | Yes | Email addresses to check (use 'primary' for user's calendar) |
| `startTime` | string | Yes | ISO 8601 search start |
| `endTime` | string | Yes | ISO 8601 search end |
| `durationMinutes` | integer | No | Minimum slot duration (default: 30) |
| `timeZone` | string | No | IANA time zone |
| `preferences` | Preferences | No | Time slot preferences |

**Preferences Object:**

```json
{
  "startHour": "HH:MM (e.g., 09:00)",
  "endHour": "HH:MM (e.g., 17:00)",
  "excludeWeekends": "boolean",
  "pageSize": "integer (default: 5, max results)"
}
```

**Returns:**

```json
{
  "suggestedSlots": [
    {
      "startTime": "ISO8601",
      "endTime": "ISO8601"
    }
  ]
}
```

**Example:**

```json
{
  "attendeeEmails": ["john@example.com", "jane@example.com"],
  "startTime": "2026-06-16T00:00:00Z",
  "endTime": "2026-06-20T23:59:59Z",
  "durationMinutes": 60,
  "preferences": {
    "startHour": "09:00",
    "endHour": "17:00",
    "excludeWeekends": true
  }
}
```

---

### Google Calendar Error Codes

| Error | Cause | Resolution |
|-------|-------|-----------|
| `UNAUTHENTICATED` | Missing/invalid OAuth token | Re-authenticate |
| `NOT_FOUND` | Event/calendar doesn't exist | Verify IDs |
| `INVALID_ARGUMENT` | Invalid time format or parameters | Check ISO 8601 format |
| `PERMISSION_DENIED` | No access to calendar | Check calendar sharing |
| `RESOURCE_EXHAUSTED` | Rate limit | Implement exponential backoff |

---

## GitHub Server

### Server Information

- **Name:** `mcp__github`
- **Total Tools:** 47
- **Authentication:** Personal Access Token (PAT) with scope: `repo`, `read:user`, `user:email`, `read:org`, `workflow`
- **API Base:** GitHub REST API v3 + GraphQL API

### Tool Categories

#### A. Repository Management (7 tools)

##### 1. `create_repository`

Create a new GitHub repository.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | Yes | Repository name |
| `description` | string | No | Repository description |
| `private` | boolean | No | Private repository (default: false) |
| `organization` | string | No | Organization name (omit for personal) |
| `autoInit` | boolean | No | Initialize with README |

**Example:**

```json
{
  "name": "mcp-documentation",
  "description": "MCP server documentation",
  "private": false,
  "autoInit": true
}
```

---

##### 2. `fork_repository`

Fork a repository to personal account or organization.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Original owner |
| `repo` | string | Yes | Original repository name |
| `organization` | string | No | Target organization |

---

##### 3. `list_branches`

List all branches in a repository.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `page` | integer | No | Page number (min: 1) |
| `perPage` | integer | No | Results per page (1-100, default: 30) |

**Returns:**

```json
{
  "branches": [
    {
      "name": "string",
      "commit": {
        "sha": "string",
        "url": "string"
      },
      "protected": "boolean"
    }
  ]
}
```

---

##### 4. `create_branch`

Create a new branch in a repository.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `branch` | string | Yes | New branch name |
| `from_branch` | string | No | Source branch (default: repo default) |

---

##### 5. `get_file_contents`

Retrieve file or directory contents.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `path` | string | No | File/directory path (default: /) |
| `ref` | string | No | Branch/tag name |
| `sha` | string | No | Commit SHA |

**Returns:**

```json
{
  "name": "string",
  "path": "string",
  "content": "base64-encoded or text",
  "encoding": "base64|utf-8",
  "size": "integer (bytes)"
}
```

---

##### 6. `create_or_update_file`

Create or update a file in repository.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `path` | string | Yes | File path |
| `content` | string | Yes | File content |
| `message` | string | Yes | Commit message |
| `branch` | string | Yes | Target branch |
| `sha` | string | No | **Required for updates** (get via git rev-parse) |

**Important:** For existing file updates, must provide SHA of original:

```bash
git rev-parse <branch>:<path to file>
```

---

##### 7. `delete_file`

Delete a file from repository.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `path` | string | Yes | File path |
| `message` | string | Yes | Commit message |
| `branch` | string | Yes | Target branch |

---

#### B. File Operations (2 tools)

##### 1. `push_files`

Push multiple files in a single commit.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `branch` | string | Yes | Target branch |
| `files` | File[] | Yes | Array of files to push |
| `message` | string | Yes | Commit message |

**File Object:**

```json
{
  "path": "string (file path)",
  "content": "string (file content)"
}
```

**Example:**

```json
{
  "owner": "username",
  "repo": "my-repo",
  "branch": "main",
  "message": "docs: update API documentation",
  "files": [
    {
      "path": "docs/api.md",
      "content": "# API Documentation\n..."
    },
    {
      "path": "docs/examples.md",
      "content": "# Examples\n..."
    }
  ]
}
```

---

#### C. Commit Operations (4 tools)

##### 1. `get_commit`

Get details about a specific commit.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `sha` | string | Yes | Commit SHA, branch, or tag |
| `detail` | string | No | none, stats (default), full_patch |

**Returns:**

```json
{
  "sha": "string",
  "message": "string",
  "author": {
    "name": "string",
    "email": "string",
    "date": "ISO8601"
  },
  "files": [
    {
      "filename": "string",
      "status": "added|modified|removed|renamed",
      "additions": "integer",
      "deletions": "integer",
      "changes": "integer",
      "patch": "string (if detail=full_patch)"
    }
  ]
}
```

---

##### 2. `list_commits`

List commits from a branch.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `sha` | string | No | Commit SHA, branch, or tag |
| `path` | string | No | Filter by file path |
| `author` | string | No | Author username or email |
| `since` | string | No | ISO 8601 date filter |
| `until` | string | No | ISO 8601 date filter |
| `page` | integer | No | Pagination |
| `perPage` | integer | No | Results per page (1-100, default: 30) |

---

##### 3. `search_commits`

Search for commits across repositories.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | Yes | Search query with qualifiers |
| `sort` | string | No | author-date, committer-date |
| `order` | string | No | asc, desc |
| `page` | integer | No | Pagination |
| `perPage` | integer | No | Results per page (1-100) |

**Query Syntax:**

```
repo:owner/repo                                # Scope to repository
org:orgname                                    # Scope to organization
author:username                                # Filter by author
author-name:"John Doe"                         # Author by display name
committer:username                             # Filter by committer
author-date:2024-01-01..2024-12-31             # Date range
merge:true|false                               # Filter merges
hash:abc1234                                   # Commit by hash
```

**Example:**

```
repo:owner/repo fix panic author:alice since:2024-01-01
```

---

##### 4. `get_tag`

Get details about a git tag.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `tag` | string | Yes | Tag name |

---

#### D. Issue & Pull Request Operations (13 tools)

##### 1. `issue_write` (create/update)

Create or update an issue.

**Parameters (Create):**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `method` | string | Yes | "create" or "update" |
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `title` | string | Yes | Issue title |
| `body` | string | No | Issue description |
| `labels` | string[] | No | Label names |
| `assignees` | string[] | No | Assignee usernames |
| `milestone` | integer | No | Milestone number |

**Parameters (Update):**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `issue_number` | integer | Yes | Issue number to update |
| `state` | string | No | "open" or "closed" |
| `state_reason` | string | No | "completed", "not_planned", "duplicate" |
| `title` | string | No | New title |
| `body` | string | No | New description |

---

##### 2. `issue_read`

Get issue details.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `method` | string | Yes | get, get_comments, get_sub_issues, get_labels |
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `issue_number` | integer | Yes | Issue number |

---

##### 3. `add_issue_comment`

Add a comment to an issue or PR.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `issue_number` | integer | Yes | Issue/PR number |
| `body` | string | Yes | Comment content |

---

##### 4. `list_issues`

List issues in a repository.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `state` | string | No | OPEN, CLOSED, or both |
| `labels` | string[] | No | Filter by labels |
| `orderBy` | string | No | CREATED_AT, UPDATED_AT, COMMENTS |
| `direction` | string | No | ASC, DESC |
| `since` | string | No | ISO 8601 date |
| `perPage` | integer | No | Results per page (1-100) |
| `after` | string | No | Pagination cursor (GraphQL) |

---

##### 5. `search_issues`

Search for issues across repositories.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | Yes | Search query |
| `owner` | string | No | Repository owner (optional) |
| `repo` | string | No | Repository name (optional) |
| `sort` | string | No | comments, reactions, created, updated |
| `order` | string | No | asc, desc |
| `perPage` | integer | No | Results per page (1-100) |

**Query Qualifiers:**

```
repo:owner/repo          # Repository filter
state:open|closed        # Issue state
label:bug               # By label
author:username         # By author
assignee:username       # By assignee
is:open|closed          # State
created:2024-01-01..    # Date range
updated:>2024-06-01     # Update date
```

---

##### 6. `create_pull_request`

Create a new pull request.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `title` | string | Yes | PR title |
| `head` | string | Yes | Source branch |
| `base` | string | Yes | Target branch |
| `body` | string | No | PR description |
| `draft` | boolean | No | Create as draft (default: false) |
| `maintainer_can_modify` | boolean | No | Allow maintainer edits |

---

##### 7. `list_pull_requests`

List pull requests.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `state` | string | No | open, closed, all |
| `base` | string | No | Filter by base branch |
| `head` | string | No | Filter by head (user/branch) |
| `sort` | string | No | created, updated, popularity, long-running |
| `direction` | string | No | asc, desc |
| `perPage` | integer | No | Results per page (1-100) |

---

##### 8. `search_pull_requests`

Search for pull requests.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | Yes | Search query |
| `owner` | string | No | Repository owner |
| `repo` | string | No | Repository name |
| `sort` | string | No | Sort field |
| `order` | string | No | asc, desc |

---

##### 9. `pull_request_read`

Get PR details with various methods.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `method` | string | Yes | get, get_diff, get_status, get_files, get_review_comments, get_reviews, get_comments, get_check_runs |
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `pullNumber` | integer | Yes | PR number |
| `page` | integer | No | Pagination |
| `perPage` | integer | No | Results per page |

---

##### 10. `pull_request_review_write`

Create, submit, or delete PR reviews.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `method` | string | Yes | create, submit_pending, delete_pending, resolve_thread, unresolve_thread |
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `pullNumber` | integer | Yes | PR number |
| `event` | string | No | APPROVE, REQUEST_CHANGES, COMMENT (for submit) |
| `body` | string | No | Review comment text |
| `commitID` | string | No | Commit SHA to review |
| `threadId` | string | No | Thread ID (for resolve/unresolve) |

---

##### 11. `update_pull_request`

Update a PR's metadata.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `pullNumber` | integer | Yes | PR number |
| `title` | string | No | New title |
| `body` | string | No | New description |
| `state` | string | No | open, closed |
| `base` | string | No | New base branch |
| `draft` | boolean | No | Mark as draft/ready |
| `reviewers` | string[] | No | Reviewer usernames |

---

##### 12. `merge_pull_request`

Merge a pull request.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `pullNumber` | integer | Yes | PR number |
| `merge_method` | string | No | merge, squash, rebase |
| `commit_title` | string | No | Custom commit title |
| `commit_message` | string | No | Custom commit message |

---

##### 13. `update_pull_request_branch`

Update a PR branch with latest base branch changes.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `pullNumber` | integer | Yes | PR number |
| `expectedHeadSha` | string | No | Expected current HEAD SHA |

---

#### E. Search Operations (5 tools)

##### 1. `search_code`

Search across code repositories.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | Yes | Search query with qualifiers |
| `sort` | string | No | indexed (only valid option) |
| `order` | string | No | asc, desc |
| `perPage` | integer | No | Results per page (1-100) |

**Query Qualifiers:**

```
repo:owner/repo              # Repository
org:orgname                  # Organization
user:username                # User
language:python|go|rust      # Programming language
path:directory               # File path (prefix match)
filename:exact_filename.ext  # Exact filename
extension:.rs                # File extension
in:file|path                 # Search location
size:1000|>100kb|<50kb       # File size
is:fork|archive              # Repository status
```

**Example:**

```
WithContext language:go repo:golang/go
"error handling" language:rust path:src/
```

---

##### 2. `search_repositories`

Find repositories by name, description, topics.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | Yes | Repository search query |
| `sort` | string | No | stars, forks, help-wanted-issues, updated |
| `order` | string | No | asc, desc |
| `minimal_output` | boolean | No | Return minimal info (default: true) |
| `perPage` | integer | No | Results per page (1-100) |

**Example:**

```
machine learning in:name stars:>1000 language:python
topic:react stars:>5000
user:facebook language:javascript
```

---

##### 3. `search_users`

Find GitHub users.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | Yes | User search query |
| `sort` | string | No | followers, repositories, joined |
| `order` | string | No | asc, desc |
| `perPage` | integer | No | Results per page (1-100) |

**Example:**

```
john smith
location:seattle
followers:>100
```

---

#### F. Release Operations (3 tools)

##### 1. `list_releases`

List releases in a repository.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `page` | integer | No | Pagination |
| `perPage` | integer | No | Results per page (1-100) |

**Returns:**

```json
{
  "releases": [
    {
      "id": "integer",
      "tag_name": "string (e.g., v1.0.0)",
      "name": "string",
      "draft": "boolean",
      "prerelease": "boolean",
      "created_at": "ISO8601",
      "published_at": "ISO8601",
      "author": {
        "login": "string"
      },
      "assets": [
        {
          "id": "integer",
          "name": "string",
          "download_count": "integer"
        }
      ]
    }
  ]
}
```

---

##### 2. `get_latest_release`

Get the latest release.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |

---

##### 3. `get_release_by_tag`

Get release by tag name.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `tag` | string | Yes | Tag name (e.g., v1.0.0) |

---

#### G. GitHub Actions (4 tools)

##### 1. `actions_list`

List workflows, runs, jobs, or artifacts.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `method` | string | Yes | list_workflows, list_workflow_runs, list_workflow_jobs, list_workflow_run_artifacts |
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `resource_id` | string | No | Workflow ID/name or run ID |
| `workflow_runs_filter` | object | No | Filters for runs (status, event, branch, actor) |
| `workflow_jobs_filter` | object | No | Filters for jobs (filter: latest|all) |
| `page` | integer | No | Pagination |
| `per_page` | integer | No | Results per page (1-100, default: 30) |

**Returns:**

```json
{
  "workflows": [
    {
      "id": "integer",
      "name": "string",
      "state": "active|deleted",
      "path": ".github/workflows/ci.yml"
    }
  ]
}
```

---

##### 2. `actions_get`

Get details about workflows, runs, jobs, or artifacts.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `method` | string | Yes | get_workflow, get_workflow_run, get_workflow_job, download_workflow_run_artifact, get_workflow_run_usage, get_workflow_run_logs_url |
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `resource_id` | string | Yes | Workflow ID/name, run ID, job ID, or artifact ID |

---

##### 3. `actions_run_trigger`

Trigger workflow operations.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `method` | string | Yes | run_workflow, rerun_workflow_run, rerun_failed_jobs, cancel_workflow_run, delete_workflow_run_logs |
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `workflow_id` | string | No | Workflow ID/name (for run_workflow) |
| `ref` | string | No | Git branch/tag (for run_workflow) |
| `run_id` | integer | No | Run ID (for other methods) |
| `inputs` | object | No | Workflow inputs (for run_workflow) |

---

##### 4. `get_job_logs`

Get logs for workflow jobs.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `job_id` | integer | No | Single job ID |
| `run_id` | integer | No | Run ID (with failed_only=true) |
| `failed_only` | boolean | No | Only failed jobs in run |
| `return_content` | boolean | No | Return content vs URLs |
| `tail_lines` | integer | No | Lines from end (default: 500) |

---

#### H. PR Auto-Merge Operations (2 tools)

##### 1. `enable_pr_auto_merge`

Enable auto-merge for a PR.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `pullNumber` | integer | Yes | PR number |
| `mergeMethod` | string | No | MERGE, SQUASH, REBASE |

---

##### 2. `disable_pr_auto_merge`

Disable auto-merge for a PR.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `pullNumber` | integer | Yes | PR number |

---

#### I. User & Team Operations (4 tools)

##### 1. `get_me`

Get authenticated user details.

**Returns:**

```json
{
  "login": "string",
  "id": "integer",
  "name": "string",
  "email": "string",
  "bio": "string",
  "company": "string",
  "location": "string",
  "public_repos": "integer",
  "followers": "integer",
  "following": "integer"
}
```

---

##### 2. `get_teams`

Get teams for a user.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `user` | string | No | Username (default: authenticated user) |

---

##### 3. `get_team_members`

Get members of a team.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `org` | string | Yes | Organization name |
| `team_slug` | string | Yes | Team slug |

---

##### 4. `list_repository_collaborators`

List collaborators of a repository.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `affiliation` | string | No | outside, direct, all (default) |
| `page` | integer | No | Pagination |
| `perPage` | integer | No | Results per page (1-100) |

---

#### J. Security Operations (2 tools)

##### 1. `run_secret_scanning`

Scan files for secrets (API keys, tokens, passwords).

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `files` | string[] | Yes | File contents or diffs to scan |
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |

**Returns:**

```json
{
  "secrets": [
    {
      "type": "string (secret type)",
      "location": "string (file location)",
      "severity": "critical|high|medium|low"
    }
  ]
}
```

---

##### 2. `request_copilot_review`

Request GitHub Copilot code review for a PR.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `pullNumber` | integer | Yes | PR number |

---

#### K. Labels & Milestones (1 tool)

##### 1. `get_label`

Get details about a label.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Repository owner |
| `repo` | string | Yes | Repository name |
| `name` | string | Yes | Label name |

---

#### L. Issue Fields (2 tools)

##### 1. `list_issue_fields`

List custom issue fields for repo or organization.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Owner (org or user) |
| `repo` | string | No | Repository name (omit for org fields) |

**Returns:**

```json
{
  "fields": [
    {
      "name": "string",
      "type": "text|number|date|single_select",
      "options": ["string"] (if single_select)
    }
  ]
}
```

---

##### 2. `list_issue_types`

List supported issue types for organization.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `owner` | string | Yes | Organization name |

---

### GitHub Authentication

**Required Scopes:**

```
repo (full control of repositories)
read:user (read public user profile)
user:email (read user email)
read:org (read organization data)
workflow (manage GitHub Actions)
```

**Setup:**

1. Go to GitHub Settings > Developer Settings > Personal Access Tokens
2. Create token with required scopes
3. Provide token to Claude Code authentication

---

### GitHub Error Codes

| Error | Cause | Resolution |
|-------|-------|-----------|
| `401 Unauthorized` | Invalid/expired PAT | Generate new PAT |
| `403 Forbidden` | Insufficient permissions | Add required scopes to PAT |
| `404 Not Found` | Repository/issue doesn't exist | Verify owner/repo names |
| `409 Conflict` | Branch exists or merge conflict | Check branch status |
| `422 Validation Failed` | Invalid parameters | Check parameter format |
| `403 Repository Access Blocked` | Rate limit exceeded | Wait before retrying |

---

### GitHub Rate Limiting

**REST API:**
- 60 requests/hour (unauthenticated)
- 5,000 requests/hour (authenticated)

**GraphQL API:**
- 5,000 points/hour
- Complex queries cost more points

**Response Headers:**

```
X-RateLimit-Limit: 5000
X-RateLimit-Remaining: 4999
X-RateLimit-Reset: 1234567890
```

---

## Mermaid Chart Server

### Server Information

- **Name:** `mcp__Mermaid_Chart`
- **Total Tools:** 1
- **Authentication:** None
- **Purpose:** Validate and render Mermaid diagrams

### Tool: `validate_and_render_mermaid_diagram`

Validate Mermaid diagram syntax and render to UI widget.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `diagramCode` | string | Yes | Valid Mermaid diagram syntax |
| `title` | string | No | Optional diagram title |

**Supported Diagram Types:**

1. **Flowchart**
   ```mermaid
   flowchart LR
       A[Start] --> B{Decision}
       B -->|Yes| C[Process]
       B -->|No| D[End]
   ```

2. **Sequence Diagram**
   ```mermaid
   sequenceDiagram
       User->>Server: Request
       Server->>Database: Query
       Database-->>Server: Result
       Server-->>User: Response
   ```

3. **Gantt Chart**
   ```mermaid
   gantt
       title Project Timeline
       section Tasks
           Task1 :t1, 2026-06-01, 30d
           Task2 :t2, after t1, 20d
   ```

4. **Class Diagram**
   ```mermaid
   classDiagram
       class Animal
       class Dog
       Dog --|> Animal
   ```

5. **State Diagram**
   ```mermaid
   stateDiagram-v2
       [*] --> Idle
       Idle --> Active: start
       Active --> Idle: stop
       Active --> [*]
   ```

6. **Entity Relationship Diagram**
   ```mermaid
   erDiagram
       USER ||--o{ ORDER : places
       ORDER ||--|{ PRODUCT : contains
   ```

7. **Pie Chart**
   ```mermaid
   pie title Browser Usage
       "Chrome" : 45
       "Safari" : 30
       "Firefox" : 25
   ```

**Returns:**

```json
{
  "valid": "boolean",
  "rendered": "string (SVG or error message)",
  "diagramCode": "string (original code)",
  "repairUrl": "string (Mermaid Live link if invalid)"
}
```

**Error Response:**

If diagram is invalid, includes link to Mermaid Live for repairs:

```json
{
  "valid": false,
  "error": "string (syntax error description)",
  "repairUrl": "https://mermaid.live?mode=edit&gist=..."
}
```

---

## Integration Guide

### Setting Up MCP Servers

#### 1. Gmail Integration

**Prerequisites:**

- Google account
- Gmail API enabled in Google Cloud Console
- OAuth 2.0 credentials configured

**Steps:**

1. Visit Claude Code settings
2. Navigate to MCP Servers
3. Add Gmail server
4. Authorize with Google account
5. Grant necessary permissions

**Test Connection:**

```json
{
  "method": "list_labels",
  "expectedResult": "Array of label objects"
}
```

---

#### 2. Google Calendar Integration

**Prerequisites:**

- Same Google account as Gmail
- Calendar API enabled
- OAuth 2.0 credentials (can share with Gmail)

**Steps:**

1. Open Claude Code settings
2. Add Google Calendar server
3. Verify OAuth scopes include calendar access
4. Grant permissions

**Test Connection:**

```json
{
  "method": "list_calendars",
  "expectedResult": "Array of calendar objects"
}
```

---

#### 3. GitHub Integration

**Prerequisites:**

- GitHub account
- Personal Access Token (PAT) with required scopes

**Steps:**

1. Go to GitHub Settings > Developer Settings > Personal Access Tokens
2. Click "Generate new token (classic)"
3. Select scopes: repo, read:user, user:email, read:org, workflow
4. Copy token
5. In Claude Code settings, add GitHub server
6. Paste PAT

**Test Connection:**

```json
{
  "method": "get_me",
  "expectedResult": "Authenticated user profile"
}
```

---

#### 4. Mermaid Chart Integration

**Setup:**

No authentication required. Mermaid server is built-in.

**Verification:**

Use any valid Mermaid diagram syntax.

---

### Common Integration Patterns

#### Pattern 1: Email → Calendar

Create calendar event from email:

```javascript
// 1. Search for email
search_threads(query: "subject:Meeting")

// 2. Extract details from email body
get_thread(threadId: "abc123")

// 3. Create calendar event
create_event({
  summary: "Meeting",
  startTime: "2026-06-15T14:00:00Z",
  endTime: "2026-06-15T15:00:00Z",
  attendees: [/* from email */]
})
```

---

#### Pattern 2: GitHub → Email Notification

Send email on PR activity:

```javascript
// 1. Get PR details
pull_request_read(method: "get", pullNumber: 123)

// 2. Compose email
create_draft({
  to: ["team@example.com"],
  subject: `PR #123: ${title}`,
  body: `Pull request details...\n${prUrl}`
})
```

---

#### Pattern 3: Issue Tracking with Calendar

Schedule issue resolution:

```javascript
// 1. List issues
list_issues(state: "OPEN")

// 2. Create calendar reminders
create_event({
  summary: `Issue #${issueNumber}: ${title}`,
  startTime: "2026-06-20T09:00:00Z",
  endTime: "2026-06-20T10:00:00Z"
})
```

---

#### Pattern 4: Multi-Repository Workflow

Monitor multiple repos:

```javascript
// For each repository
list_pull_requests(owner: "org", repo: "repo1")
list_pull_requests(owner: "org", repo: "repo2")
list_pull_requests(owner: "org", repo: "repo3")

// Aggregate and report via email
create_draft({
  to: ["devops@example.com"],
  subject: "Daily PR Summary",
  body: summarizeAllPRs()
})
```

---

## Authentication & Security

### OAuth 2.0 Flow

**Gmail & Google Calendar:**

1. Claude Code initiates OAuth request
2. User redirected to Google login
3. User grants permissions
4. Google returns authorization code
5. Claude Code exchanges code for access token
6. Access token stored securely
7. Token automatically refreshed when expired

**Security:**

- Tokens stored in encrypted storage
- No plain-text storage
- Automatic token rotation
- Revocation supported in settings

---

### Personal Access Token (PAT) Management

**GitHub:**

1. Generate PAT with minimal required scopes
2. Copy immediately (cannot be retrieved later)
3. Paste in Claude Code settings
4. Token transmitted over HTTPS only
5. Stored in encrypted credential store

**Security Best Practices:**

- Rotate PATs periodically (every 90 days)
- Use separate PATs for different projects
- Revoke compromised tokens immediately
- Monitor PAT usage in GitHub audit log

---

### Scope Limitations

**Gmail:**

- No access to other users' emails
- Limited to authenticated user's account
- Sensitive data (passwords, tokens) should not be sent via email

**Google Calendar:**

- No access to other users' calendar details (unless shared)
- Cannot create events on behalf of others without delegation

**GitHub:**

- Scopes limit access to public data and user's repositories
- Organization-level data requires explicit access
- Workflow access limited to repositories with sufficient permissions

---

## Error Handling

### Common Error Patterns

#### Rate Limiting

**Symptom:** 429 Too Many Requests

**Solution:**

```javascript
// Implement exponential backoff
async function callWithRetry(fn, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (error.status === 429) {
        const delay = Math.pow(2, i) * 1000; // 1s, 2s, 4s
        await new Promise(resolve => setTimeout(resolve, delay));
      } else {
        throw error;
      }
    }
  }
}
```

---

#### Authentication Failures

**Symptom:** 401 Unauthorized

**Solution:**

1. Check token validity
2. Verify token has required scopes
3. Re-authorize in Claude Code settings
4. Generate new token if necessary

---

#### Invalid Parameters

**Symptom:** 400 Bad Request / 422 Validation Failed

**Common Issues:**

- Email format: `user@example.com` not `User <user@example.com>`
- Date format: ISO 8601 only `2026-06-15T14:00:00Z`
- Label IDs: Use ID not display name
- Branch names: Verify exact spelling

---

### Error Response Format

**Gmail API:**

```json
{
  "error": {
    "code": 400,
    "message": "string describing error",
    "errors": [
      {
        "domain": "global",
        "reason": "badRequest",
        "message": "Invalid email address"
      }
    ]
  }
}
```

**GitHub API:**

```json
{
  "message": "string",
  "documentation_url": "https://docs.github.com/...",
  "errors": [
    {
      "message": "string",
      "documentation_url": "https://docs.github.com/...",
      "field": "string"
    }
  ]
}
```

---

## Rate Limiting

### Service-Specific Limits

#### Gmail API

| Resource | Limit | Window |
|----------|-------|--------|
| API Requests | 1,000,000,000 | Daily |
| Quota Units | 1,000,000,000 | Daily |
| Per-User Quota | 250,000 | Per day |

---

#### Google Calendar API

| Resource | Limit | Window |
|----------|-------|--------|
| Requests | 1,000,000 | Daily |
| Queries per User | 10,000 | Per minute |
| Write Operations | 1,000 | Per minute |

---

#### GitHub API

| Type | Limit | Window |
|------|-------|--------|
| REST API (Auth) | 5,000 | Per hour |
| GraphQL API | 5,000 points | Per hour |
| Search Queries | 30 | Per minute |

---

### Response Headers

Monitor these headers for rate limit status:

**Gmail:**

```
X-Quota-Remaining: integer
X-RateLimit-Remaining: integer
```

**Google Calendar:**

```
X-RateLimit-Limit: integer
X-RateLimit-Remaining: integer
X-RateLimit-Reset: unix timestamp
```

**GitHub:**

```
X-RateLimit-Limit: 5000
X-RateLimit-Remaining: 4999
X-RateLimit-Reset: 1234567890
X-RateLimit-Used: 1
X-RateLimit-Resource: core|search|graphql
```

---

## Best Practices

### General Guidelines

#### 1. Error Handling

Always wrap API calls with error handling:

```javascript
try {
  const result = await client.createDraft({
    to: ["user@example.com"],
    subject: "Test",
    body: "Test message"
  });
  console.log("Success:", result);
} catch (error) {
  if (error.status === 401) {
    // Re-authenticate
  } else if (error.status === 429) {
    // Implement backoff
  } else {
    // Handle other errors
  }
}
```

---

#### 2. Pagination

Handle pagination for list operations:

```javascript
async function getAllItems(listFn) {
  const items = [];
  let pageToken = null;
  
  while (true) {
    const response = await listFn({ pageToken });
    items.push(...response.items);
    
    if (!response.nextPageToken) break;
    pageToken = response.nextPageToken;
  }
  
  return items;
}
```

---

#### 3. Data Validation

Validate input before sending:

```javascript
function validateEmail(email) {
  const regex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  if (!regex.test(email)) {
    throw new Error("Invalid email format");
  }
}

function validateISO8601(dateString) {
  try {
    new Date(dateString).toISOString();
    return true;
  } catch {
    return false;
  }
}
```

---

#### 4. Performance Optimization

- Use pagination with appropriate page sizes
- Batch operations when possible (e.g., `push_files` for multiple files)
- Cache frequently accessed data (labels, calendars)
- Use search/filter parameters to reduce data transferred

---

#### 5. Security Best Practices

**Do:**
- Use environment variables for sensitive data
- Rotate credentials periodically
- Validate all user input
- Use HTTPS for all communications
- Log security-relevant events

**Don't:**
- Hardcode credentials in code
- Store passwords in email bodies
- Share PATs or OAuth tokens
- Commit credentials to version control
- Log sensitive data

---

### Gmail Best Practices

1. **Draft Management**
   - Always verify draft content before sending
   - Use templates for common emails
   - Implement spell-check before send

2. **Label Organization**
   - Use hierarchical labels (Projects/Client-A/Tasks)
   - Create labels for automation rules
   - Clean up unused labels periodically

3. **Search Efficiency**
   - Use specific search operators
   - Combine multiple filters
   - Cache search results when possible

---

### Google Calendar Best Practices

1. **Event Management**
   - Always include timezone information
   - Set appropriate reminder times
   - Use event types for classification

2. **Attendee Management**
   - Verify email addresses before inviting
   - Use optional attendees for FYI recipients
   - Set guest permissions appropriately

3. **Recurring Events**
   - Use RFC 5545 format for recurrence
   - Test recurring event generation
   - Include end dates to prevent infinite loops

---

### GitHub Best Practices

1. **Repository Management**
   - Use consistent branch naming conventions
   - Automate branch protection rules
   - Maintain clear commit messages

2. **Issue & PR Workflow**
   - Use templates for consistency
   - Link related issues and PRs
   - Automate PR merging with auto-merge
   - Use semantic commit messages

3. **Search Optimization**
   - Use qualified search syntax
   - Limit result sets with filters
   - Cache frequently searched queries

4. **Security**
   - Regularly rotate PATs
   - Use branch protection rules
   - Enable secret scanning
   - Review CI/CD logs for failures

---

### Mermaid Chart Best Practices

1. **Diagram Clarity**
   - Use descriptive node labels
   - Limit diagram complexity
   - Test rendering in Mermaid Live

2. **Accessibility**
   - Add titles and descriptions
   - Use clear color contrasts
   - Include text alternatives

3. **Maintenance**
   - Version diagrams with code
   - Document diagram purpose
   - Update diagrams with code changes

---

## Conclusion

This documentation covers the complete MCP server ecosystem available in Claude Code. Each server provides powerful capabilities for integrating with external services while maintaining security and performance standards.

For the latest updates and additional tools, visit:

- **Gmail API Documentation:** https://developers.google.com/gmail/api
- **Google Calendar API Documentation:** https://developers.google.com/calendar
- **GitHub API Documentation:** https://docs.github.com/en/rest
- **Mermaid Documentation:** https://mermaid.js.org

---

**Document Version:** 1.0
**Last Updated:** 2026-06-14
**Maintainer:** Claude Code Documentation Team
