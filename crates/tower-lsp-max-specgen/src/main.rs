mod metamodel;
mod render;

use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;

use crate::metamodel::MetaModel;
use crate::render::Renderer;

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Path to official LSP metaModel.json.
    #[arg(long)]
    input: PathBuf,

    /// Output Rust file path.
    #[arg(long)]
    output: PathBuf,

    /// Include proposed protocol entries.
    #[arg(long, default_value_t = false)]
    include_proposed: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let raw = fs::read_to_string(&args.input)
        .with_context(|| format!("failed to read {}", args.input.display()))?;
    let model: MetaModel = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {} as LSP meta-model", args.input.display()))?;

    let rendered = Renderer::new(args.include_proposed).render(&model)?;

    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&args.output, rendered)
        .with_context(|| format!("failed to write {}", args.output.display()))?;

    eprintln!("generated Rust types for LSP {}", model.meta_data.version);
    Ok(())
}
