use anyhow::Result;
use heck::{ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote};
use std::collections::HashMap;

use crate::metamodel::*;

struct Context<'a> {
    structures: HashMap<String, &'a Structure>,
    type_aliases: HashMap<String, &'a TypeAlias>,
    enumerations: HashMap<String, &'a Enumeration>,
}

pub struct Renderer {
    include_proposed: bool,
    generated_types: std::cell::RefCell<std::collections::BTreeMap<String, TokenStream>>,
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

        // Render generated helper types (unions, intersections, literals)
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

    fn keep(&self, proposed: Option<bool>) -> bool {
        self.include_proposed || proposed != Some(true)
    }

    fn render_type_alias(&self, alias: &TypeAlias, ctx: &Context) -> Result<TokenStream> {
        let name = ident(&alias.name);
        let ty = self.rust_type(&alias.ty, ctx)?;
        let doc = doc_attr(alias.documentation.as_deref());
        Ok(quote! {
            #doc
            pub type #name = #ty;
        })
    }

    fn render_structure(&self, st: &Structure, ctx: &Context) -> Result<TokenStream> {
        let name = ident(&st.name);
        let doc = doc_attr(st.documentation.as_deref());
        let mut fields = Vec::<TokenStream>::new();

        for ext in &st.extends {
            match ext {
                Type::Reference { name } => {
                    let field_name = ident(&format!("{}_base", name.to_snake_case()));
                    let ty = ident(name);
                    fields.push(quote! {
                        #[serde(flatten)]
                        pub #field_name: #ty,
                    });
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Non-reference extends in structure {}: {:?}",
                        st.name,
                        ext
                    ));
                }
            }
        }
        for mixin in &st.mixins {
            match mixin {
                Type::Reference { name } => {
                    let field_name = ident(&format!("{}_mixin", name.to_snake_case()));
                    let ty = ident(name);
                    fields.push(quote! {
                        #[serde(flatten)]
                        pub #field_name: #ty,
                    });
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Non-reference mixins in structure {}: {:?}",
                        st.name,
                        mixin
                    ));
                }
            }
        }
        for prop in &st.properties {
            fields.push(self.render_property(&st.name, prop, ctx)?);
        }

        Ok(quote! {
            #doc
            #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
            #[serde(rename_all = "camelCase")]
            pub struct #name {
                #(#fields)*
            }
        })
    }

    fn render_property(
        &self,
        struct_name: &str,
        prop: &Property,
        ctx: &Context,
    ) -> Result<TokenStream> {
        let raw_name = &prop.name;
        let rust_name = ident(&raw_name.to_snake_case());
        let mut ty = self.rust_type(&prop.ty, ctx)?;

        let is_self_ref = match &prop.ty {
            Type::Reference { name } => name == struct_name,
            _ => false,
        };

        if is_self_ref {
            ty = quote! { Box<#ty> };
        }

        if prop.optional.unwrap_or(false) {
            ty = quote! { Option<#ty> };
        }
        let doc = doc_attr(prop.documentation.as_deref());
        let rename = if raw_name.to_snake_case() != *raw_name {
            let lit = Literal::string(raw_name);
            quote! { #[serde(rename = #lit)] }
        } else {
            quote! {}
        };
        let default = if prop.optional.unwrap_or(false) {
            quote! { #[serde(default)] }
        } else {
            quote! {}
        };
        Ok(quote! {
            #doc
            #rename
            #default
            pub #rust_name: #ty,
        })
    }

    fn render_enumeration(&self, en: &Enumeration) -> Result<TokenStream> {
        let name = ident(&en.name);
        let doc = doc_attr(en.documentation.as_deref());
        let open = en.supports_custom_values.unwrap_or(false);
        let value_ty = match en.ty.name {
            EnumerationBaseType::String => quote! { String },
            EnumerationBaseType::Integer => quote! { i32 },
            EnumerationBaseType::Uinteger => quote! { u32 },
        };

        if open {
            let consts = en.values.iter().map(|v| {
                let cname = ident(&v.name.to_shouty_snake_case());
                match &v.value {
                    EnumValue::String(s) => {
                        let lit = Literal::string(s);
                        quote! { pub const #cname: &'static str = #lit; }
                    }
                    EnumValue::Number(n) => {
                        let lit = Literal::i64_unsuffixed(*n);
                        quote! { pub const #cname: #value_ty = #lit; }
                    }
                }
            });
            Ok(quote! {
                #doc
                #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
                #[serde(transparent)]
                pub struct #name(pub #value_ty);
                impl #name {
                    #(#consts)*
                }
            })
        } else {
            let is_integer = match en.ty.name {
                EnumerationBaseType::String => false,
                EnumerationBaseType::Integer | EnumerationBaseType::Uinteger => true,
            };

            if is_integer {
                let variants = en.values.iter().map(|v| {
                    let vname = ident(&v.name.to_upper_camel_case());
                    let vdoc = doc_attr(v.documentation.as_deref());
                    quote! { #vdoc #vname, }
                });
                let serialize_arms = en.values.iter().map(|v| {
                    let vname = ident(&v.name.to_upper_camel_case());
                    let val = match &v.value {
                        EnumValue::Number(n) => *n,
                        EnumValue::String(s) => s.parse::<i64>().unwrap_or(0),
                    };
                    quote! { Self::#vname => serializer.serialize_i64(#val), }
                });
                let deserialize_arms = en.values.iter().map(|v| {
                    let vname = ident(&v.name.to_upper_camel_case());
                    let val = match &v.value {
                        EnumValue::Number(n) => *n,
                        EnumValue::String(s) => s.parse::<i64>().unwrap_or(0),
                    };
                    quote! { #val => Ok(#name::#vname), }
                });
                Ok(quote! {
                    #doc
                    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
                    pub enum #name {
                        #(#variants)*
                    }
                    impl serde::Serialize for #name {
                        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                        where
                            S: serde::Serializer,
                        {
                            match self {
                                #(#serialize_arms)*
                            }
                        }
                    }
                    impl<'de> serde::Deserialize<'de> for #name {
                        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                        where
                            D: serde::Deserializer<'de>,
                        {
                            struct Visitor;
                            impl<'de> serde::de::Visitor<'de> for Visitor {
                                type Value = #name;
                                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                                    formatter.write_str("a number representing the enum")
                                }
                                fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
                                  where E: serde::de::Error
                                {
                                    match value {
                                        #(#deserialize_arms)*
                                        _ => Err(E::custom(format!("unknown variant: {}", value))),
                                    }
                                }
                                fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
                                  where E: serde::de::Error
                                {
                                    self.visit_i64(value as i64)
                                }
                            }
                            deserializer.deserialize_i64(Visitor)
                        }
                    }
                })
            } else {
                let variants = en.values.iter().map(|v| {
                    let vname = ident(&v.name.to_upper_camel_case());
                    let vdoc = doc_attr(v.documentation.as_deref());
                    match &v.value {
                        EnumValue::String(s) => {
                            let lit = Literal::string(s);
                            quote! { #vdoc #[serde(rename = #lit)] #vname, }
                        }
                        EnumValue::Number(n) => {
                            let lit = Literal::string(&n.to_string());
                            quote! { #vdoc #[serde(rename = #lit)] #vname, }
                        }
                    }
                });
                Ok(quote! {
                    #doc
                    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
                    pub enum #name {
                        #(#variants)*
                    }
                })
            }
        }
    }

    fn render_requests(&self, model: &MetaModel, ctx: &Context) -> Result<TokenStream> {
        let variants = model
            .requests
            .iter()
            .filter(|x| self.keep(x.proposed))
            .map(|r| {
                let name = ident(
                    r.type_name
                        .as_deref()
                        .unwrap_or(&method_type_name(&r.method)),
                );
                quote! { #name, }
            });
        let mut impls = Vec::new();
        for r in model.requests.iter().filter(|x| self.keep(x.proposed)) {
            let name = ident(
                r.type_name
                    .as_deref()
                    .unwrap_or(&method_type_name(&r.method)),
            );
            let method = Literal::string(&r.method);
            let params = match &r.params {
                Some(OneOrManyTypes::One(t)) => self.rust_type(t, ctx)?,
                Some(OneOrManyTypes::Many(_)) => quote! { LspAny },
                None => quote! { () },
            };
            let result = self.rust_type(&r.result, ctx)?;
            impls.push(quote! {
                pub struct #name;
                impl LspRequest for #name {
                    type Params = #params;
                    type Result = #result;
                    const METHOD: &'static str = #method;
                }
            });
        }
        Ok(quote! {
            pub trait LspRequest {
                type Params: Serialize + for<'de> Deserialize<'de>;
                type Result: Serialize + for<'de> Deserialize<'de>;
                const METHOD: &'static str;
            }
            #(#impls)*
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub enum KnownRequest {
                #(#variants)*
            }
        })
    }

    fn render_notifications(&self, model: &MetaModel, ctx: &Context) -> Result<TokenStream> {
        let mut impls = Vec::new();
        for n in model.notifications.iter().filter(|x| self.keep(x.proposed)) {
            let name = ident(
                n.type_name
                    .as_deref()
                    .unwrap_or(&method_type_name(&n.method)),
            );
            let method = Literal::string(&n.method);
            let params = match &n.params {
                Some(OneOrManyTypes::One(t)) => self.rust_type(t, ctx)?,
                Some(OneOrManyTypes::Many(_)) => quote! { LspAny },
                None => quote! { () },
            };
            impls.push(quote! {
                pub struct #name;
                impl LspNotification for #name {
                    type Params = #params;
                    const METHOD: &'static str = #method;
                }
            });
        }
        Ok(quote! {
            pub trait LspNotification {
                type Params: Serialize + for<'de> Deserialize<'de>;
                const METHOD: &'static str;
            }
            #(#impls)*
        })
    }

    fn type_to_string(&self, ty: &Type) -> Result<String> {
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

    fn type_supports_eq_and_hash(&self, ty: &Type, ctx: &Context) -> bool {
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

    fn is_valid_map_key_type(&self, key: &MapKeyType, ctx: &Context) -> bool {
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

    fn is_valid_type_for_map_key(&self, ty: &Type, ctx: &Context) -> bool {
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

    fn collect_properties(&self, struct_name: &str, ctx: &Context) -> Result<Vec<Property>> {
        let st = ctx
            .structures
            .get(struct_name)
            .ok_or_else(|| anyhow::anyhow!("Structure {} not found", struct_name))?;
        let mut props = Vec::new();
        for ext in &st.extends {
            match ext {
                Type::Reference { name } => {
                    props.extend(self.collect_properties(name, ctx)?);
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Non-reference extends in structure {}: {:?}",
                        struct_name,
                        ext
                    ));
                }
            }
        }
        for mixin in &st.mixins {
            match mixin {
                Type::Reference { name } => {
                    props.extend(self.collect_properties(name, ctx)?);
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Non-reference mixins in structure {}: {:?}",
                        struct_name,
                        mixin
                    ));
                }
            }
        }
        for prop in &st.properties {
            props.push(prop.clone());
        }
        Ok(props)
    }

    fn rust_type(&self, ty: &Type, ctx: &Context) -> Result<TokenStream> {
        match ty {
            Type::Base { name } => Ok(match name {
                BaseTypeName::Uri => quote! { URI },
                BaseTypeName::DocumentUri => quote! { DocumentUri },
                BaseTypeName::Integer => quote! { Integer },
                BaseTypeName::Uinteger => quote! { Uinteger },
                BaseTypeName::Decimal => quote! { Decimal },
                BaseTypeName::RegExp => quote! { RegExp },
                BaseTypeName::String => quote! { String },
                BaseTypeName::Boolean => quote! { bool },
                BaseTypeName::Null => quote! { () },
            }),
            Type::Reference { name } => {
                let name = ident(name);
                Ok(quote! { #name })
            }
            Type::Array { element } => {
                let inner = self.rust_type(element, ctx)?;
                Ok(quote! { Vec<#inner> })
            }
            Type::Map { key, value } => {
                if !self.is_valid_map_key_type(key, ctx) {
                    return Err(anyhow::anyhow!("Unsupported map key type: {:?}", key));
                }
                let key_tokens = match key {
                    MapKeyType::Base { name, .. } => match name {
                        MapKeyBaseName::Uri => quote! { URI },
                        MapKeyBaseName::DocumentUri => quote! { DocumentUri },
                        MapKeyBaseName::String => quote! { String },
                        MapKeyBaseName::Integer => quote! { Integer },
                    },
                    MapKeyType::Reference { name, .. } => {
                        if name.eq_ignore_ascii_case("DocumentUri") {
                            quote! { DocumentUri }
                        } else if name.eq_ignore_ascii_case("URI") {
                            quote! { URI }
                        } else if name.eq_ignore_ascii_case("string") {
                            quote! { String }
                        } else if name.eq_ignore_ascii_case("integer")
                            || name.eq_ignore_ascii_case("uinteger")
                        {
                            quote! { Integer }
                        } else {
                            let ident = ident(name);
                            quote! { #ident }
                        }
                    }
                };
                let value_tokens = self.rust_type(value, ctx)?;
                Ok(quote! { std::collections::BTreeMap<#key_tokens, #value_tokens> })
            }
            Type::And { items } => {
                let mut struct_names = Vec::new();
                for item in items {
                    match item {
                        Type::Reference { name } => {
                            if ctx.structures.contains_key(name) {
                                struct_names.push(name.as_str());
                            } else {
                                return Err(anyhow::anyhow!(
                                    "Intersection item is not a known structure: {}",
                                    name
                                ));
                            }
                        }
                        _ => {
                            return Err(anyhow::anyhow!(
                                "Intersection contains non-reference type: {:?}",
                                item
                            ));
                        }
                    }
                }

                let mut merged_props = indexmap::IndexMap::<String, Property>::new();
                for name in &struct_names {
                    let props = self.collect_properties(name, ctx)?;
                    for prop in props {
                        if let Some(existing) = merged_props.get(&prop.name) {
                            if !types_are_equal(&existing.ty, &prop.ty) {
                                return Err(anyhow::anyhow!(
                                    "Incompatible property overlap for field '{}' in intersection of {:?}",
                                    prop.name,
                                    struct_names
                                ));
                            }
                        } else {
                            merged_props.insert(prop.name.clone(), prop);
                        }
                    }
                }

                let mut sorted_names = struct_names.clone();
                sorted_names.sort();
                let gen_name = sorted_names.join("And");

                let exists = self.generated_types.borrow().contains_key(&gen_name);
                if !exists {
                    let mut fields = Vec::new();
                    for prop in merged_props.values() {
                        fields.push(self.render_property(&gen_name, prop, ctx)?);
                    }

                    let gen_ident = ident(&gen_name);
                    let gen_struct = quote! {
                        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
                        #[serde(rename_all = "camelCase")]
                        pub struct #gen_ident {
                            #(#fields)*
                        }
                    };

                    self.generated_types
                        .borrow_mut()
                        .insert(gen_name.clone(), gen_struct);
                }

                let gen_ident = ident(&gen_name);
                Ok(quote! { #gen_ident })
            }
            Type::Or { items } => {
                let mut non_null_items = Vec::new();
                let mut has_null = false;
                for item in items {
                    match item {
                        Type::Base {
                            name: BaseTypeName::Null,
                        } => {
                            has_null = true;
                        }
                        _ => {
                            non_null_items.push(item);
                        }
                    }
                }

                if non_null_items.is_empty() {
                    return Ok(quote! { () });
                }

                let inner_ty = if non_null_items.len() == 1 {
                    self.rust_type(non_null_items[0], ctx)?
                } else {
                    let mut items_with_info = Vec::new();
                    for item in &non_null_items {
                        let name = self.type_to_string(item)?;
                        let mut visited = std::collections::HashSet::new();
                        let complexity = self.type_complexity(item, ctx, &mut visited);
                        items_with_info.push((item, name, complexity));
                    }

                    let mut name_for_union_debug = non_null_items
                        .iter()
                        .map(|item| self.type_to_string(item).unwrap_or_default())
                        .collect::<Vec<_>>();
                    name_for_union_debug.sort();
                    let debug_gen_name = name_for_union_debug.join("Or");
                    if debug_gen_name.contains("TextEdit")
                        || debug_gen_name.contains("CallHierarchy")
                        || debug_gen_name.contains("SelectionRange")
                    {
                        eprintln!("DEBUG UNION: {}", debug_gen_name);
                        for (_, name, comp) in &items_with_info {
                            eprintln!("  - {}: complexity = {}", name, comp);
                        }
                    }

                    // Sort descending by complexity, then alphabetically by name for deterministic ordering
                    items_with_info.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(&b.1)));

                    if debug_gen_name.contains("TextEdit")
                        || debug_gen_name.contains("CallHierarchy")
                        || debug_gen_name.contains("SelectionRange")
                    {
                        eprintln!("  SORTED:");
                        for (_, name, comp) in &items_with_info {
                            eprintln!("    - {}: complexity = {}", name, comp);
                        }
                    }

                    let mut item_names = Vec::new();
                    for (_, name, _) in &items_with_info {
                        item_names.push(name.clone());
                    }
                    let mut name_for_union = item_names.clone();
                    name_for_union.sort();
                    let gen_name = name_for_union.join("Or");

                    let exists = self.generated_types.borrow().contains_key(&gen_name);
                    if !exists {
                        let contains_float_or_any = non_null_items
                            .iter()
                            .any(|x| !self.type_supports_eq_and_hash(x, ctx));
                        let derives = if contains_float_or_any {
                            quote! { #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] }
                        } else {
                            quote! { #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)] }
                        };

                        let mut variants = Vec::new();
                        for (item, name, _) in &items_with_info {
                            let vname = ident(name);
                            let ty_tokens = self.rust_type(item, ctx)?;
                            variants.push(quote! {
                                #vname(#ty_tokens),
                            });
                        }

                        let gen_ident = ident(&gen_name);
                        let gen_enum = quote! {
                            #derives
                            #[serde(untagged)]
                            pub enum #gen_ident {
                                #(#variants)*
                            }
                        };
                        self.generated_types
                            .borrow_mut()
                            .insert(gen_name.clone(), gen_enum);
                    }

                    let gen_ident = ident(&gen_name);
                    quote! { #gen_ident }
                };

                if has_null {
                    Ok(quote! { Option<#inner_ty> })
                } else {
                    Ok(inner_ty)
                }
            }
            Type::Tuple { items } => {
                let mut parts = Vec::new();
                for item in items {
                    parts.push(self.rust_type(item, ctx)?);
                }
                Ok(quote! { (#(#parts),*) })
            }
            Type::StringLiteral { .. } => Ok(quote! { String }),
            Type::IntegerLiteral { .. } => Ok(quote! { Integer }),
            Type::BooleanLiteral { .. } => Ok(quote! { bool }),
            Type::Literal { value } => {
                let props_json = serde_json::to_string(&value.properties).unwrap_or_default();
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                use std::hash::Hasher;
                hasher.write(props_json.as_bytes());
                let hash = hasher.finish();
                let gen_name = format!("Literal{:x}", hash);

                let exists = self.generated_types.borrow().contains_key(&gen_name);
                if !exists {
                    let mut fields = Vec::new();
                    for prop in &value.properties {
                        fields.push(self.render_property(&gen_name, prop, ctx)?);
                    }
                    let gen_ident = ident(&gen_name);
                    let gen_struct = quote! {
                        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
                        #[serde(rename_all = "camelCase")]
                        pub struct #gen_ident {
                            #(#fields)*
                        }
                    };
                    self.generated_types
                        .borrow_mut()
                        .insert(gen_name.clone(), gen_struct);
                }

                let gen_ident = ident(&gen_name);
                Ok(quote! { #gen_ident })
            }
        }
    }

    fn type_complexity(
        &self,
        ty: &Type,
        ctx: &Context,
        visited: &mut std::collections::HashSet<String>,
    ) -> usize {
        match ty {
            Type::Base { name } => match name {
                BaseTypeName::Null => 0,
                _ => 1,
            },
            Type::Reference { name } => {
                if visited.contains(name) {
                    return 1;
                }
                visited.insert(name.clone());
                let res = if ctx.structures.contains_key(name) {
                    if let Ok(props) = self.collect_properties(name, ctx) {
                        props.len() + 2
                    } else {
                        2
                    }
                } else if let Some(alias) = ctx.type_aliases.get(name) {
                    self.type_complexity(&alias.ty, ctx, visited)
                } else if ctx.enumerations.contains_key(name) {
                    2
                } else {
                    1
                };
                visited.remove(name);
                res
            }
            Type::Array { element } => self.type_complexity(element, ctx, visited) + 1,
            Type::Map { value, .. } => self.type_complexity(value, ctx, visited) + 2,
            Type::And { items } => items
                .iter()
                .map(|item| self.type_complexity(item, ctx, visited))
                .sum(),
            Type::Or { items } => items
                .iter()
                .map(|item| self.type_complexity(item, ctx, visited))
                .sum(),
            Type::Tuple { items } => items
                .iter()
                .map(|item| self.type_complexity(item, ctx, visited))
                .sum(),
            Type::Literal { value } => value.properties.len() + 2,
            Type::StringLiteral { .. }
            | Type::IntegerLiteral { .. }
            | Type::BooleanLiteral { .. } => 1,
        }
    }
}

fn ident(name: &str) -> Ident {
    let mut s = name.to_string();
    if s == "type"
        || s == "match"
        || s == "where"
        || s == "loop"
        || s == "move"
        || s == "box"
        || s == "async"
        || s == "await"
        || s == "crate"
        || s == "self"
        || s == "super"
    {
        s.push('_');
    }
    format_ident!("{}", s)
}

fn method_type_name(method: &str) -> String {
    method
        .replace("$/", "Dollar/")
        .replace('/', "_")
        .replace('$', "Dollar")
        .replace('-', "_")
        .to_upper_camel_case()
}

fn doc_attr(doc: Option<&str>) -> TokenStream {
    if let Some(doc) = doc {
        let doc = doc.replace('\r', "");
        quote! { #[doc = #doc] }
    } else {
        quote! {}
    }
}

fn types_are_equal(a: &Type, b: &Type) -> bool {
    if let (Ok(va), Ok(vb)) = (serde_json::to_value(a), serde_json::to_value(b)) {
        va == vb
    } else {
        false
    }
}
