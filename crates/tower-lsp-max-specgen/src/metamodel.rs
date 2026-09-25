use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaModel {
    pub meta_data: MetaData,
    pub requests: Vec<Request>,
    pub notifications: Vec<Notification>,
    pub structures: Vec<Structure>,
    pub enumerations: Vec<Enumeration>,
    pub type_aliases: Vec<TypeAlias>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaData {
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub method: String,
    #[serde(default)]
    pub type_name: Option<String>,
    pub result: Type,
    pub message_direction: MessageDirection,
    #[serde(default)]
    pub client_capability: Option<String>,
    #[serde(default)]
    pub server_capability: Option<String>,
    #[serde(default)]
    pub params: Option<OneOrManyTypes>,
    #[serde(default)]
    pub partial_result: Option<Type>,
    #[serde(default)]
    pub error_data: Option<Type>,
    #[serde(default)]
    pub registration_method: Option<String>,
    #[serde(default)]
    pub registration_options: Option<Type>,
    #[serde(default)]
    pub documentation: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub since_tags: Option<Vec<String>>,
    #[serde(default)]
    pub proposed: Option<bool>,
    #[serde(default)]
    pub deprecated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub method: String,
    pub message_direction: MessageDirection,
    #[serde(default)]
    pub type_name: Option<String>,
    #[serde(default)]
    pub client_capability: Option<String>,
    #[serde(default)]
    pub server_capability: Option<String>,
    #[serde(default)]
    pub params: Option<OneOrManyTypes>,
    #[serde(default)]
    pub registration_method: Option<String>,
    #[serde(default)]
    pub registration_options: Option<Type>,
    #[serde(default)]
    pub documentation: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub since_tags: Option<Vec<String>>,
    #[serde(default)]
    pub proposed: Option<bool>,
    #[serde(default)]
    pub deprecated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Structure {
    pub name: String,
    #[serde(default)]
    pub extends: Vec<Type>,
    #[serde(default)]
    pub mixins: Vec<Type>,
    #[serde(default)]
    pub properties: Vec<Property>,
    #[serde(default)]
    pub documentation: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub since_tags: Option<Vec<String>>,
    #[serde(default)]
    pub proposed: Option<bool>,
    #[serde(default)]
    pub deprecated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Property {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Type,
    #[serde(default)]
    pub optional: Option<bool>,
    #[serde(default)]
    pub documentation: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub since_tags: Option<Vec<String>>,
    #[serde(default)]
    pub proposed: Option<bool>,
    #[serde(default)]
    pub deprecated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Enumeration {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: EnumerationType,
    pub values: Vec<EnumerationEntry>,
    #[serde(default)]
    pub supports_custom_values: Option<bool>,
    #[serde(default)]
    pub documentation: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub since_tags: Option<Vec<String>>,
    #[serde(default)]
    pub proposed: Option<bool>,
    #[serde(default)]
    pub deprecated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumerationEntry {
    pub name: String,
    pub value: EnumValue,
    #[serde(default)]
    pub documentation: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub since_tags: Option<Vec<String>>,
    #[serde(default)]
    pub proposed: Option<bool>,
    #[serde(default)]
    pub deprecated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnumValue {
    String(String),
    Number(i64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeAlias {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Type,
    #[serde(default)]
    pub documentation: Option<String>,
    #[serde(default)]
    pub since: Option<String>,
    #[serde(default)]
    pub since_tags: Option<Vec<String>>,
    #[serde(default)]
    pub proposed: Option<bool>,
    #[serde(default)]
    pub deprecated: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnumerationType {
    pub kind: String,
    pub name: EnumerationBaseType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageDirection {
    ClientToServer,
    ServerToClient,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EnumerationBaseType {
    String,
    Integer,
    Uinteger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOrManyTypes {
    One(Type),
    Many(Vec<Type>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureLiteral {
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Type {
    Base { name: BaseTypeName },
    Reference { name: String },
    Array { element: Box<Type> },
    Map { key: MapKeyType, value: Box<Type> },
    And { items: Vec<Type> },
    Or { items: Vec<Type> },
    Tuple { items: Vec<Type> },
    StringLiteral { value: String },
    IntegerLiteral { value: i64 },
    BooleanLiteral { value: bool },
    Literal { value: StructureLiteral },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BaseTypeName {
    #[serde(rename = "URI")]
    Uri,
    #[serde(rename = "DocumentUri")]
    DocumentUri,
    Integer,
    Uinteger,
    Decimal,
    RegExp,
    String,
    Boolean,
    Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MapKeyType {
    Base { kind: String, name: MapKeyBaseName },
    Reference { kind: String, name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MapKeyBaseName {
    #[serde(rename = "URI")]
    Uri,
    #[serde(rename = "DocumentUri")]
    DocumentUri,
    String,
    Integer,
}
