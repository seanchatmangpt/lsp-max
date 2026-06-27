#[proc_macro_derive(AutoLsif)]
pub fn derive_auto_lsif(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    
    let emit_body = match &input.data {
        syn::Data::Struct(data) => {
            let fields = match &data.fields {
                syn::Fields::Named(fields) => {
                    let field_calls = fields.named.iter().map(|f| {
                        let field_ident = &f.ident;
                        quote::quote! {
                            ::lsp_max_lsif::auto_lsif::AutoLsifNode::emit_lsif(&self.#field_ident, graph);
                        }
                    });
                    quote::quote! {
                        #(#field_calls)*
                        None
                    }
                },
                syn::Fields::Unnamed(fields) => {
                    let field_calls = fields.unnamed.iter().enumerate().map(|(i, _)| {
                        let idx = syn::Index::from(i);
                        quote::quote! {
                            ::lsp_max_lsif::auto_lsif::AutoLsifNode::emit_lsif(&self.#idx, graph);
                        }
                    });
                    quote::quote! {
                        #(#field_calls)*
                        None
                    }
                },
                syn::Fields::Unit => quote::quote! { None },
            };
            fields
        },
        syn::Data::Enum(data) => {
            let variants = data.variants.iter().map(|variant| {
                let v_name = &variant.ident;
                match &variant.fields {
                    syn::Fields::Named(fields) => {
                        let field_idents: Vec<_> = fields.named.iter().map(|f| &f.ident).collect();
                        quote::quote! {
                            #name::#v_name { #(#field_idents),* } => {
                                #( ::lsp_max_lsif::auto_lsif::AutoLsifNode::emit_lsif(#field_idents, graph); )*
                                None
                            }
                        }
                    },
                    syn::Fields::Unnamed(fields) => {
                        let field_idents: Vec<_> = fields.unnamed.iter().enumerate().map(|(i, _)| {
                            quote::format_ident!("f{}", i)
                        }).collect();
                        quote::quote! {
                            #name::#v_name( #(#field_idents),* ) => {
                                #( ::lsp_max_lsif::auto_lsif::AutoLsifNode::emit_lsif(#field_idents, graph); )*
                                None
                            }
                        }
                    },
                    syn::Fields::Unit => {
                        quote::quote! {
                            #name::#v_name => None,
                        }
                    }
                }
            });
            quote::quote! {
                match self {
                    #(#variants)*
                }
            }
        },
        syn::Data::Union(_) => {
            return syn::Error::new_spanned(input, "AutoLsif cannot be derived for unions")
                .to_compile_error()
                .into();
        }
    };
    
    let expanded = quote::quote! {
        #[automatically_derived]
        impl #impl_generics ::lsp_max_lsif::auto_lsif::AutoLsifNode for #name #ty_generics #where_clause {
            fn emit_lsif(&self, graph: &mut ::lsp_max_lsif::auto_lsif::FastLsifGraph) -> ::std::option::Option<::lsp_types_max::NumberOrString> {
                #emit_body
            }
        }
    };
    
    proc_macro::TokenStream::from(expanded)
}
