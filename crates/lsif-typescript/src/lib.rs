pub mod emit;

#[allow(unused)]
mod generated;

pub use emit::index_typescript_source;

/// Canonical entry point for the CLI — wraps [`index_typescript_source`].
pub fn index_file<W: std::io::Write>(
    source: &str,
    uri: &str,
    package_name: Option<&str>,
    builder: &mut lsp_max_lsif::lsif_builder::LsifBuilder<W>,
) -> std::io::Result<()> {
    index_typescript_source(source, uri, package_name, builder)
}
