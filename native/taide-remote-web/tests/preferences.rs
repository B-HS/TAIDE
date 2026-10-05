use serde_json::json;
use taide_model::settings::Settings;
use taide_native_ui::settings_controls::{Change, Numeric, Switch};
use taide_remote_web::preferences::{PreferenceWrites, change_call};
use taide_remote_web::shell::{Failure, Read, ShellState};
use taide_remote_web::{InvokeError, ResponsePayload};

const SUCCESS_SEQ: u32 = 1;
const ERROR_SEQ: u32 = 2;
const BINARY_SEQ: u32 = 3;
const MALFORMED_SEQ: u32 = 4;
const CLOSED_SEQ: u32 = 5;
const STALE_READ_SEQ: u32 = 6;
const UNKNOWN_SEQ: u32 = 7;
const SETTINGS_GENERATION: u64 = 8;
const RESIZER_MAX: u32 = 8;
const CLAMP_INPUT: f64 = 999.0;

#[test]
fn 키바인딩_변경은_기존_settings_write의_단일_patch_ack와_closed_결과를_사용한다() {
    let overrides = taide_native_ui::keymap::catalog::Overrides::default()
        .unbind("quick-open")
        .json()
        .unwrap();
    let change = Change::KeymapOverrides(overrides.clone());
    let call = change_call(&change);
    assert_eq!(call.command, "settings_update");
    assert_eq!(call.args["patch"]["keymapOverrides"], overrides);
    assert_eq!(
        call.args["patch"]
            .as_object()
            .unwrap()
            .values()
            .filter(|value| !value.is_null())
            .count(),
        1
    );
    let mut writes = PreferenceWrites::default();
    writes.sent(SUCCESS_SEQ, change.clone(), SETTINGS_GENERATION);
    let settings = Settings {
        keymap_overrides: Some(overrides),
        ..Default::default()
    };
    let finished = writes
        .response(
            SUCCESS_SEQ,
            &Ok(ResponsePayload::Json(
                serde_json::to_value(&settings).unwrap(),
            )),
        )
        .unwrap();
    assert_eq!(finished.change, change);
    assert_eq!(finished.result.unwrap(), settings);
    assert!(!writes.is_pending());
    writes.sent(CLOSED_SEQ, change, SETTINGS_GENERATION);
    let closed = writes.disconnected();
    assert_eq!(closed.len(), 1);
    assert_eq!(
        closed[0].result.as_ref().unwrap_err(),
        &Failure::Invocation(InvokeError::Closed)
    );
    assert_eq!(
        writes.take_close_failures(),
        vec![Failure::Invocation(InvokeError::Closed)]
    );
    assert!(writes.disconnected().is_empty());
}

#[test]
fn 설정_실패_feedback은_종료_결과를_보존하고_늦은_응답을_중복_표시하지_않는다() {
    let mut writes = PreferenceWrites::default();
    let change = Change::Language("ja".into());
    let error = json!({"code": "InvalidArgument", "message": "synthetic refusal"});
    writes.sent(ERROR_SEQ, change.clone(), SETTINGS_GENERATION);
    let finished = writes.response(ERROR_SEQ, &Err(error.clone())).unwrap();
    assert_eq!(
        writes.take_close_failures(),
        vec![Failure::Remote(error.clone())]
    );
    assert!(writes.take_close_failures().is_empty());
    assert_eq!(
        finished.result.as_ref().unwrap_err(),
        &Failure::Remote(error.clone())
    );
    let feedback = writes.take_errors();
    assert!(
        matches!(feedback.as_slice(), [taide_model::error::AppError::InvalidArgument(message)] if message == "synthetic refusal")
    );
    assert!(writes.take_errors().is_empty());
    assert!(writes.response(ERROR_SEQ, &Err(error)).is_none());
    assert!(writes.take_errors().is_empty());
    assert_eq!(finished.change, change);
    writes.sent(
        SUCCESS_SEQ,
        Change::Language("en".into()),
        SETTINGS_GENERATION,
    );
    assert!(
        writes
            .response(
                SUCCESS_SEQ,
                &Ok(ResponsePayload::Json(
                    serde_json::to_value(Settings::default()).unwrap()
                ))
            )
            .unwrap()
            .result
            .is_ok()
    );
    assert!(writes.take_close_failures().is_empty());

    writes.sent(CLOSED_SEQ, change, SETTINGS_GENERATION);
    let closed = writes.disconnected();
    assert_eq!(
        writes.take_close_failures(),
        vec![Failure::Invocation(InvokeError::Closed)]
    );
    assert_eq!(closed.len(), 1);
    assert_eq!(
        closed[0].result.as_ref().unwrap_err(),
        &Failure::Invocation(InvokeError::Closed)
    );
    assert!(matches!(
        writes.take_errors().as_slice(),
        [taide_model::error::AppError::Forbidden(_)]
    ));
    assert!(writes.disconnected().is_empty());
    assert!(writes.take_close_failures().is_empty());
    assert!(writes.take_errors().is_empty());

    writes.sent(
        MALFORMED_SEQ,
        Change::Language("en".into()),
        SETTINGS_GENERATION,
    );
    let malformed = writes
        .response(MALFORMED_SEQ, &Ok(ResponsePayload::Json(json!(null))))
        .unwrap();
    assert_eq!(malformed.result.unwrap_err(), Failure::MalformedResponse);
    assert!(matches!(
        writes.take_errors().as_slice(),
        [taide_model::error::AppError::Internal(_)]
    ));
}

#[test]
fn 원격_설정_변경은_공용_단일필드_계약과_응답_오류_closed_소유권을_보존한다() {
    let theme = change_call(&Change::Theme("synthetic".into()));
    assert_eq!(theme.command, "settings_set_theme");
    assert_eq!(theme.args, json!({"themeId": "synthetic"}));
    let language = change_call(&Change::Language("ja".into()));
    assert_eq!(language.command, "settings_update");
    assert_eq!(language.args["patch"]["language"], "ja");
    let switch = Change::Switch(Switch::ShowSystemUsage, false);
    assert_eq!(change_call(&switch).args["patch"]["showSystemUsage"], false);
    let numeric = Numeric::ResizerThickness
        .commit(CLAMP_INPUT, 1)
        .unwrap()
        .unwrap();
    assert_eq!(
        change_call(&numeric).args["patch"]["resizerThickness"],
        RESIZER_MAX
    );

    let mut writes = PreferenceWrites::default();
    let settings = Settings::default();
    let reply = Ok(ResponsePayload::Json(
        serde_json::to_value(&settings).unwrap(),
    ));
    assert!(writes.response(UNKNOWN_SEQ, &reply).is_none());
    writes.sent(SUCCESS_SEQ, switch.clone(), SETTINGS_GENERATION);
    assert!(writes.is_pending());
    let success = writes.response(SUCCESS_SEQ, &reply).unwrap();
    assert_eq!(success.seq, SUCCESS_SEQ);
    assert_eq!(success.change, switch);
    assert_eq!(success.settings_generation, SETTINGS_GENERATION);
    assert_eq!(success.result.unwrap().language, settings.language);
    assert!(!writes.is_pending());
    assert!(writes.response(SUCCESS_SEQ, &reply).is_none());
    writes.sent(ERROR_SEQ, switch.clone(), SETTINGS_GENERATION);
    let error = json!({"code": "SYNTHETIC_SETTINGS", "message": "synthetic refusal"});
    assert_eq!(
        writes
            .response(ERROR_SEQ, &Err(error.clone()))
            .unwrap()
            .result
            .unwrap_err(),
        Failure::Remote(error)
    );
    writes.sent(BINARY_SEQ, switch.clone(), SETTINGS_GENERATION);
    assert_eq!(
        writes
            .response(BINARY_SEQ, &Ok(ResponsePayload::Binary(vec![])))
            .unwrap()
            .result
            .unwrap_err(),
        Failure::MalformedResponse
    );
    writes.sent(MALFORMED_SEQ, switch.clone(), SETTINGS_GENERATION);
    assert_eq!(
        writes
            .response(MALFORMED_SEQ, &Ok(ResponsePayload::Json(json!(null))))
            .unwrap()
            .result
            .unwrap_err(),
        Failure::MalformedResponse
    );
    writes.sent(CLOSED_SEQ, switch.clone(), SETTINGS_GENERATION);
    let closed = writes.disconnected();
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].seq, CLOSED_SEQ);
    assert_eq!(closed[0].change, switch);
    assert_eq!(
        closed[0].result.as_ref().unwrap_err(),
        &Failure::Invocation(InvokeError::Closed)
    );
    assert!(!writes.is_pending());
    assert!(writes.disconnected().is_empty());
    assert!(writes.response(CLOSED_SEQ, &reply).is_none());
}

#[test]
fn 설정_쓰기_응답은_더_새로운_서버_사건과_지연된_조회_덮기를_막는다() {
    let mut shell = ShellState::default();
    let initial = Settings::default();
    shell.sent(Read::Settings, SUCCESS_SEQ);
    assert!(shell.response(
        SUCCESS_SEQ,
        &Ok(ResponsePayload::Json(
            serde_json::to_value(&initial).unwrap()
        ))
    ));
    let submitted = shell.settings_generation();
    shell.sent(Read::Settings, STALE_READ_SEQ);
    let event_settings = Settings {
        language: "ja".into(),
        ..initial.clone()
    };
    shell.event(
        "settings:changed",
        &json!({"settings": event_settings}).to_string(),
    );
    assert!(shell.settings_generation() > submitted);
    assert!(!shell.settings_updated(initial.clone(), submitted));
    assert_eq!(shell.settings().unwrap().language, "ja");
    assert!(!shell.response(
        STALE_READ_SEQ,
        &Ok(ResponsePayload::Json(
            serde_json::to_value(&initial).unwrap()
        ))
    ));
    assert_eq!(shell.settings().unwrap().language, "ja");
    let generation = shell.settings_generation();
    shell.sent(Read::Settings, STALE_READ_SEQ);
    let next = Settings {
        language: "en".into(),
        ..initial.clone()
    };
    assert!(shell.settings_updated(next, generation));
    assert_eq!(shell.settings().unwrap().language, "en");
    assert_eq!(shell.settings_generation(), generation);
    let latest = Settings {
        language: "ko".into(),
        ..initial
    };
    assert!(shell.settings_updated(latest, generation));
    assert_eq!(shell.settings().unwrap().language, "ko");
    assert!(!shell.response(STALE_READ_SEQ, &Ok(ResponsePayload::Json(json!(null)))));
    assert!(shell.failures().is_empty());
}
