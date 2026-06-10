/// Free-function helpers used throughout the render module.
use heck::ToUpperCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::metamodel::Type;

pub(crate) fn ident(name: &str) -> Ident {
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

pub(crate) fn method_type_name(method: &str) -> String {
    method
        .replace("$/", "Dollar/")
        .replace('/', "_")
        .replace('$', "Dollar")
        .replace('-', "_")
        .to_upper_camel_case()
}

pub(crate) fn doc_attr(doc: Option<&str>) -> TokenStream {
    if let Some(doc) = doc {
        let doc = doc.replace('\r', "");
        quote! { #[doc = #doc] }
    } else {
        quote! {}
    }
}

pub(crate) fn types_are_equal(a: &Type, b: &Type) -> bool {
    if let (Ok(va), Ok(vb)) = (serde_json::to_value(a), serde_json::to_value(b)) {
        va == vb
    } else {
        false
    }
}
