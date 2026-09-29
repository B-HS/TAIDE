# 성능 진단 locale 키 등록 누락

## 대상 파일

- `crates/taide-locale/src/service.rs`
- `crates/taide-locale/resources/locales/{en,ko,ja}.json`

## 관찰과 원인

`main` CI #87 Rust job의 `en_메시지의_모든_키는_required_message_keys에_포함된다` 검사가 `settings.performance` 미등록으로 실패했습니다. M7 성능 진단 화면의 번역 키 7개는 세 내장 카탈로그에 동일하게 추가됐지만 `MESSAGE_NAMESPACES`의 `settings` 목록에는 추가되지 않았습니다. 제품 번역 값 누락이 아니라 Rust의 전체 키 목록 계약 누락입니다.

## 수정과 검증

`settings.performance`와 Description·Frontend·Memory·Native·Read·Reset의 7개 키를 목록에 추가했습니다. `cargo test -p taide-locale --lib en_메시지의_모든_키는_required_message_keys에_포함된다`는 1건 통과, `cargo fmt --all --check`·`git diff --check`는 exit 0입니다. 전체 Rust workspace 판정은 다음 `main` CI로 합니다.
