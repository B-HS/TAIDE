use serde_json::{json, Value};
use taide_lsp::native::protocol::{
    decode, decode_work_done, IncomingMessage, ProgressValue, Rejection, ServerNotification,
    ServerRequestKind,
};
use taide_lsp::native::Failure;

const OUTSIDE_SIGNED_INTEGER: u64 = 2147483648;
const INVALID_SEVERITY: u64 = 5;
const INVALID_PERCENTAGE: u64 = 101;

#[test]
fn jsonrpc는_request_notification_response와_integer_경계를_검증한다() {
    for id in [json!(-1), json!("server-id")] {
        assert!(matches!(
            decode(
                &json!({"jsonrpc":"2.0","id":id,"method":"workspace/configuration","params":{"items":[{}]}})
            ),
            Ok(IncomingMessage::Request {
                request: Ok(ServerRequestKind::Configuration(_)),
                ..
            })
        ));
    }
    for id in [Value::Null, json!(OUTSIDE_SIGNED_INTEGER), json!(0.5)] {
        assert_eq!(
            decode(
                &json!({"jsonrpc":"2.0","id":id,"method":"workspace/configuration","params":{"items":[]}})
            ),
            Err(Failure::MalformedResponse)
        );
    }
    assert_eq!(
        decode(
            &json!({"jsonrpc":"2.0","id":"server-id","method":"workspace/configuration","result":null,"params":{"items":[]}})
        ),
        Ok(IncomingMessage::Request {
            id: serde_json::from_value(json!("server-id")).unwrap(),
            request: Err(Rejection::InvalidRequest)
        })
    );
    for response in [
        json!({"jsonrpc":"2.0","id":0,"result":null,"error":{"code":0,"message":"both"}}),
        json!({"jsonrpc":"2.0","id":0}),
        json!({"jsonrpc":"2.0","id":0,"error":{"code":OUTSIDE_SIGNED_INTEGER,"message":"oversized"}}),
    ] {
        assert_eq!(decode(&response), Err(Failure::MalformedResponse));
    }
    assert_eq!(
        decode(
            &json!({"jsonrpc":"2.0","id":null,"error":{"code":-1,"message":"valid","data":null}})
        ),
        Ok(IncomingMessage::Response)
    );
    assert_eq!(
        decode(
            &json!({"jsonrpc":"2.0","method":"synthetic/extension","params":[{"retained":true}]})
        ),
        Ok(IncomingMessage::Notification(ServerNotification::Extension))
    );
    for params in [
        json!({"items":[{"section":null}]}),
        json!({"items":[null]}),
        json!([]),
    ] {
        assert!(matches!(
            decode(
                &json!({"jsonrpc":"2.0","id":0,"method":"workspace/configuration","params":params})
            ),
            Ok(IncomingMessage::Request {
                request: Err(Rejection::InvalidParams),
                ..
            })
        ));
    }
}

#[test]
fn diagnostics는_utf16_range_메타데이터와_clear_batch를_보존한다() {
    let range = json!({"start":{"line":0,"character":0},"end":{"line":0,"character":1}});
    let diagnostic = json!({"range":range,"message":"합성","severity":1,"code":"CODE","source":"fixture","data":null,"experimental":{"retained":true}});
    let message = json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"file:///synthetic.rs","version":-1,"diagnostics":[diagnostic]}});
    assert!(
        matches!(decode(&message), Ok(IncomingMessage::Notification(ServerNotification::Diagnostics(params))) if params.version == Some(-1) && params.diagnostics.len() == 1)
    );
    assert_eq!(
        message["params"]["diagnostics"][0]["experimental"]["retained"],
        true
    );
    assert!(
        matches!(decode(&json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":"file:///synthetic.rs","diagnostics":[]}})), Ok(IncomingMessage::Notification(ServerNotification::Diagnostics(params))) if params.diagnostics.is_empty())
    );
    let mut invalid = message.clone();
    invalid["params"]["diagnostics"][0]["severity"] = json!(INVALID_SEVERITY);
    assert_eq!(decode(&invalid), Err(Failure::MalformedResponse));
    invalid = message.clone();
    invalid["params"]["diagnostics"][0]["range"]["end"]["character"] =
        json!(OUTSIDE_SIGNED_INTEGER);
    assert_eq!(decode(&invalid), Err(Failure::MalformedResponse));
    invalid = message.clone();
    invalid["params"]["diagnostics"][0]["range"]["start"]["character"] =
        json!(OUTSIDE_SIGNED_INTEGER - 1);
    assert_eq!(decode(&invalid), Err(Failure::MalformedResponse));
    invalid = message.clone();
    invalid["params"]["version"] = Value::Null;
    assert_eq!(decode(&invalid), Err(Failure::MalformedResponse));
}

#[test]
fn progress는_work_done과_원형_partial_payload를_구분한다() {
    assert!(matches!(
        decode(
            &json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":"index","value":{"kind":"begin","title":"합성","percentage":0}}})
        ),
        Ok(IncomingMessage::Notification(
            ServerNotification::Progress {
                value: ProgressValue::Partial(_),
                ..
            }
        ))
    ));
    assert!(
        matches!(decode(&json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":-1,"value":[{"retained":true}]}})), Ok(IncomingMessage::Notification(ServerNotification::Progress {value:ProgressValue::Partial(value),..})) if value == json!([{"retained":true}]))
    );
    assert_eq!(
        decode_work_done(&json!({"kind":"report","percentage":INVALID_PERCENTAGE})),
        Err(Failure::MalformedResponse)
    );
    assert!(decode_work_done(&json!({"kind":"begin","title":"합성","percentage":0})).is_ok());
    assert!(matches!(
        decode(
            &json!({"jsonrpc":"2.0","method":"$/progress","params":{"token":"unknown-partial","value":{"kind":"begin","percentage":INVALID_PERCENTAGE,"custom":true}}})
        ),
        Ok(IncomingMessage::Notification(
            ServerNotification::Progress {
                value: ProgressValue::Partial(_),
                ..
            }
        ))
    ));
    let edit = json!({"jsonrpc":"2.0","id":0,"method":"workspace/applyEdit","params":{"edit":{"documentChanges":[{"textDocument":{"uri":"file:///synthetic.rs","version":null},"edits":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}},"newText":"合成"}]}]}}});
    assert!(matches!(
        decode(&edit),
        Ok(IncomingMessage::Request {
            request: Ok(ServerRequestKind::ApplyEdit(_)),
            ..
        })
    ));
}
