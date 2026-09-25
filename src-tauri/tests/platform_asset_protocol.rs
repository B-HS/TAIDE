use std::collections::HashMap;

use taide_lib::infra::asset_protocol as legacy_asset_protocol;
use taide_lib::platform::asset_protocol;
use tauri::http::{Request, StatusCode};

#[test]
fn 열린_프로젝트가_없으면_새_platform과_기존_infra_경계가_모두_접근을_거부한다() {
    let projects = HashMap::new();
    let request = || Request::builder().uri("/%2Ftmp%2Fmissing").body(Vec::new()).unwrap();

    let platform_response = asset_protocol::respond(&projects, request());
    let legacy_response = legacy_asset_protocol::respond(&projects, request());

    assert_eq!(platform_response.status(), StatusCode::FORBIDDEN);
    assert_eq!(legacy_response.status(), platform_response.status());
    assert_eq!(legacy_response.body(), platform_response.body());
}

#[test]
fn asset_uri_scheme_등록은_platform_응답_함수를_사용한다() {
    let app_source = include_str!("../src/lib.rs");

    assert!(app_source.contains(".register_uri_scheme_protocol(\"asset\""));
    assert!(app_source.contains("platform::asset_protocol::respond(&projects, request)"));
}
