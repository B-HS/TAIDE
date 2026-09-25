# AI 요청 취소 후 늦은 완료가 새 요청을 제거하는 경쟁 조건

## 재현

같은 `(owner, requestId)`에서 첫 요청을 시작·취소한 뒤 두 번째 요청을 시작합니다. 첫 요청의 취소 처리가 뒤늦게 `finish()`를 호출하면 기존 구현은 키만으로 항목을 제거해 두 번째 요청의 취소 송신자를 지웁니다. `cargo test -p taide --lib 취소된_요청의_늦은_완료는_새_요청을_제거하지_않는다 --quiet`가 구현 전 `assertion failed: store.begin("main", "req-1").is_none()`로 실패(exit 101)했습니다.

## 원인과 수정

기존 `AiRequestStore::finish`는 시작 시점의 요청을 식별하지 않고 `(owner, requestId)`만으로 현재 항목을 제거했습니다. runtime으로 이전한 레지스트리는 시작마다 `AiRequestToken`을 발급하고 `Arc::ptr_eq`가 현재 항목과 일치할 때만 완료 정리를 수행합니다. 취소는 기존과 같이 해당 owner의 송신자를 제거하고 one-shot 신호를 보냅니다. AI command의 provider 해석·secret 취득·원격/IPC 시그니처는 변경하지 않았습니다.

## 검증 범위

runtime 정책의 중복 시작, 완료 뒤 재사용, 취소 신호, 모르는 ID, 다중 owner 격리와 늦은 완료 회귀를 확인합니다. 전체 Tauri lib·Phase 0 IPC 계약과 정적 검사 결과는 `docs/PROCESS.md` M6-CM에 기록합니다. 실제 동시 AI provider 호출·다중 창 취소 GUI 실기는 M7에 남깁니다.
