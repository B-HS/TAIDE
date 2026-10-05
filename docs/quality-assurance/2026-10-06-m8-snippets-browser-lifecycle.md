# M8 Snippets actual Chrome 연속 수명

## 대상·계약

생산 공용 Settings/Snippets·BrowserEditor·Canvas·typed SnippetOperations를 기존 `settings-probe.rs`와 `tools/m8-remote-rust-file-probe.ts`에서 실행했습니다. 별도 제품 UI/JS fallback/강제 상태 쓰기 없이 실제 Canvas controls·pointer hit·키 입력을 사용합니다. localhost 합성 데이터와 격리 Chrome만 사용하며 제품 TS·사용자 프로젝트·OS/Keychain/보호 앱/의존성/lock/MSRV는 유지합니다. 하네스에 전역 bootstrap snippet_list와 실제 DTO camelCase 파일·저장/삭제 응답·held/rejected mutation을 추가했습니다.

## 결과

- [x] 한 Chrome 세션의 최종 연속 검사 PASS입니다. 빈 root catalog→rust.json 생성→name/prefix/body 입력→저장 hold→Close Pending→거절/Failed(Snippet)/dirty 초안·parseError Toast→Cancel→명시 retry/clean·saveSuccess·cache 갱신을 확인했습니다.
- [x] Back→실제 Snippets 목차→Manage 재진입의 fresh cache 추가 read 0, 비검색 Picker의 actual picker-dialog focus→Home/Enter→global 이름 자동 focus/입력→전역 파일 생성·Alert Cancel autofocus/삭제→미저장 prefix의 폐기 Cancel 보존·Confirm 폐기→다시 열린 원래 prefix를 확인했습니다.
- [x] 마지막 body 저장 hold→Settings unmount→close Pending→응답 승인→Ready, 합성 서버에 revised body 저장, socket 0입니다. seq 1~23 고유·snippet_save 5/거절 1·snippet_delete 1·snippet_list 6입니다. 마지막 pump 360/frame 377이며 종료 뒤 1.1초 snapshot/요청 불변·추가 연결 0·page error 0·panic false·unclaimed response 0입니다. 성공 이후 동일 검사는 반복하지 않았습니다.
- [x] 실제 종료 실패에서 빠진 Toast를 생산 응답 처리 단계의 일회성 Notice 배출로 수정했습니다. [버그 기록](../bug/2026-10-06-remote-snippet-close-feedback.md)이 정본이며 새 shared UI 1 PASS(build 2.40초/suite .11초·filtered 9), 수정 후 probe build 2.98초/정상 Canvas Wasm check .83초 exit 0·경고 0, 최종 TS strict exit 0입니다. 선행 probe build 6.47초와 global input/AX 성공은 재사용합니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo build --manifest-path native/taide-remote-web/tests/browser-probe/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
/private/tmp/taide-m8-wasm-tools.h2xBQB/wasm-bindgen-0.2.129-aarch64-apple-darwin/wasm-bindgen --target web --out-dir /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built --out-name probe /private/tmp/taide-m8-menu-build.j6Efnw/wasm32-unknown-unknown/debug/taide_remote_browser_probe.wasm
/Users/hyunseokbyun/development/js/bun/bin/bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built settings-snippets
```

원자료는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/settings-snippets-result.json`이며 같은 세션 screenshot은 `settings-snippets.png`입니다.

## 선행 실패 구분

최초 sandbox localhost bind 실패는 권한을 받은 격리 실행으로 해결했습니다. 연속 검사의 선행 실패는 없는 목차 키, raw Toast description 기대, 실제 close feedback 누락, Back 뒤 화면 밖 Manage 좌표, Popup focus 전 Home/Enter, 비검색 Picker에 검색 focus를 기대한 fixture입니다. 실제 UI/source에 맞춰 관련 부분만 정정했으며 부분 실행과 마지막 완전 성공을 혼동하지 않습니다. 키 입력 전 actual picker-dialog focus를 기다립니다. 생산 Picker 동작/source를 fixture 때문에 변경하지 않았습니다.

## 남은 범위

합성 locale 메시지의 fallback 키와 합성 palette 화면이므로 이 screenshot을 원본 번역/색상/글꼴 픽셀 일치로 주장하지 않습니다. 원본 전체 Button·label 배치/스타일·포커스/AX/theme/DPI, 실제 자동완성 UI/삽입, 전체 native shutdown·나머지 Settings/App/assets/최종 gate와 사용자 담당 마지막 CJK/VoiceOver는 미완료입니다. Snippets 1/4(25%)·provider 2/4(50%)·M8 363/433(83.83%)·최종 0/8·전체 ETA 산정 보류·goal active·main 직접·전체 완료 전 Git 없음·live handle 없음입니다.
