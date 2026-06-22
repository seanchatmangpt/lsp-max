# Negative Controls

Each file in this directory is a negative-control fixture — a code pattern that
the scaffold's law enforcement MUST detect and refuse.

A negative control is required for ADMITTED promotion of any law axis. Without
it, the axis remains CANDIDATE (not ADMITTED).

## Inventory

| File | Detects | Status |
|------|---------|--------|
| `unknown_collapse.rs` | Coercing UNKNOWN axis to ADMITTED | OPEN |
| `victory_language.rs` | Forbidden status assertions | OPEN |

Status `OPEN` means the negative-control fixture has not yet been linked to a
receipt. The axis remains CANDIDATE until the receipt chain is closed.
