pub mod emit;

#[allow(unused)]
mod generated;

pub use emit::index_rust_source;

/// Canonical entry point for the CLI — wraps [`index_rust_source`].
pub fn index_file<W: std::io::Write>(
    source: &str,
    uri: &str,
    builder: &mut lsp_max_lsif::lsif_builder::LsifBuilder<W>,
) -> std::io::Result<()> {
    index_rust_source(source, uri, builder)
}
