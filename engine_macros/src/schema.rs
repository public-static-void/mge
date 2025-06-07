//! Schema logic codegen for the #[component] macro.
//! Generates schema stubs or implementations for versioned ECS components.

use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn generate_schema_impl(name: &syn::Ident, has_schema: bool) -> TokenStream {
    if has_schema {
        quote! {
            fn generate_schema() -> Option<schemars::Schema> {
                // schema_for!(T) returns a Schema directly in schemars 0.9+
                Some(schemars::schema_for!(#name))
            }
        }
    } else {
        quote! {
            fn generate_schema() -> Option<schemars::Schema> {
                None
            }
        }
    }
}

pub(crate) fn derive_jsonschema(has_schema: bool) -> TokenStream {
    if has_schema {
        quote! { #[derive(schemars::JsonSchema)] }
    } else {
        quote! {}
    }
}
