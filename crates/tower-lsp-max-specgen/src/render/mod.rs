use anyhow::Result;
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashMap;

use crate::metamodel::*;

mod enums;
mod helpers;
mod requests;
mod serde_helpers;
mod structures;
mod type_resolver;

pub(crate) use helpers::{doc_attr, ident, method_type_name, types_are_equal};

pub(crate) struct Context<'a> {
    pub structures: HashMap<String, &'a Structure>,
    pub type_aliases: HashMap<String, &'a TypeAlias>,
    pub enumerations: HashMap<String, &'a Enumeration>,
}

pub struct Renderer {
    pub(crate) include_proposed: bool,
    pub(crate) generated_types: std::cell::RefCell<std::collections::BTreeMap<String, TokenStream>>,
}

impl Renderer {
    pub fn new(include_proposed: bool) -> Self {
        Self {
            include_proposed,
            generated_types: std::cell::RefCell::new(std::collections::BTreeMap::new()),
        }
    }

    pub fn render(&self, model: &MetaModel) -> Result<String> {
        self.generated_types.borrow_mut().clear();

        let mut structures = HashMap::new();
        for st in &model.structures {
            structures.insert(st.name.clone(), st);
        }
        let mut type_aliases = HashMap::new();
        for ta in &model.type_aliases {
            type_aliases.insert(ta.name.clone(), ta);
        }
        let mut enumerations = HashMap::new();
        for en in &model.enumerations {
            enumerations.insert(en.name.clone(), en);
        }
        let ctx = Context {
            structures,
            type_aliases,
            enumerations,
        };

        let mut out = TokenStream::new();
        let version = &model.meta_data.version;
        out.extend(quote! {
            //! Generated from the official LSP meta-model.
            //! Do not hand-edit generated protocol vocabulary.
            #![cfg_attr(rustfmt, rustfmt_skip)]
            #![allow(clippy::large_enum_variant)]
            #![allow(clippy::enum_variant_names)]
            #![allow(clippy::upper_case_acronyms)]
            #![allow(clippy::doc_lazy_continuation)]
            #![allow(clippy::derive_partial_eq_without_eq)]
            #![allow(dead_code)]
            #![allow(unused_imports)]
            #![allow(non_upper_case_globals)]

            use serde::{Deserialize, Serialize};
            use serde_json::Value as LspAny;

            pub const LSP_SPEC_VERSION: &str = #version;
            pub type URI = String;
            pub type DocumentUri = String;
            pub type RegExp = String;
            pub type Integer = i32;
            pub type Uinteger = u32;
            pub type Decimal = f64;
        });

        let mut type_aliases_out = TokenStream::new();
        for alias in model.type_aliases.iter().filter(|x| self.keep(x.proposed)) {
            type_aliases_out.extend(self.render_type_alias(alias, &ctx)?);
        }
        let mut enumerations_out = TokenStream::new();
        for en in model.enumerations.iter().filter(|x| self.keep(x.proposed)) {
            enumerations_out.extend(self.render_enumeration(en)?);
        }
        let mut structures_out = TokenStream::new();
        for st in model.structures.iter().filter(|x| self.keep(x.proposed)) {
            structures_out.extend(self.render_structure(st, &ctx)?);
        }

        let requests_out = self.render_requests(model, &ctx)?;
        let notifications_out = self.render_notifications(model, &ctx)?;

        let mut generated_out = TokenStream::new();
        {
            let gen = self.generated_types.borrow();
            for tokens in gen.values() {
                generated_out.extend(tokens.clone());
            }
        }

        out.extend(type_aliases_out);
        out.extend(enumerations_out);
        out.extend(structures_out);
        out.extend(generated_out);
        out.extend(requests_out);
        out.extend(notifications_out);

        let file = syn::parse2(out)?;
        Ok(prettyplease::unparse(&file))
    }

    pub(crate) fn keep(&self, proposed: Option<bool>) -> bool {
        self.include_proposed || proposed != Some(true)
    }

    pub(crate) fn type_to_string(&self, ty: &Type) -> Result<String> {
        match ty {
            Type::Base { name } => Ok(match name {
                BaseTypeName::Uri => "Uri".to_string(),
                BaseTypeName::DocumentUri => "DocumentUri".to_string(),
                BaseTypeName::Integer => "Integer".to_string(),
                BaseTypeName::Uinteger => "Uinteger".to_string(),
                BaseTypeName::Decimal => "Decimal".to_string(),
                BaseTypeName::RegExp => "RegExp".to_string(),
                BaseTypeName::String => "String".to_string(),
                BaseTypeName::Boolean => "Boolean".to_string(),
                BaseTypeName::Null => "Null".to_string(),
            }),
            Type::Reference { name } => Ok(name.clone()),
            Type::Array { element } => {
                let inner = self.type_to_string(element)?;
                Ok(format!("{}Array", inner))
            }
            Type::Map { key, value } => {
                let k = match key {
                    MapKeyType::Base { name, .. } => match name {
                        MapKeyBaseName::Uri => "Uri",
                        MapKeyBaseName::DocumentUri => "DocumentUri",
                        MapKeyBaseName::String => "String",
                        MapKeyBaseName::Integer => "Integer",
                    },
                    MapKeyType::Reference { name, .. } => name.as_str(),
                };
                let v = self.type_to_string(value)?;
                Ok(format!("MapOf{}To{}", k, v))
            }
            Type::And { items } => {
                let parts: Result<Vec<String>> =
                    items.iter().map(|item| self.type_to_string(item)).collect();
                Ok(parts?.join("And"))
            }
            Type::Or { items } => {
                let parts: Result<Vec<String>> =
                    items.iter().map(|item| self.type_to_string(item)).collect();
                Ok(parts?.join("Or"))
            }
            Type::Tuple { items } => {
                let parts: Result<Vec<String>> =
                    items.iter().map(|item| self.type_to_string(item)).collect();
                Ok(format!("TupleOf{}", parts?.join("And")))
            }
            Type::StringLiteral { value } => {
                Ok(format!("StringLiteral{}", value.to_upper_camel_case()))
            }
            Type::IntegerLiteral { value } => Ok(format!("IntegerLiteral{}", value)),
            Type::BooleanLiteral { value } => Ok(format!("BooleanLiteral{}", value)),
            Type::Literal { value } => {
                let props_json = serde_json::to_string(&value.properties).unwrap_or_default();
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                use std::hash::Hasher;
                hasher.write(props_json.as_bytes());
                let hash = hasher.finish();
                Ok(format!("Literal{:x}", hash))
            }
        }
    }
}
