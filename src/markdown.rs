use anyhow::{Context, Result};
use serde::Serialize;

use crate::parser::{Document, ScalarType};

pub const TEMPLATE: &str = r#"
| Name | Value |
|------|-------|
{%- for row in rows %}
| {{ row.name }} | {{ row.value }} |
{%- endfor %}
"#;

#[derive(Debug, Clone, Serialize)]
struct TableRow {
    name: String,
    value: String,
}

pub fn render_markdown(document: &Document) -> Result<String> {
    let mut tera = tera::Tera::default();
    tera.add_raw_template("main", TEMPLATE)
        .context("failed to parse template")?;

    let mut context = tera::Context::new();
    context.insert("rows", &flatten_document(document));

    tera.render("main", &context)
        .context("failed to render template")
}

fn flatten_document(document: &Document) -> Vec<TableRow> {
    let mut rows = Vec::new();
    flatten_scalar(&document.root.value, String::new(), &mut rows);
    rows
}

fn flatten_scalar(scalar: &ScalarType, key: String, rows: &mut Vec<TableRow>) {
    match &scalar {
        ScalarType::Map(map) => {
            for entry in map {
                let new_key = if key.is_empty() {
                    entry.key.clone()
                } else {
                    format!("{}.{}", key, entry.key)
                };
                flatten_scalar(&entry.value.value, new_key, rows);
            }
        }
        ScalarType::List(list) => {
            for (index, item) in list.iter().enumerate() {
                let new_key = format!("{}.{}", key, index);
                flatten_scalar(&item.value, new_key, rows);
            }
        }
        _ => {
            rows.push(TableRow {
                name: key,
                value: format_scalar_value(scalar),
            });
        }
    }
}

fn format_scalar_value(value: &ScalarType) -> String {
    match value {
        ScalarType::String(s) => s.clone(),
        ScalarType::Integer(n) => n.to_string(),
        ScalarType::Float(n) => n.to_string(),
        ScalarType::Boolean(b) => b.to_string(),
        ScalarType::Null => "null".to_string(),
        _ => "".to_string(),
    }
}
