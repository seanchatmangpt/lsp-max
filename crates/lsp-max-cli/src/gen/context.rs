use heck::{ToKebabCase, ToSnakeCase};
use std::path::PathBuf;

/// Carries naming and output-location data shared across all generator invocations.
pub struct GeneratorContext {
    /// PascalCase form of the generator target name (e.g. `"HoverHandler"`, `"MyLsp"`).
    pub name: String,
    /// snake_case form (e.g. `"hover_handler"`, `"my_lsp"`).
    pub snake_name: String,
    /// kebab-case form (e.g. `"hover-handler"`, `"my-lsp"`).
    pub kebab_name: String,
    /// Root directory for emitted files.
    pub output_dir: PathBuf,
    /// Arbitrary extra arguments forwarded from the CLI caller.
    pub extra: serde_json::Value,
}

impl GeneratorContext {
    /// Construct a context from a PascalCase `name` and an `output_dir`.
    /// `snake_name` and `kebab_name` are derived automatically via `heck`.
    pub fn new(name: &str, output_dir: PathBuf) -> Self {
        Self {
            snake_name: name.to_snake_case(),
            kebab_name: name.to_kebab_case(),
            name: name.to_owned(),
            output_dir,
            extra: serde_json::Value::Null,
        }
    }
}
