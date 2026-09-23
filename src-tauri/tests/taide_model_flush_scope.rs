use std::collections::HashSet;

use taide_lib::ids::ProjectId;
use taide_lib::state::FlushScope as FacadeFlushScope;
use taide_model::flush::FlushScope as ModelFlushScope;

fn model_scope(scope: FacadeFlushScope) -> ModelFlushScope {
    scope
}

fn facade_scope(scope: ModelFlushScope) -> FacadeFlushScope {
    scope
}

#[test]
fn flush_scope는_모델과_기존_state_경로에서_동일한_타입이다() {
    let scope = model_scope(FacadeFlushScope::Project(ProjectId("prj-fixed".to_string())));
    assert_eq!(facade_scope(scope.clone()), scope);

    let mut pending = HashSet::new();
    pending.insert(scope);
    assert!(pending.contains(&ModelFlushScope::Project(ProjectId("prj-fixed".to_string()))));
}

#[test]
fn flush_scope는_기존_외부_태그_wire를_변경하지_않는다() {
    let cases = [
        (serde_json::json!("all"), ModelFlushScope::All),
        (
            serde_json::json!({ "window": "editor-2" }),
            ModelFlushScope::Window("editor-2".to_string()),
        ),
        (
            serde_json::json!({ "project": "prj-fixed" }),
            ModelFlushScope::Project(ProjectId("prj-fixed".to_string())),
        ),
    ];

    for (wire, scope) in cases {
        assert_eq!(serde_json::to_value(&scope).expect("직렬화"), wire);
        let restored: ModelFlushScope = serde_json::from_value(wire).expect("기존 wire 역직렬화");
        assert_eq!(restored, scope);
    }
}
