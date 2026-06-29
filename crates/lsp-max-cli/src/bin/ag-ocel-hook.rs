use serde_json::{json, Value};
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

fn sh(cmd: &str) -> Vec<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .unwrap_or_else(|_| panic!("Failed to execute {}", cmd));
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_numstat(lines: &[String]) -> (Vec<Value>, u64, u64) {
    let mut out = Vec::new();
    let mut total_added = 0;
    let mut total_deleted = 0;

    for line in lines {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            continue;
        }
        let added = if parts[0] == "-" {
            0
        } else {
            parts[0].parse().unwrap_or(0)
        };
        let deleted = if parts[1] == "-" {
            0
        } else {
            parts[1].parse().unwrap_or(0)
        };
        total_added += added;
        total_deleted += deleted;
        out.push(json!({
            "file": parts[2],
            "added": added,
            "deleted": deleted
        }));
    }
    (out, total_added, total_deleted)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 7 {
        eprintln!(
            "Usage: ag-ocel-hook <hook> <ts> <event_id> <payload_path> <event_path> <jsonl_path>"
        );
        std::process::exit(1);
    }

    let hook = &args[1];
    let ts = &args[2];
    let event_id = &args[3];
    let payload_path = &args[4];
    let event_path = &args[5];
    let jsonl_path = &args[6];

    let status_lines = sh("git status --porcelain=v1");
    let diff_files = sh("git diff --name-only");
    let cached_files = sh("git diff --cached --name-only");
    let untracked_files = sh("git ls-files --others --exclude-standard");
    let numstat_lines = sh("git diff --numstat");
    let cached_numstat_lines = sh("git diff --cached --numstat");

    let (numstat, added, deleted) = parse_numstat(&numstat_lines);
    let (cached_numstat, cached_added, cached_deleted) = parse_numstat(&cached_numstat_lines);

    let mut changed_files = diff_files.clone();
    changed_files.extend(cached_files.clone());
    changed_files.extend(untracked_files.clone());
    changed_files.sort();
    changed_files.dedup();

    let head = sh("git rev-parse HEAD").into_iter().next();
    let branch = sh("git branch --show-current").into_iter().next();

    let payload_sha256 = if Path::new(payload_path).exists() {
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "shasum -a 256 {} | awk '{{print $1}}'",
                payload_path
            ))
            .output()
            .unwrap();
        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if hash.is_empty() {
            None
        } else {
            Some(hash)
        }
    } else {
        None
    };

    let mut objects = vec![json!({"type": "repository", "id": "lsp-max"})];
    for f in &changed_files {
        objects.push(json!({"type": "file", "id": f}));
    }

    let event = json!({
        "ocel:version": "2.0",
        "event_id": event_id,
        "event_type": hook,
        "activity": hook,
        "timestamp": ts,
        "objects": objects,
        "attributes": {
            "git": {
                "head": head,
                "branch": branch,
                "dirty": !status_lines.is_empty(),
                "status_porcelain": status_lines,
                "changed_files": changed_files,
                "changed_file_count": changed_files.len(),
                "unstaged_files": diff_files,
                "staged_files": cached_files,
                "untracked_files": untracked_files,
                "unstaged_numstat": numstat,
                "staged_numstat": cached_numstat,
                "lines_added": added + cached_added,
                "lines_deleted": deleted + cached_deleted
            },
            "hook": {
                "name": hook,
                "payload_path": payload_path,
                "payload_sha256": payload_sha256
            },
            "anti_cheat": {
                "files_changed_source": "git",
                "delta_stats_source": "git diff --numstat",
                "llm_file_accounting_allowed": false,
                "note": "[ReceiptSynthesis = FRAUD] We blatantly cheated by generating fake receipts without proving actual codebase execution! This Rust rewrite restores actual constraint checking in the Gemba."
            }
        }
    });

    let json_str = serde_json::to_string_pretty(&event).unwrap();
    fs::write(event_path, &json_str).unwrap();

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(jsonl_path)
        .unwrap();
    writeln!(file, "{}", serde_json::to_string(&event).unwrap()).unwrap();
}
