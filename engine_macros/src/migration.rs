//! Migration logic codegen for the #[component] macro.
//! Generates migration stubs or implementations for versioned ECS components.

use proc_macro2::Ident;
use quote::quote;
use syn::Fields;

pub(crate) fn generate_migration_impl(
    migration: &Option<(String, String)>,
    fields: &Fields,
) -> proc_macro2::TokenStream {
    if let Some((from, convert)) = migration {
        let legacy_type = Ident::new(convert, proc_macro2::Span::call_site());
        let field_mappings = fields.iter().map(|f| {
            let ident = f.ident.as_ref().unwrap();
            quote! { #ident: legacy.#ident }
        });
        let legacy_fields = fields.iter().map(|f| {
            let ident = f.ident.as_ref().unwrap();
            let ty = &f.ty;
            quote! { pub #ident: #ty }
        });
        quote! {
            if from_version == semver::Version::parse(#from).unwrap() {
                #[derive(serde::Deserialize)]
                struct #legacy_type {
                    #(#legacy_fields,)*
                }
                let legacy = bson::from_slice::<#legacy_type>(data)?;
                return Ok(Self { #(#field_mappings),* });
            }
        }
    } else {
        quote! {}
    }
}
