use clap_noun_verb_lsp as _;

fn main() -> clap_noun_verb::Result<()> {
    let registry_mutex = clap_noun_verb::cli::registry::CommandRegistry::get();
    clap_noun_verb_lsp::receipt::register_explicit();
    let registry = registry_mutex.lock().map_err(|e| {
        clap_noun_verb::error::NounVerbError::execution_error(format!(
            "Failed to lock registry: {}",
            e
        ))
    })?;
    let args: Vec<String> = std::env::args().collect();
    registry.run(args)
}
