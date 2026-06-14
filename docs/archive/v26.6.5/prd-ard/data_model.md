# Ontology & Vocabulary Mapping: lsp-max v26.6.5

This document specifies the exact mapping of LSIF, LiveLSP diagnostics, and verification receipt metadata to W3C RDF triples.

---

## 1. Overview & Architectural Purpose

The Admitted Graph Control Plane maps structural code relationships and analysis outputs into a unified semantic model. All facts flow through a unidirectional ingestion pipeline, where they are parsed and stored inside RocksDB. Snapshot isolation is enforced using named graphs (`GraphName::NamedNode`), allowing the system to query specific workspace states deterministically.

---

## 2. Data Model Boundary (Anti-Laundering Doctrine)

To prevent "ontology laundering" (where custom tool metadata is mixed into public namespaces), the following rules are strictly enforced at ingestion:
- **Public Vocabulary First**: Standard vocabularies (RDF, RDFS, PROV-O, DCTERMS, DCAT, SKOS, LSIF) are preferred for all common concepts and relationships.
- **Bounded Private Namespace**: Private namespaces (`max:`, `rcpt:`) must only be used where public ontologies are silent (e.g. tracking LiveLSP compilation states, custom cryptographic receipt properties, or agent execution policies).
- **No Masquerading**: No custom predicates or classes may be defined inside the `lsif:` or `prov:` namespaces.

---

## 3. Vocabulary Prefix Matrix

All triples must use the following standard namespace prefixes:

```turtle
@prefix rdf:     <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs:    <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd:     <http://www.w3.org/2001/XMLSchema#> .
@prefix prov:    <http://www.w3.org/ns/prov#> .
@prefix dcterms: <http://purl.org/dc/terms/> .
@prefix dcat:    <http://www.w3.org/ns/dcat#> .
@prefix skos:    <http://www.w3.org/2004/02/skos/core#> .
@prefix sh:      <http://www.w3.org/ns/shacl#> .
@prefix odrl:    <http://www.w3.org/ns/odrl/2/> .
@prefix lsif:    <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/> .
@prefix max:     <urn:lsp-max:core:> .
@prefix rcpt:    <urn:lsp-max:receipt:> .
@prefix proj:    <urn:project:local:> .
```

---

## 4. Graph Object Classes

### LSIF Core Classes
- `lsif:Project`: Represents a compilation unit.
- `lsif:Document`: Represents a source file in the workspace.
- `lsif:Range`: Represents a specific line and character span.
- `lsif:ResultSet`: A vertex representing shared hover/definition/reference results.
- `lsif:Moniker`: Represents import/export symbol identifiers.
- `lsif:PackageInformation`: Represents external dependency details.
- `lsif:HoverResult`: Represents LSP hover payload.
- `lsif:DefinitionResult`: Represents a set of definition targets.
- `lsif:ReferenceResult`: Represents a set of reference sites.

### LiveLSP & Diagnostic Classes
- `max:Diagnostic`: Represents a warning or error emitted by the compiler.
- `max:Rule`: Represents an autonomic compilation rule.

### Control Plane & Provenance Classes
- `max:Artifact`: Represents a generated file, report, or projection.
- `max:Receipt`: Represents a cryptographic proof receipt.
- `max:Replay`: Represents a replayed SPARQL execution run.
- `max:Query`: Represents a registered SPARQL query.
- `max:Shape`: Represents a registered SHACL constraint shape.

### Agent & Capability Projections
- `max:Capability`: Represents an LSP or agent capability.
- `max:Authority`: Represents the authorization scope of an agent/tool.
- `max:Policy`: Represents security policy rules (ODRL).
- `max:Agent`: Represents an autonomous agent.
- `max:Tool`: Represents an agent-executable tool.
- `max:Task`: Represents a delegated task unit.
- `max:Projection`: Represents a generated materialized view output.

---

## 5. Graph Relations (RDF Predicates)

### LSIF Topology
- `lsif:contains`: Relates a project to a document, or a document to a range.
- `lsif:next`: Relates a range or resultSet to a resultSet.
- `lsif:moniker`: Relates a range or resultSet to a moniker.
- `lsif:packageInformation`: Relates a moniker to package info.
- `lsif:attach`: Relates a range/resultSet to package info.
- `lsif:item`: Relates a result node (e.g. `lsif:ReferenceResult`) to target ranges.
- `lsif:document`: Attribute of an `item` edge linking it to its document ID.
- `lsif:property`: Attribute of an `item` edge indicating relationship type.
- `lsif:textDocument_definition`: Connects a range/resultSet to a definition result.
- `lsif:textDocument_references`: Connects a range/resultSet to a reference result.
- `lsif:textDocument_hover`: Connects a range/resultSet to a hover result.

### LiveLSP & Diagnostics
- `max:severity`: Enforces the severity of a diagnostic (`"error"`, `"warning"`, `"info"`, `"hint"`).
- `max:conformsTo`: Links a diagnostic to the `max:Rule` it violates.
- `max:identifier`: Relates a rule definition to its unique identifier string.

### Provenance & Receipts
- `prov:wasGeneratedBy`: Links an artifact or diagnostic to the generating `max:Receipt` or `max:Agent`.
- `prov:wasDerivedFrom`: Links an artifact to its source snapshot.
- `prov:startedAtTime`: Records the execution timestamp of a transaction.
- `max:graphHash`: Records the SHA256 hash of the input graph snapshot.
- `max:queryHash`: Records the SHA256 hash of the SPARQL query executed.
- `max:resultHash`: Records the SHA256 hash of the query results.

### Capabilities, Tools & Projections
- `max:requiresAuthority`: Maps a capability to its required security scope.
- `max:declaresCapability`: Registers a capability to an agent.
- `max:hasInputSchema`: Specifies the input JSON schema for a tool.
- `max:hasSideEffect`: Identifies the execution side effects of a tool.
- `max:sourceRange`: Relates a projection to its corresponding source range (e.g. linking `max:Projection` to its source `lsif:Range`).

---

## 6. RDF Serialization Examples (Turtle Syntax)

### Example 1: LSIF Range and Definition Target
```turtle
@prefix lsif: <https://microsoft.github.io/language-server-protocol/specifications/lsif/0.6.0/specification/> .
@prefix proj: <urn:project:local:> .

# Document contains a Range
proj:doc_1 lsif:contains proj:range_42 .
proj:range_42 a lsif:Range .

# Range links to Definition Result
proj:range_42 lsif:textDocument_definition proj:def_result_100 .
proj:def_result_100 a lsif:DefinitionResult .

# Definition Result points to target Range in doc_1
proj:def_result_100 lsif:item proj:range_101 .
proj:range_101 a lsif:Range ;
               lsif:document proj:doc_1 ;
               lsif:property "definitions" .
```

### Example 2: Cryptographically Receipted Diagnostic
```turtle
@prefix prov: <http://www.w3.org/ns/prov#> .
@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
@prefix max:  <urn:lsp-max:core:> .
@prefix rcpt: <urn:lsp-max:receipt:> .
@prefix proj: <urn:project:local:> .

# A compiler diagnostic warning
proj:diag_55 a max:Diagnostic ;
             max:severity "warning" ;
             max:conformsTo proj:rule_unreachable_code ;
             prov:wasGeneratedBy rcpt:receipt_999 .

# The rule definition
proj:rule_unreachable_code a max:Rule ;
                           max:identifier "E0308" .

# The cryptographic proof receipt
rcpt:receipt_999 a max:Receipt ;
                 prov:startedAtTime "2026-06-05T21:52:00Z"^^xsd:dateTime ;
                 max:graphHash "b5bb9d8014a0f9b1d61e21e796d78dccdf1352f23cd32812f4850b878ae4944c" ;
                 max:queryHash "7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069" ;
                 max:resultHash "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08" .
```
