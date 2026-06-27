/// Enumeration rendering for the LSP specgen renderer.
use anyhow::Result;
use heck::{ToShoutySnakeCase, ToUpperCamelCase};
use proc_macro2::{Literal, TokenStream};
use quote::quote;

use crate::metamodel::*;

use super::{doc_attr, ident, Renderer};

impl Renderer {
    pub(crate) fn render_enumeration(&self, en: &Enumeration) -> Result<TokenStream> {
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
}
