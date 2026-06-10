/// Structure, property, and type-alias rendering for the LSP specgen renderer.
use anyhow::Result;
use heck::ToSnakeCase;
use proc_macro2::{Literal, TokenStream};
use quote::quote;

use crate::metamodel::*;

use super::{doc_attr, ident, Context, Renderer};

impl Renderer {
    pub(crate) fn render_type_alias(
        &self,
        alias: &TypeAlias,
        ctx: &Context,
    ) -> Result<TokenStream> {
        let name = ident(&alias.name);
        let ty = self.rust_type(&alias.ty, ctx)?;
        let doc = doc_attr(alias.documentation.as_deref());
        Ok(quote! {
            #doc
            pub type #name = #ty;
        })
    }

    pub(crate) fn render_structure(&self, st: &Structure, ctx: &Context) -> Result<TokenStream> {
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

    pub(crate) fn render_property(
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

    pub(crate) fn collect_properties(
        &self,
        struct_name: &str,
        ctx: &Context,
    ) -> Result<Vec<Property>> {
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
}
