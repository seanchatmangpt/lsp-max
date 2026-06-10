# LSIF 0.6.0 & Diagnostic RDF Mapping Exploration

## 1. Executive Summary

This report establishes the formal mapping strategy for translating Language Server Index Format (LSIF) 0.6.0 elements and compilation diagnostic observations into W3C RDF quads (`oxrdf::Quad`) within `lsp-max` v26.6.5. By utilizing snapshot-isolated named graphs, standard ontologies, and a robust Rust translation layer, this design bridges the gap between interactive Language Server intelligence and semantically verifiable, cryptographically replayable static analysis datasets.

---

## 2. Ontology & Vocabulary Namespaces

In compliance with the Anti-Laundering Doctrine specified in `docs/v26.6.5/prd-ard/data_model.md`, all generated RDF triples must utilize the following standard prefixes and namespace URIs:

| Prefix | Namespace URI | Description |
|---|---|---|
| `rdf` | `http://www.w3.org/1999/02/22-rdf-syntax-ns#` | Standard RDF vocabulary |
| `rdfs` | `http://www.w3.org/2000/01/rdf-schema#` | RDF Schema vocabulary |
| `xsd` | `http://www.w3.org/2001/XMLSchema#` | XML Schema Datatypes |
| `prov` | `http://www.w3.org/ns/prov#` | W3C Provenance Ontology |
| `dcterms` | `http://purl.org/dc/terms/` | Dublin Core Metadata Initiative |
| `dcat` | `http://www.w3.org/ns/dcat#` | Data Catalog Vocabulary |
| `skos` | `http://www.w3.org/2004/02/skos/core#` | Simple Knowledge Organization System |
| `sh` | `http://www.w3.org/ns/shacl#` | W3C Shapes Constraint Language |
| `odrl` | `http://www.w3.org/ns/odrl/2/` | W3C Rights Expression Language |
| `lsif` | `https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/` | LSIF Core Specification Namespace |
| `max` | `urn:lsp-max:core:` | Bounded private core framework namespace |
| `rcpt` | `urn:lsp-max:receipt:` | Bounded private cryptographic receipts |
| `proj` | `urn:project:local:` | Local workspace project resources |

---

## 3. URI Generation Strategies

To translate LSIF elements with arbitrary identifiers (`Id::Number(u64)` or `Id::String(String)`) into standard RDF resources (`oxrdf::NamedNode`), two primary strategies are available:

### Strategy A: Uniform LSIF-Relative URIs (Recommended)
This approach maps all LSIF vertices and edges directly to a flat identifier namespace:
`urn:project:local:lsif:<id>`

- **Pros**: Does not require any context or lookups to construct the URI from an ID; highly performant for single-pass stream parsing.
- **Cons**: Less human-readable in raw Turtle listings compared to type-segmented URIs.
- **Resolution**: The resource type is already explicitly declared via `rdf:type` triples (e.g. `lsif:Document`, `lsif:Range`), making URI-embedded type descriptors functionally redundant.

### Strategy B: Type-Segmented URIs
This approach uses type prefixes in the URI:
`urn:project:local:<type_prefix>_<id>` (e.g., `proj:doc_1`, `proj:range_42`).

- **Pros**: Matches raw Turtle serialization examples in the PRD/ARD precisely.
- **Cons**: Requires keeping an in-memory cache of LSIF vertex types during edge processing to resolve edge `outV` and `inV` targets.

---

## 4. LSIF 0.6.0 Elements to RDF Quad Mapping

Every RDF statement must be ingested as an `oxrdf::Quad` bound to a specific Named Graph:
- **Graph Name**: `urn:project:local:snapshot:<snapshot_id>`

### 4.1 LSIF Vertex Mappings

#### MetaData
- **LSIF Struct**: `MetaData { id, version, position_encoding, tool_info }`
- **RDF Subject**: `urn:project:local:metadata_<id>`
- **Triples**:
  ```turtle
  proj:metadata_1 a max:Artifact ;
                  max:version "0.6.0" ;
                  max:positionEncoding "utf-16" .
  ```
  *(If `tool_info` is present, it generates a nested `max:Tool` resource)*
  ```turtle
  proj:metadata_1 max:toolInfo proj:tool_1 .
  proj:tool_1 a max:Tool ;
              max:name "lsp-max" ;
              max:version "1.0.0" .
  ```

#### Project
- **LSIF Struct**: `Project { id, kind, resource, contents }`
- **RDF Subject**: `urn:project:local:project_<id>` (or uniform `urn:project:local:lsif:<id>`)
- **Triples**:
  ```turtle
  proj:project_2 a lsif:Project ;
                 max:kind "rust" ;
                 max:resource "file:///path/to/project" ;
                 max:contents "..." .
  ```

#### Document
- **LSIF Struct**: `Document { id, uri, language_id, contents }`
- **RDF Subject**: `urn:project:local:doc_<id>`
- **Triples**:
  ```turtle
  proj:doc_3 a lsif:Document ;
             max:uri "file:///path/to/project/src/main.rs" ;
             max:languageId "rust" ;
             max:contents "fn main() {}" . # Optional
  ```

#### Range
- **LSIF Struct**: `Range { id, start, end, tag }`
- **RDF Subject**: `urn:project:local:range_<id>`
- **Triples**:
  ```turtle
  proj:range_4 a lsif:Range ;
               max:startLine "10"^^xsd:integer ;
               max:startCharacter "4"^^xsd:integer ;
               max:endLine "10"^^xsd:integer ;
               max:endCharacter "8"^^xsd:integer .
  ```
  If `tag` is present (e.g. `Definition`):
  ```turtle
  proj:range_4 max:tagType "definition" ;
               max:text "main" ;
               max:symbolKind "Function" ;
               max:fullStartLine "9"^^xsd:integer ;
               max:fullStartCharacter "0"^^xsd:integer ;
               max:fullEndLine "11"^^xsd:integer ;
               max:fullEndCharacter "1"^^xsd:integer ;
               max:detail "fn main()" .
  ```

#### ResultSet
- **LSIF Struct**: `ResultSet { id }`
- **RDF Subject**: `urn:project:local:result_set_<id>`
- **Triples**:
  ```turtle
  proj:result_set_5 a lsif:ResultSet .
  ```

#### Moniker
- **LSIF Struct**: `Moniker { id, scheme, identifier, kind, unique }`
- **RDF Subject**: `urn:project:local:moniker_<id>`
- **Triples**:
  ```turtle
  proj:moniker_6 a lsif:Moniker ;
                 max:scheme "cargo" ;
                 max:identifier "core::main" ;
                 max:monikerKind "export" ;
                 max:uniquenessLevel "workspace" .
  ```

#### PackageInformation
- **LSIF Struct**: `PackageInformation { id, name, manager, version, repository }`
- **RDF Subject**: `urn:project:local:package_info_<id>`
- **Triples**:
  ```turtle
  proj:package_info_7 a lsif:PackageInformation ;
                      max:packageName "tokio" ;
                      max:packageManager "cargo" ;
                      max:packageVersion "1.17.0" .
  ```
  *(If `repository` is present, attach `max:repositoryUrl` and `max:repositoryType`)*

#### HoverResult
- **LSIF Struct**: `HoverResult { id, result }`
- **RDF Subject**: `urn:project:local:hover_result_<id>`
- **Triples**:
  ```turtle
  proj:hover_result_8 a lsif:HoverResult ;
                      lsif:contents "Hover payload markup or plain text" .
  ```

#### Other Results (DefinitionResult, ReferenceResult, etc.)
- **LSIF Struct**: `<Type>Result { id }`
- **RDF Subject**: `urn:project:local:<type_prefix>_result_<id>`
- **Triples**:
  ```turtle
  proj:def_result_9 a lsif:DefinitionResult .
  proj:ref_result_10 a lsif:ReferenceResult .
  proj:decl_result_11 a lsif:DeclarationResult .
  proj:impl_result_12 a lsif:ImplementationResult .
  proj:type_def_result_13 a lsif:TypeDefinitionResult .
  ```

---

### 4.2 LSIF Edge Mappings

LSIF edges represent semantic and structural links between vertices. In RDF, these are mapped directly as predicate relations between subject and object URIs.

#### contains, next, moniker, attach, packageInformation
- **Structure**: `Edge { out_v, in_v/in_vs }`
- **Triples**:
  ```turtle
  # contains (Project to Document, or Document to Range)
  proj:project_2 lsif:contains proj:doc_3 .
  proj:doc_3 lsif:contains proj:range_4 .

  # next (Range/ResultSet to ResultSet)
  proj:range_4 lsif:next proj:result_set_5 .

  # moniker (Range/ResultSet to Moniker)
  proj:result_set_5 lsif:moniker proj:moniker_6 .

  # packageInformation (Moniker to PackageInformation)
  proj:moniker_6 lsif:packageInformation proj:package_info_7 .

  # attach (Range/ResultSet to PackageInformation)
  proj:result_set_5 lsif:attach proj:package_info_7 .
  ```

#### Special Edge: item
The `item` edge relates a result set/node to target ranges, but carries custom attributes (`document` and `property`). These are mapped by asserting the relation on the target range itself.
- **LSIF Struct**: `Item { out_v, in_vs, document, property }`
- **RDF Triples** (for each `in_v` in `in_vs`):
  ```turtle
  # The definition result references the range
  proj:def_result_9 lsif:item proj:range_4 .

  # Range links back to its home document and item property classification
  proj:range_4 lsif:document proj:doc_3 ;
               lsif:property "definitions" .
  ```

#### TextDocument Hover/Definition/References/etc.
LSIF edges represent language queries (hover, definition, references, implementation, typeDefinition, declaration, foldingRange, documentLink, documentSymbol, diagnostic, semanticTokens) by mapping the forward slash to an underscore predicate:
- **LSIF Struct**: `TextDocumentDefinition { out_v, in_v }`
- **RDF Triples**:
  ```turtle
  proj:range_4 lsif:textDocument_definition proj:def_result_9 .
  proj:range_4 lsif:textDocument_hover proj:hover_result_8 .
  proj:range_4 lsif:textDocument_references proj:ref_result_10 .
  proj:range_4 lsif:textDocument_declaration proj:decl_result_11 .
  proj:range_4 lsif:textDocument_implementation proj:impl_result_12 .
  proj:range_4 lsif:textDocument_typeDefinition proj:type_def_result_13 .
  ```

---

## 5. Diagnostic Observations & Provenance Mapping

Compilation rules, errors, warnings, and their cryptographic proof receipts are mapped to RDF quads to satisfy the `PRD-R6` (Receipt Binding) and `PRD-R1` (Admitted RDF Graph State) requirements.

### 5.1 Diagnostic Mapping Layout
For every `MaxDiagnostic` or `lsp_types::Diagnostic` encountered:
- **RDF Subject**: `urn:project:local:diag_<diagnostic_id>`
- **Triples**:
  ```turtle
  proj:diag_101 a max:Diagnostic ;
                max:severity "error" ;
                max:message "Mismatched types: expected String, found integer" ;
                max:lawAxis "soundness" ;
                max:violatedInvariant "TypeSafety" ;
                max:repairability "automated" ;
                max:terminality "non_terminal" ;
                lsif:range proj:diag_range_101 .
  ```

### 5.2 Conformance Rule Binding
Diagnostics are linked to the specific compilation/analysis rules they violate:
```turtle
proj:diag_101 max:conformsTo proj:rule_E0308 .

proj:rule_E0308 a max:Rule ;
                max:identifier "E0308" .
```

### 5.3 Cryptographic Receipt Binding (`prov:wasGeneratedBy`)
To guarantee mathematical proof of deterministic execution and query state, diagnostics must point to their generating receipts:
```turtle
proj:diag_101 prov:wasGeneratedBy rcpt:receipt_999 .

rcpt:receipt_999 a max:Receipt ;
                 prov:startedAtTime "2026-06-05T21:52:00Z"^^xsd:dateTime ;
                 max:graphHash "b5bb9d8014a0f9b1d61e21e796d78dccdf1352f23cd32812f4850b878ae4944c" ;
                 max:queryHash "7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069" ;
                 max:resultHash "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08" .
```

---

## 6. Rust Translation Layout

Below is the proposed implementation interface for the RDF translation module. It uses `oxrdf` structures (`Quad`, `NamedNode`, `Literal`, `GraphName`) to convert LSIF `Element` streams and `MaxDiagnostic` structs into database-ready quads.

```rust
use oxrdf::{Quad, NamedNode, Subject, Term, GraphName, Literal};
use lsp_max_lsif::lsif::{Element, Vertex, Edge, Id, ItemEdgeProperty};
use lsp_max_protocol::MaxDiagnostic;

/// Const prefixes for target namespaces
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const LSIF_NS: &str = "https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/";
pub const MAX_NS: &str = "urn:lsp-max:core:";
pub const RCPT_NS: &str = "urn:lsp-max:receipt:";
pub const PROJ_NS: &str = "urn:project:local:";
pub const PROV_NS: &str = "http://www.w3.org/ns/prov#";
pub const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

pub struct LsifRdfMapper {
    graph_name: GraphName,
}

impl LsifRdfMapper {
    /// Create a new mapper bound to a specific snapshot ID
    pub fn new(snapshot_id: &str) -> Self {
        let graph_uri = format!("{}snapshot:{}", PROJ_NS, snapshot_id);
        let graph_name = GraphName::NamedNode(NamedNode::new(graph_uri).expect("Valid Graph Name"));
        Self { graph_name }
    }

    /// Helper to convert LSIF identifier to project-local NamedNode
    pub fn id_to_uri(&self, id: &Id) -> String {
        match id {
            Id::Number(n) => format!("{}lsif:{}", PROJ_NS, n),
            Id::String(s) => format!("{}lsif:{}", PROJ_NS, s),
        }
    }

    /// Primary admission entrypoint for LSIF stream elements
    pub fn map_element(&self, element: &Element) -> Vec<Quad> {
        match element {
            Element::Vertex(vertex) => self.map_vertex(vertex),
            Element::Edge(edge) => self.map_edge(edge),
        }
    }

    /// Translate a single LSIF Vertex to a set of Quads
    pub fn map_vertex(&self, vertex: &Vertex) -> Vec<Quad> {
        let mut quads = Vec::new();

        // Helper closures for quad creation
        let new_node = |uri: &str| NamedNode::new(uri).unwrap();
        let make_quad = |s: &str, p: &str, o: Term| {
            Quad::new(new_node(s), new_node(p), o, self.graph_name.clone())
        };

        match vertex {
            Vertex::MetaData { id, version, position_encoding, tool_info, .. } => {
                let s = format!("{}metadata:{}", PROJ_NS, match id { Id::Number(n) => n.to_string(), Id::String(str) => str.clone() });
                quads.push(make_quad(&s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}Artifact", MAX_NS)))));
                quads.push(make_quad(&s, &format!("{}version", MAX_NS), Term::Literal(Literal::new_simple_literal(version))));
                quads.push(make_quad(
                    &s,
                    &format!("{}positionEncoding", MAX_NS),
                    Term::Literal(Literal::new_simple_literal(match position_encoding {
                        lsp_max_lsif::lsif::PositionEncoding::Utf8 => "utf-8",
                        lsp_max_lsif::lsif::PositionEncoding::Utf16 => "utf-16",
                        lsp_max_lsif::lsif::PositionEncoding::Utf32 => "utf-32",
                    }))
                ));

                if let Some(tool) = tool_info {
                    let tool_s = format!("{}tool:{}", PROJ_NS, match id { Id::Number(n) => n.to_string(), Id::String(str) => str.clone() });
                    quads.push(make_quad(&s, &format!("{}toolInfo", MAX_NS), Term::NamedNode(new_node(&tool_s))));
                    quads.push(make_quad(&tool_s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}Tool", MAX_NS)))));
                    quads.push(make_quad(&tool_s, &format!("{}name", MAX_NS), Term::Literal(Literal::new_simple_literal(&tool.name))));
                    if let Some(v) = &tool.version {
                        quads.push(make_quad(&tool_s, &format!("{}version", MAX_NS), Term::Literal(Literal::new_simple_literal(v))));
                    }
                }
            }

            Vertex::Project { id, kind, resource, contents, .. } => {
                let s = self.id_to_uri(id);
                quads.push(make_quad(&s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}Project", LSIF_NS)))));
                quads.push(make_quad(&s, &format!("{}kind", MAX_NS), Term::Literal(Literal::new_simple_literal(kind))));
                if let Some(res) = resource {
                    quads.push(make_quad(&s, &format!("{}resource", MAX_NS), Term::Literal(Literal::new_simple_literal(res))));
                }
                if let Some(cont) = contents {
                    quads.push(make_quad(&s, &format!("{}contents", MAX_NS), Term::Literal(Literal::new_simple_literal(cont))));
                }
            }

            Vertex::Document { id, uri, language_id, contents, .. } => {
                let s = self.id_to_uri(id);
                quads.push(make_quad(&s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}Document", LSIF_NS)))));
                quads.push(make_quad(&s, &format!("{}uri", MAX_NS), Term::Literal(Literal::new_simple_literal(uri))));
                quads.push(make_quad(&s, &format!("{}languageId", MAX_NS), Term::Literal(Literal::new_simple_literal(language_id))));
                if let Some(cont) = contents {
                    quads.push(make_quad(&s, &format!("{}contents", MAX_NS), Term::Literal(Literal::new_simple_literal(cont))));
                }
            }

            Vertex::Range { id, start, end, tag, .. } => {
                let s = self.id_to_uri(id);
                let int_type = new_node(&format!("{}integer", XSD_NS));
                
                quads.push(make_quad(&s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}Range", LSIF_NS)))));
                quads.push(make_quad(&s, &format!("{}startLine", MAX_NS), Term::Literal(Literal::new_typed_literal(start.line.to_string(), int_type.clone()))));
                quads.push(make_quad(&s, &format!("{}startCharacter", MAX_NS), Term::Literal(Literal::new_typed_literal(start.character.to_string(), int_type.clone()))));
                quads.push(make_quad(&s, &format!("{}endLine", MAX_NS), Term::Literal(Literal::new_typed_literal(end.line.to_string(), int_type.clone()))));
                quads.push(make_quad(&s, &format!("{}endCharacter", MAX_NS), Term::Literal(Literal::new_typed_literal(end.character.to_string(), int_type.clone()))));

                if let Some(range_tag) = tag {
                    match range_tag {
                        lsp_max_lsif::lsif::RangeTag::Declaration { text, kind, full_range, detail }
                        | lsp_max_lsif::lsif::RangeTag::Definition { text, kind, full_range, detail } => {
                            let tag_type = match range_tag {
                                lsp_max_lsif::lsif::RangeTag::Declaration { .. } => "declaration",
                                _ => "definition",
                            };
                            quads.push(make_quad(&s, &format!("{}tagType", MAX_NS), Term::Literal(Literal::new_simple_literal(tag_type))));
                            quads.push(make_quad(&s, &format!("{}text", MAX_NS), Term::Literal(Literal::new_simple_literal(text))));
                            quads.push(make_quad(&s, &format!("{}symbolKind", MAX_NS), Term::Literal(Literal::new_simple_literal(format!("{:?}", kind)))));
                            quads.push(make_quad(&s, &format!("{}fullStartLine", MAX_NS), Term::Literal(Literal::new_typed_literal(full_range.start.line.to_string(), int_type.clone()))));
                            quads.push(make_quad(&s, &format!("{}fullStartCharacter", MAX_NS), Term::Literal(Literal::new_typed_literal(full_range.start.character.to_string(), int_type.clone()))));
                            quads.push(make_quad(&s, &format!("{}fullEndLine", MAX_NS), Term::Literal(Literal::new_typed_literal(full_range.end.line.to_string(), int_type.clone()))));
                            quads.push(make_quad(&s, &format!("{}fullEndCharacter", MAX_NS), Term::Literal(Literal::new_typed_literal(full_range.end.character.to_string(), int_type))));
                            if let Some(det) = detail {
                                quads.push(make_quad(&s, &format!("{}detail", MAX_NS), Term::Literal(Literal::new_simple_literal(det))));
                            }
                        }
                        lsp_max_lsif::lsif::RangeTag::Reference { text } => {
                            quads.push(make_quad(&s, &format!("{}tagType", MAX_NS), Term::Literal(Literal::new_simple_literal("reference"))));
                            quads.push(make_quad(&s, &format!("{}text", MAX_NS), Term::Literal(Literal::new_simple_literal(text))));
                        }
                        lsp_max_lsif::lsif::RangeTag::Unknown { text } => {
                            quads.push(make_quad(&s, &format!("{}tagType", MAX_NS), Term::Literal(Literal::new_simple_literal("unknown"))));
                            quads.push(make_quad(&s, &format!("{}text", MAX_NS), Term::Literal(Literal::new_simple_literal(text))));
                        }
                    }
                }
            }

            Vertex::ResultSet { id, .. } => {
                let s = self.id_to_uri(id);
                quads.push(make_quad(&s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}ResultSet", LSIF_NS)))));
            }

            Vertex::Moniker { id, scheme, identifier, kind, unique, .. } => {
                let s = self.id_to_uri(id);
                quads.push(make_quad(&s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}Moniker", LSIF_NS)))));
                quads.push(make_quad(&s, &format!("{}scheme", MAX_NS), Term::Literal(Literal::new_simple_literal(scheme))));
                quads.push(make_quad(&s, &format!("{}identifier", MAX_NS), Term::Literal(Literal::new_simple_literal(identifier))));
                quads.push(make_quad(&s, &format!("{}monikerKind", MAX_NS), Term::Literal(Literal::new_simple_literal(format!("{:?}", kind).to_lowercase()))));
                quads.push(make_quad(&s, &format!("{}uniquenessLevel", MAX_NS), Term::Literal(Literal::new_simple_literal(format!("{:?}", unique).to_lowercase()))));
            }

            Vertex::PackageInformation { id, name, manager, version, repository, .. } => {
                let s = self.id_to_uri(id);
                quads.push(make_quad(&s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}PackageInformation", LSIF_NS)))));
                quads.push(make_quad(&s, &format!("{}packageName", MAX_NS), Term::Literal(Literal::new_simple_literal(name))));
                quads.push(make_quad(&s, &format!("{}packageManager", MAX_NS), Term::Literal(Literal::new_simple_literal(manager))));
                quads.push(make_quad(&s, &format!("{}packageVersion", MAX_NS), Term::Literal(Literal::new_simple_literal(version))));
                if let Some(repo) = repository {
                    quads.push(make_quad(&s, &format!("{}repositoryUrl", MAX_NS), Term::Literal(Literal::new_simple_literal(&repo.url))));
                    quads.push(make_quad(&s, &format!("{}repositoryType", MAX_NS), Term::Literal(Literal::new_simple_literal(&repo.type_))));
                }
            }

            Vertex::HoverResult { id, result, .. } => {
                let s = self.id_to_uri(id);
                quads.push(make_quad(&s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}HoverResult", LSIF_NS)))));
                
                let content_str = match &result.contents {
                    lsp_max_lsif::lsif::HoverContents::String(str) => str.clone(),
                    lsp_max_lsif::lsif::HoverContents::Markup(m) => m.value.clone(),
                    lsp_max_lsif::lsif::HoverContents::MarkedString(ms) => format!("{:?}", ms),
                    lsp_max_lsif::lsif::HoverContents::MarkedStringArray(arr) => format!("{:?}", arr),
                };
                quads.push(make_quad(&s, &format!("{}contents", LSIF_NS), Term::Literal(Literal::new_simple_literal(&content_str))));
            }

            Vertex::DefinitionResult { id, .. } => {
                let s = self.id_to_uri(id);
                quads.push(make_quad(&s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}DefinitionResult", LSIF_NS)))));
            }

            Vertex::ReferenceResult { id, .. } => {
                let s = self.id_to_uri(id);
                quads.push(make_quad(&s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}ReferenceResult", LSIF_NS)))));
            }

            Vertex::DeclarationResult { id, .. } => {
                let s = self.id_to_uri(id);
                quads.push(make_quad(&s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}DeclarationResult", MAX_NS))))); // Or custom private class
            }
            
            Vertex::ImplementationResult { id, .. } => {
                let s = self.id_to_uri(id);
                quads.push(make_quad(&s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}ImplementationResult", MAX_NS)))));
            }

            Vertex::TypeDefinitionResult { id, .. } => {
                let s = self.id_to_uri(id);
                quads.push(make_quad(&s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}TypeDefinitionResult", MAX_NS)))));
            }

            _ => {} // Handle remaining result/metadata kind variants defensively
        }

        quads
    }

    /// Translate a single LSIF Edge to a set of Quads
    pub fn map_edge(&self, edge: &Edge) -> Vec<Quad> {
        let mut quads = Vec::new();
        let new_node = |uri: &str| NamedNode::new(uri).unwrap();
        let make_quad = |s: &str, p: &str, o: Term| {
            Quad::new(new_node(s), new_node(p), o, self.graph_name.clone())
        };

        match edge {
            Edge::Contains { out_v, in_vs, .. } => {
                let s = self.id_to_uri(out_v);
                for target in in_vs {
                    quads.push(make_quad(&s, &format!("{}contains", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(target)))));
                }
            }

            Edge::Next { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}next", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }

            Edge::Moniker { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}moniker", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }

            Edge::Attach { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}attach", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }

            Edge::PackageInformation { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}packageInformation", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }

            Edge::Item { out_v, in_vs, document, property, .. } => {
                let s = self.id_to_uri(out_v);
                let doc_uri = self.id_to_uri(document);
                for target in in_vs {
                    let target_uri = self.id_to_uri(target);
                    quads.push(make_quad(&s, &format!("{}item", LSIF_NS), Term::NamedNode(new_node(&target_uri))));
                    quads.push(make_quad(&target_uri, &format!("{}document", LSIF_NS), Term::NamedNode(new_node(&doc_uri))));
                    if let Some(prop) = property {
                        let prop_str = match prop {
                            ItemEdgeProperty::Definitions => "definitions",
                            ItemEdgeProperty::Declarations => "declarations",
                            ItemEdgeProperty::References => "references",
                            ItemEdgeProperty::ReferenceResults => "referenceResults",
                            ItemEdgeProperty::ImplementationResults => "implementationResults",
                            ItemEdgeProperty::TypeDefinitions => "typeDefinitionResults",
                            ItemEdgeProperty::ReferenceLinks => "referenceLinks",
                        };
                        quads.push(make_quad(&target_uri, &format!("{}property", LSIF_NS), Term::Literal(Literal::new_simple_literal(prop_str))));
                    }
                }
            }

            // LSP Query relation edges (e.g. textDocument/hover, textDocument/definition)
            Edge::TextDocumentHover { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}textDocument_hover", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }
            Edge::TextDocumentDefinition { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}textDocument_definition", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }
            Edge::TextDocumentDeclaration { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}textDocument_declaration", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }
            Edge::TextDocumentReferences { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}textDocument_references", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }
            Edge::TextDocumentImplementation { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}textDocument_implementation", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }
            Edge::TextDocumentTypeDefinition { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}textDocument_typeDefinition", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }
            Edge::TextDocumentFoldingRange { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}textDocument_foldingRange", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }
            Edge::TextDocumentDocumentLink { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}textDocument_documentLink", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }
            Edge::TextDocumentDocumentSymbol { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}textDocument_documentSymbol", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }
            Edge::TextDocumentDiagnostic { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}textDocument_diagnostic", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }
            Edge::TextDocumentSemanticTokens { out_v, in_v, .. } => {
                let s = self.id_to_uri(out_v);
                quads.push(make_quad(&s, &format!("{}textDocument_semanticTokens", LSIF_NS), Term::NamedNode(new_node(&self.id_to_uri(in_v)))));
            }
        }

        quads
    }

    /// Map a MaxDiagnostic observation, rules, and optional receipt to RDF Quads
    pub fn map_diagnostic(&self, diag: &MaxDiagnostic) -> Vec<Quad> {
        let mut quads = Vec::new();
        let new_node = |uri: &str| NamedNode::new(uri).unwrap();
        let make_quad = |s: &str, p: &str, o: Term| {
            Quad::new(new_node(s), new_node(p), o, self.graph_name.clone())
        };

        let diag_s = format!("{}diag:{}", PROJ_NS, diag.diagnostic_id);
        
        // 1. Diagnostic base metadata
        quads.push(make_quad(&diag_s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}Diagnostic", MAX_NS)))));
        quads.push(make_quad(&diag_s, &format!("{}message", MAX_NS), Term::Literal(Literal::new_simple_literal(&diag.lsp.message))));
        
        let severity_str = match diag.lsp.severity {
            Some(lsp_types::DiagnosticSeverity::ERROR) => "error",
            Some(lsp_types::DiagnosticSeverity::WARNING) => "warning",
            Some(lsp_types::DiagnosticSeverity::INFORMATION) => "info",
            Some(lsp_types::DiagnosticSeverity::HINT) => "hint",
            _ => "error",
        };
        quads.push(make_quad(&diag_s, &format!("{}severity", MAX_NS), Term::Literal(Literal::new_simple_literal(severity_str))));
        
        // 2. Conformance rule mapping
        if !diag.law_id.is_empty() {
            let rule_s = format!("{}rule:{}", PROJ_NS, diag.law_id);
            quads.push(make_quad(&diag_s, &format!("{}conformsTo", MAX_NS), Term::NamedNode(new_node(&rule_s))));
            quads.push(make_quad(&rule_s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}Rule", MAX_NS)))));
            quads.push(make_quad(&rule_s, &format!("{}identifier", MAX_NS), Term::Literal(Literal::new_simple_literal(&diag.law_id))));
        }

        // 3. Extended diagnostic fields (law axis, violated invariant, etc.)
        quads.push(make_quad(&diag_s, &format!("{}lawAxis", MAX_NS), Term::Literal(Literal::new_simple_literal(format!("{:?}", diag.law_axis)))));
        if !diag.violated_invariant.is_empty() {
            quads.push(make_quad(&diag_s, &format!("{}violatedInvariant", MAX_NS), Term::Literal(Literal::new_simple_literal(&diag.violated_invariant))));
        }
        quads.push(make_quad(&diag_s, &format!("{}repairability", MAX_NS), Term::Literal(Literal::new_simple_literal(format!("{:?}", diag.repairability)))));
        quads.push(make_quad(&diag_s, &format!("{}terminality", MAX_NS), Term::Literal(Literal::new_simple_literal(format!("{:?}", diag.terminality)))));

        // 4. Source range linkage
        let range_s = format!("{}diag_range:{}", PROJ_NS, diag.diagnostic_id);
        quads.push(make_quad(&diag_s, &format!("{}range", LSIF_NS), Term::NamedNode(new_node(&range_s))));
        quads.push(make_quad(&range_s, RDF_TYPE, Term::NamedNode(new_node(&format!("{}Range", LSIF_NS)))));
        
        let int_type = new_node(&format!("{}integer", XSD_NS));
        quads.push(make_quad(&range_s, &format!("{}startLine", MAX_NS), Term::Literal(Literal::new_typed_literal(diag.lsp.range.start.line.to_string(), int_type.clone()))));
        quads.push(make_quad(&range_s, &format!("{}startCharacter", MAX_NS), Term::Literal(Literal::new_typed_literal(diag.lsp.range.start.character.to_string(), int_type.clone()))));
        quads.push(make_quad(&range_s, &format!("{}endLine", MAX_NS), Term::Literal(Literal::new_typed_literal(diag.lsp.range.end.line.to_string(), int_type.clone()))));
        quads.push(make_quad(&range_s, &format!("{}endCharacter", MAX_NS), Term::Literal(Literal::new_typed_literal(diag.lsp.range.end.character.to_string(), int_type))));

        // 5. Cryptographic receipt obligation / provenance link
        if let Some(obligation) = &diag.receipt_obligation {
            let rcpt_s = format!("{}receipt:{}", RCPT_NS, obligation.receipt_id);
            quads.push(make_quad(&diag_s, &format!("wasGeneratedBy", PROV_NS), Term::NamedNode(new_node(&rcpt_s))));
            
            // Note: Full receipt details will be generated/asserted as part of the receipt creation transaction.
            // Under PRD-R6, the receipt maps graph hash, query hash, and result hash:
            //
            // rcpt_s a max:Receipt .
            // rcpt_s prov:startedAtTime "timestamp"^^xsd:dateTime .
            // rcpt_s max:graphHash "..." .
            // rcpt_s max:queryHash "..." .
            // rcpt_s max:resultHash "..." .
        }

        quads
    }
}
```

---

## 7. Verification Invariant SPARQL Projections

To confirm that the mapped LSIF element relations and diagnostic observations form a structurally sound and fully receipted control plane graph, the following built-in SPARQL queries should be evaluated inside the ingestion/transaction pipeline.

### 7.1 Orphan Node Verification Query
Detects any references or edges where the target node has not been declared as an LSIF resource (violating database referential integrity):
```sparql
PREFIX lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/>

ASK {
  ?s ?p ?o .
  FILTER(STRSTARTS(STR(?p), STR(lsif:)))
  FILTER(ISIRI(?o))
  FILTER NOT EXISTS { ?o ?any_p ?any_o }
}
# A result of false indicates referential soundness (0 orphans).
```

### 7.2 Diagnostic Unsigned Provenance Verification Query
Detects any diagnostic observations that lack a cryptographically bound receipt under requirement `PRD-R6`:
```sparql
PREFIX max:  <urn:lsp-max:core:>
PREFIX prov: <http://www.w3.org/ns/prov#>

SELECT ?diagnostic WHERE {
  ?diagnostic a max:Diagnostic .
  FILTER NOT EXISTS {
    ?diagnostic prov:wasGeneratedBy ?receipt .
    ?receipt a max:Receipt .
  }
}
# A result set of 0 solutions indicates all diagnostics have valid provenance.
```
