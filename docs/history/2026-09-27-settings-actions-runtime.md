# 설정 application action과 공통 apply runtime 분리

상태: 이 변경 단위의 자동 검증 완료. M6 전체와 M7·M8은 미완료이며 남은 M 전체 완료 목표를 유지합니다.

## 대상과 변경

`crates/taide-runtime/src/settings_actions.rs`가 settings_get·settings_update·settings_set_theme와 비IPC 공통 apply_and_broadcast를 제공합니다. `src-tauri/src/domain/settings/commands.rs`는 기존 AppHandle·State·입력/응답 타입·공개 경로와 command 문서를 보존한 얇은 adapter입니다. 실제 integration observer 등록은 lib.rs의 IDE→agent→remote 순서와 기존 sequential await loop를 유지합니다.

runtime은 기존 `taide-settings::service`의 sanitize·apply_patch·save_settings·set_theme를 그대로 소비합니다. 이를 위해 이미 workspace에 있는 taide-settings의 local path 의존만 추가했고 Cargo.lock의 taide-runtime 의존 목록 한 줄이 바뀌었습니다. 외부 패키지·버전·AppServices 생성자나 State 등록은 변경하지 않았습니다.

공통 apply의 순서는 이전 설정 snapshot→sanitize→persist→live state 적용→주입 callback await→SettingsChanged입니다. 저장 오류는 live state·observer·이벤트 변경 전에 반환합니다. callback은 이전/새 Settings snapshot을 소유하고 outer AppHandle을 빌린 async future를 반환할 수 있습니다. 낮은 빈도의 설정 적용에서 새 snapshot clone 한 번을 사용해 AppHandle 없이 이 계약을 표현합니다. [Rust FnOnce](https://doc.rust-lang.org/std/ops/trait.FnOnce.html)와 [Future](https://doc.rust-lang.org/std/future/trait.Future.html) 계약을 확인했고 callback은 정확히 한 번 호출·await합니다.

공통 apply는 mutation guard를 취하지 않습니다. app_file_write·apply_settings_file·sync_download가 이미 같은 guard를 취한 뒤 기존 SettingsApplyPort를 호출하므로 재진입하지 않습니다. patch와 theme action은 기존처럼 guard를 취득한 뒤 최신 snapshot을 읽고 observer·이벤트 완료까지 유지합니다. 테마 변경은 존재 검증과 follow_system_theme 해제를 보존하며 SettingsChanged 뒤 ThemeChanged를 추가 발행합니다. patch로 theme_id를 바꾸는 기존 정책을 새로 제한하지 않았습니다.

## 검증

- 변경 전 `cargo test -p taide-settings --quiet`: 69건 통과. service 구현은 변경하지 않았으므로 이 성공 증거를 재사용합니다. 기존 platform_event_sink 29건도 통과했습니다.
- 신규 `cargo test -p taide --test settings_actions_runtime --quiet`는 runtime module 부재 E0432(exit 101)로 먼저 실패했습니다. 이후 같은 테스트 5건과 platform_event_sink 29건이 통과했습니다. 새 검사는 이미 guard를 취한 공통 apply의 비재진입, 저장/live/await/이벤트 순서, sanitization, 저장 오류의 무변경, patch·theme 잠금 대기와 최신 상태, 없는 테마와 두 이벤트 순서를 확인합니다. 잠금 대기는 [std::future::poll_fn](https://doc.rust-lang.org/std/future/fn.poll_fn.html)으로 실제 Pending을 검사하고 고정 sleep을 사용하지 않습니다.
- `cargo test -p taide-runtime --quiet`: 46건 통과. `cargo test -p taide --test app_services_runtime --test domain_boundaries --test rust_native_phase0_contract --quiet`: 각각 2·3·7건 통과. `cargo test -p taide --lib app과_sync의_설정_적용은_조립부의_공통_경로를_사용한다 --quiet`: 공통 port 조립 1건 통과. 변경 후 관련 검사는 총 93건이며 변경 전 settings 정책 69건은 별도 재사용합니다.
- `cargo clippy -p taide-runtime -p taide --all-targets -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps`, `cargo fmt --all -- --check`, `git diff --check`: exit 0. strict rustdoc 성공은 runtime에 한정되며 Tauri 전체 rustdoc 완료를 뜻하지 않습니다.

runtime normal 의존 그래프에 Tauri는 없고 bindings의 SHA-256은 `49ff1b20f9fedd9001c5443014fb86608dadac8d93dc45180030012b088742a4`로 불변입니다. 기존 공개 command 타입·문서를 보존해 bindings·manifest 변경과 생성 검사는 필요하지 않았습니다. IPC 계약 7건은 실제 현행 source와 manifest/hash를 대조했습니다.

새 테스트는 UUID로 분리한 임시 데이터 디렉터리에 synthetic 기본 설정만 쓰고 정리했습니다. 사용자 설정·시크릿·키링·앱에는 접근하지 않았습니다. 전체 workspace·frontend·다중 창/remote·재시작 GUI 실기는 이번 변경에서 실행하지 않았고 M7·M8 gate로 남습니다. GitHub B-HS/TAIDE의 to_rust_native 일반 push는 승인된 조건대로 M6 전체 완료 후 수행합니다.
