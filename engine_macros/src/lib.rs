use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{DeriveInput, Fields, Ident, Meta, Path, Token, parse_macro_input};

#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse the attribute as a list of Meta items, supporting trailing commas
    let args = Punctuated::<Meta, Token![,]>::parse_terminated
        .parse(attr)
        .expect("Failed to parse component attribute arguments");

    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;

    // Defaults
    let mut modes: Vec<Ident> = Vec::new();
    let mut version = "1.0.0".to_string();
    let mut has_schema = false;
    let mut migration: Option<(String, String)> = None;

    for arg in args {
        match arg {
            Meta::List(ref list) if list.path.is_ident("modes") => {
                // Parse the inner list as a list of identifiers, supporting trailing commas
                let punctuated = Punctuated::<Path, Token![,]>::parse_terminated
                    .parse2(list.tokens.clone())
                    .expect("Failed to parse modes list");
                for path in punctuated {
                    if let Some(ident) = path.get_ident() {
                        modes.push(ident.clone());
                    } else {
                        panic!("Expected identifier in modes list");
                    }
                }
            }
            Meta::Path(ref path) if path.is_ident("schema") => {
                has_schema = true;
            }
            Meta::NameValue(ref nv) if nv.path.is_ident("version") => {
                if let syn::Expr::Lit(expr_lit) = &nv.value {
                    if let syn::Lit::Str(litstr) = &expr_lit.lit {
                        version = litstr.value();
                    }
                }
            }
            Meta::List(ref list) if list.path.is_ident("migration") => {
                let mut from = None;
                let mut convert = None;
                let migration_inner = Punctuated::<Meta, Token![,]>::parse_terminated
                    .parse2(list.tokens.clone())
                    .expect("Failed to parse migration list");
                for m in migration_inner {
                    if let Meta::NameValue(nv) = m {
                        if nv.path.is_ident("from") {
                            if let syn::Expr::Lit(expr_lit) = &nv.value {
                                if let syn::Lit::Str(litstr) = &expr_lit.lit {
                                    from = Some(litstr.value());
                                }
                            }
                        }
                        if nv.path.is_ident("convert") {
                            if let syn::Expr::Lit(expr_lit) = &nv.value {
                                if let syn::Lit::Str(litstr) = &expr_lit.lit {
                                    convert = Some(litstr.value());
                                }
                            }
                        }
                    }
                }
                if let (Some(f), Some(c)) = (from, convert) {
                    migration = Some((f, c));
                }
            }
            _ => {}
        }
    }

    // Extract fields
    let fields: &Fields = match &input.data {
        syn::Data::Struct(data) => &data.fields,
        _ => panic!("Only structs are supported"),
    };
    let field_idents: Vec<_> = fields.iter().filter_map(|f| f.ident.clone()).collect();

    // Migration implementation
    let migration_impl = if let Some((from, convert)) = &migration {
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
    };

    // Schema implementation
    let schema_impl = if has_schema {
        quote! {
            fn generate_schema() -> Option<schemars::schema::RootSchema> {
                Some(schemars::schema_for!(#name))
            }
        }
    } else {
        quote! {
            fn generate_schema() -> Option<schemars::schema::RootSchema> {
                None
            }
        }
    };

    // Serde Serialize
    let ser_fields = fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        let ident_str = ident.to_string();
        quote! { state.serialize_field(#ident_str, &self.#ident)?; }
    });

    // Serde Deserialize
    let deser_field_decls = fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        quote! { let mut #ident = None; }
    });
    let deser_field_matches = fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        let ident_str = ident.to_string();
        quote! {
            #ident_str => { #ident = Some(map.next_value()?); }
        }
    });
    let deser_field_build = fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        let ident_str = ident.to_string();
        quote! {
            #ident: #ident.ok_or_else(|| serde::de::Error::missing_field(#ident_str))?
        }
    });

    let field_count = field_idents.len();

    let derive_jsonschema = if has_schema {
        quote! { #[derive(schemars::JsonSchema)] }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #derive_jsonschema
        #input

        impl crate::ecs::Component for #name {
            #schema_impl

            fn version() -> semver::Version {
                semver::Version::parse(#version).unwrap()
            }

            fn migrate(from_version: semver::Version, data: &[u8]) -> Result<Self, crate::ecs::error::MigrationError>
            where
                Self: Sized + serde::de::DeserializeOwned,
            {
                #migration_impl
                Err(crate::ecs::error::MigrationError::UnsupportedVersion(from_version))
            }
        }

        impl crate::modes::ModeRestrictedComponent for #name {
            fn supported_modes() -> Vec<crate::modes::GameMode> {
                vec![ #(crate::modes::GameMode::#modes),* ]
            }
        }

        impl serde::Serialize for #name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use serde::ser::SerializeStruct;
                let mut state = serializer.serialize_struct(stringify!(#name), #field_count)?;
                #(#ser_fields)*
                state.end()
            }
        }

        impl<'de> serde::Deserialize<'de> for #name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                use serde::de::{self, MapAccess, Visitor};
                use std::fmt;

                struct StructVisitor;

                impl<'de> Visitor<'de> for StructVisitor {
                    type Value = #name;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str(concat!("struct ", stringify!(#name)))
                    }

                    fn visit_map<V>(self, mut map: V) -> Result<#name, V::Error>
                    where
                        V: MapAccess<'de>,
                    {
                        #(#deser_field_decls)*

                        while let Some(key) = map.next_key::<&str>()? {
                            match key {
                                #(#deser_field_matches)*
                                _ => { let _: serde::de::IgnoredAny = map.next_value()?; }
                            }
                        }

                        Ok(#name {
                            #(#deser_field_build),*
                        })
                    }
                }

                deserializer.deserialize_struct(
                    stringify!(#name),
                    &[#(stringify!(#field_idents)),*],
                    StructVisitor,
                )
            }
        }

    };

    TokenStream::from(expanded)
}
