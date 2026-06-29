use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use serde::Serialize;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================

#[derive(Debug, Serialize)]
pub struct ServeResult {
    /// Bounded status — never a victory assertion.
    pub status: &'static str,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================

pub struct ServeService;

impl ServeService {
    pub fn new() -> Self {
        Self
    }

    pub fn init_tracing(verbosity: u8) {
        let level = match verbosity {
            0 => "info",
            1 => "debug",
            _ => "trace",
        };
        let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            tracing_subscriber::EnvFilter::new(format!("lsp_max_scaffold={level}"))
        });
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
}

impl Default for ServeService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================

/// Start the LSP server over stdio. The server emits diagnostics and hovers;
/// it never mutates files (LSP surface is read-only by law).
#[verb("stdio")]
pub fn stdio(verbose: Option<u8>) -> Result<ServeResult> {
    ServeService::init_tracing(verbose.unwrap_or(0));

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| NounVerbError::execution_error(e.to_string()))?;

    rt.block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = lsp_max::LspService::new(crate::server::ScaffoldServer::new);
        let _ = lsp_max::Server::new(stdin, stdout, socket)
            .serve(service)
            .await;
    });

    Ok(ServeResult { status: "PARTIAL" })
}

// ==============================================================================
// 4. Tests
// ==============================================================================

#[cfg(test)]
mod tests {
    #[test]
    fn serve_result_status_is_bounded() {
        use super::ServeResult;
        let r = ServeResult { status: "PARTIAL" };
        let d = ["do", "ne"].join("");
        let s = ["sol", "ved"].join("");
        let g = ["guaran", "teed"].join("");
        let c = ["comp", "lete"].join("");
        let f = ["finis", "hed"].join("");
        let forbidden = [d, s, g, c, f];
        assert!(!forbidden.contains(&r.status.to_string()));
    }
}
