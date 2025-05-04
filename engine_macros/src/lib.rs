//! # engine_macros
//!
//! Procedural macros for the Modular Game Engine project.
//!
//! This crate provides the `#[component]` attribute macro, which generates boilerplate
//! for ECS (Entity-Component-System) components, including:
//! - Versioning and migration support
//! - Mode restrictions
//! - Serde (de)serialization
//! - JSON schema generation (optional)
//!
//! ## Usage
//!
//! ```
//! use engine_macros::component;
//!
//! #[component(modes(Single, Multi), version = "1.2.3", schema)]
//! #[derive(Debug, PartialEq)]
//! struct Position {
//!     x: f32,
//!     y: f32,
//! }
//! ```
//!
//! See the main project README for more context.

use proc_macro::TokenStream;
use quote::quote;
mod migration;
mod parse;
mod schema;
mod serde_impl;

/// Attribute macro for ECS components.
///
/// Generates trait implementations and boilerplate for versioning, migration,
/// mode restriction, serialization, and schema support.
///
/// # Arguments
///
/// - `modes(...)`: List of supported game modes.
/// - `version = "...""`: Semver version string.
/// - `schema`: Enable JSON schema generation.
/// - `migration(from = "...", convert = "...")`: Migration from legacy struct.
///
/// See crate-level docs for usage.
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Parse attributes and struct
    let args = parse::parse_component_attr(attr);
    let input = parse::parse_input(item);
    let name = &input.ident;
    let fields = parse::extract_fields(&input);

    let attr_info = parse::process_args(args);
    let modes = attr_info.modes;
    let version = attr_info.version;
    let has_schema = attr_info.has_schema;
    let migration = attr_info.migration;

    // Get field idents/count for serde
    let field_idents: Vec<_> = fields.iter().filter_map(|f| f.ident.clone()).collect();
    let field_count = field_idents.len();

    // Generate code for each part
    let migration_impl = migration::generate_migration_impl(&migration, fields);
    let schema_impl = schema::generate_schema_impl(name, has_schema);
    let derive_jsonschema = schema::derive_jsonschema(has_schema);
    let serialize_impl = serde_impl::generate_serialize_impl(name, fields, field_count);
    let deserialize_impl = serde_impl::generate_deserialize_impl(name, fields, &field_idents);

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

        #serialize_impl
        #deserialize_impl
    };

    TokenStream::from(expanded)
}
