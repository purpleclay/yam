use anyhow::{Context, Result, anyhow};
use tree_sitter::{Node, Parser};

#[derive(Debug)]
pub struct Document {
    pub root: Scalar,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scalar {
    pub value: ScalarType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScalarType {
    Null,
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    List(Vec<Scalar>),
    Map(Vec<MapItem>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapItem {
    pub key: String,
    pub value: Scalar,
}

struct YamlParser<'a> {
    source: &'a str,
}

impl<'a> YamlParser<'a> {
    fn new(source: &'a str) -> Self {
        Self { source }
    }

    fn parse_tree(&self, node: &Node) -> Result<Scalar> {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "document" | "stream" => return self.parse_tree(&child),
                "-" => {}
                _ => return self.parse_value(child),
            }
        }

        Err(anyhow!("no parseable content found"))
    }

    fn parse_value(&self, node: Node) -> Result<Scalar> {
        match node.kind() {
            "flow_node" | "block_node" => {
                let value = node
                    .child(0)
                    .ok_or_else(|| anyhow!("flow_node/block_node should have a child"))?;
                self.parse_value(value)
            }
            "plain_scalar" => self.parse_plain_scalar(node),
            "single_quote_scalar" | "double_quote_scalar" => self.parse_quoted_scalar(node),
            "block_sequence" => {
                let scalar_items = self.parse_block_sequence(node)?;
                Ok(Scalar {
                    value: ScalarType::List(scalar_items),
                })
            }
            "block_mapping" => {
                let map_items = self.parse_block_mapping(node)?;
                Ok(Scalar {
                    value: ScalarType::Map(map_items),
                })
            }
            _ => Err(anyhow!("unexpected node kind {}", node.kind())),
        }
    }

    fn parse_quoted_scalar(&self, node: Node) -> Result<Scalar> {
        let text = &self.source[node.byte_range()];
        Ok(Scalar {
            value: ScalarType::String(text[1..text.len() - 1].to_string()),
        })
    }

    fn parse_plain_scalar(&self, node: Node) -> Result<Scalar> {
        let scalar = node
            .child(0)
            .ok_or_else(|| anyhow!("should have a child"))?;

        match scalar.kind() {
            "integer_scalar" => {
                let text = &self.source[scalar.byte_range()];
                let value = text
                    .parse::<i64>()
                    .map_err(|_| anyhow!("invalid integer"))?;
                Ok(Scalar {
                    value: ScalarType::Integer(value),
                })
            }
            "float_scalar" => {
                let text = &self.source[scalar.byte_range()];
                let value = text.parse::<f64>().map_err(|_| anyhow!("invalid float"))?;
                Ok(Scalar {
                    value: ScalarType::Float(value),
                })
            }
            "boolean_scalar" => {
                let text = &self.source[scalar.byte_range()];
                let value = text
                    .parse::<bool>()
                    .map_err(|_| anyhow!("invalid boolean"))?;
                Ok(Scalar {
                    value: ScalarType::Boolean(value),
                })
            }
            "string_scalar" => {
                let text = &self.source[scalar.byte_range()];
                Ok(Scalar {
                    value: ScalarType::String(text.to_string()),
                })
            }
            "null_scalar" => Ok(Scalar {
                value: ScalarType::Null,
            }),
            _ => Err(anyhow!("unexpected node kind {}", scalar.kind())),
        }
    }

    fn parse_block_sequence(&self, node: Node) -> Result<Vec<Scalar>> {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .filter(|child| child.kind() == "block_sequence_item")
            .map(|child| self.parse_tree(&child))
            .collect()
    }

    fn parse_block_mapping(&self, node: Node) -> Result<Vec<MapItem>> {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .filter(|child| child.kind() == "block_mapping_pair")
            .map(|child| {
                let key_node = child
                    .child_by_field_name("key")
                    .ok_or_else(|| anyhow!("mandatory map key is missing"))?;
                let key = self.parse_key_as_string(&key_node)?;

                let value = match child.child_by_field_name("value") {
                    Some(value_node) => self.parse_tree(&value_node)?,
                    None => Scalar {
                        value: ScalarType::Null,
                    },
                };
                Ok(MapItem { key, value })
            })
            .collect()
    }

    fn parse_key_as_string(&self, node: &Node) -> Result<String> {
        let scalar = self.parse_tree(node)?;
        match scalar.value {
            ScalarType::String(s) => Ok(s),
            ScalarType::Integer(i) => Ok(i.to_string()),
            ScalarType::Float(f) => Ok(f.to_string()),
            ScalarType::Boolean(b) => Ok(b.to_string()),
            _ => Err(anyhow!("complex types cannot be used as map keys")),
        }
    }
}

pub fn parse(text: &str) -> Result<Document> {
    let mut parser = Parser::new();
    let language = tree_sitter_yaml::LANGUAGE;

    parser
        .set_language(&language.into())
        .context("failed to set YAML language")?;

    let tree = parser
        .parse(text, None)
        .ok_or_else(|| anyhow!("failed to parse YAML document"))?;

    let root_node = tree.root_node();
    let yaml_parser = YamlParser::new(text);
    let root_scalar = yaml_parser.parse_tree(&root_node)?;

    Ok(Document { root: root_scalar })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scalar_integer() -> Result<()> {
        let document = parse("42")?;
        assert!(matches!(document.root.value, ScalarType::Integer(42)));

        Ok(())
    }

    #[test]
    fn parse_scalar_float() -> Result<()> {
        let document = parse("42.56")?;
        assert!(matches!(document.root.value, ScalarType::Float(42.56)));

        Ok(())
    }

    #[test]
    fn parse_scalar_boolean() -> Result<()> {
        let document = parse("true")?;
        assert!(matches!(document.root.value, ScalarType::Boolean(true)));

        Ok(())
    }

    #[test]
    fn parse_scalar_double_quoted_string() -> Result<()> {
        let document = parse("\"hello, world!\"")?;
        assert!(matches!(
            document.root.value,
            ScalarType::String(ref s) if s == "hello, world!"
        ));

        Ok(())
    }

    #[test]
    fn parse_scalar_single_quoted_string() -> Result<()> {
        let document = parse("'good afternoon, good evening, and good night'")?;
        assert!(matches!(
            document.root.value,
            ScalarType::String(ref s) if s == "good afternoon, good evening, and good night"
        ));

        Ok(())
    }

    #[test]
    fn parse_scalar_null() -> Result<()> {
        let document = parse("null")?;
        assert!(matches!(document.root.value, ScalarType::Null));
        Ok(())
    }

    #[test]
    fn parse_scalar_null_as_tilde() -> Result<()> {
        let document = parse("~")?;
        assert!(matches!(document.root.value, ScalarType::Null));
        Ok(())
    }

    #[test]
    fn parse_scalar_list() -> Result<()> {
        let yaml = r#"
            - 42
            - 42.56
            - true
            - "hello, world!"
            - 'good afternoon, good evening, and good night'
            "#;
        let document = parse(yaml)?;
        assert!(matches!(
            document.root.value,
            ScalarType::List(ref items) if items.len() == 5
        ));

        assert!(matches!(
            document.root.value,
            ScalarType::List(ref items) if items[0] == Scalar { value: ScalarType::Integer(42) }
        ));

        assert!(matches!(
            document.root.value,
            ScalarType::List(ref items) if items[1] == Scalar { value: ScalarType::Float(42.56) }
        ));

        assert!(matches!(
            document.root.value,
            ScalarType::List(ref items) if items[2] == Scalar { value: ScalarType::Boolean(true) }
        ));

        assert!(matches!(
            document.root.value,
            ScalarType::List(ref items) if items[3] == Scalar { value: ScalarType::String("hello, world!".to_string()) }
        ));

        assert!(matches!(
            document.root.value,
            ScalarType::List(ref items) if items[4] == Scalar { value: ScalarType::String("good afternoon, good evening, and good night".to_string()) }
        ));

        Ok(())
    }

    #[test]
    fn parse_scalar_map() -> Result<()> {
        let document = parse("name: truman")?;

        match document.root.value {
            ScalarType::Map(ref map) => {
                assert_eq!(map.len(), 1);
                assert_eq!(map[0].key, "name");
                assert_eq!(
                    map[0].value,
                    Scalar {
                        value: ScalarType::String("truman".to_string())
                    }
                );
            }
            _ => panic!("root node should contain a map scalar"),
        }

        Ok(())
    }

    #[test]
    fn parse_scalar_map_with_empty_value() -> Result<()> {
        let document = parse("name: ")?;

        match document.root.value {
            ScalarType::Map(ref map) => {
                assert_eq!(map.len(), 1);
                assert_eq!(map[0].key, "name");
                assert_eq!(
                    map[0].value,
                    Scalar {
                        value: ScalarType::Null
                    }
                );
            }
            _ => panic!("root node should contain a map scalar"),
        }

        Ok(())
    }
}
