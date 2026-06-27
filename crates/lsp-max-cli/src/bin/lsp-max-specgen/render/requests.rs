/// Request and notification rendering for the LSP specgen renderer.
use anyhow::Result;
use proc_macro2::{Literal, TokenStream};
use quote::quote;

use crate::metamodel::*;

use super::{ident, method_type_name, Context, Renderer};

impl Renderer {
    pub(crate) fn render_requests(&self, model: &MetaModel, ctx: &Context) -> Result<TokenStream> {
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

    pub(crate) fn render_notifications(
        &self,
        model: &MetaModel,
        ctx: &Context,
    ) -> Result<TokenStream> {
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
}
