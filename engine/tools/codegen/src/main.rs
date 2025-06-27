use serde_json::Value;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: codegen <schema.json> <output_dir> [--lang <langs>]");
        std::process::exit(1);
    }
    let schema_path = &args[1];
    let output_dir = &args[2];

    // Parse optional --lang argument
    let mut langs = vec!["rust".to_string()];
    if args.len() > 3 {
        if args[3] == "--lang" && args.len() > 4 {
            langs = args[4]
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .collect::<Vec<String>>();
        } else {
            eprintln!("Usage: codegen <schema.json> <output_dir> [--lang <langs>]");
            std::process::exit(1);
        }
    }

    let schema_str = fs::read_to_string(schema_path).expect("Failed to read schema file");
    let schema: Value = serde_json::from_str(&schema_str).expect("Failed to parse schema JSON");

    let file_stem = schema
        .get("name")
        .or_else(|| schema.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("component")
        .to_lowercase();

    for lang in langs {
        match lang.as_str() {
            "rust" => {
                let rust_code = generate_rust_component(&schema);
                let out_path = Path::new(output_dir).join(format!("{file_stem}.rs"));
                fs::write(out_path, rust_code).expect("Failed to write Rust output file");
            }
            "lua" => {
                let lua_code = generate_lua_stub(&schema);
                let out_path = Path::new(output_dir).join(format!("{file_stem}.lua"));
                fs::write(out_path, lua_code).expect("Failed to write Lua output file");
            }
            "python" => {
                let py_code = generate_python_stub(&schema);
                let out_path = Path::new(output_dir).join(format!("{file_stem}.py"));
                fs::write(out_path, py_code).expect("Failed to write Python output file");
            }
            "c" => {
                let c_code = generate_c_header(&schema);
                let out_path = Path::new(output_dir).join(format!("{file_stem}.h"));
                fs::write(out_path, c_code).expect("Failed to write C header output file");
            }
            "md" => {
                let md_doc = generate_markdown_doc(&schema, schema_path);
                let out_path = Path::new(output_dir).join(format!("{file_stem}.md"));
                fs::write(out_path, md_doc).expect("Failed to write Markdown output file");
            }
            other => {
                eprintln!("Unknown language: {other}");
                std::process::exit(1);
            }
        }
    }
}

fn generate_rust_component(schema: &serde_json::Value) -> String {
    let title = schema
        .get("title")
        .or_else(|| schema.get("name"))
        .expect("Schema must have a title or name")
        .as_str()
        .unwrap();
    let properties = schema.get("properties").unwrap().as_object().unwrap();

    // Handle enums for "pos" field (special-cased for Position)
    let mut enums = String::new();
    let mut fields = String::new();

    for (field, prop) in properties.iter() {
        if field == "pos" && prop.get("oneOf").is_some() {
            // Generate enum for Position
            enums.push_str(
                "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]\n\
                 pub enum Position {\n",
            );
            for variant in prop.get("oneOf").unwrap().as_array().unwrap() {
                let variant_obj = variant.get("properties").unwrap().as_object().unwrap();
                for (variant_name, variant_schema) in variant_obj.iter() {
                    enums.push_str(&format!("    {variant_name} {{ "));
                    let fields_obj = variant_schema
                        .get("properties")
                        .unwrap()
                        .as_object()
                        .unwrap();
                    let mut variant_fields = Vec::new();
                    for (f, f_schema) in fields_obj.iter() {
                        let ty = match f_schema.get("type").and_then(|t| t.as_str()) {
                            Some("integer") => "i32",
                            Some("string") => "String",
                            _ => "i32",
                        };
                        variant_fields.push(format!("{f}: {ty}"));
                    }
                    enums.push_str(&variant_fields.join(", "));
                    enums.push_str(" },\n");
                }
            }
            enums.push_str("}\n\n");
            fields.push_str("    pub pos: Position,\n");
        } else {
            // Fallback: treat as i32 or String for demo purposes
            let ty = match prop.get("type").and_then(|t| t.as_str()) {
                Some("integer") => "i32",
                Some("string") => "String",
                _ => "i32",
            };
            fields.push_str(&format!("    pub {field}: {ty},\n"));
        }
    }

    format!(
        r#"use crate::ecs::Component;
use schemars::JsonSchema;
use serde::{{Deserialize, Serialize}};

/// {title} for any map topology (square, hex, region, etc.)
{enums}#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct {title}Component {{
{fields}}}

impl Component for {title}Component {{
    fn generate_schema() -> Option<schemars::schema::Schema> {{
        Some(schemars::schema_for!({title}Component))
    }}

    fn version() -> semver::Version {{
        semver::Version::parse("3.0.0").unwrap()
    }}

    fn migrate(
        from_version: semver::Version,
        data: &[u8],
    ) -> Result<Self, crate::ecs::error::MigrationError>
    where
        Self: Sized + serde::de::DeserializeOwned,
    {{
        Err(crate::ecs::error::MigrationError::UnsupportedVersion(
            from_version,
        ))
    }}
}}
"#
    )
}

fn generate_lua_stub(schema: &serde_json::Value) -> String {
    let title = schema
        .get("title")
        .or_else(|| schema.get("name"))
        .unwrap()
        .as_str()
        .unwrap();
    let properties = schema.get("properties").unwrap().as_object().unwrap();

    let mut out = format!(
        "--- {title}Component type stub\n---@class {title}Component\n"
    );
    for (field, prop) in properties.iter() {
        if field == "pos" && prop.get("oneOf").is_some() {
            out.push_str("---@field pos Position\n\n---@class Position\n");
            for variant in prop.get("oneOf").unwrap().as_array().unwrap() {
                let variant_obj = variant.get("properties").unwrap().as_object().unwrap();
                for (variant_name, variant_schema) in variant_obj.iter() {
                    out.push_str(&format!("---@field {variant_name}? {{ "));
                    let fields_obj = variant_schema
                        .get("properties")
                        .unwrap()
                        .as_object()
                        .unwrap();
                    let mut variant_fields = Vec::new();
                    for (f, f_schema) in fields_obj.iter() {
                        let ty = match f_schema.get("type").and_then(|t| t.as_str()) {
                            Some("integer") => "integer",
                            Some("string") => "string",
                            _ => "any",
                        };
                        variant_fields.push(format!("{f}: {ty}"));
                    }
                    out.push_str(&variant_fields.join(", "));
                    out.push_str(" }\n");
                }
            }
        } else {
            let ty = match prop.get("type").and_then(|t| t.as_str()) {
                Some("integer") => "integer",
                Some("string") => "string",
                _ => "any",
            };
            out.push_str(&format!("---@field {field} {ty}\n"));
        }
    }
    out
}

fn generate_python_stub(schema: &serde_json::Value) -> String {
    let title = schema
        .get("title")
        .or_else(|| schema.get("name"))
        .unwrap()
        .as_str()
        .unwrap();
    let properties = schema.get("properties").unwrap().as_object().unwrap();

    let mut out = format!(
        "# {title}Component type stub\nfrom typing import Optional, TypedDict, Union\n\n"
    );
    let mut union_types = Vec::new();
    let mut union_names = Vec::new();

    for (field, prop) in properties.iter() {
        if field == "pos" && prop.get("oneOf").is_some() {
            for variant in prop.get("oneOf").unwrap().as_array().unwrap() {
                let variant_obj = variant.get("properties").unwrap().as_object().unwrap();
                for (variant_name, variant_schema) in variant_obj.iter() {
                    union_names.push(variant_name.clone());
                    out.push_str(&format!("class {variant_name}(TypedDict):\n"));
                    let fields_obj = variant_schema
                        .get("properties")
                        .unwrap()
                        .as_object()
                        .unwrap();
                    for (f, f_schema) in fields_obj.iter() {
                        let ty = match f_schema.get("type").and_then(|t| t.as_str()) {
                            Some("integer") => "int",
                            Some("string") => "str",
                            _ => "any",
                        };
                        out.push_str(&format!("    {f}: {ty}\n"));
                    }
                    out.push('\n');
                }
            }
            union_types.push(format!("Position = Union[{}]\n", union_names.join(", ")));
            out.push_str(&union_types.join(""));
            out.push_str(&format!(
                "\nclass {title}Component(TypedDict):\n    pos: Position\n"
            ));
        } else {
            let ty = match prop.get("type").and_then(|t| t.as_str()) {
                Some("integer") => "int",
                Some("string") => "str",
                _ => "any",
            };
            out.push_str(&format!(
                "class {title}Component(TypedDict):\n    {field}: {ty}\n"
            ));
        }
    }
    out
}

fn generate_c_header(schema: &serde_json::Value) -> String {
    let title = schema
        .get("title")
        .or_else(|| schema.get("name"))
        .unwrap()
        .as_str()
        .unwrap();
    let guard = format!("{}_COMPONENT_H", title.to_uppercase());
    let mut out = format!(
        "// AUTO-GENERATED FILE: DO NOT EDIT!\n// Schema: {title}Component\n\n#ifndef {guard}\n#define {guard}\n\n#include <stdint.h>\n\n"
    );

    let properties = schema.get("properties").unwrap().as_object().unwrap();
    for (field, prop) in properties.iter() {
        if field == "pos" && prop.get("oneOf").is_some() {
            // Enum kind
            out.push_str("typedef enum {\n");
            out.push_str("  POSITION_KIND_SQUARE,\n");
            out.push_str("  POSITION_KIND_HEX,\n");
            out.push_str("  POSITION_KIND_REGION\n");
            out.push_str("} PositionKind;\n\n");

            // Union struct
            out.push_str("typedef struct {\n");
            out.push_str("  PositionKind kind;\n");
            out.push_str("  union {\n");
            for variant in prop.get("oneOf").unwrap().as_array().unwrap() {
                let variant_obj = variant.get("properties").unwrap().as_object().unwrap();
                for (variant_name, _variant_schema) in variant_obj.iter() {
                    match variant_name.as_str() {
                        "Square" => {
                            out.push_str("    struct {\n      int32_t x, y, z;\n    } Square;\n");
                        }
                        "Hex" => {
                            out.push_str("    struct {\n      int32_t q, r, z;\n    } Hex;\n");
                        }
                        "Region" => {
                            out.push_str("    struct {\n      const char *id;\n    } Region;\n");
                        }
                        _ => {}
                    }
                }
            }
            out.push_str("  };\n");
            out.push_str("} Position;\n\n");

            // Component struct
            out.push_str("typedef struct {\n  Position pos;\n} PositionComponent;\n\n");
        }
    }
    out.push_str(&format!("#endif // {guard}\n"));
    out
}

fn generate_markdown_doc(schema: &serde_json::Value, schema_path: &str) -> String {
    use std::path::Path;

    let title = schema
        .get("title")
        .or_else(|| schema.get("name"))
        .unwrap()
        .as_str()
        .unwrap();
    let mut out = format!("# {title}Component\n\n");
    out.push_str("**Kind:** Component\n");
    out.push_str(&format!(
        "**Source schema:** `{}`\n\n",
        Path::new(schema_path)
            .file_name()
            .unwrap()
            .to_string_lossy()
    ));

    let properties = schema.get("properties").unwrap().as_object().unwrap();
    let mut has_description = false;
    let mut rows = Vec::new();

    for (field, prop) in properties.iter() {
        let typ = if field == "pos" && prop.get("oneOf").is_some() {
            "Position".to_string()
        } else {
            match prop.get("type").and_then(|t| t.as_str()) {
                Some("integer") => "integer".to_string(),
                Some("string") => "string".to_string(),
                Some(other) => other.to_string(),
                None => "unknown".to_string(),
            }
        };
        let desc = if field == "pos" {
            prop.get("description")
                .and_then(|d| d.as_str())
                .or_else(|| schema.get("description").and_then(|d| d.as_str()))
                .unwrap_or("")
        } else {
            prop.get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
        };
        if !desc.trim().is_empty() {
            has_description = true;
        }
        rows.push((field, typ, desc));
    }

    out.push_str("## Fields\n\n");
    if has_description {
        out.push_str("| Name | Type     | Description                                               |\n| ---- | -------- | --------------------------------------------------------- |\n");
        for (field, typ, desc) in &rows {
            out.push_str(&format!("| {field:<4} | {typ:<8} | {desc:<57} |\n"));
        }
    } else {
        out.push_str("| Name | Type     |\n| ---- | -------- |\n");
        for (field, typ, _) in &rows {
            out.push_str(&format!("| {field:<4} | {typ:<8} |\n"));
        }
    }

    // Document the union/enum
    if let Some(pos_prop) = properties.get("pos") {
        if let Some(one_of) = pos_prop.get("oneOf") {
            out.push_str(
                "\n### Position\n\nA tagged union (enum) with the following variants:\n\n",
            );
            let mut first = true;
            for variant in one_of.as_array().unwrap() {
                let variant_obj = variant.get("properties").unwrap().as_object().unwrap();
                for (variant_name, variant_schema) in variant_obj.iter() {
                    if !first {
                        out.push('\n');
                    }
                    first = false;
                    out.push_str(&format!("- **{variant_name}**:\n\n"));
                    let fields_obj = variant_schema
                        .get("properties")
                        .unwrap()
                        .as_object()
                        .unwrap();
                    for (f, f_schema) in fields_obj.iter() {
                        let ty = match f_schema.get("type").and_then(|t| t.as_str()) {
                            Some("integer") => "integer",
                            Some("string") => "string",
                            Some(other) => other,
                            None => "unknown",
                        };
                        out.push_str(&format!("  - `{f}` ({ty})\n"));
                    }
                }
            }
        }
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}
