use serde_json::Value;
use taide_ide::protocol;

const IDE_FIXTURE: &str = include_str!("fixtures/rust-native/ide-mcp-wire-v1.json");
const SERVER_NAME: &str = "TAIDE";
const SERVER_VERSION: &str = "fixture";

#[test]
fn ide_mcp_초기화_요청과_응답은_fixture와_같다() {
    let fixture: Value = serde_json::from_str(IDE_FIXTURE).expect("IDE fixture");
    assert_eq!(fixture["schemaVersion"], 1);
    let request = &fixture["initialize"]["request"];
    let incoming = protocol::parse_incoming(&request.to_string()).expect("initialize request");
    assert_eq!(incoming.method, "initialize");
    assert_eq!(incoming.id, Some(request["id"].clone()));
    assert_eq!(incoming.params, request["params"]);

    let response = protocol::success_response(
        incoming.id.expect("request id"),
        protocol::initialize_result(&incoming.params, SERVER_NAME, SERVER_VERSION),
    );
    let encoded: Value = serde_json::from_str(&protocol::encode(&response)).expect("initialize response");
    assert_eq!(encoded, fixture["initialize"]["response"]);
}

#[test]
fn ide_mcp_도구_목록과_호출_응답은_fixture와_같다() {
    let fixture: Value = serde_json::from_str(IDE_FIXTURE).expect("IDE fixture");
    let list_request = &fixture["toolsList"]["request"];
    let list = protocol::parse_incoming(&list_request.to_string()).expect("tools list request");
    assert_eq!(list.method, "tools/list");
    assert_eq!(list.id, Some(list_request["id"].clone()));
    let tools = protocol::tools_list_result();
    let names = tools["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|tool| tool["name"].clone())
        .collect::<Vec<_>>();
    assert_eq!(names, fixture["toolsList"]["names"].as_array().expect("tool names").clone());

    let call_request = &fixture["toolCall"]["request"];
    let call = protocol::parse_incoming(&call_request.to_string()).expect("tools call request");
    assert_eq!(call.method, "tools/call");
    assert_eq!(call.params, call_request["params"]);
    let response = protocol::success_response(call.id.expect("request id"), protocol::text_content("FILE_SAVED"));
    let encoded: Value = serde_json::from_str(&protocol::encode(&response)).expect("tools call response");
    assert_eq!(encoded, fixture["toolCall"]["response"]);
}

#[test]
fn ide_mcp_오류와_알림은_fixture와_같다() {
    let fixture: Value = serde_json::from_str(IDE_FIXTURE).expect("IDE fixture");
    let response = protocol::error_response(4.into(), protocol::RPC_INVALID_PARAMS, "invalid fixture argument");
    let encoded: Value = serde_json::from_str(&protocol::encode(&response)).expect("error response");
    assert_eq!(encoded, fixture["invalidParams"]["response"]);

    let expected = &fixture["notification"];
    let notification = protocol::build_notification(expected["method"].as_str().expect("method"), expected["params"].clone());
    let encoded: Value = serde_json::from_str(&protocol::encode(&notification)).expect("notification");
    assert_eq!(encoded, *expected);
}
