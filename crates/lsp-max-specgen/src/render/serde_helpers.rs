/// Serde / type-compatibility helpers for the LSP specgen renderer.
use crate::metamodel::*;

use super::{Context, Renderer};

impl Renderer {
    pub(crate) fn type_supports_eq_and_hash(&self, ty: &Type, ctx: &Context) -> bool {
        match ty {
            Type::Base { name } => !matches!(name, BaseTypeName::Decimal),
            Type::Reference { name } => {
                if name == "LspAny" || name == "LSPAny" {
                    return false;
                }
                if ctx.structures.contains_key(name) {
                    return false;
                }
                if let Some(alias) = ctx.type_aliases.get(name) {
                    return self.type_supports_eq_and_hash(&alias.ty, ctx);
                }
                if ctx.enumerations.contains_key(name) {
                    return true;
                }
                false
            }
            Type::Array { element } => self.type_supports_eq_and_hash(element, ctx),
            Type::Map { value, .. } => self.type_supports_eq_and_hash(value, ctx),
            Type::And { items } => items.iter().all(|x| self.type_supports_eq_and_hash(x, ctx)),
            Type::Or { items } => items.iter().all(|x| self.type_supports_eq_and_hash(x, ctx)),
            Type::Tuple { items } => items.iter().all(|x| self.type_supports_eq_and_hash(x, ctx)),
            Type::Literal { .. } => false,
            Type::StringLiteral { .. }
            | Type::IntegerLiteral { .. }
            | Type::BooleanLiteral { .. } => true,
        }
    }

    pub(crate) fn is_valid_map_key_type(&self, key: &MapKeyType, ctx: &Context) -> bool {
        match key {
            MapKeyType::Base { .. } => true,
            MapKeyType::Reference { name, .. } => {
                if name.eq_ignore_ascii_case("DocumentUri")
                    || name.eq_ignore_ascii_case("URI")
                    || name.eq_ignore_ascii_case("string")
                    || name.eq_ignore_ascii_case("integer")
                    || name.eq_ignore_ascii_case("uinteger")
                {
                    return true;
                }
                if let Some(alias) = ctx.type_aliases.get(name) {
                    return self.is_valid_type_for_map_key(&alias.ty, ctx);
                }
                if ctx.enumerations.contains_key(name) {
                    return true;
                }
                false
            }
        }
    }

    pub(crate) fn is_valid_type_for_map_key(&self, ty: &Type, ctx: &Context) -> bool {
        match ty {
            Type::Base { name } => matches!(
                name,
                BaseTypeName::Uri
                    | BaseTypeName::DocumentUri
                    | BaseTypeName::Integer
                    | BaseTypeName::Uinteger
                    | BaseTypeName::String
            ),
            Type::Reference { name } => {
                if let Some(alias) = ctx.type_aliases.get(name) {
                    return self.is_valid_type_for_map_key(&alias.ty, ctx);
                }
                if ctx.enumerations.contains_key(name) {
                    return true;
                }
                false
            }
            _ => false,
        }
    }
}
