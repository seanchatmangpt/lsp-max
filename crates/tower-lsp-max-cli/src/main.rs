pub mod nouns;

fn main() -> clap_noun_verb::Result<()> {
    if let Ok(port_str) = std::env::var("TOWER_LSP_MAX_RUN_SERVER_DAEMON") {
        if let Ok(port) = port_str.parse::<u16>() {
            let rt = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("Failed to start tokio runtime: {}", e);
                    std::process::exit(1);
                }
            };
            rt.block_on(async {
                if let Err(e) = nouns::server::run_server_daemon(port).await {
                    eprintln!("Server daemon error: {}", e);
                    std::process::exit(1);
                }
            });
            std::process::exit(0);
        }
    }

    if let Ok(url) = std::env::var("TOWER_LSP_MAX_RUN_CLIENT_DAEMON") {
        let rt = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to start tokio runtime: {}", e);
                std::process::exit(1);
            }
        };
        rt.block_on(async {
            if let Err(e) = nouns::client::run_client_daemon(&url).await {
                eprintln!("Client daemon error: {}", e);
                std::process::exit(1);
            }
        });
        std::process::exit(0);
    }

    clap_noun_verb::run()
}
