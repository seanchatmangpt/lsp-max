use crate::client::LanguageClient;
use crate::server_handle::ServerHandle;
use tokio::io::{AsyncRead, AsyncWrite};

/// Builder for constructing a client connection to a Language Server.
pub struct ClientBuilder {
    // Configuration options could go here
}

impl ClientBuilder {
    /// Create a new ClientBuilder.
    pub fn new() -> Self {
        Self {}
    }

    /// Build the client connection. Takes a type implementing `LanguageClient`
    /// to handle inbound server messages, and the I/O streams for the transport.
    /// Returns the `ServerHandle` which can be used to send outbound requests.
    pub fn build<C, I, O>(self, _client: C, _stdin: I, _stdout: O) -> ServerHandle
    where
        C: LanguageClient,
        I: AsyncRead + Unpin + Send + 'static,
        O: AsyncWrite + Unpin + Send + 'static,
    {
        ServerHandle {}
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
