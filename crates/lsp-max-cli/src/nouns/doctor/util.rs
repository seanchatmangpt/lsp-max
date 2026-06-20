//! Read-only parsing and probe helpers for the `doctor` noun.
//!
//! Nothing here mutates tracked files, manifests, or sibling repositories. The
//! functions observe the filesystem, the git index, and the volume's free space.

use std::path::{Path, PathBuf};

pub(super) fn workspace_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub(super) fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// First top-level `version = "..."` in a manifest.
pub(super) fn read_manifest_version(manifest: &Path) -> Option<String> {
    let text = std::fs::read_to_string(manifest).ok()?;
    text.lines()
        .find_map(|l| extract_kv(l.trim_start(), "version"))
}

/// Whether a (trimmed) manifest line is the dependency declaration for `name`,
/// i.e. starts with `name =` or `name.` (e.g. `name.workspace = true`).
pub(super) fn line_declares_crate(trimmed: &str, name: &str) -> bool {
    if let Some(rest) = trimmed.strip_prefix(name) {
        rest.trim_start().starts_with(['=', '.'])
    } else {
        false
    }
}

/// Extract `key = "value"` from a single line, honoring both top-level keys and
/// inline-table fragments (`crate = { version = "x", path = "y" }`).
pub(super) fn extract_kv(line: &str, key: &str) -> Option<String> {
    let mut search = line;
    while let Some(idx) = search.find(key) {
        let after = &search[idx + key.len()..];
        let before_ok = idx == 0
            || !search[..idx]
                .chars()
                .next_back()
                .map(|c| c.is_alphanumeric() || c == '_' || c == '-')
                .unwrap_or(false);
        if before_ok {
            let trimmed = after.trim_start();
            if let Some(eq) = trimmed.strip_prefix('=') {
                if let Some(v) = read_quoted(eq.trim_start()) {
                    return Some(v);
                }
            }
        }
        search = after;
    }
    None
}

/// Read a leading `"..."` quoted token.
fn read_quoted(s: &str) -> Option<String> {
    let s = s.strip_prefix('"')?;
    let end = s.find('"')?;
    Some(s[..end].to_string())
}

/// Append every `"..."` quoted token found in `line` to `out` (used to read the
/// entries of the `members = [ "a", "b" ]` array, single- or multi-line).
pub(super) fn collect_quoted_into(line: &str, out: &mut Vec<String>) {
    let mut rest = line;
    while let Some(open) = rest.find('"') {
        let after = &rest[open + 1..];
        match after.find('"') {
            Some(close) => {
                out.push(after[..close].to_string());
                rest = &after[close + 1..];
            }
            None => break,
        }
    }
}

/// Extract the remainder after a `marker:` (used for `toolchain: <value>`),
/// stripping any trailing inline comment.
pub(super) fn extract_after(line: &str, marker: &str) -> Option<String> {
    line.find(marker).map(|i| {
        let rest = &line[i + marker.len()..];
        rest.split('#').next().unwrap_or(rest).trim().to_string()
    })
}

pub(super) fn line_is_conflict_marker(line: &str) -> bool {
    // A committed merge leaves a 7-char run of one sigil at the line start.
    const MARKERS: [&str; 3] = ["<<<<<<<", "=======", ">>>>>>>"];
    MARKERS.iter().any(|m| line.starts_with(m))
}

#[derive(PartialEq, Eq, Debug)]
pub(super) enum Satisfies {
    Yes,
    No,
    Unknown,
}

/// Caret comparison: `actual` must be >= `required` and share the leading major
/// component, matching Cargo's default `version=` floor. CalVer 26.x.y treats
/// the major (26) as the pinned component.
pub(super) fn caret_satisfies(actual: &str, required: &str) -> Satisfies {
    let (Some(a), Some(r)) = (parse_triple(actual), parse_triple(required)) else {
        return Satisfies::Unknown;
    };
    if a.0 != r.0 {
        return Satisfies::No;
    }
    match a.1.cmp(&r.1) {
        std::cmp::Ordering::Greater => Satisfies::Yes,
        std::cmp::Ordering::Less => Satisfies::No,
        std::cmp::Ordering::Equal => {
            if a.2 >= r.2 {
                Satisfies::Yes
            } else {
                Satisfies::No
            }
        }
    }
}

fn parse_triple(v: &str) -> Option<(u64, u64, u64)> {
    let mut parts = v.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// `git ls-files`, returned as repo-relative paths. Empty on any failure.
pub(super) fn git_tracked_files(root: &Path) -> Vec<String> {
    run_git_lines(root, &["ls-files"])
}

/// Files that are both tracked AND match a .gitignore rule (leaked artifacts).
pub(super) fn git_tracked_but_ignored(root: &Path) -> Vec<String> {
    let tracked = git_tracked_files(root);
    if tracked.is_empty() {
        return Vec::new();
    }
    let mut args = vec!["check-ignore"];
    let refs: Vec<&str> = tracked.iter().map(String::as_str).collect();
    args.extend_from_slice(&refs);
    run_git_lines(root, &args)
}

fn run_git_lines(root: &Path, args: &[&str]) -> Vec<String> {
    match std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
    {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .filter(|l| !l.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Join `rel` onto `base`, collapsing `.`/`..` lexically so the result matches
/// how Cargo resolves a `path = "..."` (without touching the filesystem, so a
/// wrong `../` depth is reported rather than silently followed through symlinks).
pub(super) fn normalize_join(base: &Path, rel: &str) -> PathBuf {
    use std::path::Component;
    let mut out = base.to_path_buf();
    for comp in Path::new(rel).components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Whether a manifest (repo-relative path) is a build input for THIS workspace:
/// the root manifest, or a declared workspace member directory.
pub(super) fn is_build_input(manifest_rel: &str, member_dirs: &[String]) -> bool {
    if manifest_rel == "Cargo.toml" {
        return true;
    }
    let dir = manifest_rel.strip_suffix("/Cargo.toml").unwrap_or(manifest_rel);
    member_dirs.iter().any(|m| m == dir)
}

/// Free space in GiB on the volume holding `path`, via `df -Pk`.
pub(super) fn available_gib(path: &Path) -> Option<u64> {
    let output = std::process::Command::new("df")
        .arg("-Pk")
        .arg(path)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let data = text.lines().nth(1)?;
    let avail_kb: u64 = data.split_whitespace().nth(3)?.parse().ok()?;
    Some(avail_kb / 1024 / 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caret_floor_semantics() {
        assert_eq!(caret_satisfies("26.6.14", "26.6.11"), Satisfies::Yes);
        assert_eq!(caret_satisfies("26.6.5", "26.6.5"), Satisfies::Yes);
        assert_eq!(caret_satisfies("26.6.4", "26.6.5"), Satisfies::No);
        assert_eq!(caret_satisfies("27.0.0", "26.6.5"), Satisfies::No);
        assert_eq!(caret_satisfies("26.7.0", "26.6.9"), Satisfies::Yes);
        assert_eq!(caret_satisfies("nightly", "26.6.5"), Satisfies::Unknown);
    }

    #[test]
    fn extract_kv_reads_inline_table_and_toplevel() {
        assert_eq!(
            extract_kv(r#"version = "26.6.5""#, "version").as_deref(),
            Some("26.6.5")
        );
        assert_eq!(
            extract_kv(
                r#"lsp-types-max = { path = "../x", version = "26.6.5" }"#,
                "version"
            )
            .as_deref(),
            Some("26.6.5")
        );
        assert_eq!(
            extract_kv(
                r#"lsp-types-max = { path = "../../../lsp-types-max", version = "26.6.5" }"#,
                "path"
            )
            .as_deref(),
            Some("../../../lsp-types-max")
        );
        // `rust-version` must not be mistaken for `version`.
        assert_eq!(extract_kv(r#"rust-version = "1.82.0""#, "version"), None);
    }

    #[test]
    fn conflict_marker_detection() {
        assert!(line_is_conflict_marker("<<<<<<< HEAD"));
        assert!(line_is_conflict_marker("======="));
        assert!(line_is_conflict_marker(">>>>>>> branch"));
        assert!(!line_is_conflict_marker("===== short divider"));
        assert!(!line_is_conflict_marker("let x = 1; // ======="));
    }

    #[test]
    fn line_declares_crate_matches_forms() {
        assert!(line_declares_crate(
            r#"wasm4pm-compat = { version = "1" }"#,
            "wasm4pm-compat"
        ));
        assert!(line_declares_crate("lsp-max.workspace = true", "lsp-max"));
        assert!(!line_declares_crate("lsp-max-protocol = { }", "lsp-max"));
    }

    #[test]
    fn extract_after_strips_inline_comment() {
        assert_eq!(
            extract_after("  toolchain: nightly-2026-04-15  # keep in sync", "toolchain:")
                .as_deref(),
            Some("nightly-2026-04-15")
        );
    }
}
