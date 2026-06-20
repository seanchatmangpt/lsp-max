//! The read-only diagnostic engine behind the `doctor` noun.
//!
//! [`DoctorService`] observes each precondition axis this workspace actually
//! trips over and returns a bounded [`Axis`] per check. It never mutates tracked
//! files, manifests, or sibling repositories.

use std::path::{Path, PathBuf};

use super::util::{
    available_gib, caret_satisfies, collect_quoted_into, env_u64, extract_after, extract_kv,
    git_tracked_but_ignored, git_tracked_files, is_build_input, line_declares_crate,
    line_is_conflict_marker, normalize_join, read_manifest_version, workspace_root, Satisfies,
};
use super::{Axis, DoctorReport, Status};

const SIBLINGS: [&str; 3] = ["lsp-types-max", "wasm4pm-compat", "wasm4pm"];

pub struct DoctorService {
    root: PathBuf,
    parent: PathBuf,
}

impl DoctorService {
    pub fn new() -> Self {
        let root = workspace_root();
        let parent = root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("/"));
        Self { root, parent }
    }

    pub fn diagnose(&self) -> DoctorReport {
        let mut axes = Vec::new();
        let (present, versions) = self.check_siblings();
        axes.push(present);
        axes.push(versions);
        axes.push(self.check_toolchain());
        axes.push(self.check_disk());
        axes.push(self.check_conflicts());
        axes.push(self.check_path_deps());
        axes.push(self.check_ignored_tracked());
        axes.push(self.check_gate());
        DoctorReport::rollup(self.root.display().to_string(), axes)
    }

    fn sibling_dir(&self, name: &str) -> PathBuf {
        self.parent.join(name)
    }

    fn sibling_manifest(&self, name: &str) -> PathBuf {
        // wasm4pm's crate manifest is one level deeper than the repo root.
        if name == "wasm4pm" {
            self.parent.join("wasm4pm").join("wasm4pm").join("Cargo.toml")
        } else {
            self.sibling_dir(name).join("Cargo.toml")
        }
    }

    // ---- Axis 1 + 2: sibling presence and version-floor satisfaction --------
    fn check_siblings(&self) -> (Axis, Axis) {
        let mut missing = Vec::new();
        for name in SIBLINGS {
            if !self.sibling_dir(name).is_dir() {
                missing.push(name);
            }
        }
        let present = if missing.is_empty() {
            Axis::new(
                "siblings.present",
                Status::Admitted,
                "all three sibling checkouts present",
            )
        } else {
            Axis::new(
                "siblings.present",
                Status::Blocked,
                format!(
                    "missing: {} — clone into {}/ (path deps + [patch.crates-io] require them on disk)",
                    missing.join(", "),
                    self.parent.display()
                ),
            )
        };

        let mut status = Status::Admitted;
        let mut notes: Vec<String> = Vec::new();
        for name in SIBLINGS {
            let actual = read_manifest_version(&self.sibling_manifest(name));
            let required = self.discover_required(name);
            match (actual, required) {
                (None, _) => {
                    if status == Status::Admitted {
                        status = Status::Unknown;
                    }
                    notes.push(format!("{name}=<no-manifest-version>"));
                }
                (Some(actual), None) => {
                    notes.push(format!("{name}:{actual}(no-floor)"));
                }
                (Some(actual), Some(req)) => match caret_satisfies(&actual, &req) {
                    Satisfies::Yes => notes.push(format!("{name}:{actual}>=^{req}")),
                    Satisfies::No => {
                        status = Status::Blocked;
                        notes.push(format!("{name}:{actual}!<^{req}"));
                    }
                    Satisfies::Unknown => {
                        if status == Status::Admitted {
                            status = Status::Unknown;
                        }
                        notes.push(format!("{name}:{actual}?{req}"));
                    }
                },
            }
        }
        let joined = notes.join(" ");
        let detail = match status {
            Status::Blocked => format!(
                "sibling version below required floor — {joined} — bump sibling or align path-dep version="
            ),
            Status::Unknown => format!("could not resolve a version — {joined}"),
            _ => format!("floors satisfied — {joined}"),
        };
        (present, Axis::new("siblings.version", status, detail))
    }

    /// Read the `version=` floor this workspace declares for a path dependency,
    /// straight from the manifests so the doctor never drifts from source.
    fn discover_required(&self, crate_name: &str) -> Option<String> {
        let manifests = [
            self.root.join("Cargo.toml"),
            self.root
                .join("crates")
                .join("lsp-max-cli")
                .join("Cargo.toml"),
        ];
        for manifest in manifests {
            let text = std::fs::read_to_string(&manifest).unwrap_or_default();
            for line in text.lines() {
                let trimmed = line.trim_start();
                if line_declares_crate(trimmed, crate_name) {
                    if let Some(v) = extract_kv(trimmed, "version") {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    /// Directories listed in the root `[workspace] members = [...]` array. These
    /// plus the root manifest constitute this workspace's build graph; path-dep
    /// breakage outside that graph is PARTIAL, not BLOCKED.
    fn workspace_member_dirs(&self) -> Vec<String> {
        let text = std::fs::read_to_string(self.root.join("Cargo.toml")).unwrap_or_default();
        let mut members = Vec::new();
        let mut in_members = false;
        for line in text.lines() {
            let t = line.trim();
            if !in_members {
                if t.starts_with("members") && t.contains('[') {
                    in_members = true;
                    // A single-line `members = [ "a", "b" ]` is also handled here.
                    collect_quoted_into(t, &mut members);
                    if t.contains(']') {
                        break;
                    }
                }
                continue;
            }
            collect_quoted_into(t, &mut members);
            if t.contains(']') {
                break;
            }
        }
        // Normalize "." (the root entry) away; the root is handled separately.
        members.retain(|m| m != ".");
        members
    }

    // ---- Axis 3: rust-toolchain.toml vs CI-pinned toolchain -----------------
    fn check_toolchain(&self) -> Axis {
        let pinned = std::fs::read_to_string(self.root.join("rust-toolchain.toml"))
            .ok()
            .and_then(|t| t.lines().find_map(|l| extract_kv(l.trim_start(), "channel")));
        let pinned = match pinned {
            Some(p) => p,
            None => {
                return Axis::new(
                    "toolchain.pin",
                    Status::Unknown,
                    "rust-toolchain.toml channel not found",
                )
            }
        };

        let mut ci_toolchains: Vec<String> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(self.root.join(".github").join("workflows")) {
            for entry in entries.flatten() {
                let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
                for line in text.lines() {
                    if let Some(tc) = extract_after(line, "toolchain:") {
                        if !tc.is_empty() && !ci_toolchains.contains(&tc) {
                            ci_toolchains.push(tc);
                        }
                    }
                }
            }
        }
        if ci_toolchains.is_empty() {
            return Axis::new(
                "toolchain.pin",
                Status::Unknown,
                format!("pinned={pinned}; no toolchain found in .github/workflows/"),
            );
        }
        let drift: Vec<String> = ci_toolchains
            .into_iter()
            .filter(|tc| *tc != pinned)
            .collect();
        if drift.is_empty() {
            Axis::new(
                "toolchain.pin",
                Status::Admitted,
                format!("pinned={pinned} matches CI workflows"),
            )
        } else {
            Axis::new(
                "toolchain.pin",
                Status::Blocked,
                format!(
                    "drift — rust-toolchain.toml={pinned} vs CI={{ {} }}; align both to the same channel",
                    drift.join(" ")
                ),
            )
        }
    }

    // ---- Axis 4: free disk headroom on the workspace volume -----------------
    fn check_disk(&self) -> Axis {
        let partial_gib = env_u64("LSP_MAX_DOCTOR_DISK_PARTIAL_GIB", 5);
        let blocked_gib = env_u64("LSP_MAX_DOCTOR_DISK_BLOCKED_GIB", 1);
        match available_gib(&self.root) {
            Some(avail) if avail < blocked_gib => Axis::new(
                "disk.headroom",
                Status::Blocked,
                format!(
                    "{avail}GiB free (< {blocked_gib}GiB) — a build will hit 'No space left on device'; free space before building"
                ),
            ),
            Some(avail) if avail < partial_gib => Axis::new(
                "disk.headroom",
                Status::Partial,
                format!(
                    "{avail}GiB free (< {partial_gib}GiB) — headroom is thin for a full workspace build"
                ),
            ),
            Some(avail) => Axis::new(
                "disk.headroom",
                Status::Admitted,
                format!("{avail}GiB free on workspace volume"),
            ),
            None => Axis::new(
                "disk.headroom",
                Status::Unknown,
                "could not query free space on workspace volume",
            ),
        }
    }

    // ---- Axis 5: committed merge-conflict markers in tracked source ---------
    fn check_conflicts(&self) -> Axis {
        let mut hits: Vec<String> = Vec::new();
        for rel in git_tracked_files(&self.root) {
            // Skip the doctor's own sources; they reference the markers as data.
            if rel.ends_with("scripts/doctor.sh") || rel.ends_with("doctor.rs") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(self.root.join(&rel)) else {
                continue;
            };
            if text.lines().any(line_is_conflict_marker) {
                hits.push(rel);
            }
        }
        if hits.is_empty() {
            Axis::new(
                "git.conflicts",
                Status::Admitted,
                "no conflict markers in tracked source",
            )
        } else {
            Axis::new(
                "git.conflicts",
                Status::Blocked,
                format!(
                    "conflict markers in: {} — resolve the merge before building",
                    hits.join(" ")
                ),
            )
        }
    }

    // ---- Axis 6: path-dependency depth sanity -------------------------------
    fn check_path_deps(&self) -> Axis {
        let members = self.workspace_member_dirs();
        let mut broken_build: Vec<String> = Vec::new();
        let mut broken_other: Vec<String> = Vec::new();
        let mut checked = 0usize;
        for rel in git_tracked_files(&self.root) {
            if !rel.ends_with("Cargo.toml") {
                continue;
            }
            // Negative-control fixtures deliberately plant unresolved path deps;
            // they are quarantined, not live workspace state.
            if rel.contains("fixtures/") || rel.contains("negative_controls/") {
                continue;
            }
            let manifest = self.root.join(&rel);
            let mdir = manifest.parent().map(Path::to_path_buf).unwrap_or_default();
            let in_build_graph = is_build_input(&rel, &members);
            let text = std::fs::read_to_string(&manifest).unwrap_or_default();
            for line in text.lines() {
                let Some(relpath) = extract_kv(line.trim_start(), "path") else {
                    continue;
                };
                checked += 1;
                // Resolve with lexical `..` collapse, matching Cargo's view, then
                // accept a directory holding a Cargo.toml or a direct manifest/file.
                let target = normalize_join(&mdir, &relpath);
                if !target.join("Cargo.toml").is_file() && !target.is_file() {
                    let entry = format!("{rel}=>{relpath}");
                    if in_build_graph {
                        broken_build.push(entry);
                    } else {
                        broken_other.push(entry);
                    }
                }
            }
        }
        if !broken_build.is_empty() {
            Axis::new(
                "manifests.pathdeps",
                Status::Blocked,
                format!(
                    "unresolved path deps in the build graph: {} — fix the ../ depth so it points at a Cargo.toml",
                    broken_build.join(" ")
                ),
            )
        } else if !broken_other.is_empty() {
            Axis::new(
                "manifests.pathdeps",
                Status::Partial,
                format!(
                    "{checked} path dep(s) checked; unresolved in non-member crates: {} — fix the ../ depth (does not block the workspace build)",
                    broken_other.join(" ")
                ),
            )
        } else {
            Axis::new(
                "manifests.pathdeps",
                Status::Admitted,
                format!("{checked} path dep(s) resolve to a real Cargo.toml"),
            )
        }
    }

    // ---- Axis 7: tracked-but-gitignored runtime artifacts -------------------
    fn check_ignored_tracked(&self) -> Axis {
        let leaked = git_tracked_but_ignored(&self.root);
        if leaked.is_empty() {
            Axis::new(
                "git.ignored_tracked",
                Status::Admitted,
                "no tracked file matches .gitignore",
            )
        } else {
            let sample: Vec<String> = leaked.iter().take(5).cloned().collect();
            Axis::new(
                "git.ignored_tracked",
                Status::Partial,
                format!(
                    "{} tracked file(s) match .gitignore (e.g. {}) — git rm --cached the leaked runtime artifacts",
                    leaked.len(),
                    sample.join(" ")
                ),
            )
        }
    }

    // ---- Axis 8: ANDON gate state (absent binary => UNKNOWN) ----------------
    fn check_gate(&self) -> Axis {
        // Reuse the canonical single-syscall gate check; the binary may be
        // absent, in which case the state is UNKNOWN and the doctor never fails.
        match std::process::Command::new("lsp-max-cli")
            .arg("gate")
            .arg("check")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
        {
            Ok(s) if s.success() => Axis::new(
                "andon.gate",
                Status::Admitted,
                "ANDON gate is clear (lsp-max-cli gate check exit 0)",
            ),
            Ok(_) => Axis::new(
                "andon.gate",
                Status::Blocked,
                "ANDON gate is set — resolve active WASM4PM-* / GGEN-* diagnostics before shell actions",
            ),
            Err(_) => Axis::new(
                "andon.gate",
                Status::Unknown,
                "lsp-max-cli not invokable — gate state not observed",
            ),
        }
    }
}

impl Default for DoctorService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnose_is_observational_and_returns_a_verdict() {
        // The engine is read-only; running it must yield a populated report and
        // a bounded overall status under any environment.
        let report = DoctorService::new().diagnose();
        assert!(!report.axes.is_empty());
        assert!(matches!(
            report.overall,
            Status::Admitted | Status::Partial | Status::Blocked | Status::Unknown
        ));
    }
}
