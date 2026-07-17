//! Static schema types for lsp-max cognitive breed and pipeline metadata.
//!
//! `StaticSchema` and `SchemaNode` provide a composable type system for describing
//! breed invariants, pipeline stage contracts, and conformance predicates.
//! They serve as the type authority for TPOT2-style breed discovery and selection.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A schema node representing a single type or constraint in the type hierarchy.
///
/// Schema nodes are immutable, hashable, and compose recursively to form complete
/// type signatures for breed inputs, outputs, and intermediate transformations.
///
/// # Variants
///
/// - `Primitive`: Atomic types (String, Integer, Boolean, Bytes).
/// - `Optional`: A node that may be absent (like `Option<T>` in Rust).
/// - `Array`: A homogeneous collection of nodes.
/// - `Record`: A named tuple of keyed fields (like a struct).
/// - `Enum`: A sum type with named variants.
/// - `Ref`: A reference to a named type in the schema registry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum SchemaNode {
    /// Primitive scalar type.
    #[serde(rename = "primitive")]
    Primitive(PrimitiveType),

    /// Optional node: may be absent or present.
    #[serde(rename = "optional")]
    Optional(Box<SchemaNode>),

    /// Homogeneous array of nodes.
    #[serde(rename = "array")]
    Array(Box<SchemaNode>),

    /// Named record (struct-like) with keyed fields.
    #[serde(rename = "record")]
    Record {
        /// Field name to schema node mapping.
        fields: BTreeMap<String, SchemaNode>,
    },

    /// Sum type (enum-like) with named variants.
    #[serde(rename = "enum")]
    Enum {
        /// Variant name to optional inner schema mapping.
        variants: BTreeMap<String, Option<Box<SchemaNode>>>,
    },

    /// Reference to a named type in the schema registry.
    #[serde(rename = "ref")]
    Ref(String),
}

/// Primitive scalar types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrimitiveType {
    /// UTF-8 string.
    String,
    /// Arbitrary-precision integer.
    Integer,
    /// IEEE 754 double-precision floating-point.
    Float,
    /// Boolean true/false.
    Boolean,
    /// Arbitrary binary data.
    Bytes,
}

impl SchemaNode {
    /// Create a primitive string node.
    pub fn string() -> Self {
        SchemaNode::Primitive(PrimitiveType::String)
    }

    /// Create a primitive integer node.
    pub fn integer() -> Self {
        SchemaNode::Primitive(PrimitiveType::Integer)
    }

    /// Create a primitive float node.
    pub fn float() -> Self {
        SchemaNode::Primitive(PrimitiveType::Float)
    }

    /// Create a primitive boolean node.
    pub fn boolean() -> Self {
        SchemaNode::Primitive(PrimitiveType::Boolean)
    }

    /// Create a primitive bytes node.
    pub fn bytes() -> Self {
        SchemaNode::Primitive(PrimitiveType::Bytes)
    }

    /// Wrap this node in an `Optional`.
    pub fn optional(self) -> Self {
        SchemaNode::Optional(Box::new(self))
    }

    /// Wrap this node in an `Array`.
    pub fn array(self) -> Self {
        SchemaNode::Array(Box::new(self))
    }

    /// Create a record node from a map of field names to schemas.
    pub fn record(fields: BTreeMap<String, SchemaNode>) -> Self {
        SchemaNode::Record { fields }
    }

    /// Create an enum node from a map of variant names to optional inner schemas.
    pub fn enum_type(variants: BTreeMap<String, Option<Box<SchemaNode>>>) -> Self {
        SchemaNode::Enum { variants }
    }

    /// Create a reference to a named type.
    pub fn ref_to(name: impl Into<String>) -> Self {
        SchemaNode::Ref(name.into())
    }
}

/// A complete static schema registry mapping type names to schema nodes.
///
/// `StaticSchema` is the configuration for a cognitive breed or pipeline stage.
/// It defines the invariants that the runtime must respect: input shapes, output
/// shapes, intermediate transformations, and conformance predicates.
///
/// Schemas are immutable after construction and are typically created at compile time
/// or loaded from TOML/JSON manifests during server initialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticSchema {
    /// Human-readable name for this schema.
    pub name: String,

    /// Version identifier (e.g., "1.0", "26.6.24" in CalVer format).
    pub version: String,

    /// Registry mapping type names to their schema nodes.
    /// The root schema is typically named "root" or derived from the breed name.
    pub types: BTreeMap<String, SchemaNode>,

    /// Optional docstring describing the schema's purpose and constraints.
    #[serde(default)]
    pub description: String,

    /// Conformance predicate: a set of invariant names that MUST be satisfied.
    /// These map to runtime law-axis checks (e.g., "determinism", "receipt-chain", "no-oracle").
    #[serde(default)]
    pub invariants: Vec<String>,

    /// Metadata key-value store for schema-level annotations.
    /// Common keys: "breed_id", "pipeline_stage", "author", "tags".
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl StaticSchema {
    /// Create a new schema with a given name and version.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        StaticSchema {
            name: name.into(),
            version: version.into(),
            types: BTreeMap::new(),
            description: String::new(),
            invariants: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Register a named type in this schema.
    pub fn register(mut self, name: impl Into<String>, node: SchemaNode) -> Self {
        self.types.insert(name.into(), node);
        self
    }

    /// Set the schema description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a conformance invariant.
    pub fn with_invariant(mut self, invariant: impl Into<String>) -> Self {
        self.invariants.push(invariant.into());
        self
    }

    /// Add a metadata key-value pair.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Look up a type by name, resolving references recursively.
    ///
    /// # Returns
    ///
    /// - `Ok(SchemaNode)` if the type exists and all references resolve.
    /// - `Err(String)` if a type is not found or a reference cycle is detected.
    pub fn resolve(&self, name: &str) -> Result<SchemaNode, String> {
        let mut visited = std::collections::HashSet::new();
        self.resolve_inner(name, &mut visited)
    }

    fn resolve_inner(
        &self,
        name: &str,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<SchemaNode, String> {
        if visited.contains(name) {
            return Err(format!("Reference cycle detected: {}", name));
        }

        let node = self
            .types
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Type not found: {}", name))?;

        visited.insert(name.to_string());
        self.resolve_node(&node, visited)
    }

    fn resolve_node(
        &self,
        node: &SchemaNode,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<SchemaNode, String> {
        match node {
            SchemaNode::Ref(name) => {
                if visited.contains(name) {
                    return Err(format!("Reference cycle detected: {}", name));
                }
                let mut new_visited = visited.clone();
                new_visited.insert(name.clone());
                let resolved = self
                    .types
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("Type not found: {}", name))?;
                self.resolve_node(&resolved, &mut new_visited)
            }
            SchemaNode::Optional(inner) => {
                let resolved = self.resolve_node(inner, visited)?;
                Ok(SchemaNode::Optional(Box::new(resolved)))
            }
            SchemaNode::Array(inner) => {
                let resolved = self.resolve_node(inner, visited)?;
                Ok(SchemaNode::Array(Box::new(resolved)))
            }
            SchemaNode::Record { fields } => {
                let resolved_fields = fields
                    .iter()
                    .map(|(k, v)| {
                        self.resolve_node(v, visited).map(|resolved| (k.clone(), resolved))
                    })
                    .collect::<Result<BTreeMap<String, SchemaNode>, String>>()?;
                Ok(SchemaNode::Record {
                    fields: resolved_fields,
                })
            }
            SchemaNode::Enum { variants } => {
                let resolved_variants = variants
                    .iter()
                    .map(|(k, v)| {
                        let resolved_inner = match v {
                            Some(inner) => {
                                let resolved = self.resolve_node(inner, visited)?;
                                Ok(Some(Box::new(resolved)))
                            }
                            None => Ok(None),
                        };
                        resolved_inner.map(|inner| (k.clone(), inner))
                    })
                    .collect::<Result<BTreeMap<String, Option<Box<SchemaNode>>>, String>>()?;
                Ok(SchemaNode::Enum {
                    variants: resolved_variants,
                })
            }
            other => Ok(other.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_constructors() {
        assert_eq!(SchemaNode::string(), SchemaNode::Primitive(PrimitiveType::String));
        assert_eq!(SchemaNode::integer(), SchemaNode::Primitive(PrimitiveType::Integer));
        assert_eq!(SchemaNode::float(), SchemaNode::Primitive(PrimitiveType::Float));
        assert_eq!(SchemaNode::boolean(), SchemaNode::Primitive(PrimitiveType::Boolean));
        assert_eq!(SchemaNode::bytes(), SchemaNode::Primitive(PrimitiveType::Bytes));
    }

    #[test]
    fn test_optional_and_array() {
        let string_opt = SchemaNode::string().optional();
        assert!(matches!(string_opt, SchemaNode::Optional(_)));

        let int_array = SchemaNode::integer().array();
        assert!(matches!(int_array, SchemaNode::Array(_)));

        let nested = SchemaNode::string().optional().array();
        assert!(matches!(nested, SchemaNode::Array(_)));
    }

    #[test]
    fn test_record_construction() {
        let mut fields = BTreeMap::new();
        fields.insert("name".to_string(), SchemaNode::string());
        fields.insert("age".to_string(), SchemaNode::integer());

        let record = SchemaNode::record(fields);
        match record {
            SchemaNode::Record { fields } => {
                assert_eq!(fields.len(), 2);
                assert!(fields.contains_key("name"));
                assert!(fields.contains_key("age"));
            }
            _ => panic!("Expected Record variant"),
        }
    }

    #[test]
    fn test_enum_construction() {
        let mut variants = BTreeMap::new();
        variants.insert("Some".to_string(), Some(Box::new(SchemaNode::string())));
        variants.insert("None".to_string(), None);

        let enum_node = SchemaNode::enum_type(variants);
        match enum_node {
            SchemaNode::Enum { variants } => {
                assert_eq!(variants.len(), 2);
                assert!(variants.contains_key("Some"));
                assert!(variants.contains_key("None"));
            }
            _ => panic!("Expected Enum variant"),
        }
    }

    #[test]
    fn test_static_schema_builder() {
        let schema = StaticSchema::new("test_breed", "1.0")
            .with_description("A test schema")
            .register("input", SchemaNode::string())
            .register("output", SchemaNode::integer())
            .with_invariant("determinism")
            .with_metadata("breed_id", "test-breed-v1");

        assert_eq!(schema.name, "test_breed");
        assert_eq!(schema.version, "1.0");
        assert_eq!(schema.description, "A test schema");
        assert_eq!(schema.types.len(), 2);
        assert_eq!(schema.invariants.len(), 1);
        assert_eq!(schema.metadata.len(), 1);
    }

    #[test]
    fn test_resolve_simple_type() {
        let schema = StaticSchema::new("test", "1.0")
            .register("input", SchemaNode::string());

        let resolved = schema.resolve("input").expect("resolve failed");
        assert_eq!(resolved, SchemaNode::string());
    }

    #[test]
    fn test_resolve_reference() {
        let schema = StaticSchema::new("test", "1.0")
            .register("input", SchemaNode::string())
            .register("alias", SchemaNode::ref_to("input"));

        let resolved = schema.resolve("alias").expect("resolve failed");
        assert_eq!(resolved, SchemaNode::string());
    }

    #[test]
    fn test_resolve_reference_cycle_detection() {
        let schema = StaticSchema::new("test", "1.0")
            .register("a", SchemaNode::ref_to("b"))
            .register("b", SchemaNode::ref_to("a"));

        let result = schema.resolve("a");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Reference cycle"));
    }

    #[test]
    fn test_resolve_nested_record() {
        let mut fields = BTreeMap::new();
        fields.insert("id".to_string(), SchemaNode::integer());
        fields.insert("name".to_string(), SchemaNode::ref_to("name_type"));

        let schema = StaticSchema::new("test", "1.0")
            .register("record", SchemaNode::record(fields))
            .register("name_type", SchemaNode::string());

        let resolved = schema.resolve("record").expect("resolve failed");
        match resolved {
            SchemaNode::Record { fields } => {
                assert_eq!(fields.len(), 2);
                let name_field = &fields["name"];
                assert_eq!(*name_field, SchemaNode::string());
            }
            _ => panic!("Expected Record"),
        }
    }

    #[test]
    fn test_serde_round_trip() {
        let schema = StaticSchema::new("test_breed", "26.6.24")
            .with_description("A test schema for serialization")
            .register("input", SchemaNode::string().array())
            .register("output", SchemaNode::integer().optional())
            .with_invariant("determinism")
            .with_metadata("breed_id", "test-breed-26.6.24");

        let json = serde_json::to_string(&schema).expect("serialization failed");
        let deserialized: StaticSchema = serde_json::from_str(&json).expect("deserialization failed");

        assert_eq!(schema, deserialized);
    }
}
