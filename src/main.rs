use tree_sitter::{Node, Parser};

const YAML_CONTENT: &str = r#"
name: hello

# We are now commenting on the image section
image:
  registry: docker.io # registry comment
  repository: mongo
  tag: latest
  pullSecrets: []
  pullPolicy: IfNotPresent
"#;

fn main() {
    let mut parser = Parser::new();
    let language = tree_sitter_yaml::LANGUAGE.into();
    parser
        .set_language(&language)
        .expect("Error loading YAML grammar");

    let tree = parser
        .parse(YAML_CONTENT, None)
        .expect("Error parsing YAML");
    let root_node = tree.root_node();

    println!("=== YAML Configuration with Comments ===");
    println!("PATH\tVALUE\tTYPE\tCOMMENT");
    println!("----\t-----\t----\t-------");
    traverse_yaml(&root_node, YAML_CONTENT, "");

    println!("\n=== Tree Structure (for debugging) ===");
    print_tree_structure(&root_node, YAML_CONTENT, 0);

    println!("\n=== Node Types Summary ===");
    let node_types = collect_node_types(&root_node);
    for (node_type, count) in node_types {
        println!("{}: {} occurrences", node_type, count);
    }
}

fn print_tree_structure(node: &Node, source: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    let node_text = get_node_text(node, source);
    let truncated_text = if node_text.len() > 50 {
        format!("{}...", &node_text[..47])
    } else {
        node_text
    };

    println!(
        "{}{}[{}] '{}' ({}:{}-{}:{})",
        indent,
        node.kind(),
        if node.is_named() {
            "named"
        } else {
            "anonymous"
        },
        truncated_text.replace('\n', "\\n"),
        node.start_position().row + 1,
        node.start_position().column + 1,
        node.end_position().row + 1,
        node.end_position().column + 1
    );

    // Print field names for named children
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if let Some(field_name) = node.field_name_for_child(i as u32) {
                let child_indent = "  ".repeat(depth + 1);
                println!("{}field '{}' ->", child_indent, field_name);
            }
            print_tree_structure(&child, source, depth + 1);
        }
    }
}

fn collect_node_types(node: &Node) -> std::collections::BTreeMap<String, usize> {
    let mut node_types = std::collections::BTreeMap::new();
    collect_node_types_recursive(node, &mut node_types);
    node_types
}

fn collect_node_types_recursive(
    node: &Node,
    node_types: &mut std::collections::BTreeMap<String, usize>,
) {
    let kind = if node.is_named() {
        format!("{} [named]", node.kind())
    } else {
        format!("{} [anonymous]", node.kind())
    };

    *node_types.entry(kind).or_insert(0) += 1;

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            collect_node_types_recursive(&child, node_types);
        }
    }
}

fn traverse_yaml(node: &Node, source: &str, path: &str) {
    match node.kind() {
        "block_mapping" => {
            // Process all children of the mapping to handle pairs and comments together
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();

            for (i, child) in children.iter().enumerate() {
                if child.kind() == "block_mapping_pair" {
                    process_mapping_pair(child, source, path, &children, i);
                }
            }
        }
        "block_mapping_pair" => {
            // Handle top-level pairs that aren't in a mapping context
            process_mapping_pair(node, source, path, &[*node], 0);
        }
        _ => {
            // Continue traversing children
            for child in node.children(&mut node.walk()) {
                traverse_yaml(&child, source, path);
            }
        }
    }
}

fn process_mapping_pair(
    pair_node: &Node,
    source: &str,
    path: &str,
    siblings: &[Node],
    current_index: usize,
) {
    let key_node = pair_node.child_by_field_name("key");
    let value_node = pair_node.child_by_field_name("value");

    if let Some(key) = key_node {
        let key_text = get_node_text(&key, source);
        let current_path = if path.is_empty() {
            key_text.clone()
        } else {
            format!("{}.{}", path, key_text)
        };

        if let Some(value) = value_node {
            match value.kind() {
                "block_node" => {
                    // For nested structures, recurse into the block
                    for child in value.children(&mut value.walk()) {
                        traverse_yaml(&child, source, &current_path);
                    }
                }
                _ => {
                    let value_text = get_node_text(&value, source);
                    let value_type = get_scalar_type(&value);
                    let comment = find_associated_comment(siblings, current_index, source);

                    println!(
                        "{}\t{}\t{}\t{}",
                        current_path,
                        value_text,
                        value_type,
                        comment.unwrap_or_default()
                    );
                }
            }
        }
    }
}

fn find_associated_comment(siblings: &[Node], pair_index: usize, source: &str) -> Option<String> {
    // First, look for inline comment (next sibling)
    if pair_index + 1 < siblings.len() {
        let next_sibling = &siblings[pair_index + 1];
        if next_sibling.kind() == "comment" {
            return Some(extract_comment_text(next_sibling, source));
        }
    }

    // Then, look for header comment (previous sibling)
    if pair_index > 0 {
        let prev_sibling = &siblings[pair_index - 1];
        if prev_sibling.kind() == "comment" {
            return Some(extract_comment_text(prev_sibling, source));
        }
    }

    None
}

fn extract_comment_text(comment_node: &Node, source: &str) -> String {
    // Comment nodes include the '#' character, so we need to extract just the text
    let full_text = if let Ok(text) = comment_node.utf8_text(source.as_bytes()) {
        text
    } else {
        &source[comment_node.byte_range()]
    };
    if let Some(hash_pos) = full_text.find('#') {
        full_text[hash_pos + 1..].trim().to_string()
    } else {
        full_text.trim().to_string()
    }
}

fn get_node_text(node: &Node, source: &str) -> String {
    // Use the node's utf8_text method for proper text extraction
    if let Ok(text) = node.utf8_text(source.as_bytes()) {
        text.trim_matches('"').to_string()
    } else {
        // Fallback to byte range method
        source[node.byte_range()]
            .to_string()
            .trim_matches('"')
            .to_string()
    }
}

fn get_scalar_type(node: &Node) -> String {
    // Walk down the tree to find the actual scalar type
    let mut current = *node;
    loop {
        match current.kind() {
            "string_scalar" => return "string".to_string(),
            "integer_scalar" => return "integer".to_string(),
            "boolean_scalar" => return "boolean".to_string(),
            "float_scalar" => return "float".to_string(),
            "double_quote_scalar" => return "string".to_string(),
            "single_quote_scalar" => return "string".to_string(),
            "literal_scalar" => return "string".to_string(),
            "folded_scalar" => return "string".to_string(),
            "null_scalar" => return "null".to_string(),
            "plain_scalar" => {
                // For plain scalars, we need to look at their children to determine type
                if let Some(child) = current.child(0) {
                    current = child;
                    continue;
                }
                return "string".to_string();
            }
            "flow_node" => {
                // Flow nodes wrap the actual scalar, look at their children
                if let Some(child) = current.child(0) {
                    current = child;
                    continue;
                }
                return "unknown".to_string();
            }
            "block_node" => return "object".to_string(),
            "flow_sequence" => return "array".to_string(),
            "block_sequence" => return "array".to_string(),
            _ => return "unknown".to_string(),
        }
    }
}
