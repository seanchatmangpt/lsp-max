use lsp_max_compositor::{CompositorConfig, ExtensionRouter, MergeContext};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let config = CompositorConfig::load();
    let router = match &config {
        Some(cfg) => ExtensionRouter::from_config(cfg),
        None => {
            eprintln!("lsp-max-compositor: no lsp-max.toml found, starting with empty router");
            ExtensionRouter::default()
        }
    };
    // Print startup summary
    let server_count = config.as_ref().map(|c| c.server.len()).unwrap_or(0);
    eprintln!("lsp-max-compositor: {} server(s) registered", server_count);
    if let Some(cfg) = &config {
        for s in &cfg.server {
            eprintln!(
                "  {} ({}): primary={:?} secondary={:?}",
                s.id, s.priority, s.primary_extensions, s.secondary_extensions
            );
        }
    }
    let merge_ctx = match &config {
        Some(cfg) => MergeContext::from_config(cfg),
        None => {
            tracing::warn!(
                "lsp-max.toml not found in workspace tree — using static ANDON prefixes \
                 [WASM4PM-, ANTI-LLM-, GGEN-]; C_D = static default"
            );
            MergeContext::new(vec!["WASM4PM-".into(), "ANTI-LLM-".into(), "GGEN-".into()])
        }
    };
    eprintln!(
        "merge context: {} andon prefix(es)",
        merge_ctx.andon_prefixes_count()
    );
    let config_arc = Arc::new(config.unwrap_or_else(|| CompositorConfig { server: vec![] }));
    lsp_max_compositor::server::run_stdio(router, merge_ctx, config_arc).await;
}
