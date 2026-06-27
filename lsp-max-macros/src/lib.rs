//! Internal procedural macros for lsp-max.
//!
//! This crate should not be used directly.

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemTrait, LitStr, ReturnType, TraitItem};

/// Macro for generating LSP server implementation from lsp-types.
///
/// This procedural macro annotates the `lsp_max::LanguageServer` trait and generates a
/// corresponding `register_lsp_methods()` function which registers all the methods on that trait
/// as RPC handlers.
#[proc_macro_attribute]
pub fn rpc(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Attribute will be parsed later in `parse_method_calls()`.
    if !attr.is_empty() {
        return item;
    }

    let lang_server_trait = parse_macro_input!(item as ItemTrait);
    match parse_and_gen(&lang_server_trait) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn parse_and_gen(lang_server_trait: &ItemTrait) -> Result<proc_macro2::TokenStream, syn::Error> {
    let method_calls = parse_method_calls(lang_server_trait)?;
    let req_types_and_router_fn = gen_server_router(&lang_server_trait.ident, &method_calls)?;
    Ok(quote! {
        #lang_server_trait
        #req_types_and_router_fn
    })
}

struct MethodCall<'a> {
    rpc_name: String,
    handler_name: &'a syn::Ident,
    params: Option<&'a syn::Type>,
    result: Option<&'a syn::Type>,
    layer: Option<String>,
}

fn parse_method_calls(lang_server_trait: &ItemTrait) -> Result<Vec<MethodCall<'_>>, syn::Error> {
    let mut calls = Vec::new();

    for item in &lang_server_trait.items {
        let method = match item {
            TraitItem::Fn(m) => m,
            _ => continue,
        };

        let attr = method
            .attrs
            .iter()
            .find(|attr| attr.meta.path().is_ident("rpc"))
            .ok_or_else(|| {
                syn::Error::new_spanned(method, "expected `#[rpc(name = \"foo\")]` attribute")
            })?;

        let mut rpc_name = String::new();
        let mut layer = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let s: LitStr = meta.value().and_then(|v| v.parse())?;
                rpc_name = s.value();
                Ok(())
            } else if meta.path.is_ident("layer") {
                let s: LitStr = meta.value().and_then(|v| v.parse())?;
                layer = Some(s.value());
                Ok(())
            } else {
                Err(meta.error("expected `name` or `layer` identifier in `#[rpc]`"))
            }
        })?;

        let params = method.sig.inputs.iter().nth(1).and_then(|arg| match arg {
            FnArg::Typed(pat) => Some(&*pat.ty),
            _ => None,
        });

        let result = match &method.sig.output {
            ReturnType::Default => None,
            ReturnType::Type(_, ty) => Some(&**ty),
        };

        calls.push(MethodCall {
            rpc_name,
            handler_name: &method.sig.ident,
            params,
            result,
            layer,
        });
    }

    Ok(calls)
}

fn gen_server_router(
    trait_name: &syn::Ident,
    methods: &[MethodCall],
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let mut route_registrations = Vec::new();
    for method in methods {
        let rpc_name = &method.rpc_name;
        let handler = &method.handler_name;

        let layer = if let Some(ref l) = method.layer {
            let l: syn::Path =
                syn::parse_str(l).map_err(|e| syn::Error::new(e.span(), "invalid layer path"))?;
            quote! { #l::new(state.clone(), pending.clone()) }
        } else {
            match &rpc_name[..] {
                "initialize" => {
                    quote! { layers::Initialize::new(state.clone(), pending.clone()) }
                }
                "shutdown" => quote! { layers::Shutdown::new(state.clone(), pending.clone()) },
                _ => quote! {
                    tower::ServiceBuilder::new()
                        .layer(layers::Normal::new(state.clone(), pending.clone()))
                        .layer(doc_sync.clone())
                        .into_inner()
                },
            }
        };

        let registration = match (method.params, method.result) {
            (Some(params), Some(result)) => quote! {
                async fn #handler<S: #trait_name>(server: &S, params: #params) -> #result {
                    server.#handler(params).await
                }
                router.method(#rpc_name, #handler, #layer);
            },
            (None, Some(result)) => quote! {
                async fn #handler<S: #trait_name>(server: &S) -> #result {
                    server.#handler().await
                }
                router.method(#rpc_name, #handler, #layer);
            },
            (Some(params), None) => quote! {
                async fn #handler<S: #trait_name>(server: &S, params: #params) {
                    server.#handler(params).await
                }
                router.method(#rpc_name, #handler, #layer);
            },
            (None, None) => quote! {
                async fn #handler<S: #trait_name>(server: &S) {
                    server.#handler().await
                }
                router.method(#rpc_name, #handler, #layer);
            },
        };
        route_registrations.push(registration);
    }

    let route_registrations_tokens = quote! { #(#route_registrations)* };

    Ok(quote! {
        pub(crate) trait RegisterLspMethods {
            fn register_lsp_methods(
                router: crate::jsonrpc::Router<Self, crate::service::ExitedError>,
                state: std::sync::Arc<crate::service::ServerState>,
                pending: std::sync::Arc<crate::service::Pending>,
                client: crate::service::Client,
                doc_sync: crate::service::layers::DocumentSync,
            ) -> crate::jsonrpc::Router<Self, crate::service::ExitedError>
            where
                Self: Sized;
        }

        const _: () = {
            use std::sync::Arc;
            use std::future::{Future, Ready};

            use lsp_types_max::*;
            use lsp_types_max::notification::*;
            use lsp_types_max::request::*;
            use serde_json::Value;

            use crate::jsonrpc::{Result, Router};
            use crate::service::{layers, Client, Pending, ServerState, State, ExitedError};

            fn cancel_request(params: CancelParams, p: &Pending) -> Ready<()> {
                p.cancel(&params.id.into());
                std::future::ready(())
            }

            impl<S: #trait_name> RegisterLspMethods for S {
                fn register_lsp_methods(
                    mut router: Router<Self, ExitedError>,
                    state: Arc<ServerState>,
                    pending: Arc<Pending>,
                    client: Client,
                    doc_sync: layers::DocumentSync,
                ) -> Router<Self, ExitedError> {
                    #route_registrations_tokens

                    let p = pending.clone();
                    router.method(
                        "$/cancelRequest",
                        move |_: &Self, params| cancel_request(params, &p),
                        tower::layer::util::Identity::new(),
                    );
                    router.method(
                        "exit",
                        |_: &Self| std::future::ready(()),
                        layers::Exit::new(state.clone(), pending, client.clone()),
                    );

                    router
                }
            }
        };
    })
}
