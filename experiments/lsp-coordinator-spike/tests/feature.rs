use serde_json::{json, Value};
use taide_lsp::native::feature::{
    validate_request, validate_response, SemanticTokensDeltaResult, TypedReply, METHODS,
};
use taide_lsp::native::protocol::lsp_types::request::{
    ExecuteCommand, HoverRequest, SemanticTokensFullDeltaRequest,
};
use taide_lsp::native::Failure;

const FEATURE_COUNT: usize = 27;
const OUTSIDE_UINT: u64 = 2147483648;
const TAB_SIZE: u64 = 4;
const URI: &str = "file:///synthetic.rs";

fn range() -> Value {
    json!({"start":{"line":0,"character":0},"end":{"line":0,"character":1}})
}

fn params(method: &str) -> Value {
    let mut params = json!({"textDocument":{"uri":URI},"position":{"line":0,"character":0}});
    match method {
        "workspace/symbol" => return json!({"query":"synthetic"}),
        "workspace/executeCommand" => {
            return json!({"command":"synthetic.allowed","arguments":[{"line":-1,"character":false}]})
        }
        "codeAction/resolve" => return json!({"title":"synthetic","data":{"range":"opaque"}}),
        "codeLens/resolve" => return json!({"range":range(),"data":{"line":-1}}),
        "textDocument/references" => params["context"] = json!({"includeDeclaration":true}),
        "textDocument/rename" => params["newName"] = json!("synthetic"),
        "textDocument/selectionRange" => params["positions"] = json!([{"line":0,"character":0}]),
        "textDocument/codeAction" => {
            params["range"] = range();
            params["context"] = json!({"diagnostics":[]});
        }
        "textDocument/inlayHint" | "textDocument/rangeFormatting" => params["range"] = range(),
        "textDocument/semanticTokens/full/delta" => params["previousResultId"] = json!("previous"),
        _ => {}
    }
    if matches!(
        method,
        "textDocument/formatting"
            | "textDocument/rangeFormatting"
            | "textDocument/onTypeFormatting"
    ) {
        params["options"] = json!({"tabSize":TAB_SIZE,"insertSpaces":true,"startLine":-1});
    }
    if method == "textDocument/onTypeFormatting" {
        params["ch"] = json!(";");
    }
    params
}

#[test]
fn 전체_27개_feature는_원본_request_연관_타입으로_필수_params와_result를_검증한다() {
    assert_eq!(METHODS.len(), FEATURE_COUNT);
    for method in METHODS {
        assert!(
            validate_request(method, &params(method)).is_ok(),
            "{method}"
        );
        assert_eq!(
            validate_request(method, &json!({})),
            Err(Failure::MalformedRequest),
            "{method}"
        );
        let result = match *method {
            "codeAction/resolve" => json!({"title":"synthetic","data":null}),
            "codeLens/resolve" => json!({"range":range(),"data":{"opaque":true}}),
            "textDocument/diagnostic" => json!({"kind":"full","items":[]}),
            _ => Value::Null,
        };
        assert!(validate_response(method, &result).is_ok(), "{method}");
        if *method != "workspace/executeCommand" {
            assert_eq!(
                validate_response(method, &json!(false)),
                Err(Failure::MalformedResponse),
                "{method}"
            );
        }
    }
}

#[test]
fn typed_reply는_원형_metadata와_명령의_임의_result를_보존한다() {
    let raw = json!({"contents":{"kind":"plaintext","value":"한글 e\u{301} 𐐀"},"range":range(),"experimental":{"range":"opaque"}});
    let reply = TypedReply::<HoverRequest>::decode(raw.clone()).unwrap();
    assert!(reply.value.is_some());
    assert_eq!(reply.raw, raw);
    for raw in [
        json!(false),
        json!({"line":-1,"character":"opaque","range":{"custom":true}}),
    ] {
        let reply = TypedReply::<ExecuteCommand>::decode(raw.clone()).unwrap();
        assert_eq!(reply.value, Some(raw.clone()));
        assert_eq!(reply.raw, raw);
        assert!(validate_response("workspace/executeCommand", &raw).is_ok());
    }
}

#[test]
fn position_range_uri와_semantic_token의_integer_경계를_검증하고_opaque_data는_해석하지_않는다() {
    let mut request = params("textDocument/hover");
    request["position"]["character"] = json!(OUTSIDE_UINT);
    assert_eq!(
        validate_request("textDocument/hover", &request),
        Err(Failure::MalformedRequest)
    );
    request = params("textDocument/hover");
    request["textDocument"]["uri"] = json!("relative.rs");
    assert_eq!(
        validate_request("textDocument/hover", &request),
        Err(Failure::MalformedRequest)
    );
    let reversed = json!({"start":{"line":1,"character":0},"end":{"line":0,"character":0}});
    assert_eq!(
        validate_response(
            "textDocument/hover",
            &json!({"contents":"synthetic","range":reversed})
        ),
        Err(Failure::MalformedResponse)
    );
    let completion = json!({"isIncomplete":false,"items":[{"label":"synthetic","data":{"line":-1,"range":"opaque"},"experimental":{"retained":true}}],"itemDefaults":{"editRange":range(),"data":null,"insertTextFormat":1}});
    assert!(validate_response("textDocument/completion", &completion).is_ok());
    let mut invalid = completion.clone();
    invalid["itemDefaults"]["editRange"] = reversed;
    assert_eq!(
        validate_response("textDocument/completion", &invalid),
        Err(Failure::MalformedResponse)
    );
    assert!(validate_response(
        "textDocument/semanticTokens/full",
        &json!({"data":[0,0,1,0,0]})
    )
    .is_ok());
    for data in [json!([0, 0, 1, 0]), json!([0, 0, OUTSIDE_UINT, 0, 0])] {
        assert_eq!(
            validate_response("textDocument/semanticTokens/full", &json!({"data":data})),
            Err(Failure::MalformedResponse)
        );
    }
    assert!(validate_response(
        "textDocument/semanticTokens/full/delta",
        &json!({"edits":[{"start":0,"deleteCount":1,"data":[1]}]})
    )
    .is_ok());
    let raw = json!({"resultId":"next","edits":[{"start":0,"deleteCount":1,"data":[1]},{"start":1,"deleteCount":1}]});
    let reply = TypedReply::<SemanticTokensFullDeltaRequest>::decode(raw.clone()).unwrap();
    let Some(SemanticTokensDeltaResult::Delta(delta)) = reply.value else {
        panic!("expected typed delta")
    };
    assert_eq!(delta.edits[0].data, Some(vec![1]));
    assert_eq!(delta.edits[1].data, None);
    assert_eq!(reply.raw, raw);
    for edit in [
        json!({"start":OUTSIDE_UINT,"deleteCount":1}),
        json!({"start":0,"deleteCount":1,"data":[OUTSIDE_UINT]}),
    ] {
        assert_eq!(
            validate_response(
                "textDocument/semanticTokens/full/delta",
                &json!({"edits":[edit]})
            ),
            Err(Failure::MalformedResponse)
        );
    }
}
