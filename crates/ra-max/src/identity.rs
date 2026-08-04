use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{LSP_MAX_SOURCE_SHA, RUST_ANALYZER_SOURCE_SHA};

/// Immutable source identities that bound a semantic execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticSubject {
    pub lsp_max_sha: String,
    pub rust_analyzer_sha: String,
    pub engine: String,
}

impl SemanticSubject {
    pub fn tree_sitter_vertical_slice() -> Self {
        Self {
            lsp_max_sha: LSP_MAX_SOURCE_SHA.to_owned(),
            rust_analyzer_sha: RUST_ANALYZER_SOURCE_SHA.to_owned(),
            engine: "tree-sitter-rust/0.23 bounded-semantic-slice".to_owned(),
        }
    }

    pub fn digest(&self) -> String {
        digest_parts([
            self.lsp_max_sha.as_bytes(),
            self.rust_analyzer_sha.as_bytes(),
            self.engine.as_bytes(),
        ])
    }
}

/// Admitted project observation. Paths are ordered and every file is content-addressed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectAdmission {
    pub subject: SemanticSubject,
    pub root: String,
    pub file_hashes: BTreeMap<String, String>,
    pub project_hash: String,
}

impl ProjectAdmission {
    pub fn from_files(
        subject: SemanticSubject,
        root: impl Into<String>,
        files: &BTreeMap<String, String>,
    ) -> Self {
        let root = root.into();
        let file_hashes = files
            .iter()
            .map(|(path, content)| (path.clone(), digest_bytes(content.as_bytes())))
            .collect::<BTreeMap<_, _>>();

        let mut hasher = blake3::Hasher::new();
        hasher.update(b"ra-max/project-admission/v1\0");
        hasher.update(subject.digest().as_bytes());
        hasher.update(&[0]);
        hasher.update(root.as_bytes());
        hasher.update(&[0]);
        for (path, hash) in &file_hashes {
            hasher.update(path.as_bytes());
            hasher.update(&[0]);
            hasher.update(hash.as_bytes());
            hasher.update(&[0]);
        }

        Self {
            subject,
            root,
            file_hashes,
            project_hash: hasher.finalize().to_hex().to_string(),
        }
    }

    pub fn verify(&self, files: &BTreeMap<String, String>) -> bool {
        Self::from_files(self.subject.clone(), self.root.clone(), files) == *self
    }
}

pub fn digest_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

pub fn digest_parts<'a>(parts: impl IntoIterator<Item = &'a [u8]>) -> String {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(&(part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("Cargo.toml".to_owned(), "[package]\nname='demo'\n".to_owned()),
            ("src/main.rs".to_owned(), "fn main() {}\n".to_owned()),
        ])
    }

    #[test]
    fn admission_is_deterministic_and_exact() {
        let first = ProjectAdmission::from_files(
            SemanticSubject::tree_sitter_vertical_slice(),
            "/workspace",
            &files(),
        );
        let second = ProjectAdmission::from_files(
            SemanticSubject::tree_sitter_vertical_slice(),
            "/workspace",
            &files(),
        );

        assert_eq!(first, second);
        assert!(first.verify(&files()));
    }

    #[test]
    fn admission_changes_when_one_byte_changes() {
        let baseline = files();
        let mut changed = baseline.clone();
        changed.insert("src/main.rs".to_owned(), "fn main(){ }\n".to_owned());

        let subject = SemanticSubject::tree_sitter_vertical_slice();
        let first = ProjectAdmission::from_files(subject.clone(), "/workspace", &baseline);
        let second = ProjectAdmission::from_files(subject, "/workspace", &changed);

        assert_ne!(first.project_hash, second.project_hash);
        assert!(!first.verify(&changed));
    }
}
