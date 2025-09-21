mod parser;

use anyhow::{Context, Result};
use clap::Parser;
use std::{
    fs,
    io::{self, Read},
};

use crate::parser::parse;

#[derive(Parser, Debug)]
#[command(author)]
struct Args {
    #[arg(value_name = "FILE")]
    file: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut content = String::new();
    if args.file == "-" {
        io::stdin()
            .read_to_string(&mut content)
            .context("Failed to read from stdin")?;
    } else {
        content = fs::read_to_string(&args.file)
            .with_context(|| format!("failed to read file: {}", args.file))?;
    }

    let document = parse(&content)?;
    println!("{}", document);
    Ok(())
}
