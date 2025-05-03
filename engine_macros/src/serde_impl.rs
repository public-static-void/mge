use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident};

pub(crate) fn generate_serialize_impl(
    name: &Ident,
    fields: &Fields,
    field_count: usize,
) -> TokenStream {
    let ser_fields = fields.iter().map(|f| {
        let ident = f.ident.as_ref().unwrap();
        let ident_str = ident.to_string();
        quote! { state.serialize_field(#ident_str, &self.#ident)?; }
    });
    quote! {
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
    }
}

pub(crate) fn generate_deserialize_impl(
    name: &Ident,
    fields: &Fields,
    field_idents: &[Ident],
) -> TokenStream {
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

    quote! {
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
    }
}
