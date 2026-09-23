# rust-native Phase 0 IPC 계약 fixture

`ipc-contract-manifest.json`은 `src-tauri/tests/rust_native_phase0_contract.rs`가 원천 코드와 양방향 비교하는 IPC 계약 기준선입니다. 스키마와 실측값은 `docs/acknowledge/2026-09-23-rust-native-phase0-contract-baseline.md` §5에 기록돼 있습니다.

- `ordering`은 `source-declaration-order`입니다. event는 `events.rs`, 나머지 목록은 각 등록·정책 원천의 선언 순서를 그대로 쓰며 `collect_events!`에는 등록 집합 일치만 요구합니다.
- `generatedBindings.sha256`은 `src/shared/api/bindings.ts` 전체 바이트의 SHA-256입니다.

## 갱신 절차

1. 원천(`src-tauri/src/lib.rs`, `events.rs`, `error.rs`, `domain/remote/dispatch.rs`)을 변경합니다.
2. 목록 항목을 해당 원천 선언 순서대로 manifest에 반영합니다.
3. `bindings.ts` digest를 다시 계산해 `generatedBindings.sha256`에 넣습니다.

```sh
shasum -a 256 src/shared/api/bindings.ts
```

4. `cargo test -p taide --test rust_native_phase0_contract`로 집합·순서·해시가 모두 맞는지 확인합니다.

manifest를 손으로 고치지 않고 생성 스크립트를 두지 않는 이유는, 생성기가 있으면 원천 파싱 규칙이 테스트와 생성기 두 곳으로 갈라지기 때문입니다. 검증 규칙의 단일 출처는 테스트 파일입니다.
