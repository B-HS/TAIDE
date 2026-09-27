# 앱 파일 application action runtime 분리

상태: 이 변경 단위 자동 검증 완료. M6 전체와 M7·M8은 미완료입니다.

## 대상과 변경

`crates/taide-runtime/src/app_actions.rs`가 app_file_read·app_file_write와 비IPC apply_settings_file을 소유합니다. 읽기는 live settings snapshot과 디스크/번들 fallback을, 쓰기는 전역 mutation guard→설정 파싱 또는 프롬프트 검증/저장→설정 callback await를 기존 순서대로 수행합니다. callback은 [FnOnce](https://doc.rust-lang.org/std/ops/trait.FnOnce.html)로 owned Settings를 받고 [Future](https://doc.rust-lang.org/std/future/trait.Future.html)의 AppResult<Settings> 완료를 기다립니다. runtime에 이미 workspace에 있는 taide-app local path 의존만 추가했고 lock의 소비 목록만 동기화했습니다.

Tauri adapter의 공개 인수·응답·문서와 SettingsApplyPort는 유지합니다. app_get_info는 Tauri 제품 package 버전 원천을 그대로 쓰며 perf_snapshot·perf_reset은 기존 process-wide 진단 registry를 소비합니다. remote_gateway는 설정 JSON을 파싱하고 gated 필드를 strip한 뒤 기존 apply_settings_file adapter를 호출하며 이 경로는 변경하지 않았습니다. sync도 같은 공통 apply를 유지합니다. 프롬프트는 설정 callback이나 이벤트를 호출하지 않습니다. 기존 동기 읽기/저장의 실행 정책을 이번 분리에서 변경하지 않았습니다.

## 검증

- 변경 전 `cargo test -p taide-app --quiet` service 9건과 app/sync의 기존 공통 포트 unit 1건이 통과했습니다. 새 action 검사는 app_actions 부재 E0432(exit 101)로 먼저 실패했습니다. 첫 구현 컴파일에서 parsed 설정 테스트 callback이 로컬 값을 빌리는 E0373이 발생해 테스트의 owned 캡처만 수정했습니다. 실패한 테스트 묶음은 수정 뒤 1회 재실행했습니다.
- `cargo test -p taide --test taide_app_extraction --test app_actions_runtime --test settings_actions_runtime --test rust_native_phase0_contract --test domain_boundaries --quiet`: 기존 경로 1건·새 action 6건·공통 설정 action 5건·IPC 7건·도메인 3건 통과, exit 0입니다. live fallback/디스크 원문, 잘못된 설정의 포트 미호출·파일 보존, guard 아래 공통 apply/event 완료 await, prompt의 guard·검증·포트 미호출, parsed 적용의 오류 전파와 remote strip/metadata adapter 경계를 확인합니다.
- `cargo test -p taide-runtime --quiet`: 51건 통과. app/sync 공통 포트 unit 1건도 이전 후 통과했습니다. 변경 후 관련 검사 총 74건이며 불변 service 9건의 이전 전 성공은 별도 재사용합니다.
- runtime/Tauri all-target clippy, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`, fmt·diff 검사 exit 0입니다. normal runtime dependency graph에 taide-app이 포함되고 Tauri는 없습니다. runtime 엄격 문서 성공을 Tauri 전체 엄격 문서 성공으로 확대하지 않습니다.

bindings/manifest diff가 없고 SHA-256 `0710cb30ffff17322796e655f2fb8c41979bbf032e2b8dacc6fe06c6e99f20e7`는 불변입니다. 기존 공개 문서를 유지해 bindings 생성을 반복하지 않았습니다. 테스트는 UUID 임시 디렉터리의 synthetic 설정/프롬프트만 사용했습니다. 사용자 파일·설정·시크릿·키링·앱 실행에는 접근하지 않았으며 전체 workspace·frontend·GUI 실기는 실행하지 않았습니다. 승인된 일반 push는 M6 전체 완료 후 수행합니다.
