//! Encoder and decoder for Language Server Protocol messages.

use std::marker::PhantomData;

use bytes::BytesMut;
use serde::{de::DeserializeOwned, Serialize};
use tracing::trace;

#[cfg(all(feature = "runtime-agnostic", not(feature = "runtime-tokio")))]
use async_codec_lite::{Decoder, Encoder};
#[cfg(feature = "runtime-tokio")]
use tokio_util::codec::{Decoder, Encoder};

pub use tower_lsp_max_base::protocol::ParseError;

/// Encodes and decodes Language Server Protocol messages.
pub struct LanguageServerCodec<T> {
    content_len: Option<usize>,
    _marker: PhantomData<T>,
}

impl<T> Default for LanguageServerCodec<T> {
    fn default() -> Self {
        LanguageServerCodec {
            content_len: None,
            _marker: PhantomData,
        }
    }
}

#[cfg(all(feature = "runtime-agnostic", not(feature = "runtime-tokio")))]
impl<T: Serialize> Encoder for LanguageServerCodec<T> {
    type Item = T;
    type Error = ParseError;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = serde_json::to_string(&item).unwrap_or_default();
        trace!("-> {}", msg);
        tower_lsp_max_base::protocol::encode_message(item, dst)
    }
}

#[cfg(feature = "runtime-tokio")]
impl<T: Serialize> Encoder<T> for LanguageServerCodec<T> {
    type Error = ParseError;

    fn encode(&mut self, item: T, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = serde_json::to_string(&item).unwrap_or_default();
        trace!("-> {}", msg);
        tower_lsp_max_base::protocol::encode_message(item, dst)
    }
}

impl<T: DeserializeOwned> Decoder for LanguageServerCodec<T> {
    type Item = T;
    type Error = ParseError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        tower_lsp_max_base::protocol::decode_message(src, &mut self.content_len)
    }
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use serde_json::Value;

    use super::*;

    #[cfg(all(feature = "runtime-agnostic", not(feature = "runtime-tokio")))]
    use async_codec_lite::{Decoder, Encoder};
    #[cfg(feature = "runtime-tokio")]
    use tokio_util::codec::{Decoder, Encoder};

    macro_rules! assert_err {
        ($expression:expr, $($pattern:tt)+) => {
            match $expression {
                $($pattern)+ => (),
                ref e => panic!("expected `{}` but got `{:?}`", stringify!($($pattern)+), e),
            }
        }
    }

    fn encode_message(content_type: Option<&str>, message: &str) -> String {
        let content_type = content_type
            .map(|ty| format!("\r\nContent-Type: {ty}"))
            .unwrap_or_default();

        format!(
            "Content-Length: {}{}\r\n\r\n{}",
            message.len(),
            content_type,
            message
        )
    }

    #[test]
    fn encode_and_decode() {
        let decoded = r#"{"jsonrpc":"2.0","method":"exit"}"#;
        let encoded = encode_message(None, decoded);

        let mut codec = LanguageServerCodec::default();
        let mut buffer = BytesMut::new();
        let item: Value = serde_json::from_str(decoded).unwrap();
        codec.encode(item, &mut buffer).unwrap();
        assert_eq!(buffer, BytesMut::from(encoded.as_str()));

        let mut buffer = BytesMut::from(encoded.as_str());
        let message = codec.decode(&mut buffer).unwrap();
        let decoded = serde_json::from_str(decoded).unwrap();
        assert_eq!(message, Some(decoded));
    }

    #[test]
    fn decodes_optional_content_type() {
        let decoded = r#"{"jsonrpc":"2.0","method":"exit"}"#;
        let content_type = "application/vscode-jsonrpc; charset=utf-8";
        let encoded = encode_message(Some(content_type), decoded);

        let mut codec = LanguageServerCodec::default();
        let mut buffer = BytesMut::from(encoded.as_str());
        let message = codec.decode(&mut buffer).unwrap();
        let decoded_: Value = serde_json::from_str(decoded).unwrap();
        assert_eq!(message, Some(decoded_));

        let content_type = "application/vscode-jsonrpc; charset=utf8";
        let encoded = encode_message(Some(content_type), decoded);

        let mut buffer = BytesMut::from(encoded.as_str());
        let message = codec.decode(&mut buffer).unwrap();
        let decoded_: Value = serde_json::from_str(decoded).unwrap();
        assert_eq!(message, Some(decoded_));

        let content_type = "application/vscode-jsonrpc; charset=invalid";
        let encoded = encode_message(Some(content_type), decoded);

        let mut buffer = BytesMut::from(encoded.as_str());
        assert_err!(
            codec.decode(&mut buffer),
            Err(ParseError::InvalidContentType)
        );

        let content_type = "application/vscode-jsonrpc";
        let encoded = encode_message(Some(content_type), decoded);

        let mut buffer = BytesMut::from(encoded.as_str());
        let message = codec.decode(&mut buffer).unwrap();
        let decoded_: Value = serde_json::from_str(decoded).unwrap();
        assert_eq!(message, Some(decoded_));

        let content_type = "this-mime-should-be-ignored; charset=utf8";
        let encoded = encode_message(Some(content_type), decoded);

        let mut buffer = BytesMut::from(encoded.as_str());
        let message = codec.decode(&mut buffer).unwrap();
        let decoded_: Value = serde_json::from_str(decoded).unwrap();
        assert_eq!(message, Some(decoded_));
    }

    #[test]
    fn decodes_zero_length_message() {
        let content_type = "application/vscode-jsonrpc; charset=utf-8";
        let encoded = encode_message(Some(content_type), "");

        let mut codec = LanguageServerCodec::<Value>::default();
        let mut buffer = BytesMut::from(encoded.as_str());
        assert_err!(codec.decode(&mut buffer), Err(ParseError::EmptyMessage));
    }

    #[test]
    fn recovers_from_parse_error() {
        let decoded = r#"{"jsonrpc":"2.0","method":"exit"}"#;
        let encoded = encode_message(None, decoded);
        let mixed = format!("foobar{encoded}Content-Length: foobar\r\n\r\n{encoded}");

        let mut codec = LanguageServerCodec::default();
        let mut buffer = BytesMut::from(mixed.as_str());
        assert_err!(
            codec.decode(&mut buffer),
            Err(ParseError::MissingContentLength)
        );

        let message: Option<Value> = codec.decode(&mut buffer).unwrap();
        let first_valid = serde_json::from_str(decoded).unwrap();
        assert_eq!(message, Some(first_valid));
        assert_err!(
            codec.decode(&mut buffer),
            Err(ParseError::InvalidContentLength(_))
        );

        let message = codec.decode(&mut buffer).unwrap();
        let second_valid = serde_json::from_str(decoded).unwrap();
        assert_eq!(message, Some(second_valid));

        let message = codec.decode(&mut buffer).unwrap();
        assert_eq!(message, None);
    }

    #[test]
    fn decodes_small_chunks() {
        let decoded = r#"{"jsonrpc":"2.0","method":"exit"}"#;
        let content_type = "application/vscode-jsonrpc; charset=utf-8";
        let encoded = encode_message(Some(content_type), decoded);

        let mut codec = LanguageServerCodec::default();
        let mut buffer = BytesMut::from(encoded.as_str());

        let rest = buffer.split_off(40);
        let message = codec.decode(&mut buffer).unwrap();
        assert_eq!(message, None);
        buffer.unsplit(rest);

        let rest = buffer.split_off(80);
        let message = codec.decode(&mut buffer).unwrap();
        assert_eq!(message, None);
        buffer.unsplit(rest);

        let rest = buffer.split_off(16);
        let message = codec.decode(&mut buffer).unwrap();
        assert_eq!(message, None);
        buffer.unsplit(rest);

        let decoded: Value = serde_json::from_str(decoded).unwrap();
        let message = codec.decode(&mut buffer).unwrap();
        assert_eq!(message, Some(decoded));
    }
}
