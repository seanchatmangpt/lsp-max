#[cfg(test)]
mod client_tests {
    use crate::client::builder::ClientBuilder;
    use crate::client::LanguageClient;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Minimal no-op client for tests.
    struct NoopClient;
    impl LanguageClient for NoopClient {}

    /// Produce a valid LSP-framed JSON-RPC response with the given id and result.
    fn make_response(id: u64, result: serde_json::Value) -> Vec<u8> {
        let body = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}",
            id = id,
            result = result
        );
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        frame.into_bytes()
    }

    /// Test 1: write_loop frames an outgoing message with Content-Length header.
    #[tokio::test]
    async fn write_loop_frames_outgoing_message() {
        // duplex(4096): client_side is what we build the handle with (output = write end);
        // server_side is where we read back what was written.
        let (client_side, mut server_side) = tokio::io::duplex(4096);

        // For input (server→client direction) we give an empty reader that never yields.
        let (_, empty_input) = tokio::io::duplex(64);

        let handle = ClientBuilder::new().build(NoopClient, empty_input, client_side);

        // Send a notification — this enqueues a message without waiting for a response.
        handle.exit().await;

        // Give the write loop a moment to flush.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // Read whatever arrived on the server side.
        let mut buf = vec![0u8; 256];
        let n = server_side.read(&mut buf).await.unwrap_or(0);
        let written = std::str::from_utf8(&buf[..n]).unwrap_or("");

        assert!(
            written.starts_with("Content-Length: "),
            "expected LSP framing, got: {:?}",
            written
        );
    }

    /// Test 2: read_loop delivers a response to a pending request.
    #[tokio::test]
    async fn read_loop_delivers_response_to_request() {
        // The "server's stdout" that we feed responses into.
        let (mut response_writer, response_reader) = tokio::io::duplex(4096);
        // The "server's stdin" that receives outgoing requests — we discard them.
        let (_, discard_output) = tokio::io::duplex(4096);

        let handle = ClientBuilder::new().build(NoopClient, response_reader, discard_output);

        // The first request will be id=1 (AtomicU64 starts at 1).
        // Spawn a task that writes the response after a brief delay.
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            // shutdown returns () — respond with null result so the deserialization succeeds.
            let frame = make_response(1, serde_json::Value::Null);
            response_writer.write_all(&frame).await.ok();
        });

        // shutdown() sends a request with id=1 and awaits the response.
        let result = handle.shutdown().await;
        assert!(
            result.is_ok(),
            "shutdown should return Ok, got {:?}",
            result
        );
    }

    /// Test 3: read_loop handles a server notification without panicking.
    #[tokio::test]
    async fn read_loop_handles_notification_without_panicking() {
        let (mut notification_writer, notification_reader) = tokio::io::duplex(4096);
        let (_, discard_output) = tokio::io::duplex(64);

        let _handle = ClientBuilder::new().build(NoopClient, notification_reader, discard_output);

        // Write a notification (no "id" field) — the read loop should log and continue.
        let body = r#"{"jsonrpc":"2.0","method":"window/logMessage","params":{"type":3,"message":"hello"}}"#;
        let frame = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        notification_writer
            .write_all(frame.as_bytes())
            .await
            .unwrap();

        // Give the read loop time to process it.
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // If we reach here without a panic, the test passes.
    }
}
