//! Implementation of the LSP Base Protocol 0.9.
//!
//! This module provides conversion utilities between `lsp_types_max` Base Protocol types
//! and `tower-lsp-max` JSON-RPC types.

use std::borrow::Cow;

use lsp_types_max::{
    NotificationMessage, NumberOrString, RequestMessage, ResponseError, ResponseMessage,
};
use serde_json::Value;

use super::{Error, ErrorCode, Id, Message, Request, Response};

/// Conversion from `lsp_types_max` types to `tower-lsp-max` types.
pub trait IntoTower {
    /// The target `tower-lsp-max` type.
    type Target;
    /// Converts this type into the target `tower-lsp-max` type.
    fn into_tower(self) -> Self::Target;
}

/// Conversion from `tower-lsp-max` types to `lsp_types_max` types.
pub trait IntoLsp {
    /// The target `lsp_types_max` type.
    type Target;
    /// Converts this type into the target `lsp_types_max` type.
    fn into_lsp(self) -> Self::Target;
}

impl IntoTower for RequestMessage {
    type Target = Request;

    fn into_tower(self) -> Self::Target {
        Request::build(self.method)
            .id(Id::from(self.id))
            .params(self.params.unwrap_or(Value::Null))
            .finish()
    }
}

impl IntoTower for NotificationMessage {
    type Target = Request;

    fn into_tower(self) -> Self::Target {
        Request::build(self.method)
            .params(self.params.unwrap_or(Value::Null))
            .finish()
    }
}

impl IntoTower for ResponseError {
    type Target = Error;

    fn into_tower(self) -> Self::Target {
        Error {
            code: ErrorCode::from(self.code as i64),
            message: Cow::Owned(self.message),
            data: self.data,
        }
    }
}

impl IntoTower for ResponseMessage {
    type Target = Response;

    fn into_tower(self) -> Self::Target {
        match self {
            ResponseMessage::Success { id, result, .. } => {
                Response::from_ok(id.map(Id::from).unwrap_or(Id::Null), result)
            }
            ResponseMessage::Error { id, error, .. } => {
                Response::from_error(id.map(Id::from).unwrap_or(Id::Null), error.into_tower())
            }
        }
    }
}

impl IntoLsp for Error {
    type Target = ResponseError;

    fn into_lsp(self) -> Self::Target {
        ResponseError {
            code: self.code.code() as i32,
            message: self.message.into_owned(),
            data: self.data,
        }
    }
}

impl IntoLsp for Response {
    type Target = ResponseMessage;

    fn into_lsp(self) -> Self::Target {
        let (id, result) = self.into_parts();
        match result {
            Ok(result) => ResponseMessage::Success {
                jsonrpc: "2.0".to_string(),
                id: tower_id_to_lsp(id),
                result,
            },
            Err(error) => ResponseMessage::Error {
                jsonrpc: "2.0".to_string(),
                id: tower_id_to_lsp(id),
                error: error.into_lsp(),
            },
        }
    }
}

/// Converts a `tower-lsp-max` `Id` to an `lsp_types_max` `Option<NumberOrString>`.
fn tower_id_to_lsp(id: Id) -> Option<NumberOrString> {
    match id {
        Id::Number(n) => Some(NumberOrString::Number(n as i32)),
        Id::String(s) => Some(NumberOrString::String(s)),
        Id::Null => None,
    }
}

/// A wrapper enum that encapsulates all possible LSP Base Protocol 0.9 messages.
///
/// This provides a bridge between the unified `Request` type in `tower-lsp-max`
/// and the explicit `RequestMessage` and `NotificationMessage` types in the LSP specification.
#[derive(Debug, Clone, PartialEq)]
pub enum BaseProtocolMessage {
    /// A request message to describe a request between the client and the server.
    Request(RequestMessage),
    /// A notification message. A NotificationMessage works like an event and does not send a response back.
    Notification(NotificationMessage),
    /// A response message is sent as a result of a request.
    Response(ResponseMessage),
}

impl BaseProtocolMessage {
    /// Converts a `tower-lsp-max` `Request` into a `BaseProtocolMessage`.
    ///
    /// This will be either a `Request` or a `Notification` depending on whether an ID is present.
    pub fn from_request(request: Request) -> Self {
        let (method, id, params) = request.into_parts();
        if let Some(id) = id {
            BaseProtocolMessage::Request(RequestMessage {
                jsonrpc: "2.0".to_string(),
                id: match id {
                    Id::Number(n) => NumberOrString::Number(n as i32),
                    Id::String(s) => NumberOrString::String(s),
                    Id::Null => NumberOrString::Number(0), // Fallback for discouraged null ID
                },
                method: method.into_owned(),
                params,
            })
        } else {
            BaseProtocolMessage::Notification(NotificationMessage {
                jsonrpc: "2.0".to_string(),
                method: method.into_owned(),
                params,
            })
        }
    }

    /// Converts a `tower-lsp-max` `Response` into a `BaseProtocolMessage`.
    pub fn from_response(response: Response) -> Self {
        BaseProtocolMessage::Response(response.into_lsp())
    }

    /// Converts a `tower-lsp-max` `Message` into a `BaseProtocolMessage`.
    pub fn from_message(message: Message) -> Self {
        match message {
            Message::Request(req) => Self::from_request(req),
            Message::Response(res) => Self::from_response(res),
        }
    }

    /// Converts this `BaseProtocolMessage` into the corresponding `tower-lsp-max` message types.
    pub fn into_tower(self) -> Message {
        match self {
            BaseProtocolMessage::Request(r) => Message::Request(r.into_tower()),
            BaseProtocolMessage::Notification(n) => Message::Request(n.into_tower()),
            BaseProtocolMessage::Response(r) => Message::Response(r.into_tower()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_message_bridge() {
        let lsp_req = RequestMessage {
            jsonrpc: "2.0".to_string(),
            id: NumberOrString::Number(42),
            method: "test/method".to_string(),
            params: Some(json!({"foo": "bar"})),
        };

        let tower_req = lsp_req.clone().into_tower();
        assert_eq!(tower_req.method(), "test/method");
        assert_eq!(tower_req.id(), Some(&Id::Number(42)));
        assert_eq!(tower_req.params(), Some(&json!({"foo": "bar"})));

        let bridge_msg = BaseProtocolMessage::from_request(tower_req);
        if let BaseProtocolMessage::Request(back) = bridge_msg {
            assert_eq!(back.id, lsp_req.id);
            assert_eq!(back.method, lsp_req.method);
            assert_eq!(back.params, lsp_req.params);
        } else {
            panic!("Expected RequestMessage variant");
        }
    }

    #[test]
    fn test_notification_message_bridge() {
        let lsp_notif = NotificationMessage {
            jsonrpc: "2.0".to_string(),
            method: "test/notify".to_string(),
            params: Some(json!([1, 2, 3])),
        };

        let tower_req = lsp_notif.clone().into_tower();
        assert_eq!(tower_req.method(), "test/notify");
        assert_eq!(tower_req.id(), None);
        assert_eq!(tower_req.params(), Some(&json!([1, 2, 3])));

        let bridge_msg = BaseProtocolMessage::from_request(tower_req);
        if let BaseProtocolMessage::Notification(back) = bridge_msg {
            assert_eq!(back.method, lsp_notif.method);
            assert_eq!(back.params, lsp_notif.params);
        } else {
            panic!("Expected NotificationMessage variant");
        }
    }

    #[test]
    fn test_response_message_bridge() {
        let lsp_res = ResponseMessage::Success {
            jsonrpc: "2.0".to_string(),
            id: Some(NumberOrString::String("req-1".to_string())),
            result: json!({"status": "ok"}),
        };

        let tower_res = lsp_res.clone().into_tower();
        assert_eq!(tower_res.id(), &Id::String("req-1".to_string()));
        assert_eq!(tower_res.result(), Some(&json!({"status": "ok"})));

        let bridge_msg = BaseProtocolMessage::from_response(tower_res);
        if let BaseProtocolMessage::Response(back) = bridge_msg {
            assert_eq!(back, lsp_res);
        } else {
            panic!("Expected ResponseMessage variant");
        }
    }

    #[test]
    fn test_error_bridge() {
        let lsp_err = ResponseError {
            code: -32601,
            message: "Method not found".to_string(),
            data: None,
        };

        let tower_err: Error = lsp_err.clone().into_tower();
        assert_eq!(tower_err.code, ErrorCode::MethodNotFound);
        assert_eq!(tower_err.message, "Method not found");

        let back_lsp_err = tower_err.into_lsp();
        assert_eq!(back_lsp_err, lsp_err);
    }
}
