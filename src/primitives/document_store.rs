use lsp_types_max::{TextDocumentContentChangeEvent, Url};
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// A single open document tracked by the server.
#[derive(Debug)]
pub struct VersionedDocument {
    /// UTF-8 text content, kept in sync with all `didChange` events.
    pub content: String,
    /// LSP document version counter, incremented by the client on every edit.
    pub version: i32,
    /// Number of times this document has been updated since it was opened.
    ///
    /// Used by adaptive debounce to estimate activation density ρ_act: the
    /// higher this value, the longer the debounce quiet window should be.
    pub activations: u64,
}

/// Versioned concurrent document store — replaces `RwLock<HashMap<Url, String>>`
/// in every LSP backend.
///
/// Clone is O(1): all clones share the same inner map via `Arc`.
#[derive(Clone, Debug, Default)]
pub struct DocumentStore {
    inner: Arc<RwLock<FxHashMap<Url, VersionedDocument>>>,
}

impl DocumentStore {
    /// Creates an empty `DocumentStore`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an opened document with its initial content and version.
    pub fn open(&self, uri: Url, content: String, version: i32) {
        self.inner.write().insert(
            uri,
            VersionedDocument {
                content,
                version,
                activations: 0,
            },
        );
    }

    /// Apply a batch of content-change events, then stamp the new version.
    pub fn update(&self, uri: &Url, changes: Vec<TextDocumentContentChangeEvent>, version: i32) {
        let mut map = self.inner.write();
        if let Some(doc) = map.get_mut(uri) {
            for change in changes {
                if change.range.is_none() {
                    doc.content = change.text;
                } else {
                    doc.content = apply_incremental(&doc.content, &change);
                }
            }
            doc.version = version;
            doc.activations = doc.activations.saturating_add(1);
        }
    }

    /// Removes a document from the store (called on `textDocument/didClose`).
    pub fn close(&self, uri: &Url) {
        self.inner.write().remove(uri);
    }

    /// Returns a clone of the document's current content, or `None` if not open.
    pub fn get_content(&self, uri: &Url) -> Option<String> {
        self.inner.read().get(uri).map(|d| d.content.clone())
    }

    /// Returns the current version counter for the document.
    pub fn version(&self, uri: &Url) -> Option<i32> {
        self.inner.read().get(uri).map(|d| d.version)
    }

    /// Applies `f` to the document without cloning the content string.
    pub fn with<F, T>(&self, uri: &Url, f: F) -> Option<T>
    where
        F: FnOnce(&VersionedDocument) -> T,
    {
        self.inner.read().get(uri).map(f)
    }

    /// Returns `true` if the URI is currently open in the store.
    pub fn is_open(&self, uri: &Url) -> bool {
        self.inner.read().contains_key(uri)
    }

    /// Returns a FNV-1a hash of the document content, or `None` if not open.
    ///
    /// Callers can use this to skip re-analysis when content hasn't changed,
    /// without cloning the full String.
    pub fn content_hash(&self, uri: &Url) -> Option<u64> {
        self.inner
            .read()
            .get(uri)
            .map(|d| fnv1a_64(d.content.as_bytes()))
    }

    /// Returns the number of times this document has been updated since open.
    ///
    /// Used to scale debounce delay: high-activation documents (many edits)
    /// benefit from a longer quiet window before re-analysis fires.
    pub fn activation_count(&self, uri: &Url) -> u64 {
        self.inner
            .read()
            .get(uri)
            .map(|d| d.activations)
            .unwrap_or(0)
    }
}

/// FNV-1a 64-bit hash — deterministic, non-cryptographic, allocation-free.
///
/// The ring structure: each byte is XOR'd into the accumulator then multiplied
/// by the FNV prime. The result is an element of ℤ/2^64ℤ used as a content
/// address; collision probability over realistic diagnostic sets is negligible.
#[inline]
pub(crate) fn fnv1a_64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    bytes
        .iter()
        .fold(OFFSET, |h, &b| (h ^ b as u64).wrapping_mul(PRIME))
}

/// Apply one incremental change event to the document content.
///
/// LSP line/character offsets are UTF-16 code units.  We convert to byte
/// positions before splicing so that multi-byte characters (emoji, CJK, etc.)
/// are handled correctly.
fn apply_incremental(content: &str, change: &TextDocumentContentChangeEvent) -> String {
    let Some(range) = change.range else {
        return change.text.clone();
    };

    let start = lsp_pos_to_byte(
        content,
        range.start.line as usize,
        range.start.character as usize,
    );
    let end = lsp_pos_to_byte(
        content,
        range.end.line as usize,
        range.end.character as usize,
    );

    let mut out = String::with_capacity(content.len() + change.text.len());
    out.push_str(&content[..start]);
    out.push_str(&change.text);
    out.push_str(&content[end..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types_max::Url;

    fn test_url() -> Url {
        Url::parse("file:///tmp/test.rs").unwrap()
    }

    fn whole_change(text: &str) -> TextDocumentContentChangeEvent {
        TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: text.to_string(),
        }
    }

    #[test]
    fn fnv1a_64_empty_returns_offset_basis() {
        assert_eq!(fnv1a_64(b""), 0xcbf29ce484222325u64);
    }

    #[test]
    fn fnv1a_64_deterministic() {
        assert_eq!(fnv1a_64(b"hello"), fnv1a_64(b"hello"));
    }

    #[test]
    fn fnv1a_64_distinct_inputs_produce_different_hashes() {
        assert_ne!(fnv1a_64(b"hello"), fnv1a_64(b"world"));
    }

    #[test]
    fn open_and_get_content_round_trips() {
        let store = DocumentStore::new();
        let uri = test_url();
        store.open(uri.clone(), "hello".to_string(), 1);
        assert_eq!(store.get_content(&uri).as_deref(), Some("hello"));
    }

    #[test]
    fn version_tracks_open_version() {
        let store = DocumentStore::new();
        let uri = test_url();
        store.open(uri.clone(), String::new(), 7);
        assert_eq!(store.version(&uri), Some(7));
    }

    #[test]
    fn update_whole_document_replaces_content_and_version() {
        let store = DocumentStore::new();
        let uri = test_url();
        store.open(uri.clone(), "old".to_string(), 1);
        store.update(&uri, vec![whole_change("new")], 2);
        assert_eq!(store.get_content(&uri).as_deref(), Some("new"));
        assert_eq!(store.version(&uri), Some(2));
    }

    #[test]
    fn close_removes_document() {
        let store = DocumentStore::new();
        let uri = test_url();
        store.open(uri.clone(), "x".to_string(), 1);
        store.close(&uri);
        assert_eq!(store.get_content(&uri), None);
        assert!(!store.is_open(&uri));
    }

    #[test]
    fn is_open_false_for_never_opened_uri() {
        let store = DocumentStore::new();
        assert!(!store.is_open(&test_url()));
    }

    #[test]
    fn activation_count_increments_on_each_update() {
        let store = DocumentStore::new();
        let uri = test_url();
        store.open(uri.clone(), "v1".to_string(), 1);
        assert_eq!(store.activation_count(&uri), 0);
        store.update(&uri, vec![whole_change("v2")], 2);
        assert_eq!(store.activation_count(&uri), 1);
        store.update(&uri, vec![whole_change("v3")], 3);
        assert_eq!(store.activation_count(&uri), 2);
    }

    #[test]
    fn content_hash_changes_when_content_changes() {
        let store = DocumentStore::new();
        let uri = test_url();
        store.open(uri.clone(), "a".to_string(), 1);
        let h1 = store.content_hash(&uri).unwrap();
        store.update(&uri, vec![whole_change("b")], 2);
        let h2 = store.content_hash(&uri).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn with_applies_function_without_cloning_content() {
        let store = DocumentStore::new();
        let uri = test_url();
        store.open(uri.clone(), "probe".to_string(), 42);
        let version = store.with(&uri, |doc| doc.version).unwrap();
        assert_eq!(version, 42);
    }

    #[test]
    fn clone_shares_inner_store() {
        let store = DocumentStore::new();
        let clone = store.clone();
        let uri = test_url();
        store.open(uri.clone(), "shared".to_string(), 1);
        assert_eq!(clone.get_content(&uri).as_deref(), Some("shared"));
    }
}

/// Convert a `(line, utf16_col)` LSP position to a UTF-8 byte offset.
fn lsp_pos_to_byte(content: &str, line: usize, character: usize) -> usize {
    // Walk to the start of `line`.
    let mut byte = 0;
    let mut current_line = 0;
    for ch in content.chars() {
        if current_line == line {
            break;
        }
        byte += ch.len_utf8();
        if ch == '\n' {
            current_line += 1;
        }
    }
    // Advance `character` UTF-16 code units from the line start.
    let mut utf16 = 0;
    for ch in content[byte..].chars() {
        if utf16 >= character {
            break;
        }
        byte += ch.len_utf8();
        utf16 += ch.len_utf16();
    }
    byte.min(content.len())
}
