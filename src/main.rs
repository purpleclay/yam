mod markdown;
mod parser;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::{
    fs,
    io::{self, Read},
};

use crate::{markdown::render_markdown, parser::parse};

pub mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[derive(Parser, Debug)]
#[command(author, about, long_about = None, disable_version_flag = true)]
struct Args {
    #[arg(value_name = "FILE")]
    file: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Print build time version information
    Version {
        /// Only print the version number
        #[arg(short, long)]
        short: bool,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Commands::Version { short }) => {
            if short {
                print_version_short();
            } else {
                print_version_info();
            }
            return Ok(());
        }
        None => {}
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

fn print_version_short() {
    println!("{}", built_info::PKG_VERSION);
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
