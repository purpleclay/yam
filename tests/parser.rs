use anyhow::{Ok, Result};
use yam::parser::*;

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
fn parse_scalar_integer_negative() -> Result<()> {
    let document = parse("-42")?.unwrap();
    assert_eq!(document.root.value, ScalarType::Integer(-42));

    Ok(())
}

#[test]
fn parse_scalar_integer_octal() -> Result<()> {
    let document = parse("0o10")?.unwrap();
    assert_eq!(document.root.value, ScalarType::Integer(8));

    Ok(())
}

#[test]
fn parse_scalar_integer_hexadecimal() -> Result<()> {
    let document = parse("0x2A")?.unwrap();
    assert_eq!(document.root.value, ScalarType::Integer(42));

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
    assert_eq!(document.root.value, ScalarType::String("hello, world!"));

    Ok(())
}

#[test]
fn parse_scalar_double_quoted_string_with_comment() -> Result<()> {
    let document = parse("\"hello, world!\" # comment")?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("hello, world!"));
    assert_eq!(document.root.comment, Some("comment".to_string()));

    Ok(())
}

#[test]
fn parse_scalar_single_quoted_string() -> Result<()> {
    let document = parse("'good afternoon, good evening, and good night'")?.unwrap();
    assert_eq!(
        document.root.value,
        ScalarType::String("good afternoon, good evening, and good night")
    );

    Ok(())
}

#[test]
fn parse_scalar_single_quoted_string_with_comment() -> Result<()> {
    let document = parse("'good afternoon, good evening, and good night' # comment")?.unwrap();
    assert_eq!(
        document.root.value,
        ScalarType::String("good afternoon, good evening, and good night")
    );

    assert_eq!(document.root.comment, Some("comment".to_string()));

    Ok(())
}

#[test]
fn parse_scalar_empty_string_double_quoted() -> Result<()> {
    let document = parse(r#""""#)?.unwrap();
    assert_eq!(document.root.value, ScalarType::String(""));

    Ok(())
}

#[test]
fn parse_scalar_empty_string_single_quoted() -> Result<()> {
    let document = parse("''")?.unwrap();
    assert_eq!(document.root.value, ScalarType::String(""));

    Ok(())
}

#[test]
fn parse_scalar_string_with_escape_newline() -> Result<()> {
    let document = parse(r#""hello\nworld""#)?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("hello\\nworld"));

    Ok(())
}

#[test]
fn parse_scalar_string_with_escape_tab() -> Result<()> {
    let document = parse(r#""hello\tworld""#)?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("hello\\tworld"));

    Ok(())
}

#[test]
fn parse_scalar_string_with_escape_backslash() -> Result<()> {
    let document = parse(r#""hello\\world""#)?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("hello\\\\world"));

    Ok(())
}

#[test]
fn parse_scalar_string_with_escape_quote() -> Result<()> {
    let document = parse(r#""hello\"world""#)?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("hello\\\"world"));

    Ok(())
}

#[test]
fn parse_scalar_string_with_escape_carriage_return() -> Result<()> {
    let document = parse(r#""hello\rworld""#)?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("hello\\rworld"));

    Ok(())
}

#[test]
fn parse_scalar_string_with_escape_null() -> Result<()> {
    let document = parse(r#""hello\0world""#)?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("hello\\0world"));

    Ok(())
}

#[test]
fn parse_scalar_string_with_unicode_escape_short() -> Result<()> {
    let document = parse(r#""\u0041""#)?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("\\u0041"));

    Ok(())
}

#[test]
fn parse_scalar_string_with_unicode_escape_long() -> Result<()> {
    let document = parse(r#""\U00000041""#)?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("\\U00000041"));

    Ok(())
}

#[test]
fn parse_scalar_string_with_unicode_emoji() -> Result<()> {
    let document = parse(r#""\U0001F600""#)?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("\\U0001F600"));

    Ok(())
}

#[test]
fn parse_scalar_string_single_quote_escape() -> Result<()> {
    let document = parse("'it''s'")?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("it''s"));

    Ok(())
}

#[test]
fn parse_scalar_string_unquoted() -> Result<()> {
    let document = parse("hello world")?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("hello world"));

    Ok(())
}

#[test]
fn parse_scalar_string_unquoted_with_colon() -> Result<()> {
    let document = parse("http://example.com")?.unwrap();
    assert_eq!(
        document.root.value,
        ScalarType::String("http://example.com")
    );

    Ok(())
}

#[test]
fn parse_scalar_string_whitespace_double_quoted() -> Result<()> {
    let document = parse(r#""   ""#)?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("   "));

    Ok(())
}

#[test]
fn parse_scalar_string_whitespace_single_quoted() -> Result<()> {
    let document = parse("'   '")?.unwrap();
    assert_eq!(document.root.value, ScalarType::String("   "));

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
            value: ScalarType::String("hello, world!"),
            comment: None
        }
    );
    assert_eq!(
        items[4],
        Scalar {
            value: ScalarType::String("good afternoon, good evening, and good night"),
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
                    value: ScalarType::String("truman"),
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
