// engine_macros/src/parse.rs

use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{DeriveInput, Fields, Ident, Meta, Path, Token};

pub(crate) struct ComponentAttr {
    pub modes: Vec<Ident>,
    pub version: String,
    pub has_schema: bool,
    pub migration: Option<(String, String)>,
}

pub(crate) fn process_args(args: Vec<Meta>) -> ComponentAttr {
    let mut modes = Vec::new();
    let mut version = "1.0.0".to_string();
    let mut has_schema = false;
    let mut migration = None;

    for arg in args {
        match arg {
            Meta::List(ref list) if list.path.is_ident("modes") => {
                use syn::Token;
                use syn::parse::Parser;
                use syn::punctuated::Punctuated;
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
                use syn::Token;
                use syn::parse::Parser;
                use syn::punctuated::Punctuated;
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

    ComponentAttr {
        modes,
        version,
        has_schema,
        migration,
    }
}

pub(crate) fn parse_component_attr(attr: proc_macro::TokenStream) -> Vec<Meta> {
    Punctuated::<Meta, Token![,]>::parse_terminated
        .parse(attr)
        .expect("Failed to parse component attribute arguments")
        .into_iter()
        .collect()
}

pub(crate) fn parse_input(item: proc_macro::TokenStream) -> DeriveInput {
    syn::parse(item).expect("Failed to parse input as DeriveInput")
}

pub(crate) fn extract_fields(input: &DeriveInput) -> &Fields {
    match &input.data {
        syn::Data::Struct(data) => &data.fields,
        _ => panic!("Only structs are supported"),
    }
}
