use std::path::PathBuf;

use tera::{Context as TeraContext, Tera};

use crate::gen::{GenManifest, GeneratedFile, Generator, GeneratorContext, GeneratorError, WriteMode};

/// Read `gen.toml` from `ctx.output_dir` and emit a `SYNC_PLAN.md` describing
/// what each `[[generate]]` entry would produce.
///
/// `SyncGenerator` does NOT invoke other generators — that would create circular
/// dependencies in the registry. The CLI layer is responsible for executing the
/// plan entries. The emitted `SYNC_PLAN.md` is the authoritative record of
/// pending generation work.
///
/// Status: CANDIDATE — receipt chain OPEN.
pub struct SyncGenerator;

const PLAN_TEMPLATE: &str = r#"# gen.toml Sync Plan — OPEN

Project: {{ project_name }}

## Pending generations

{% for entry in entries %}- kind={{ entry.kind }} name={{ entry.name }} → {{ entry.output_dir }}
{% endfor %}
Status: OPEN — run individual generators to materialize
"#;

/// Serialisable view of a `GenerateEntry` for the Tera context.
#[derive(serde::Serialize)]
struct EntryView {
    kind: String,
    name: String,
    output_dir: String,
}

impl Generator for SyncGenerator {
    fn name(&self) -> &str {
        "sync"
    }

    fn description(&self) -> &str {
        "Read gen.toml and emit a SYNC_PLAN.md describing pending generations"
    }

    fn generate(&self, ctx: &GeneratorContext) -> Result<Vec<GeneratedFile>, GeneratorError> {
        let manifest_path = ctx.output_dir.join("gen.toml");
        let manifest = GenManifest::from_path(&manifest_path)?;

        let entries: Vec<EntryView> = manifest
            .generate
            .into_iter()
            .map(|e| EntryView {
                kind: e.kind,
                name: e.name,
                output_dir: e.output_dir,
            })
            .collect();

        let mut tera_ctx = TeraContext::new();
        tera_ctx.insert("project_name", &manifest.project.name);
        tera_ctx.insert("entries", &entries);

        let content = Tera::one_off(PLAN_TEMPLATE, &tera_ctx, false)?;

        Ok(vec![GeneratedFile {
            path: PathBuf::from("SYNC_PLAN.md"),
            content,
            mode: WriteMode::Overwrite,
        }])
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::gen::GeneratorContext;

    fn write_gen_toml(dir: &TempDir, content: &str) {
        fs::write(dir.path().join("gen.toml"), content).unwrap();
    }

    fn ctx(dir: &TempDir) -> GeneratorContext {
        GeneratorContext::new("Sync", dir.path().to_owned())
    }

    #[test]
    fn emits_single_sync_plan_file() {
        let dir = TempDir::new().unwrap();
        write_gen_toml(
            &dir,
            r#"
[project]
name = "my-lsp"

[[generate]]
kind = "server"
name = "MyLsp"
output_dir = "."
"#,
        );
        let gen = SyncGenerator;
        let files = gen.generate(&ctx(&dir)).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, std::path::PathBuf::from("SYNC_PLAN.md"));
    }

    #[test]
    fn plan_contains_project_name_and_entries() {
        let dir = TempDir::new().unwrap();
        write_gen_toml(
            &dir,
            r#"
[project]
name = "example-lsp"

[[generate]]
kind = "handler"
name = "Hover"
output_dir = "src/handlers"

[[generate]]
kind = "protocol"
name = "Snapshot"
output_dir = "src"
"#,
        );
        let gen = SyncGenerator;
        let files = gen.generate(&ctx(&dir)).unwrap();
        let content = &files[0].content;
        assert!(content.contains("example-lsp"));
        assert!(content.contains("kind=handler"));
        assert!(content.contains("name=Hover"));
        assert!(content.contains("kind=protocol"));
        assert!(content.contains("name=Snapshot"));
        assert!(content.contains("OPEN"));
    }

    #[test]
    fn plan_write_mode_is_overwrite() {
        let dir = TempDir::new().unwrap();
        write_gen_toml(
            &dir,
            r#"
[project]
name = "test"
"#,
        );
        let gen = SyncGenerator;
        let files = gen.generate(&ctx(&dir)).unwrap();
        assert!(matches!(files[0].mode, WriteMode::Overwrite));
    }

    #[test]
    fn missing_gen_toml_returns_io_error() {
        let dir = TempDir::new().unwrap();
        let gen = SyncGenerator;
        let result = gen.generate(&ctx(&dir));
        match result {
            Err(GeneratorError::Io { .. }) => {}
            other => panic!("expected Io error, got: {:?}", other.is_ok()),
        }
    }
}
