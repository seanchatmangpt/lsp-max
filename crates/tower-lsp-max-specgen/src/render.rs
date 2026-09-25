use anyhow::Result;
use heck::{ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote};

use crate::metamodel::*;

pub struct Renderer {
    include_proposed: bool,
}

impl Renderer {
    pub fn new(include_proposed: bool) -> Self {
        Self { include_proposed }
    }

    pub fn render(&self, model: &MetaModel) -> Result<String> {
        let mut out = TokenStream::new();
        let version = &model.meta_data.version;
        out.extend(quote! {
            //! Generated from the official LSP meta-model.
            //! Do not hand-edit generated protocol vocabulary.
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

        for alias in model.type_aliases.iter().filter(|x| self.keep(x.proposed)) {
            out.extend(self.render_type_alias(alias)?);
        }
        for en in model.enumerations.iter().filter(|x| self.keep(x.proposed)) {
            out.extend(self.render_enumeration(en)?);
        }
        for st in model.structures.iter().filter(|x| self.keep(x.proposed)) {
            out.extend(self.render_structure(st)?);
        }
        out.extend(self.render_requests(model)?);
        out.extend(self.render_notifications(model)?);

        let file = syn::parse2(out)?;
        Ok(prettyplease::unparse(&file))
    }

    fn keep(&self, proposed: Option<bool>) -> bool {
        self.include_proposed || proposed != Some(true)
    }

    fn render_type_alias(&self, alias: &TypeAlias) -> Result<TokenStream> {
        let name = ident(&alias.name);
        let ty = self.rust_type(&alias.ty);
        let doc = doc_attr(alias.documentation.as_deref());
        Ok(quote! {
            #doc
            pub type #name = #ty;
        })
    }

    fn render_structure(&self, st: &Structure) -> Result<TokenStream> {
        let name = ident(&st.name);
        let doc = doc_attr(st.documentation.as_deref());
        let mut fields = Vec::<TokenStream>::new();

        for ext in &st.extends {
            if let Type::Reference { name } = ext {
                let field_name = ident(&format!("{}_base", name.to_snake_case()));
                let ty = ident(name);
                fields.push(quote! {
                    #[serde(flatten)]
                    pub #field_name: #ty,
                });
            }
        }
        for mixin in &st.mixins {
            if let Type::Reference { name } = mixin {
                let field_name = ident(&format!("{}_mixin", name.to_snake_case()));
                let ty = ident(name);
                fields.push(quote! {
                    #[serde(flatten)]
                    pub #field_name: #ty,
                });
            }
        }
        for prop in &st.properties {
            fields.push(self.render_property(&st.name, prop));
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

    fn render_property(&self, struct_name: &str, prop: &Property) -> TokenStream {
        let raw_name = &prop.name;
        let rust_name = ident(&raw_name.to_snake_case());
        let mut ty = self.rust_type(&prop.ty);

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
        quote! {
            #doc
            #rename
            #default
            pub #rust_name: #ty,
        }
    }

    fn render_enumeration(&self, en: &Enumeration) -> Result<TokenStream> {
        let name = ident(&en.name);
        let doc = doc_attr(en.documentation.as_deref());
        let open =
            en.supports_custom_values.unwrap_or(false) || en.ty.name != EnumerationBaseType::String;
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
                    quote! { #val => Ok(Self::#vname), }
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

    fn render_requests(&self, model: &MetaModel) -> Result<TokenStream> {
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
        let impls = model
            .requests
            .iter()
            .filter(|x| self.keep(x.proposed))
            .map(|r| {
                let name = ident(
                    r.type_name
                        .as_deref()
                        .unwrap_or(&method_type_name(&r.method)),
                );
                let method = Literal::string(&r.method);
                let params = match &r.params {
                    Some(OneOrManyTypes::One(t)) => self.rust_type(t),
                    Some(OneOrManyTypes::Many(_)) => quote! { LspAny },
                    None => quote! { () },
                };
                let result = self.rust_type(&r.result);
                quote! {
                    pub struct #name;
                    impl LspRequest for #name {
                        type Params = #params;
                        type Result = #result;
                        const METHOD: &'static str = #method;
                    }
                }
            });
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

    fn render_notifications(&self, model: &MetaModel) -> Result<TokenStream> {
        let impls = model
            .notifications
            .iter()
            .filter(|x| self.keep(x.proposed))
            .map(|n| {
                let name = ident(
                    n.type_name
                        .as_deref()
                        .unwrap_or(&method_type_name(&n.method)),
                );
                let method = Literal::string(&n.method);
                let params = match &n.params {
                    Some(OneOrManyTypes::One(t)) => self.rust_type(t),
                    Some(OneOrManyTypes::Many(_)) => quote! { LspAny },
                    None => quote! { () },
                };
                quote! {
                    pub struct #name;
                    impl LspNotification for #name {
                        type Params = #params;
                        const METHOD: &'static str = #method;
                    }
                }
            });
        Ok(quote! {
            pub trait LspNotification {
                type Params: Serialize + for<'de> Deserialize<'de>;
                const METHOD: &'static str;
            }
            #(#impls)*
        })
    }

    fn rust_type(&self, ty: &Type) -> TokenStream {
        match ty {
            Type::Base { name } => match name {
                BaseTypeName::Uri => quote! { URI },
                BaseTypeName::DocumentUri => quote! { DocumentUri },
                BaseTypeName::Integer => quote! { Integer },
                BaseTypeName::Uinteger => quote! { Uinteger },
                BaseTypeName::Decimal => quote! { Decimal },
                BaseTypeName::RegExp => quote! { RegExp },
                BaseTypeName::String => quote! { String },
                BaseTypeName::Boolean => quote! { bool },
                BaseTypeName::Null => quote! { () },
            },
            Type::Reference { name } => {
                let name = ident(name);
                quote! { #name }
            }
            Type::Array { element } => {
                let inner = self.rust_type(element);
                quote! { Vec<#inner> }
            }
            Type::Map { value, .. } => {
                let value = self.rust_type(value);
                quote! { std::collections::BTreeMap<String, #value> }
            }
            // First-pass conservative lowering. The Max layer should later generate named sum/product forms.
            Type::And { .. } | Type::Or { .. } | Type::Tuple { .. } | Type::Literal { .. } => {
                quote! { LspAny }
            }
            Type::StringLiteral { .. } => quote! { String },
            Type::IntegerLiteral { .. } => quote! { Integer },
            Type::BooleanLiteral { .. } => quote! { bool },
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
