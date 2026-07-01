# LSP 3.18 Metamodel Research Report

- **Metadata Version:** 3.18.0
- **Date of Analysis:** 2026-06-04

## Executive Summary

This report details the changes and additions introduced in the Language Server Protocol (LSP) version 3.18.0 metamodel compared to version 3.17.0. The analysis was conducted directly on the official metamodel JSON schema file (`metaModel-3.18.json`).

### Summary of Additions
- **New Requests:** 5
- **New Notifications:** 0
- **New Structures:** 70
- **New Fields in Existing Structures:** 18
- **New Enumerations:** 4
- **New Values in Existing Enumerations:** 3
- **New Type Aliases:** 0

---

## 1. New Requests

### `workspace/foldingRange/refresh`
- **Message Direction:** `serverToClient`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  A request to refresh the folding ranges in a document.

@since 3.18.0


### `textDocument/inlineCompletion`
- **Message Direction:** `clientToServer`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  A request to provide inline completions in a document. The request's parameter is of
type {@link InlineCompletionParams}, the response is of type
{@link InlineCompletion InlineCompletion[]} or a Thenable that resolves to such.

@since 3.18.0


### `workspace/textDocumentContent`
- **Message Direction:** `clientToServer`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  The `workspace/textDocumentContent` request is sent from the client to the
server to request the content of a text document.

@since 3.18.0


### `workspace/textDocumentContent/refresh`
- **Message Direction:** `serverToClient`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  The `workspace/textDocumentContent` request is sent from the server to the client to refresh
the content of a specific text document.

@since 3.18.0


### `textDocument/rangesFormatting`
- **Message Direction:** `clientToServer`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  A request to format ranges in a document.

@since 3.18.0


## 2. New Notifications

*No new notifications.*
## 3. New Structures

### `InlineCompletionParams`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  A parameter literal used in inline completion requests.

@since 3.18.0


### `InlineCompletionList`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Represents a collection of {@link InlineCompletionItem inline completion items} to be presented in the editor.

@since 3.18.0


### `InlineCompletionItem`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  An inline completion item represents a text snippet that is proposed inline to complete text that is being typed.

@since 3.18.0


### `InlineCompletionRegistrationOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Inline completion options used during static or dynamic registration.

@since 3.18.0


### `TextDocumentContentParams`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Parameters for the `workspace/textDocumentContent` request.

@since 3.18.0


### `TextDocumentContentResult`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Result of the `workspace/textDocumentContent` request.

@since 3.18.0


### `TextDocumentContentRegistrationOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Text document content provider registration options.

@since 3.18.0


### `TextDocumentContentRefreshParams`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Parameters for the `workspace/textDocumentContent/refresh` request.

@since 3.18.0


### `DocumentRangesFormattingParams`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  The parameters of a {@link DocumentRangesFormattingRequest}.

@since 3.18.0


### `InlineCompletionContext`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Provides information about the context in which an inline completion was requested.

@since 3.18.0


### `StringValue`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  A string value used as a snippet is a template which allows to insert text
and to control the editor cursor when insertion happens.

A snippet can define tab stops and placeholders with `$1`, `$2`
and `${3:foo}`. `$0` defines the final tab stop, it defaults to
the end of the snippet. Variables are defined with `$name` and
`${name:default value}`.

@since 3.18.0


### `InlineCompletionOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Inline completion options used during static registration.

@since 3.18.0


### `TextDocumentContentOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Text document content provider options.

@since 3.18.0


### `ServerInfo`
- **Status/Reason:** `since 3.18.0 ServerInfo type name added.`
- **Documentation:**

  Information about the server

@since 3.15.0
@since 3.18.0 ServerInfo type name added.


### `CompletionItemApplyKinds`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Specifies how fields from a completion item should be combined with those
from `completionList.itemDefaults`.

If unspecified, all fields will be treated as ApplyKind.Replace.

If a field's value is ApplyKind.Replace, the value from a completion item (if
provided and not `null`) will always be used instead of the value from
`completionItem.itemDefaults`.

If a field's value is ApplyKind.Merge, the values will be merged using the rules
defined against each field below.

Servers are only allowed to return `applyKind` if the client
signals support for this via the `completionList.applyKindSupport`
capability.

@since 3.18.0


### `CodeActionDisabled`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Captures why the code action is currently disabled.

@since 3.18.0


### `LocationUriOnly`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Location with only uri and does not include range.

@since 3.18.0


### `PrepareRenamePlaceholder`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `PrepareRenameDefaultBehavior`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `WorkspaceEditMetadata`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Additional data about a workspace edit.

@since 3.18.0


### `SemanticTokensFullDelta`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Semantic tokens options to support deltas for full documents

@since 3.18.0


### `SnippetTextEdit`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  An interactive text edit.

@since 3.18.0


### `NotebookDocumentFilterWithNotebook`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `NotebookDocumentFilterWithCells`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `NotebookDocumentCellChanges`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Cell changes to a notebook document.

@since 3.18.0


### `SelectedCompletionInfo`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Describes the currently selected completion item.

@since 3.18.0


### `ClientInfo`
- **Status/Reason:** `since 3.18.0 ClientInfo type name added.`
- **Documentation:**

  Information about the client

@since 3.15.0
@since 3.18.0 ClientInfo type name added.


### `WorkspaceOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Defines workspace specific capabilities of the server.

@since 3.18.0


### `TextDocumentContentChangePartial`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `TextDocumentContentChangeWholeDocument`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `EditRangeWithInsertReplace`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Edit range variant that includes ranges for insert and replace operations.

@since 3.18.0


### `ServerCompletionItemOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `MarkedStringWithLanguage`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0
@deprecated use MarkupContent instead.


### `CodeActionKindDocumentation`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Documentation for a class of code actions.

@since 3.18.0


### `NotebookCellLanguage`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `NotebookDocumentCellChangeStructure`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Structural changes to cells in a notebook document.

@since 3.18.0


### `NotebookDocumentCellContentChanges`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Content changes to a cell in a notebook document.

@since 3.18.0


### `TextDocumentFilterLanguage`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  A document filter where `language` is required field.

@since 3.18.0


### `TextDocumentFilterScheme`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  A document filter where `scheme` is required field.

@since 3.18.0


### `TextDocumentFilterPattern`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  A document filter where `pattern` is required field.

@since 3.18.0


### `NotebookDocumentFilterNotebookType`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  A notebook document filter where `notebookType` is required field.

@since 3.18.0


### `NotebookDocumentFilterScheme`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  A notebook document filter where `scheme` is required field.

@since 3.18.0


### `NotebookDocumentFilterPattern`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  A notebook document filter where `pattern` is required field.

@since 3.18.0


### `FoldingRangeWorkspaceClientCapabilities`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Client workspace capabilities specific to folding ranges

@since 3.18.0


### `TextDocumentContentClientCapabilities`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Client capabilities for a text document content provider.

@since 3.18.0


### `InlineCompletionClientCapabilities`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Client capabilities specific to inline completions.

@since 3.18.0


### `StaleRequestSupportOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ChangeAnnotationsSupportOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientSymbolKindOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientSymbolTagOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientSymbolResolveOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientCompletionItemOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientCompletionItemOptionsKind`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientSignatureInformationOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientCodeActionLiteralOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientCodeActionResolveOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `CodeActionTagOptions`
- **Status/Reason:** `since 3.18.0 - proposed`
- **Documentation:**

  @since 3.18.0 - proposed


### `ClientCodeLensResolveOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientFoldingRangeKindOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientFoldingRangeOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientSemanticTokensRequestOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientInlayHintResolveOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientShowMessageActionItemOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `CompletionItemTagOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientCompletionItemResolveOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientCompletionItemInsertTextModeOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientSignatureParameterInformationOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientCodeActionKindOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientDiagnosticsTagOptions`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


### `ClientSemanticTokensRequestFullDelta`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  @since 3.18.0


## 4. New Fields in Existing Structures

### `CompletionList`
- **Field:** `applyKind`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Specifies how fields from a completion item should be combined with those
from `completionList.itemDefaults`.

If unspecified, all fields will be treated as ApplyKind.Replace.

If a field's value is ApplyKind.Replace, the value from a completion item
(if provided and not `null`) will always be used instead of the value
from `completionItem.itemDefaults`.

If a field's value is ApplyKind.Merge, the values will be merged using
the rules defined against each field below.

Servers are only allowed to return `applyKind` if the client
signals support for this via the `completionList.applyKindSupport`
capability.

@since 3.18.0

### `Command`
- **Field:** `tooltip`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** An optional tooltip.

@since 3.18.0

### `CodeAction`
- **Field:** `tags`
  - **Status/Reason:** `since 3.18.0 - proposed`
  - **Documentation:** Tags for this code action.

@since 3.18.0 - proposed

### `ApplyWorkspaceEditParams`
- **Field:** `metadata`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Additional data about the edit.

@since 3.18.0

### `TextDocumentEdit`
- **Field:** `edits`
  - **Status/Reason:** `since 3.18.0 - support for SnippetTextEdit. This is guarded using a
client capability.`
  - **Documentation:** The edits to be applied.

@since 3.16.0 - support for AnnotatedTextEdit. This is guarded using a
client capability.

@since 3.18.0 - support for SnippetTextEdit. This is guarded using a
client capability.

### `ServerCapabilities`
- **Field:** `inlineCompletionProvider`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Inline completion options used during static registration.

@since 3.18.0

### `Diagnostic`
- **Field:** `message`
  - **Status/Reason:** `since 3.18.0 - support for MarkupContent. This is guarded by the client
capability `textDocument.diagnostic.markupMessageSupport`.`
  - **Documentation:** The diagnostic's message. It usually appears in the user interface.

@since 3.18.0 - support for MarkupContent. This is guarded by the client
capability `textDocument.diagnostic.markupMessageSupport`.

### `CodeActionOptions`
- **Field:** `documentation`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Static documentation for a class of code actions.

Documentation from the provider should be shown in the code actions menu if either:

- Code actions of `kind` are requested by the editor. In this case, the editor will show the documentation that
  most closely matches the requested code action kind. For example, if a provider has documentation for
  both `Refactor` and `RefactorExtract`, when the user requests code actions for `RefactorExtract`,
  the editor will use the documentation for `RefactorExtract` instead of the documentation for `Refactor`.

- Any code actions of `kind` are returned by the provider.

At most one documentation entry should be shown per provider.

@since 3.18.0

### `DocumentRangeFormattingOptions`
- **Field:** `rangesSupport`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Whether the server supports formatting multiple ranges at once.

@since 3.18.0

### `WorkspaceClientCapabilities`
- **Field:** `foldingRange`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Capabilities specific to the folding range requests scoped to the workspace.

@since 3.18.0
- **Field:** `textDocumentContent`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Capabilities specific to the `workspace/textDocumentContent` request.

@since 3.18.0

### `TextDocumentClientCapabilities`
- **Field:** `filters`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Defines which filters the client supports.

@since 3.18.0
- **Field:** `inlineCompletion`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Client capabilities specific to inline completions.

@since 3.18.0

### `WorkspaceEditClientCapabilities`
- **Field:** `metadataSupport`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Whether the client supports `WorkspaceEditMetadata` in `WorkspaceEdit`s.

@since 3.18.0
- **Field:** `snippetEditSupport`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Whether the client supports snippets as text edits.

@since 3.18.0

### `TextDocumentFilterClientCapabilities`
- **Field:** `relativePatternSupport`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** The client supports Relative Patterns.

@since 3.18.0

### `CodeActionClientCapabilities`
- **Field:** `documentationSupport`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Whether the client supports documentation for a class of
code actions.

@since 3.18.0
- **Field:** `tagSupport`
  - **Status/Reason:** `since 3.18.0 - proposed`
  - **Documentation:** Client supports the tag property on a code action. Clients
supporting tags have to handle unknown tags gracefully.

@since 3.18.0 - proposed

### `CodeLensClientCapabilities`
- **Field:** `resolveSupport`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Whether the client supports resolving additional code lens
properties via a separate `codeLens/resolve` request.

@since 3.18.0

### `DocumentRangeFormattingClientCapabilities`
- **Field:** `rangesSupport`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Whether the client supports formatting multiple ranges at once.

@since 3.18.0

### `DiagnosticClientCapabilities`
- **Field:** `markupMessageSupport`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Whether the client supports `MarkupContent` in diagnostic messages.

@since 3.18.0

### `CompletionListCapabilities`
- **Field:** `applyKindSupport`
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Specifies whether the client supports `CompletionList.applyKind` to
indicate how supported values from `completionList.itemDefaults`
and `completion` will be combined.

If a client supports `applyKind` it must support it for all fields
that it supports that are listed in `CompletionList.applyKind`. This
means when clients add support for new/future fields in completion
items the MUST also support merge for them if those fields are
defined in `CompletionList.applyKind`.

@since 3.18.0

## 5. New Enumerations

### `CodeActionTag`
- **Status/Reason:** `since 3.18.0 - proposed`
- **Documentation:**

  Code action tags are extra annotations that tweak the behavior of a code action.

@since 3.18.0 - proposed


### `LanguageKind`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Predefined Language kinds
@since 3.18.0


### `InlineCompletionTriggerKind`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Describes how an {@link InlineCompletionItemProvider inline completion provider} was triggered.

@since 3.18.0


### `ApplyKind`
- **Status/Reason:** `since 3.18.0`
- **Documentation:**

  Defines how values from a set of defaults and an individual item will be
merged.

@since 3.18.0


## 6. New Values in Existing Enumerations

### `SemanticTokenTypes`
- **Value Name:** `label` (Value: `label`)
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** @since 3.18.0

### `MessageType`
- **Value Name:** `Debug` (Value: `5`)
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** A debug message.

@since 3.18.0

### `CodeActionKind`
- **Value Name:** `RefactorMove` (Value: `refactor.move`)
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Base kind for refactoring move actions: `refactor.move`

Example move actions:

- Move a function to a new file
- Move a property between classes
- Move method to base class
- ...

@since 3.18.0
- **Value Name:** `Notebook` (Value: `notebook`)
  - **Status/Reason:** `since 3.18.0`
  - **Documentation:** Base kind for all code actions applying to the entire notebook's scope. CodeActionKinds using
this should always begin with `notebook.`

@since 3.18.0

## 7. New Type Aliases

*No new type aliases.*