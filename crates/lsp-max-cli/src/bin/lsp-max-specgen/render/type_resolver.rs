/// rust_type and type_complexity implementations for the LSP specgen renderer.
use anyhow::Result;
use proc_macro2::TokenStream;
use quote::quote;

use crate::metamodel::*;

use super::{ident, types_are_equal, Context, Renderer};

impl Renderer {
    pub(crate) fn rust_type(&self, ty: &Type, ctx: &Context) -> Result<TokenStream> {
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
                            let id = ident(name);
                            quote! { #id }
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

    pub(crate) fn type_complexity(
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
