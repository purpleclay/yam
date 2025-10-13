use std::collections::HashMap;

use anyhow::{Context, Result, anyhow};
use tree_sitter::{Node, Parser};

#[derive(Debug)]
pub struct Document {
    pub root: Scalar,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scalar {
    pub value: ScalarType,
    pub comment: Option<String>,
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

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("an empty document")]
    EmptyDocument,
    #[error("parsing error: {0}")]
    Generic(#[from] anyhow::Error),
}

struct YamlParser<'a> {
    source: &'a str,
    comments: HashMap<usize, String>,
}

impl<'a> YamlParser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            comments: HashMap::new(),
        }
    }

    fn parse(&mut self, node: &Node) -> Result<Scalar, ParseError> {
        self.parse_comments(node);
        self.parse_tree(node)
    }

    fn parse_comments(&mut self, node: &Node) {
        let mut cursor = node.walk();
        let mut children = node.children(&mut cursor).peekable();

        while let Some(child) = children.next() {
            if child.kind() == "comment" {
                let mut comment_parts = vec![self.extract_comment_text(&child)];
                let mut last_line = child.start_position().row;

                while let Some(next) = children.peek() {
                    if next.kind() == "comment" {
                        let next_child = children.next().unwrap();
                        last_line = next_child.start_position().row;
                        comment_parts.push(self.extract_comment_text(&next_child));
                    } else {
                        break;
                    }
                }

                self.comments.insert(last_line, comment_parts.join(" "));
            } else {
                self.parse_comments(&child);
            }
        }
    }

    fn extract_comment_text(&self, node: &Node) -> String {
        self.source[node.byte_range()]
            .trim_start_matches('#')
            .trim()
            .to_string()
    }

    fn find_comment_for_node(&self, node: &Node) -> Option<String> {
        let line_number = node.start_position().row;

        if let Some(comment) = self.comments.get(&line_number) {
            return Some(comment.clone());
        }

        if line_number > 0
            && let Some(comment) = self.comments.get(&(line_number - 1))
        {
            return Some(comment.clone());
        }

        None
    }

    fn parse_tree(&self, node: &Node) -> Result<Scalar, ParseError> {
        let mut cursor = node.walk();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "document" | "stream" => return self.parse_tree(&child),
                "-" | "comment" => {}
                _ => {
                    let mut scalar = self.parse_value(child).map_err(ParseError::Generic)?;
                    if scalar.comment.is_none() {
                        scalar.comment = self.find_comment_for_node(&child);
                    }

                    return Ok(scalar);
                }
            }
        }

        Err(ParseError::EmptyDocument)
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
            "block_scalar" => self.parse_block_scalar(node),
            "block_sequence" => {
                let scalar_items = self.parse_block_sequence(node)?;
                Ok(Scalar {
                    value: ScalarType::List(scalar_items),
                    comment: None,
                })
            }
            "block_mapping" | "flow_mapping" => {
                let map_items = self.parse_mapping(node)?;
                Ok(Scalar {
                    value: ScalarType::Map(map_items),
                    comment: None,
                })
            }
            "flow_sequence" => {
                let scalar_items = self.parse_flow_sequence(node)?;
                Ok(Scalar {
                    value: ScalarType::List(scalar_items),
                    comment: None,
                })
            }
            _ => {
                let pos = node.start_position();
                Err(anyhow!(
                    "unexpected node kind {} at line {}, column {}",
                    node.kind(),
                    pos.row + 1,
                    pos.column + 1
                ))
            }
        }
    }

    fn parse_quoted_scalar(&self, node: Node) -> Result<Scalar> {
        let text = &self.source[node.byte_range()];
        Ok(Scalar {
            value: ScalarType::String(text[1..text.len() - 1].to_string()),
            comment: None,
        })
    }

    fn parse_block_scalar(&self, node: Node) -> Result<Scalar> {
        let text = &self.source[node.byte_range()];

        if let Some(newline_pos) = text.find('\n') {
            let content = &text[newline_pos + 1..];
            Ok(Scalar {
                value: ScalarType::String(content.to_string()),
                comment: None,
            })
        } else {
            Ok(Scalar {
                value: ScalarType::String(String::new()),
                comment: None,
            })
        }
    }

    fn parse_plain_scalar(&self, node: Node) -> Result<Scalar> {
        let scalar = node
            .child(0)
            .ok_or_else(|| anyhow!("should have a child"))?;

        match scalar.kind() {
            "integer_scalar" => {
                let text = &self.source[scalar.byte_range()];
                let value = text.parse::<i64>().map_err(|_| {
                    let pos = scalar.start_position();
                    anyhow!(
                        "invalid integer at line {}, column {}",
                        pos.row + 1,
                        pos.column + 1
                    )
                })?;
                Ok(Scalar {
                    value: ScalarType::Integer(value),
                    comment: None,
                })
            }
            "float_scalar" => {
                let text = &self.source[scalar.byte_range()];
                let value = match text.to_lowercase().as_str() {
                    ".inf" => f64::INFINITY,
                    "-.inf" => f64::NEG_INFINITY,
                    ".nan" => f64::NAN,
                    _ => text.parse::<f64>().map_err(|_| {
                        let pos = scalar.start_position();
                        anyhow!(
                            "invalid float at line {}, column {}",
                            pos.row + 1,
                            pos.column + 1
                        )
                    })?,
                };

                Ok(Scalar {
                    value: ScalarType::Float(value),
                    comment: None,
                })
            }
            "boolean_scalar" => {
                let text = &self.source[scalar.byte_range()];
                let value = text.parse::<bool>().map_err(|_| {
                    let pos = scalar.start_position();
                    anyhow!(
                        "invalid boolean at line {}, column {}",
                        pos.row + 1,
                        pos.column + 1
                    )
                })?;
                Ok(Scalar {
                    value: ScalarType::Boolean(value),
                    comment: None,
                })
            }
            "string_scalar" => {
                let text = &self.source[scalar.byte_range()];
                Ok(Scalar {
                    value: ScalarType::String(text.to_string()),
                    comment: None,
                })
            }
            "null_scalar" => Ok(Scalar {
                value: ScalarType::Null,
                comment: None,
            }),
            _ => {
                let pos = scalar.start_position();
                Err(anyhow!(
                    "unexpected node kind {} at line {}, column {}",
                    scalar.kind(),
                    pos.row + 1,
                    pos.column + 1
                ))
            }
        }
    }

    fn parse_block_sequence(&self, node: Node) -> Result<Vec<Scalar>, ParseError> {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .filter(|child| child.kind() == "block_sequence_item")
            .map(|child| self.parse_tree(&child))
            .collect()
    }

    fn parse_flow_sequence(&self, node: Node) -> Result<Vec<Scalar>, ParseError> {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .filter(|child| child.kind() == "flow_node")
            .map(|child| self.parse_tree(&child))
            .collect()
    }

    fn parse_mapping(&self, node: Node) -> Result<Vec<MapItem>> {
        let mut cursor = node.walk();
        let mut items = Vec::new();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "block_mapping_pair" | "flow_pair" => {
                    let key_node = child
                        .child_by_field_name("key")
                        .ok_or_else(|| anyhow!("mandatory map key is missing"))?;
                    let key = self.parse_key_as_string(&key_node)?;

                    let value = match child.child_by_field_name("value") {
                        Some(value_node) => self.parse_tree(&value_node)?,
                        None => Scalar {
                            value: ScalarType::Null,
                            comment: None,
                        },
                    };
                    items.push(MapItem { key, value });
                }
                "flow_node" => {
                    let key = self.parse_key_as_string(&child)?;
                    let value = Scalar {
                        value: ScalarType::Null,
                        comment: None,
                    };
                    items.push(MapItem { key, value });
                }
                _ => {}
            }
        }

        Ok(items)
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

pub fn parse(text: &str) -> Result<Option<Document>> {
    let mut parser = Parser::new();
    let language = tree_sitter_yaml::LANGUAGE;

    parser
        .set_language(&language.into())
        .context("failed to set YAML language")?;

    let tree = parser
        .parse(text, None)
        .ok_or_else(|| anyhow!("failed to parse YAML document"))?;

    let root_node = tree.root_node();
    let mut yaml_parser = YamlParser::new(text);

    match yaml_parser.parse(&root_node) {
        Ok(root_scalar) => Ok(Some(Document { root: root_scalar })),
        Err(ParseError::EmptyDocument) => Ok(None),
        Err(ParseError::Generic(e)) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Ok;

    use super::*;

    #[test]
    fn parse_scalar_integer() -> Result<()> {
        let document = parse("42")?.unwrap();
        assert_eq!(document.root.value, ScalarType::Integer(42));

        Ok(())
    }

    #[test]
    fn parse_scalar_integer_with_comment() -> Result<()> {
        let document = parse("42 # comment")?.unwrap();
        assert_eq!(document.root.value, ScalarType::Integer(42));
        assert_eq!(document.root.comment, Some("comment".to_string()));

        Ok(())
    }

    #[test]
    fn parse_scalar_float() -> Result<()> {
        let document = parse("42.56")?.unwrap();
        assert_eq!(document.root.value, ScalarType::Float(42.56));

        Ok(())
    }

    #[test]
    fn parse_scalar_float_with_comment() -> Result<()> {
        let yaml = r#"
        # comment
        42.56
        "#;
        let document = parse(yaml)?.unwrap();
        assert_eq!(document.root.value, ScalarType::Float(42.56));
        assert_eq!(document.root.comment, Some("comment".to_string()));

        Ok(())
    }

    #[test]
    fn parse_scalar_float_negative() -> Result<()> {
        let document = parse("-42.56")?.unwrap();
        assert_eq!(document.root.value, ScalarType::Float(-42.56));

        Ok(())
    }

    #[test]
    fn parse_scalar_float_scientific_notation() -> Result<()> {
        let document = parse("1.23e+2")?.unwrap();
        assert_eq!(document.root.value, ScalarType::Float(123.0));

        Ok(())
    }

    #[test]
    fn parse_scalar_float_scientific_notation_negative() -> Result<()> {
        let document = parse("-1.23e+2")?.unwrap();
        assert_eq!(document.root.value, ScalarType::Float(-123.0));

        Ok(())
    }

    #[test]
    fn parse_scalar_float_positive_infinity() -> Result<()> {
        let document = parse(".inf")?.unwrap();
        match document.root.value {
            ScalarType::Float(f) => assert!(f.is_infinite() && f.is_sign_positive()),
            _ => panic!("expected positive infinity float"),
        }

        Ok(())
    }

    #[test]
    fn parse_scalar_float_negative_infinity() -> Result<()> {
        let document = parse("-.inf")?.unwrap();
        match document.root.value {
            ScalarType::Float(f) => assert!(f.is_infinite() && f.is_sign_negative()),
            _ => panic!("expected negative infinity float"),
        }

        Ok(())
    }

    #[test]
    fn parse_scalar_float_nan() -> Result<()> {
        let document = parse(".nan")?.unwrap();
        match document.root.value {
            ScalarType::Float(f) => assert!(f.is_nan()),
            _ => panic!("expected NaN float"),
        }

        Ok(())
    }

    #[test]
    fn parse_scalar_boolean() -> Result<()> {
        let document = parse("true")?.unwrap();
        assert_eq!(document.root.value, ScalarType::Boolean(true));

        Ok(())
    }

    #[test]
    fn parse_scalar_boolean_with_comment() -> Result<()> {
        let document = parse("true # comment")?.unwrap();
        assert_eq!(document.root.value, ScalarType::Boolean(true));
        assert_eq!(document.root.comment, Some("comment".to_string()));

        Ok(())
    }

    #[test]
    fn parse_scalar_double_quoted_string() -> Result<()> {
        let document = parse("\"hello, world!\"")?.unwrap();
        assert_eq!(
            document.root.value,
            ScalarType::String("hello, world!".to_string())
        );

        Ok(())
    }

    #[test]
    fn parse_scalar_double_quoted_string_with_comment() -> Result<()> {
        let document = parse("\"hello, world!\" # comment")?.unwrap();
        assert_eq!(
            document.root.value,
            ScalarType::String("hello, world!".to_string())
        );
        assert_eq!(document.root.comment, Some("comment".to_string()));

        Ok(())
    }

    #[test]
    fn parse_scalar_single_quoted_string() -> Result<()> {
        let document = parse("'good afternoon, good evening, and good night'")?.unwrap();
        assert_eq!(
            document.root.value,
            ScalarType::String("good afternoon, good evening, and good night".to_string())
        );

        Ok(())
    }

    #[test]
    fn parse_scalar_single_quoted_string_with_comment() -> Result<()> {
        let document = parse("'good afternoon, good evening, and good night' # comment")?.unwrap();
        assert_eq!(
            document.root.value,
            ScalarType::String("good afternoon, good evening, and good night".to_string())
        );

        assert_eq!(document.root.comment, Some("comment".to_string()));

        Ok(())
    }

    #[test]
    fn parse_scalar_null() -> Result<()> {
        let document = parse("null")?.unwrap();
        assert_eq!(document.root.value, ScalarType::Null);
        Ok(())
    }

    #[test]
    fn parse_scalar_null_as_tilde() -> Result<()> {
        let document = parse("~")?.unwrap();
        assert_eq!(document.root.value, ScalarType::Null);
        Ok(())
    }

    #[test]
    fn parse_scalar_null_with_comment() -> Result<()> {
        let document = parse("null # comment")?.unwrap();
        assert_eq!(document.root.value, ScalarType::Null);
        assert_eq!(document.root.comment, Some("comment".to_string()));
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
        let document = parse(yaml)?.unwrap();

        let items = match &document.root.value {
            ScalarType::List(items) => items,
            _ => panic!("root node should contain a list scalar"),
        };

        assert_eq!(items.len(), 5);
        assert_eq!(
            items[0],
            Scalar {
                value: ScalarType::Integer(42),
                comment: None
            }
        );
        assert_eq!(
            items[1],
            Scalar {
                value: ScalarType::Float(42.56),
                comment: None
            }
        );
        assert_eq!(
            items[2],
            Scalar {
                value: ScalarType::Boolean(true),
                comment: None
            }
        );
        assert_eq!(
            items[3],
            Scalar {
                value: ScalarType::String("hello, world!".to_string()),
                comment: None
            }
        );
        assert_eq!(
            items[4],
            Scalar {
                value: ScalarType::String(
                    "good afternoon, good evening, and good night".to_string()
                ),
                comment: None
            }
        );

        Ok(())
    }

    #[test]
    fn parse_scalar_list_with_comments() -> Result<()> {
        let yaml = r#"
            # comment for item 1
            - 42
            - 42.56 # comment for item 2
            "#;
        let document = parse(yaml)?.unwrap();

        let items = match &document.root.value {
            ScalarType::List(items) => items,
            _ => panic!("root node should contain a list scalar"),
        };

        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0],
            Scalar {
                value: ScalarType::Integer(42),
                comment: Some("comment for item 1".to_string())
            }
        );
        assert_eq!(
            items[1],
            Scalar {
                value: ScalarType::Float(42.56),
                comment: Some("comment for item 2".to_string())
            }
        );

        Ok(())
    }

    #[test]
    fn parse_scalar_list_with_flow_sequence() -> Result<()> {
        let document = parse("[1,2,3]")?.unwrap();

        let items = match &document.root.value {
            ScalarType::List(items) => items,
            _ => panic!("root node should contain a list scalar"),
        };

        assert_eq!(items.len(), 3);
        assert_eq!(
            items[0],
            Scalar {
                value: ScalarType::Integer(1),
                comment: None
            }
        );
        assert_eq!(
            items[1],
            Scalar {
                value: ScalarType::Integer(2),
                comment: None
            }
        );
        assert_eq!(
            items[2],
            Scalar {
                value: ScalarType::Integer(3),
                comment: None
            }
        );

        Ok(())
    }

    #[test]
    fn parse_scalar_list_with_empty_flow_sequence() -> Result<()> {
        let document = parse("[]")?.unwrap();

        let items = match &document.root.value {
            ScalarType::List(items) => items,
            _ => panic!("root node should contain a list scalar"),
        };

        assert_eq!(items.len(), 0);
        Ok(())
    }

    #[test]
    fn parse_scalar_map() -> Result<()> {
        let document = parse("name: truman")?.unwrap();

        match document.root.value {
            ScalarType::Map(ref map) => {
                assert_eq!(map.len(), 1);
                assert_eq!(map[0].key, "name");
                assert_eq!(
                    map[0].value,
                    Scalar {
                        value: ScalarType::String("truman".to_string()),
                        comment: None,
                    }
                );
            }
            _ => panic!("root node should contain a map scalar"),
        }

        Ok(())
    }

    #[test]
    fn parse_scalar_map_with_comments() -> Result<()> {
        let yaml = r#"
        # comment for x
        x: 1
        y: 2 # comment for y
        "#;

        let document = parse(yaml)?.unwrap();
        match document.root.value {
            ScalarType::Map(ref map) => {
                assert_eq!(map.len(), 2);
                assert_eq!(map[0].key, "x");
                assert_eq!(
                    map[0].value,
                    Scalar {
                        value: ScalarType::Integer(1),
                        comment: Some("comment for x".to_string()),
                    }
                );
                assert_eq!(map[1].key, "y");
                assert_eq!(
                    map[1].value,
                    Scalar {
                        value: ScalarType::Integer(2),
                        comment: Some("comment for y".to_string()),
                    }
                );
            }
            _ => panic!("root node should contain a map scalar"),
        }

        Ok(())
    }

    #[test]
    fn parse_scalar_map_with_empty_value() -> Result<()> {
        let document = parse("name: ")?.unwrap();

        match document.root.value {
            ScalarType::Map(ref map) => {
                assert_eq!(map.len(), 1);
                assert_eq!(map[0].key, "name");
                assert_eq!(
                    map[0].value,
                    Scalar {
                        value: ScalarType::Null,
                        comment: None,
                    }
                );
            }
            _ => panic!("root node should contain a map scalar"),
        }

        Ok(())
    }

    #[test]
    fn parse_scalar_map_with_flow_sequence() -> Result<()> {
        let document = parse("{x: 1, y: 2}")?.unwrap();
        match document.root.value {
            ScalarType::Map(ref map) => {
                assert_eq!(map.len(), 2);
                assert_eq!(map[0].key, "x");
                assert_eq!(
                    map[0].value,
                    Scalar {
                        value: ScalarType::Integer(1),
                        comment: None,
                    }
                );
                assert_eq!(map[1].key, "y");
                assert_eq!(
                    map[1].value,
                    Scalar {
                        value: ScalarType::Integer(2),
                        comment: None,
                    }
                );
            }
            _ => panic!("root node should contain a map scalar"),
        }
        Ok(())
    }

    #[test]
    fn parse_scalar_map_with_empty_flow_sequence() -> Result<()> {
        let document = parse("{}")?.unwrap();
        match document.root.value {
            ScalarType::Map(ref map) => {
                assert_eq!(map.len(), 0);
            }
            _ => panic!("root node should contain a map scalar"),
        }
        Ok(())
    }

    #[test]
    fn parse_scalar_map_with_flow_sequence_only_keys() -> Result<()> {
        let document = parse("{x, y:}")?.unwrap();
        match document.root.value {
            ScalarType::Map(ref map) => {
                assert_eq!(map.len(), 2);
                assert_eq!(map[0].key, "x");
                assert_eq!(
                    map[0].value,
                    Scalar {
                        value: ScalarType::Null,
                        comment: None,
                    }
                );
                assert_eq!(map[1].key, "y");
                assert_eq!(
                    map[1].value,
                    Scalar {
                        value: ScalarType::Null,
                        comment: None,
                    }
                );
            }
            _ => panic!("root node should contain a map scalar"),
        }
        Ok(())
    }

    #[test]
    fn parse_scalar_with_preceding_and_inline_comment() -> Result<()> {
        let yaml = r#"
        # preceding comment
        38 # inline comment
        "#;

        let document = parse(yaml)?.unwrap();
        assert_eq!(document.root.value, ScalarType::Integer(38));
        assert_eq!(document.root.comment, Some("inline comment".to_string()));

        Ok(())
    }

    #[test]
    fn parse_scalar_with_multiline_comment() -> Result<()> {
        let yaml = r#"
        # this is a comment that will be spread over
        # multiple lines
        22
        "#;

        let document = parse(yaml)?.unwrap();
        assert_eq!(document.root.value, ScalarType::Integer(22));
        assert_eq!(
            document.root.comment,
            Some("this is a comment that will be spread over multiple lines".to_string())
        );
        Ok(())
    }

    #[test]
    fn parse_scalar_map_with_multiline_comment() -> Result<()> {
        let yaml = r#"
        image:
          # this is a multiline comment
          # for a key within a map
          registry: docker.io
        "#;

        let document = parse(yaml)?.unwrap();
        match document.root.value {
            ScalarType::Map(ref map) => {
                assert_eq!(map.len(), 1);
                match map[0].value.value {
                    ScalarType::Map(ref map) => {
                        assert_eq!(map.len(), 1);
                        assert_eq!(
                            map[0].value.comment,
                            Some("this is a multiline comment for a key within a map".to_string())
                        );
                    }
                    _ => panic!("child node should contain a map scalar"),
                }
            }
            _ => panic!("root node should contain a map scalar"),
        }

        Ok(())
    }

    #[test]
    fn parse_block_scalar_literal() -> Result<()> {
        let yaml = r#"key: |
  this is a multiline
  string spread over multiple lines"#;

        let document = parse(yaml)?.unwrap();
        match document.root.value {
            ScalarType::Map(ref map) => {
                assert_eq!(map.len(), 1);
                assert_eq!(map[0].key, "key");
                match &map[0].value.value {
                    ScalarType::String(s) => {
                        assert!(s.contains("this is a multiline"));
                        assert!(s.contains("string spread over multiple lines"));
                    }
                    _ => panic!("value should be a string"),
                }
            }
            _ => panic!("root node should contain a map scalar"),
        }

        Ok(())
    }

    #[test]
    fn parse_block_scalar_with_chomping() -> Result<()> {
        let yaml = r#"key: |+2
  this is a multiline
  string spread over multiple lines
"#;

        let document = parse(yaml)?.unwrap();
        match document.root.value {
            ScalarType::Map(ref map) => {
                assert_eq!(map.len(), 1);
                assert_eq!(map[0].key, "key");
                match &map[0].value.value {
                    ScalarType::String(s) => {
                        assert!(s.contains("this is a multiline"));
                        assert!(s.contains("string spread over multiple lines"));
                    }
                    _ => panic!("value should be a string"),
                }
            }
            _ => panic!("root node should contain a map scalar"),
        }

        Ok(())
    }

    #[test]
    fn parse_block_scalar_folded() -> Result<()> {
        let yaml = r#"key: >
  this is a multiline
  string that will be folded"#;

        let document = parse(yaml)?.unwrap();
        match document.root.value {
            ScalarType::Map(ref map) => {
                assert_eq!(map.len(), 1);
                assert_eq!(map[0].key, "key");
                match &map[0].value.value {
                    ScalarType::String(s) => {
                        assert!(s.contains("this is a multiline"));
                        assert!(s.contains("string that will be folded"));
                    }
                    _ => panic!("value should be a string"),
                }
            }
            _ => panic!("root node should contain a map scalar"),
        }

        Ok(())
    }
}
