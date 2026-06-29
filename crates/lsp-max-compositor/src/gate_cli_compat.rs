use serde::Serialize;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GoverningAxes {
    pub refused: Vec<String>,
    pub unknown: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AvailableRepair {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_lawful_step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_command: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AgentContextResult {
    pub admission_allowed: bool,
    pub andon_blocked: bool,
    pub status: String,
    pub since_seq: Option<u64>,
    pub active_andon_codes: Vec<String>,
    pub active_invariant_ids: Vec<String>,
    pub governing_axes: GoverningAxes,
    pub available_repairs: Vec<AvailableRepair>,
    pub required_commands: Vec<String>,
    pub virtual_doc_uris: Vec<String>,
    pub compositor_active: bool,
    pub gate_file: String,
}

#[derive(Debug, Serialize)]
pub struct GateListResult {
    pub andon_blocked: bool,
    pub gate_file: String,
    pub compositor_active: bool,
    pub active_codes: Vec<String>,
    pub active_invariant_ids: Vec<String>,
    pub governing_axes: GoverningAxes,
    pub available_repairs: Vec<AvailableRepair>,
    pub required_commands: Vec<String>,
    pub virtual_doc_uris: Vec<String>,
    pub agent_scope: String,
    pub since_seq: Option<u64>,
}

struct ParsedGate {
    blocked: bool,
    codes: Vec<String>,
    seq: Option<u64>,
    invariant_ids: Vec<String>,
    required_commands: Vec<String>,
    virtual_doc_uris: Vec<String>,
    repairs: Vec<AvailableRepair>,
}

fn extract_string_array(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn parse_gate_json(raw: &[u8]) -> ParsedGate {
    let v: serde_json::Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(_) => {
            return ParsedGate {
                blocked: false,
                codes: vec![],
                seq: None,
                invariant_ids: vec![],
                required_commands: vec![],
                virtual_doc_uris: vec![],
                repairs: vec![],
            }
        }
    };
    let blocked = v["blocked"].as_bool().unwrap_or(false);
    let codes = extract_string_array(&v["codes"]);
    let seq = v["seq"].as_u64();
    let invariant_ids = extract_string_array(&v["active_invariant_ids"]);
    let required_commands = extract_string_array(&v["required_commands"]);
    let virtual_doc_uris = extract_string_array(&v["virtual_doc_uris"]);
    let repairs = v["available_repairs"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|r| serde_json::from_value(r.clone()).ok())
                .collect()
        })
        .unwrap_or_default();
    ParsedGate {
        blocked,
        codes,
        seq,
        invariant_ids,
        required_commands,
        virtual_doc_uris,
        repairs,
    }
}

pub fn check_agent_context() -> AgentContextResult {
    let path = lsp_max::primitives::gate_file_path();
    let compositor_active = path.exists();

    let raw = if compositor_active {
        std::fs::read(&path).unwrap_or_default()
    } else {
        vec![]
    };

    let parsed = if raw.first().copied() == Some(b'{') {
        parse_gate_json(&raw)
    } else {
        let blocked = raw.first().copied().map(|b| b == b'1').unwrap_or(false);
        ParsedGate {
            blocked,
            codes: vec![],
            seq: None,
            invariant_ids: vec![],
            required_commands: vec![],
            virtual_doc_uris: vec![],
            repairs: vec![],
        }
    };

    let status = if parsed.blocked {
        "BLOCKED".to_string()
    } else {
        "ADMITTED".to_string()
    };

    let (refused, unknown) = if parsed.blocked && parsed.codes.is_empty() {
        (vec![], vec!["LSPMAX-AGENT-CONTEXT-MISSING".to_string()])
    } else {
        (parsed.codes.clone(), vec![])
    };

    let mut available_repairs = parsed.repairs;
    if available_repairs.is_empty() {
        available_repairs.push(AvailableRepair {
            action_id: Some("emit-receipt".to_string()),
            verb: Some("diagnostics repair-plan emit".to_string()),
            next_lawful_step: None,
            required_command: None,
        });
    }
    let virtual_doc_uris = if parsed.virtual_doc_uris.is_empty() {
        vec![
            "lsp-max://truth/andon".to_string(),
            "lsp-max://gate/context".to_string(),
        ]
    } else {
        parsed.virtual_doc_uris
    };

    AgentContextResult {
        admission_allowed: !parsed.blocked,
        andon_blocked: parsed.blocked,
        status,
        since_seq: parsed.seq,
        active_andon_codes: parsed.codes,
        active_invariant_ids: parsed.invariant_ids,
        governing_axes: GoverningAxes { refused, unknown },
        available_repairs,
        required_commands: parsed.required_commands,
        virtual_doc_uris,
        compositor_active,
        gate_file: path.display().to_string(),
    }
}

pub fn list() -> GateListResult {
    let ctx = check_agent_context();
    let mut active_codes = ctx.active_andon_codes;
    if active_codes.is_empty() && ctx.andon_blocked {
        active_codes = vec!["WASM4PM-*".to_string(), "GGEN-*".to_string()];
    }
    GateListResult {
        andon_blocked: ctx.andon_blocked,
        gate_file: ctx.gate_file,
        compositor_active: ctx.compositor_active,
        active_codes,
        active_invariant_ids: ctx.active_invariant_ids,
        governing_axes: ctx.governing_axes,
        available_repairs: ctx.available_repairs,
        required_commands: ctx.required_commands,
        virtual_doc_uris: ctx.virtual_doc_uris,
        agent_scope: "global".to_string(),
        since_seq: ctx.since_seq,
    }
}
