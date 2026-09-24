#[test]
fn lsp_워크스페이스_알림은_기존_uri와_json_형식을_유지한다() {
    let notification =
        taide_lsp::protocol::workspace_folders_notification(&["/workspace/my project".to_string()], &["/tmp/한글 루트".to_string()]);
    let parsed: serde_json::Value = serde_json::from_str(&notification).expect("유효한 LSP JSON 알림");
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["method"], "workspace/didChangeWorkspaceFolders");
    assert_eq!(parsed["params"]["event"]["added"][0]["uri"], "file:///workspace/my%20project");
    assert_eq!(parsed["params"]["event"]["added"][0]["name"], "my project");
    assert_eq!(
        parsed["params"]["event"]["removed"][0]["uri"],
        "file:///tmp/%ED%95%9C%EA%B8%80%20%EB%A3%A8%ED%8A%B8"
    );
}
