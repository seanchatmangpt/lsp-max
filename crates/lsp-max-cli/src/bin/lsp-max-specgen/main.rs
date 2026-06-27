mod emit;
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

    /// Output Rust file path (required when --emit-spec-graph is not set).
    #[arg(long)]
    output: Option<PathBuf>,

    /// Include proposed protocol entries.
    #[arg(long, default_value_t = false)]
    include_proposed: bool,

    /// Emit spec-graph JSON artifacts to this directory instead of generating Rust types.
    #[arg(long)]
    emit_spec_graph: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let raw = fs::read_to_string(&args.input)
        .with_context(|| format!("failed to read {}", args.input.display()))?;
    let model: MetaModel = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {} as LSP meta-model", args.input.display()))?;

    if let Some(dir) = &args.emit_spec_graph {
        fs::create_dir_all(dir)?;

        let spec_graph = emit::emit_spec_graph(&model)?;
        let sg_path = dir.join("lsp318_spec_graph.json");
        fs::write(&sg_path, &spec_graph)
            .with_context(|| format!("failed to write {}", sg_path.display()))?;
        eprintln!("wrote {}", sg_path.display());

        let inventory = emit::emit_message_inventory(&model)?;
        let inv_path = dir.join("lsp318_message_inventory.json");
        fs::write(&inv_path, &inventory)
            .with_context(|| format!("failed to write {}", inv_path.display()))?;
        eprintln!("wrote {}", inv_path.display());

        eprintln!(
            "spec-graph artifacts emitted for LSP {}",
            model.meta_data.version
        );
        return Ok(());
    }

    let output = args
        .output
        .as_ref()
        .context("--output is required when --emit-spec-graph is not set")?;

    let rendered = Renderer::new(args.include_proposed).render(&model)?;

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, rendered).with_context(|| format!("failed to write {}", output.display()))?;

    eprintln!("generated Rust types for LSP {}", model.meta_data.version);
    Ok(())
}
