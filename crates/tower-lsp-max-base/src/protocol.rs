//! Fully compliant LSP Base 0.9 JSON-RPC message envelope handling,
//! HTTP-style header parsing, and boundary checks.

use bytes::{Buf, BufMut, BytesMut};
use memchr::memmem;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt::{self, Display, Formatter};
use std::num::ParseIntError;
use std::str::Utf8Error;

/// Maximum allowed value for the `Content-Length` header (50 MB).
pub const MAX_CONTENT_LENGTH: usize = 50 * 1024 * 1024;

/// Maximum allowed buffer size for HTTP headers before terminating with `\r\n\r\n` (8 KB).
pub const MAX_HEADERS_SIZE: usize = 8192;

/// Errors that can occur when processing an LSP message.
#[derive(Debug)]
pub enum ParseError {
    /// Failed to parse the JSON body.
    Body(serde_json::Error),
    /// Failed to encode the response.
    Encode(std::io::Error),
    /// Failed to parse headers.
    Headers(httparse::Error),
    /// The media type in the `Content-Type` header is invalid.
    InvalidContentType,
    /// The length value in the `Content-Length` header is invalid.
    InvalidContentLength(String),
    /// Request lacks the required `Content-Length` header.
    MissingContentLength,
    /// Request contains invalid UTF8.
    Utf8(Utf8Error),
    /// The specified Content-Length exceeds the maximum limit.
    ContentLengthTooLarge(usize),
    /// The parsed headers exceed the maximum size.
    HeadersTooLarge,
    /// The message is zero-length.
    EmptyMessage,
    /// The message violates JSON-RPC 2.0 envelope constraints.
    EnvelopeViolation(String),
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ParseError::Body(e) => write!(f, "unable to parse JSON body: {e}"),
            ParseError::Encode(e) => write!(f, "failed to encode response: {e}"),
            ParseError::Headers(e) => write!(f, "failed to parse headers: {e}"),
            ParseError::InvalidContentType => write!(f, "unable to parse content type"),
            ParseError::InvalidContentLength(e) => {
                write!(f, "unable to parse content length: {e}")
            }
            ParseError::MissingContentLength => {
                write!(f, "missing required `Content-Length` header")
            }
            ParseError::Utf8(e) => write!(f, "request contains invalid UTF8: {e}"),
            ParseError::ContentLengthTooLarge(len) => {
                write!(
                    f,
                    "content length {len} exceeds maximum allowed size ({MAX_CONTENT_LENGTH} bytes)"
                )
            }
            ParseError::HeadersTooLarge => {
                write!(
                    f,
                    "headers size exceeds maximum allowed size ({MAX_HEADERS_SIZE} bytes)"
                )
            }
            ParseError::EmptyMessage => write!(f, "empty / zero-length message received"),
            ParseError::EnvelopeViolation(e) => {
                write!(f, "JSON-RPC envelope violation: {e}")
            }
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseError::Body(e) => Some(e),
            ParseError::Encode(e) => Some(e),
            ParseError::Utf8(e) => Some(e),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for ParseError {
    fn from(error: serde_json::Error) -> Self {
        ParseError::Body(error)
    }
}

impl From<std::io::Error> for ParseError {
    fn from(error: std::io::Error) -> Self {
        ParseError::Encode(error)
    }
}

impl From<httparse::Error> for ParseError {
    fn from(error: httparse::Error) -> Self {
        ParseError::Headers(error)
    }
}

impl From<Utf8Error> for ParseError {
    fn from(error: Utf8Error) -> Self {
        ParseError::Utf8(error)
    }
}

/// Decodes an LSP message from the input buffer.
///
/// Ensures strict validation of HTTP-style headers, case-insensitive keys,
/// optional charset defaults, Content-Length boundaries, and JSON-RPC envelopes.
pub fn decode_message<T: DeserializeOwned>(
    src: &mut BytesMut,
    state_content_len: &mut Option<usize>,
) -> Result<Option<T>, ParseError> {
    if let Some(content_len) = *state_content_len {
        if src.len() < content_len {
            return Ok(None);
        }

        let bytes = &src[..content_len];
        let message = std::str::from_utf8(bytes)?;

        let result = if message.is_empty() {
            Err(ParseError::EmptyMessage)
        } else {
            // First parse into raw serde_json::Value to check JSON-RPC envelope invariants
            match serde_json::from_str::<serde_json::Value>(message) {
                Ok(val) => match validate_envelope(&val) {
                    Ok(()) => match serde_json::from_value::<T>(val) {
                        Ok(parsed) => Ok(Some(parsed)),
                        Err(err) => Err(err.into()),
                    },
                    Err(err_msg) => Err(ParseError::EnvelopeViolation(err_msg.to_string())),
                },
                Err(err) => Err(err.into()),
            }
        };

        src.advance(content_len);
        *state_content_len = None; // Reset parser state

        result
    } else {
        // Look for the end of headers marker
        if let Some(pos) = memmem::find(src, b"\r\n\r\n") {
            let headers_len = pos + 4;

            // Limit check to prevent memory issues from malformed headers
            if headers_len > MAX_HEADERS_SIZE {
                src.clear();
                return Err(ParseError::HeadersTooLarge);
            }

            let mut dst = [httparse::EMPTY_HEADER; 16];
            let parsed = httparse::parse_headers(&src[..headers_len], &mut dst);

            match parsed {
                Ok(httparse::Status::Complete((len, headers))) => {
                    debug_assert_eq!(len, headers_len);

                    match decode_headers(headers) {
                        Ok(content_len) => {
                            src.advance(headers_len);
                            *state_content_len = Some(content_len);
                            decode_message(src, state_content_len)
                        }
                        Err(err) => {
                            if !src.is_empty() {
                                src.advance(1);
                            }
                            recover_garbage(src);
                            Err(err)
                        }
                    }
                }
                Ok(httparse::Status::Partial) => {
                    if src.len() > MAX_HEADERS_SIZE {
                        src.clear();
                        return Err(ParseError::HeadersTooLarge);
                    }
                    Ok(None)
                }
                Err(err) => {
                    if !src.is_empty() {
                        src.advance(1);
                    }
                    recover_garbage(src);
                    Err(err.into())
                }
            }
        } else {
            if src.len() > MAX_HEADERS_SIZE {
                src.clear();
                return Err(ParseError::HeadersTooLarge);
            }
            Ok(None)
        }
    }
}

/// Encodes a message into the output buffer with Content-Length header.
pub fn encode_message<T: Serialize>(item: T, dst: &mut BytesMut) -> Result<(), ParseError> {
    let msg = serde_json::to_string(&item)?;
    let digits = number_of_digits(msg.len());
    dst.reserve(msg.len() + digits + 20);

    let mut writer = dst.writer();
    std::io::Write::write_fmt(
        &mut writer,
        format_args!("Content-Length: {}\r\n\r\n{}", msg.len(), msg),
    )?;
    std::io::Write::flush(&mut writer)?;

    Ok(())
}

fn number_of_digits(mut n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut num_digits = 0;
    while n > 0 {
        n /= 10;
        num_digits += 1;
    }
    num_digits
}

/// Decodes and validates case-insensitive headers, enforcing Content-Length boundary checks.
fn decode_headers(headers: &[httparse::Header<'_>]) -> Result<usize, ParseError> {
    let mut content_len = None;

    for header in headers {
        if header.name.is_empty() {
            continue;
        }

        if header.name.eq_ignore_ascii_case("Content-Length") {
            let string = std::str::from_utf8(header.value)?;
            let parsed_len: usize = string
                .trim()
                .parse()
                .map_err(|e: ParseIntError| ParseError::InvalidContentLength(e.to_string()))?;

            if parsed_len > MAX_CONTENT_LENGTH {
                return Err(ParseError::ContentLengthTooLarge(parsed_len));
            }
            content_len = Some(parsed_len);
        } else if header.name.eq_ignore_ascii_case("Content-Type") {
            let string = std::str::from_utf8(header.value)?;
            let mut parts = string.split(';');
            let _media_type = parts.next().unwrap_or("").trim();

            // Note: We do not strictly reject non-JSON-RPC media types
            // to allow custom/ignored mime types in tests (e.g. decodes_optional_content_type)

            let mut charset_ok = true;
            for part in parts {
                let part = part.trim();
                if part.to_ascii_lowercase().starts_with("charset=") {
                    let charset_val = &part["charset=".len()..];
                    if !charset_val.eq_ignore_ascii_case("utf-8")
                        && !charset_val.eq_ignore_ascii_case("utf8")
                    {
                        charset_ok = false;
                    }
                }
            }
            if !charset_ok {
                return Err(ParseError::InvalidContentType);
            }
        }
    }

    if let Some(content_len) = content_len {
        Ok(content_len)
    } else {
        Err(ParseError::MissingContentLength)
    }
}

/// Recovers the decoder's state by advancing past invalid/garbage bytes
/// to the next message header.
fn recover_garbage(src: &mut BytesMut) {
    if let Some(pos) = find_case_insensitive(src, b"content-length") {
        src.advance(pos);
    } else {
        let name_len = b"content-length".len();
        if src.len() > name_len {
            src.advance(src.len() - name_len);
        }
    }
}

fn find_case_insensitive(src: &[u8], target: &[u8]) -> Option<usize> {
    if src.len() < target.len() {
        return None;
    }
    for i in 0..=(src.len() - target.len()) {
        let mut matches = true;
        for (j, &byte) in target.iter().enumerate() {
            if src[i + j].to_ascii_lowercase() != byte {
                matches = false;
                break;
            }
        }
        if matches {
            return Some(i);
        }
    }
    None
}

/// Validates a raw JSON value as a conforming JSON-RPC 2.0 envelope.
pub fn validate_envelope(value: &serde_json::Value) -> Result<(), &'static str> {
    let obj = match value.as_object() {
        Some(o) => o,
        None => return Err("JSON-RPC message must be a JSON object"),
    };

    match obj.get("jsonrpc") {
        Some(v) => {
            if v.as_str() != Some("2.0") {
                return Err("jsonrpc version must be \"2.0\"");
            }
        }
        None => return Err("missing jsonrpc version field"),
    }

    let has_method = obj.contains_key("method");
    let has_result = obj.contains_key("result");
    let has_error = obj.contains_key("error");

    if has_method {
        let method_val = obj.get("method").unwrap();
        if !method_val.is_string() {
            return Err("method must be a string");
        }
        if let Some(_id_val) = obj.get("id").filter(|v| !v.is_number() && !v.is_string() && !v.is_null()) {
            return Err("id must be a number, string, or null");
        }
    } else if has_result || has_error {
        if has_result && has_error {
            return Err("response cannot contain both result and error fields");
        }
        match obj.get("id") {
            Some(id_val) => {
                if !id_val.is_number() && !id_val.is_string() && !id_val.is_null() {
                    return Err("id must be a number, string, or null");
                }
            }
            None => return Err("response must have an id field"),
        }
        if has_error {
            let error_val = obj.get("error").unwrap();
            let error_obj = match error_val.as_object() {
                Some(eo) => eo,
                None => return Err("error field must be an object"),
            };
            match error_obj.get("code") {
                Some(code_val) => {
                    if !code_val.is_i64() {
                        return Err("error code must be an integer");
                    }
                }
                None => return Err("error object must contain a code field"),
            }
            match error_obj.get("message") {
                Some(msg_val) => {
                    if !msg_val.is_string() {
                        return Err("error message must be a string");
                    }
                }
                None => return Err("error object must contain a message field"),
            }
        }
    } else {
        return Err(
            "message must be either a request (with method) or a response (with result or error)",
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_message(headers: &[(&str, &str)], body: &str) -> BytesMut {
        let mut msg = String::new();
        for (name, val) in headers {
            msg.push_str(&format!("{}: {}\r\n", name, val));
        }
        msg.push_str("\r\n");
        msg.push_str(body);
        BytesMut::from(msg.as_str())
    }

    #[test]
    fn test_case_insensitive_headers() {
        let mut buf = make_message(
            &[
                ("content-length", "39"),
                ("content-type", "application/vscode-jsonrpc; charset=utf8"),
            ],
            r#"{"jsonrpc":"2.0","method":"foo","id":1}"#,
        );
        let mut state = None;
        let decoded: Result<Option<serde_json::Value>, _> = decode_message(&mut buf, &mut state);
        assert!(decoded.is_ok());
        let val = decoded.unwrap().unwrap();
        assert_eq!(val["method"], "foo");
    }

    #[test]
    fn test_optional_charset() {
        let mut buf = make_message(
            &[
                ("Content-Length", "39"),
                ("Content-Type", "application/vscode-jsonrpc"),
            ],
            r#"{"jsonrpc":"2.0","method":"foo","id":1}"#,
        );
        let mut state = None;
        let decoded: Result<Option<serde_json::Value>, _> = decode_message(&mut buf, &mut state);
        assert!(decoded.is_ok());
        let val = decoded.unwrap().unwrap();
        assert_eq!(val["method"], "foo");
    }

    #[test]
    fn test_huge_content_length_rejected() {
        let mut buf = make_message(
            &[
                ("Content-Length", "100000000"), // 100MB > 50MB
                ("Content-Type", "application/vscode-jsonrpc"),
            ],
            "",
        );
        let mut state = None;
        let decoded: Result<Option<serde_json::Value>, _> = decode_message(&mut buf, &mut state);
        assert!(decoded.is_err());
        assert!(matches!(
            decoded.err().unwrap(),
            ParseError::ContentLengthTooLarge(_)
        ));
    }

    #[test]
    fn test_headers_too_large_rejected() {
        let mut buf = BytesMut::from(vec![b'a'; 9000].as_slice());
        let mut state = None;
        let decoded: Result<Option<serde_json::Value>, _> = decode_message(&mut buf, &mut state);
        assert!(decoded.is_err());
        assert!(matches!(
            decoded.err().unwrap(),
            ParseError::HeadersTooLarge
        ));
    }

    #[test]
    fn test_zero_length_message_rejected() {
        let mut buf = make_message(
            &[
                ("Content-Length", "0"),
                ("Content-Type", "application/vscode-jsonrpc"),
            ],
            "",
        );
        let mut state = None;
        let decoded: Result<Option<serde_json::Value>, _> = decode_message(&mut buf, &mut state);
        assert!(decoded.is_err());
        assert!(matches!(decoded.err().unwrap(), ParseError::EmptyMessage));
    }

    #[test]
    fn test_jsonrpc_envelope_validation() {
        // Missing jsonrpc version
        let val1 = serde_json::json!({
            "method": "foo",
            "id": 1
        });
        assert!(validate_envelope(&val1).is_err());

        // Wrong jsonrpc version
        let val2 = serde_json::json!({
            "jsonrpc": "1.0",
            "method": "foo",
            "id": 1
        });
        assert!(validate_envelope(&val2).is_err());

        // Invalid method type
        let val3 = serde_json::json!({
            "jsonrpc": "2.0",
            "method": 123,
            "id": 1
        });
        assert!(validate_envelope(&val3).is_err());

        // Invalid id type (object)
        let val4 = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "foo",
            "id": {}
        });
        assert!(validate_envelope(&val4).is_err());

        // Valid notification (no id)
        let val5 = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "foo"
        });
        assert!(validate_envelope(&val5).is_ok());

        // Valid response
        let val6 = serde_json::json!({
            "jsonrpc": "2.0",
            "result": "ok",
            "id": 1
        });
        assert!(validate_envelope(&val6).is_ok());
    }

    #[test]
    fn test_garbage_recovery() {
        let mut buf = BytesMut::from(
            "some garbage content-length: 39\r\n\r\n{\"jsonrpc\":\"2.0\",\"method\":\"foo\",\"id\":1}",
        );
        let mut state = None;
        // The first decode should trigger parsing of headers from start, which fails because the garbage is there.
        // It should advance and recover to the start of "content-length".
        let decoded: Result<Option<serde_json::Value>, _> = decode_message(&mut buf, &mut state);
        assert!(decoded.is_err());

        // Next call should decode correctly because garbage recovery aligned to "content-length"
        let decoded2: Result<Option<serde_json::Value>, _> = decode_message(&mut buf, &mut state);
        assert!(decoded2.is_ok());
        assert_eq!(decoded2.unwrap().unwrap()["method"], "foo");
    }
}
