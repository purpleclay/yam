use std::collections::HashMap;

use anyhow::{Context, Result, anyhow};
use tree_sitter::{Node, Parser};

#[derive(Debug)]
pub struct Document<'a> {
    pub root: Scalar<'a>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scalar<'a> {
    pub value: ScalarType<'a>,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScalarType<'a> {
    Null,
    String(&'a str),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    List(Vec<Scalar<'a>>),
    Map(Vec<MapItem<'a>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MapItem<'a> {
    pub key: &'a str,
    pub value: Scalar<'a>,
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

    fn parse(&mut self, node: &Node) -> Result<Scalar<'a>, ParseError> {
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

                let joined_comment = comment_parts.join(" ");
                self.comments.insert(last_line, joined_comment);
            } else {
                self.parse_comments(&child);
            }
        }
    }

    fn extract_comment_text(&self, node: &Node) -> &'a str {
        let text = &self.source[node.byte_range()];
        text.trim_start_matches('#').trim()
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

    fn parse_tree(&self, node: &Node) -> Result<Scalar<'a>, ParseError> {
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

    fn parse_value(&self, node: Node) -> Result<Scalar<'a>> {
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

    fn parse_quoted_scalar(&self, node: Node) -> Result<Scalar<'a>> {
        let text = &self.source[node.byte_range()];
        Ok(Scalar {
            value: ScalarType::String(&text[1..text.len() - 1]),
            comment: None,
        })
    }

    fn parse_block_scalar(&self, node: Node) -> Result<Scalar<'a>> {
        let text = &self.source[node.byte_range()];

        if let Some(newline_pos) = text.find('\n') {
            let content = &text[newline_pos + 1..];
            Ok(Scalar {
                value: ScalarType::String(content),
                comment: None,
            })
        } else {
            Ok(Scalar {
                value: ScalarType::String(""),
                comment: None,
            })
        }
    }

    fn parse_plain_scalar(&self, node: Node) -> Result<Scalar<'a>> {
        let scalar = node
            .child(0)
            .ok_or_else(|| anyhow!("should have a child"))?;

        match scalar.kind() {
            "integer_scalar" => {
                let text = &self.source[scalar.byte_range()];
                let pos = scalar.start_position();

                let parse_int = |num_str: &str, radix: u32, format: &str| {
                    i64::from_str_radix(num_str, radix).map_err(|_| {
                        anyhow!(
                            "invalid {} integer at line {}, column {}",
                            format,
                            pos.row + 1,
                            pos.column + 1
                        )
                    })
                };

                let value = if text.len() > 2 {
                    match &text[..2].to_ascii_lowercase()[..] {
                        "0x" => parse_int(&text[2..], 16, "hexadecimal")?,
                        "0o" => parse_int(&text[2..], 8, "octal")?,
                        _ => text.parse::<i64>().map_err(|_| {
                            anyhow!(
                                "invalid integer at line {}, column {}",
                                pos.row + 1,
                                pos.column + 1
                            )
                        })?,
                    }
                } else {
                    text.parse::<i64>().map_err(|_| {
                        anyhow!(
                            "invalid integer at line {}, column {}",
                            pos.row + 1,
                            pos.column + 1
                        )
                    })?
                };

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
                    value: ScalarType::String(text),
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

    fn parse_block_sequence(&self, node: Node) -> Result<Vec<Scalar<'a>>, ParseError> {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .filter(|child| child.kind() == "block_sequence_item")
            .map(|child| self.parse_tree(&child))
            .collect()
    }

    fn parse_flow_sequence(&self, node: Node) -> Result<Vec<Scalar<'a>>, ParseError> {
        let mut cursor = node.walk();
        node.children(&mut cursor)
            .filter(|child| child.kind() == "flow_node")
            .map(|child| self.parse_tree(&child))
            .collect()
    }

    fn parse_mapping(&self, node: Node) -> Result<Vec<MapItem<'a>>> {
        let mut cursor = node.walk();
        let mut items = Vec::new();

        for child in node.children(&mut cursor) {
            match child.kind() {
                "block_mapping_pair" | "flow_pair" => {
                    let key_node = child
                        .child_by_field_name("key")
                        .ok_or_else(|| anyhow!("mandatory map key is missing"))?;
                    let key = self.parse_key_as_str(&key_node)?;

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
                    let key = self.parse_key_as_str(&child)?;
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

    fn parse_key_as_str(&self, node: &Node) -> Result<&'a str> {
        match node.kind() {
            "flow_node" | "block_node" => {
                if let Some(child) = node.child(0) {
                    self.parse_key_as_str(&child)
                } else {
                    Err(anyhow!("expected child node"))
                }
            }
            "plain_scalar" => {
                if let Some(child) = node.child(0) {
                    Ok(&self.source[child.byte_range()])
                } else {
                    Ok(&self.source[node.byte_range()])
                }
            }
            "single_quote_scalar" | "double_quote_scalar" => {
                let text = &self.source[node.byte_range()];
                Ok(&text[1..text.len() - 1])
            }
            _ => Ok(&self.source[node.byte_range()]),
        }
    }
}

pub fn parse(text: &str) -> Result<Option<Document<'_>>> {
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
