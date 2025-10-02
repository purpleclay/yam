use anyhow::{Context, Result};
use serde::Serialize;

use crate::parser::{Document, Scalar, ScalarType};

pub const TEMPLATE: &str = r#"
| Name | Value | Description |
|------|-------|-------------|
{%- for row in rows %}
| {{ row.name }} | {{ row.value }} | {{ row.description }} |
{%- endfor %}
"#;

#[derive(Debug, Clone, Serialize)]
struct TableRow {
    name: String,
    value: String,
    description: String,
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
    flatten_scalar(&document.root, String::new(), &mut rows);
    rows
}

fn flatten_scalar(scalar: &Scalar, key: String, rows: &mut Vec<TableRow>) {
    match &scalar.value {
        ScalarType::Map(map) => {
            for entry in map {
                let new_key = if key.is_empty() {
                    entry.key.clone()
                } else {
                    format!("{}.{}", key, entry.key)
                };
                flatten_scalar(&entry.value, new_key, rows);
            }
        }
        ScalarType::List(list) => {
            for (index, item) in list.iter().enumerate() {
                let new_key = format!("{}.{}", key, index);
                flatten_scalar(&item, new_key, rows);
            }
        }
        _ => {
            rows.push(TableRow {
                name: key,
                value: format_scalar_value(&scalar.value),
                description: scalar.comment.clone().unwrap_or_default(),
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
