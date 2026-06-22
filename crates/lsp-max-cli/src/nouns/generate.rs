use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use heck::ToKebabCase;
use lsp_max_gen::{
    generators::{
        capability::CapabilityGenerator, handler::HandlerGenerator, protocol::ProtocolGenerator,
        scaffold::ScaffoldGenerator, server::ServerGenerator, sync::SyncGenerator,
    },
    GeneratorContext, GeneratorEngine, GeneratorRegistry, TestMatrixGenerator,
};
use serde::Serialize;

// ==========================================
// 1. Domain Tier
// ==========================================

#[derive(Debug, Clone, Serialize)]
pub struct GenerateOutput {
    pub status: String,
    pub generator: String,
    pub name: String,
    pub files_written: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GeneratorEntry {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerateListOutput {
    pub generators: Vec<GeneratorEntry>,
}

// ==========================================
// 2. Service Tier
// ==========================================

pub struct GenerateService;

impl GenerateService {
    pub fn new() -> Self {
        Self
    }

    fn all_generators() -> Vec<Box<dyn lsp_max_gen::Generator>> {
        vec![
            Box::new(CapabilityGenerator),
            Box::new(HandlerGenerator),
            Box::new(ProtocolGenerator),
            Box::new(ScaffoldGenerator),
            Box::new(ServerGenerator),
            Box::new(SyncGenerator),
            Box::new(TestMatrixGenerator),
        ]
    }

    pub fn run_generator(
        &self,
        kind: &str,
        name: &str,
        output_dir: &str,
    ) -> Result<GenerateOutput> {
        let engine = GeneratorEngine::new(Self::all_generators());
        let ctx = GeneratorContext::new(name, std::path::PathBuf::from(output_dir));
        match engine.run(kind, &ctx) {
            Ok(written) => Ok(GenerateOutput {
                status: "CANDIDATE".to_string(),
                files_written: written
                    .iter()
                    .map(|f| f.path.display().to_string())
                    .collect(),
                generator: kind.to_string(),
                name: name.to_string(),
            }),
            Err(e) => Err(clap_noun_verb::error::NounVerbError::execution_error(
                e.to_string(),
            )),
        }
    }

    pub fn list_generators(&self) -> GenerateListOutput {
        let mut registry = GeneratorRegistry::default();
        for g in Self::all_generators() {
            registry.register(g);
        }
        let generators = registry
            .list()
            .iter()
            .map(|(name, desc)| GeneratorEntry {
                name: name.to_string(),
                description: desc.to_string(),
            })
            .collect();
        GenerateListOutput { generators }
    }
}

// ==========================================
// 3. CLI Tier
// ==========================================

#[verb("server")]
pub fn server(name: String, output_dir: Option<String>) -> Result<GenerateOutput> {
    let service = GenerateService::new();
    let dir = output_dir.unwrap_or_else(|| format!("./{}", name.to_kebab_case()));
    service.run_generator("server", &name, &dir)
}

#[verb("scaffold")]
pub fn scaffold(name: String, output_dir: Option<String>) -> Result<GenerateOutput> {
    let service = GenerateService::new();
    let dir = output_dir.unwrap_or_else(|| format!("./{}", name.to_kebab_case()));
    service.run_generator("scaffold", &name, &dir)
}

#[verb("handler")]
pub fn handler(method: String, output_dir: Option<String>) -> Result<GenerateOutput> {
    let service = GenerateService::new();
    let dir = output_dir.unwrap_or_else(|| "./src".to_string());
    service.run_generator("handler", &method, &dir)
}

#[verb("capability")]
pub fn capability(name: String, output_dir: Option<String>) -> Result<GenerateOutput> {
    let service = GenerateService::new();
    let dir = output_dir.unwrap_or_else(|| "./src".to_string());
    service.run_generator("capability", &name, &dir)
}

#[verb("protocol")]
pub fn protocol(name: String, output_dir: Option<String>) -> Result<GenerateOutput> {
    let service = GenerateService::new();
    let dir = output_dir.unwrap_or_else(|| "./src".to_string());
    service.run_generator("protocol", &name, &dir)
}

#[verb("list")]
pub fn list() -> Result<GenerateListOutput> {
    let service = GenerateService::new();
    Ok(service.list_generators())
}
