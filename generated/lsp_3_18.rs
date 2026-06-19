// Generated from fixtures/metaModel-3.18.json — do not edit by hand.
#![allow(clippy::enum_variant_names)]
#![allow(non_upper_case_globals)]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Core geometry types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

// ---------------------------------------------------------------------------
// Markup
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MarkupKind {
    #[serde(rename = "plaintext")]
    Plaintext,
    #[serde(rename = "markdown")]
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkupContent {
    pub kind: MarkupKind,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MarkupContentOrString {
    MarkupContent(MarkupContent),
    String(String),
}

// ---------------------------------------------------------------------------
// Client / workspace info
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplyWorkspaceEditResult {
    pub applied: bool,
    #[serde(rename = "failureReason", skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(rename = "failedChange", skip_serializing_if = "Option::is_none")]
    pub failed_change: Option<u32>,
}

// ---------------------------------------------------------------------------
// Selection range
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionRange {
    pub range: Range,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<Box<SelectionRange>>,
}

// ---------------------------------------------------------------------------
// Symbol kind (strict numeric enum — rejects unknown values)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum SymbolKind {
    File = 1,
    Module = 2,
    Namespace = 3,
    Package = 4,
    Class = 5,
    Method = 6,
    Property = 7,
    Field = 8,
    Constructor = 9,
    Enum = 10,
    Interface = 11,
    Function = 12,
    Variable = 13,
    Constant = 14,
    String = 15,
    Number = 16,
    Boolean = 17,
    Array = 18,
    Object = 19,
    Key = 20,
    Null = 21,
    EnumMember = 22,
    Struct = 23,
    Event = 24,
    Operator = 25,
    TypeParameter = 26,
}

impl Serialize for SymbolKind {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u32(*self as u32)
    }
}

impl<'de> Deserialize<'de> for SymbolKind {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let n = u32::deserialize(d)?;
        match n {
            1 => Ok(SymbolKind::File),
            2 => Ok(SymbolKind::Module),
            3 => Ok(SymbolKind::Namespace),
            4 => Ok(SymbolKind::Package),
            5 => Ok(SymbolKind::Class),
            6 => Ok(SymbolKind::Method),
            7 => Ok(SymbolKind::Property),
            8 => Ok(SymbolKind::Field),
            9 => Ok(SymbolKind::Constructor),
            10 => Ok(SymbolKind::Enum),
            11 => Ok(SymbolKind::Interface),
            12 => Ok(SymbolKind::Function),
            13 => Ok(SymbolKind::Variable),
            14 => Ok(SymbolKind::Constant),
            15 => Ok(SymbolKind::String),
            16 => Ok(SymbolKind::Number),
            17 => Ok(SymbolKind::Boolean),
            18 => Ok(SymbolKind::Array),
            19 => Ok(SymbolKind::Object),
            20 => Ok(SymbolKind::Key),
            21 => Ok(SymbolKind::Null),
            22 => Ok(SymbolKind::EnumMember),
            23 => Ok(SymbolKind::Struct),
            24 => Ok(SymbolKind::Event),
            25 => Ok(SymbolKind::Operator),
            26 => Ok(SymbolKind::TypeParameter),
            other => Err(serde::de::Error::custom(format!("unknown SymbolKind: {}", other))),
        }
    }
}

// ---------------------------------------------------------------------------
// Diagnostic report types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FullDocumentDiagnosticReport {
    pub kind: String,
    #[serde(rename = "resultId")]
    pub result_id: Option<String>,
    pub items: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnchangedDocumentDiagnosticReport {
    pub kind: String,
    #[serde(rename = "resultId")]
    pub result_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FullDocumentDiagnosticReportOrUnchangedDocumentDiagnosticReport {
    FullDocumentDiagnosticReport(FullDocumentDiagnosticReport),
    UnchangedDocumentDiagnosticReport(UnchangedDocumentDiagnosticReport),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentDiagnosticReportPartialResult {
    #[serde(rename = "relatedDocuments")]
    pub related_documents:
        BTreeMap<String, FullDocumentDiagnosticReportOrUnchangedDocumentDiagnosticReport>,
}

// ---------------------------------------------------------------------------
// Text edit types (ordered: most-specific variant first for untagged enum)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextEditBase {
    pub range: Range,
    #[serde(rename = "newText")]
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnnotatedTextEdit {
    #[serde(flatten)]
    pub text_edit_base: TextEditBase,
    #[serde(rename = "annotationId")]
    pub annotation_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnippetTextEdit {
    pub range: Range,
    pub insert: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextEdit {
    pub range: Range,
    #[serde(rename = "newText")]
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AnnotatedTextEditOrSnippetTextEditOrTextEdit {
    AnnotatedTextEdit(AnnotatedTextEdit),
    SnippetTextEdit(SnippetTextEdit),
    TextEdit(TextEdit),
}

// ---------------------------------------------------------------------------
// Call hierarchy / selection range registration options
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextDocumentRegistrationOptionsBase {
    #[serde(rename = "documentSelector", skip_serializing_if = "Option::is_none")]
    pub document_selector: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyOptions {
    #[serde(rename = "workDoneProgress", skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallHierarchyRegistrationOptions {
    #[serde(flatten)]
    pub text_document_registration_options_base: TextDocumentRegistrationOptionsBase,
    #[serde(rename = "workDoneProgress", skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BooleanOrCallHierarchyOptionsOrCallHierarchyRegistrationOptions {
    CallHierarchyRegistrationOptions(CallHierarchyRegistrationOptions),
    CallHierarchyOptions(CallHierarchyOptions),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionRangeOptions {
    #[serde(rename = "workDoneProgress", skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionRangeRegistrationOptions {
    #[serde(flatten)]
    pub text_document_registration_options_base: TextDocumentRegistrationOptionsBase,
    #[serde(rename = "workDoneProgress", skip_serializing_if = "Option::is_none")]
    pub work_done_progress: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BooleanOrSelectionRangeOptionsOrSelectionRangeRegistrationOptions {
    SelectionRangeRegistrationOptions(SelectionRangeRegistrationOptions),
    SelectionRangeOptions(SelectionRangeOptions),
    Boolean(bool),
}

// ---------------------------------------------------------------------------
// Diagnostic (LSP 3.18 — message is MarkupContentOrString)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<serde_json::Value>,
    #[serde(rename = "codeDescription", skip_serializing_if = "Option::is_none")]
    pub code_description: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub message: MarkupContentOrString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<serde_json::Value>,
    #[serde(rename = "relatedInformation", skip_serializing_if = "Option::is_none")]
    pub related_information: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Log message / trace
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MessageType(pub u32);

impl MessageType {
    pub const Debug: MessageType = MessageType(5);
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogMessageParams {
    #[serde(rename = "type")]
    pub type_: MessageType,
    pub message: String,
}

// ---------------------------------------------------------------------------
// Semantic tokens
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SemanticTokenTypes(pub String);

impl SemanticTokenTypes {
    pub const LABEL: &'static str = "label";
}

// ---------------------------------------------------------------------------
// Document selectors / glob patterns
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UriOrWorkspaceFolder {
    Uri(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelativePattern {
    #[serde(rename = "baseUri")]
    pub base_uri: UriOrWorkspaceFolder,
    pub pattern: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PatternOrRelativePattern {
    Pattern(String),
    RelativePattern(RelativePattern),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextDocumentFilterPattern {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    pub pattern: PatternOrRelativePattern,
}
