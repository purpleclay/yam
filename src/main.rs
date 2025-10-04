mod markdown;
mod parser;

use anyhow::{Context, Result};
use clap::Parser;
use std::{
    fs,
    io::{self, Read},
};

use crate::{markdown::render_markdown, parser::parse};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

const LONG_ABOUT: &str = r#"A context-aware YAML to markdown document generator that parses YAML files
and renders them as markdown tables."#;

#[derive(Parser, Debug)]
#[command(
    author,
    about,
    long_about = LONG_ABOUT,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
struct Args {
    /// Path to a YAML file to convert to markdown
    ///
    /// Use '-' to read from stdin (e.g., cat file.yaml | yam -)
    #[arg(value_name = "FILE")]
    file: Option<String>,

    /// Print build time version information
    #[arg(short = 'V', long)]
    version: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.version {
        print_version_info();
        return Ok(());
    }

    let file = args.file.context("FILE argument is required")?;

    let mut content = String::new();
    if file == "-" {
        io::stdin()
            .read_to_string(&mut content)
            .context("Failed to read from stdin")?;
    } else {
        content =
            fs::read_to_string(&file).with_context(|| format!("failed to read file: {}", file))?;
    }

    let document = parse(&content)?;
    if let Some(doc) = document {
        let markdown = render_markdown(&doc)?;
        println!("{}", markdown);
    }
    Ok(())
}

fn print_version_info() {
    println!("version:    {}", built_info::PKG_VERSION);
    println!("rustc:      {}", built_info::RUSTC_VERSION);
    println!("target:     {}", built_info::TARGET);

    if let Some(git_ref) = built_info::GIT_HEAD_REF {
        println!(
            "git_branch: {}",
            git_ref.strip_prefix("refs/heads/").unwrap_or(git_ref)
        );
    }

    if let Some(commit_hash) = built_info::GIT_COMMIT_HASH {
        println!("git_commit: {commit_hash}");
    }
    println!("build_date: {}", built_info::BUILT_TIME_UTC);
}
