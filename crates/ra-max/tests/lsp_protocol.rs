use lsp_max::jsonrpc::Request;
use lsp_max::LspService;
use ra_max::RaMaxServer;
use serde_json::{json, Value};
use tower::{Service, ServiceExt};

async fn request(
    service: &mut lsp_max::LspService<RaMaxServer>,
    message: Request,
) -> Value {
    let response = service
        .ready()
        .await
        .expect("service must be ready")
        .call(message)
        .await
        .expect("service call must execute")
        .expect("request must produce a response");
    response
        .result()
        .cloned()
        .expect("request must return a successful result")
}

#[tokio::test(flavor = "current_thread")]
async fn routes_the_80_20_editor_surface_through_json_rpc() {
    let (mut service, _socket) = LspService::build(RaMaxServer::new)
        .custom_method("max/raSnapshot", RaMaxServer::max_snapshot)
        .finish();

    let initialize = Request::build("initialize")
        .params(json!({
            "capabilities": {
                "general": {"positionEncodings": ["utf-8"]}
            }
        }))
        .id(1)
        .finish();
    let initialized = request(&mut service, initialize).await;
    assert_eq!(initialized["capabilities"]["hoverProvider"], true);
    assert_eq!(
        initialized["capabilities"]["positionEncoding"],
        "utf-8"
    );

    let uri = "file:///workspace/src/main.rs";
    let source = "fn meaning() -> u32 { 42 }\nfn main() { let answer = meaning(); assert_eq!(answer, 42); }\n";
    let did_open = Request::build("textDocument/didOpen")
        .params(json!({
            "textDocument": {
                "uri": uri,
                "languageId": "rust",
                "version": 1,
                "text": source
            }
        }))
        .finish();
    let notification = service
        .ready()
        .await
        .expect("service must accept didOpen")
        .call(did_open)
        .await
        .expect("didOpen must execute");
    assert!(notification.is_none());

    let hover = Request::build("textDocument/hover")
        .params(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 25}
        }))
        .id(2)
        .finish();
    let hover = request(&mut service, hover).await;
    assert!(hover["contents"]["value"]
        .as_str()
        .expect("markdown hover")
        .contains("fn meaning() -> u32"));

    let definition = Request::build("textDocument/definition")
        .params(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 25}
        }))
        .id(3)
        .finish();
    let definition = request(&mut service, definition).await;
    assert_eq!(definition["uri"], uri);
    assert_eq!(definition["range"]["start"]["line"], 0);

    let completion = Request::build("textDocument/completion")
        .params(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 28}
        }))
        .id(4)
        .finish();
    let completion = request(&mut service, completion).await;
    assert!(completion
        .as_array()
        .expect("completion array")
        .iter()
        .any(|item| item["label"] == "meaning"));

    let rename = Request::build("textDocument/rename")
        .params(json!({
            "textDocument": {"uri": uri},
            "position": {"line": 1, "character": 25},
            "newName": "ultimate_meaning"
        }))
        .id(5)
        .finish();
    let rename = request(&mut service, rename).await;
    assert_eq!(
        rename["changes"][uri]
            .as_array()
            .expect("rename edits")
            .len(),
        2
    );

    let snapshot = Request::build("max/raSnapshot").id(6).finish();
    let snapshot = request(&mut service, snapshot).await;
    assert_eq!(snapshot["index"]["verified"], true);
    assert_eq!(snapshot["index"]["lexical_only"], true);
    assert_eq!(snapshot["receipt_chain_valid"], true);
    assert!(snapshot["receipts"]
        .as_array()
        .expect("receipts")
        .iter()
        .any(|receipt| {
            receipt["kind"] == "construction" && receipt["outcome"] == "constructed"
        }));
}
