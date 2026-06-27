use lsp_max_compositor::{
    fanout::{dispatch_strategy, servers_for_uri, DispatchStrategy},
    CompositorConfig, ExtensionRouter,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

// ── JSON-RPC 2.0 wire types ───────────────────────────────────────────────────

#[derive(Deserialize)]
struct Request {
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize)]
struct Response {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

impl Response {
    fn ok(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }
    fn err(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
            }),
        }
    }
}

// ── Tool definitions for tools/list ──────────────────────────────────────────

fn tool_list() -> Value {
    json!({
        "tools": [
            {
                "name": "lsp_discover",
                "description": "Scan the workspace for installed LSP servers and return a server-to-extensions map derived from CompositorConfig::load_with_auto().",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            {
                "name": "lsp_route",
                "description": "Return the ordered list of LSP servers that handle a given URI for a given LSP method, using the live ExtensionRouter.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "uri":    { "type": "string", "description": "Document URI (file://...)" },
                        "method": { "type": "string", "description": "LSP method name (e.g. textDocument/hover)" }
                    },
                    "required": ["uri", "method"]
                }
            },
            {
                "name": "lsp_health",
                "description": "Check whether a child server binary is reachable. Returns {id, status} where status is ADMITTED, OPEN, or UNKNOWN.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "server_id": { "type": "string", "description": "Server id from CompositorConfig (e.g. wasm4pm-lsp)" }
                    },
                    "required": ["server_id"]
                }
            },
            {
                "name": "lsp_reload_config",
                "description": "Re-read lsp-max.toml and .claude/lsp-max-auto.toml and return the merged routing table. Use after editing lsp-max.toml without restarting the compositor.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            {
                "name": "lsp_violations",
                "description": "Return the full Declare constraint violation list from the last flush cycle. Use after lsp_route shows law_status != ADMITTED to identify exactly which constraints were violated.",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            },
            {
                "name": "lsp_repair_plan",
                "description": "Given a Declare constraint string (from lsp_violations), return an ordered repair plan with bounded status and clap-noun-verb actuation hints.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "constraint": { "type": "string", "description": "The constraint string from a lsp_violations entry, e.g. 'response(CompositorFlush, CompositorFlushAdmitted)'" }
                    },
                    "required": ["constraint"]
                }
            }
        ]
    })
}

// ── Tool handlers ─────────────────────────────────────────────────────────────

fn handle_lsp_discover() -> Value {
    match CompositorConfig::load_with_auto() {
        None => {
            json!({ "status": "OPEN", "servers": [], "note": "no lsp-max.toml or .claude/lsp-max-auto.toml found" })
        }
        Some(cfg) => {
            let servers: Vec<Value> = cfg
                .server
                .iter()
                .map(|s| {
                    json!({
                        "id": s.id,
                        "priority": s.priority,
                        "primary_extensions": s.primary_extensions,
                        "secondary_extensions": s.secondary_extensions,
                        "command": s.command
                    })
                })
                .collect();
            json!({ "status": "ADMITTED", "servers": servers })
        }
    }
}

fn handle_lsp_route(uri: &str, method: &str) -> Value {
    let cfg = match CompositorConfig::load_with_auto() {
        None => return json!({ "servers": [], "strategy": "UNKNOWN", "note": "no config found" }),
        Some(c) => c,
    };
    let router = build_router(&cfg);
    let servers = servers_for_uri(&router, uri);
    let strategy = dispatch_strategy(method);
    let strategy_str = match strategy {
        DispatchStrategy::FanAll => "FanAll",
        DispatchStrategy::FirstSuccess => "FirstSuccess",
        DispatchStrategy::Notify => "Notify",
        DispatchStrategy::PrimaryOnly => "PrimaryOnly",
    };
    let results: Vec<Value> = servers
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "tier": format!("{:?}", s.tier),
                "extensions": s.extensions
            })
        })
        .collect();
    let fitness = read_fitness_status();
    let law_status = if servers.is_empty() {
        json!("UNROUTABLE")
    } else {
        fitness["law_status"].clone()
    };
    json!({
        "uri": uri,
        "method": method,
        "strategy": strategy_str,
        "servers": results,
        "law_status": law_status,
        "fitness_snapshot": fitness
    })
}

fn handle_lsp_health(server_id: &str) -> Value {
    let cfg = match CompositorConfig::load_with_auto() {
        None => return json!({ "id": server_id, "status": "UNKNOWN", "note": "no config found" }),
        Some(c) => c,
    };
    let entry = cfg.server.iter().find(|s| s.id == server_id);
    match entry {
        None => json!({ "id": server_id, "status": "OPEN", "note": "server not in config" }),
        Some(s) => {
            let cmd = match &s.command {
                None => {
                    return json!({ "id": server_id, "status": "UNKNOWN", "note": "no command configured; manual spawn" })
                }
                Some(c) => c,
            };
            let reachable = std::process::Command::new("which")
                .arg(cmd)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
                || std::path::Path::new(cmd).exists();
            if reachable {
                json!({ "id": server_id, "status": "ADMITTED", "command": cmd })
            } else {
                json!({ "id": server_id, "status": "OPEN", "command": cmd, "note": "binary not found in PATH" })
            }
        }
    }
}

fn handle_lsp_reload_config() -> Value {
    match CompositorConfig::load_with_auto() {
        None => json!({ "status": "OPEN", "note": "no config found after reload" }),
        Some(cfg) => {
            let servers: Vec<Value> = cfg
                .server
                .iter()
                .map(|s| {
                    json!({
                        "id": s.id,
                        "priority": s.priority,
                        "primary_extensions": s.primary_extensions
                    })
                })
                .collect();
            json!({ "status": "CANDIDATE", "server_count": servers.len(), "servers": servers })
        }
    }
}

fn handle_lsp_violations() -> Value {
    let fitness = read_fitness_status();
    let violations = fitness
        .get("violations")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let count = violations.as_array().map(|a| a.len()).unwrap_or(0);
    json!({
        "law_status": fitness["law_status"],
        "declare_violations": count,
        "violations": violations
    })
}

fn constraint_to_diag_id(constraint: &str) -> &'static str {
    if constraint.contains("CompositorFlush") || constraint.contains("AndonCodePresent") {
        "WASM4PM-ANDON"
    } else if constraint.contains("GGEN") {
        "GGEN-GATE"
    } else {
        "UNKNOWN"
    }
}

fn handle_lsp_repair_plan(constraint: &str) -> Value {
    use lsp_max_protocol::repair::repair_plan_for;
    let diag_id = constraint_to_diag_id(constraint);
    let plan = repair_plan_for(diag_id);
    json!({
        "constraint": constraint,
        "diagnostic_id": diag_id,
        "status": plan.status,
        "summary": plan.summary,
        "steps": plan.steps.iter().map(|s| json!({
            "order": s.order,
            "action": s.action,
            "rationale": s.rationale,
            "verb": s.verb
        })).collect::<Vec<_>>()
    })
}

fn read_fitness_status() -> Value {
    let workspace = std::env::current_dir().unwrap_or_default();
    let path = workspace.join(".claude/lsp-max-fitness.json");
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| json!({"law_status": "UNKNOWN"}))
}

// ── Router builder ────────────────────────────────────────────────────────────

fn build_router(cfg: &CompositorConfig) -> ExtensionRouter {
    use lsp_max_compositor::{ChildServer, ChildTier};
    let router = ExtensionRouter::new();
    for entry in &cfg.server {
        let tier = match entry.priority.as_str() {
            "diagnostics-only" => ChildTier::DiagnosticsOnly,
            "secondary" => ChildTier::Secondary,
            _ => ChildTier::Primary,
        };
        for ext in &entry.primary_extensions {
            router.register(
                ext,
                ChildServer {
                    id: entry.id.clone(),
                    tier: tier.clone(),
                    extensions: entry.primary_extensions.clone(),
                },
            );
        }
        for ext in &entry.secondary_extensions {
            router.register(
                ext,
                ChildServer {
                    id: entry.id.clone(),
                    tier: ChildTier::Secondary,
                    extensions: entry.secondary_extensions.clone(),
                },
            );
        }
    }
    router
}

// ── Dispatch ──────────────────────────────────────────────────────────────────

fn dispatch(req: Request) -> Response {
    let id = req.id.clone();
    match req.method.as_str() {
        "initialize" => Response::ok(
            id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "lsp-max-mcp", "version": env!("CARGO_PKG_VERSION") }
            }),
        ),
        "notifications/initialized" => return Response::ok(None, json!({})),
        "tools/list" => Response::ok(id, tool_list()),
        "tools/call" => {
            let params = req.params.unwrap_or_default();
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or_default();
            let result = match name {
                "lsp_discover" => handle_lsp_discover(),
                "lsp_route" => {
                    let uri = args.get("uri").and_then(Value::as_str).unwrap_or("");
                    let method = args.get("method").and_then(Value::as_str).unwrap_or("");
                    handle_lsp_route(uri, method)
                }
                "lsp_health" => {
                    let sid = args.get("server_id").and_then(Value::as_str).unwrap_or("");
                    handle_lsp_health(sid)
                }
                "lsp_reload_config" => handle_lsp_reload_config(),
                "lsp_violations" => handle_lsp_violations(),
                "lsp_repair_plan" => {
                    let constraint = args.get("constraint").and_then(Value::as_str).unwrap_or("");
                    handle_lsp_repair_plan(constraint)
                }
                _ => return Response::err(id, -32601, format!("unknown tool: {name}")),
            };
            Response::ok(
                id,
                json!({
                    "content": [{ "type": "text", "text": result.to_string() }]
                }),
            )
        }
        "ping" => Response::ok(id, json!({})),
        _ => Response::err(id, -32601, format!("method not found: {}", req.method)),
    }
}

// ── stdio subcommand: reload-config (called from FileChanged hook) ────────────

fn reload_config_subcommand() {
    match handle_lsp_reload_config() {
        v => {
            println!("{}", v);
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("reload-config") {
        reload_config_subcommand();
        return;
    }

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) if l.trim().is_empty() => continue,
            Ok(l) => l,
            Err(_) => break,
        };
        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let err = Response::err(None, -32700, format!("parse error: {e}"));
                let _ = writeln!(out, "{}", serde_json::to_string(&err).unwrap());
                let _ = out.flush();
                continue;
            }
        };
        let is_notification = req.id.is_none();
        let resp = dispatch(req);
        if !is_notification {
            let _ = writeln!(out, "{}", serde_json::to_string(&resp).unwrap());
            let _ = out.flush();
        }
    }
}
