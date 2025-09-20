mod parser;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::fs;

use crate::parser::parse;

#[derive(Parser, Debug)]
#[command(author)]
struct Args {
    #[command(subcommand)]
    commands: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate markdown from a given YAML file
    Generate {
        /// Path to the YAML file
        #[arg(short, long)]
        file: String,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    match args.commands {
        Some(Commands::Generate { file }) => {
            let content = fs::read_to_string(&file)
                .with_context(|| format!("failed to read file: {}", file))?;
            let document = parse(&content)?;
            println!("{}", document);
            Ok(())
        }
        None => Ok(()),
    }
}
