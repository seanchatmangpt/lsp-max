use std::future::Future;

use futures::stream::StreamExt;
use lsp_types_max::notification::{
    LogMessage, LogTrace, PublishDiagnostics, ShowMessage, TelemetryEvent,
};
use serde_json::json;

use super::*;

async fn assert_client_message<F, Fut>(f: F, expected: Request)
where
    F: FnOnce(Client) -> Fut,
    Fut: Future,
{
    let state = Arc::new(ServerState::new());
    state.set(State::Initialized);

    let (client, socket) = Client::new(state);
    f(client).await;

    let messages: Vec<_> = socket.collect().await;
    assert_eq!(messages, vec![expected]);
}

#[tokio::test(flavor = "current_thread")]
async fn log_message() {
    let (typ, msg) = (MessageType::LOG, "foo bar".to_owned());
    let expected = Request::from_notification::<LogMessage>(LogMessageParams {
        typ,
        message: msg.clone(),
    });

    assert_client_message(|p| async move { p.log_message(typ, msg).await }, expected).await;
}

#[tokio::test(flavor = "current_thread")]
async fn log_trace() {
    let params = LogTraceParams {
        message: "foo bar".to_owned(),
        verbose: Some("verbose info".to_owned()),
    };
    let expected = Request::from_notification::<LogTrace>(params.clone());

    assert_client_message(|p| async move { p.log_trace(params).await }, expected).await;
}

#[tokio::test(flavor = "current_thread")]
async fn show_message() {
    let (typ, msg) = (MessageType::LOG, "foo bar".to_owned());
    let expected = Request::from_notification::<ShowMessage>(ShowMessageParams {
        typ,
        message: msg.clone(),
    });

    assert_client_message(|p| async move { p.show_message(typ, msg).await }, expected).await;
}

#[tokio::test(flavor = "current_thread")]
async fn telemetry_event() {
    let null = json!(null);
    let expected = Request::from_notification::<TelemetryEvent>(OneOf::Right(Vec::new()));
    assert_client_message(|p| async move { p.telemetry_event(null).await }, expected).await;

    let array = json!([1, 2, 3]);
    let expected = Request::from_notification::<TelemetryEvent>(OneOf::Right(vec![
        1.into(),
        2.into(),
        3.into(),
    ]));
    assert_client_message(|p| async move { p.telemetry_event(array).await }, expected).await;

    let object = json!({});
    let expected =
        Request::from_notification::<TelemetryEvent>(OneOf::Left(serde_json::Map::new()));
    assert_client_message(|p| async move { p.telemetry_event(object).await }, expected).await;

    let other = json!("hello");
    let expected = Request::from_notification::<TelemetryEvent>(OneOf::Right(vec![other.clone()]));
    assert_client_message(|p| async move { p.telemetry_event(other).await }, expected).await;
}

#[tokio::test(flavor = "current_thread")]
async fn publish_diagnostics() {
    let uri: Uri = "file:///path/to/file".parse().unwrap();
    let diagnostics = vec![Diagnostic::new_simple(Default::default(), "example".into())];

    let params = PublishDiagnosticsParams::new(uri.clone(), diagnostics.clone(), None);
    let expected = Request::from_notification::<PublishDiagnostics>(params);

    assert_client_message(
        |p| async move { p.publish_diagnostics(uri, diagnostics, None).await },
        expected,
    )
    .await;
}
