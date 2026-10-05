# M8 공용 키바인딩 편집기·실제 browser 소비자

## 대상 파일

- `native/taide-native-ui/src/keymap.rs`, `keybinding-catalog.rs`, `keybinding-capture.rs`, `keybinding-search.rs`, `keybinding-editor.rs` 및 두 JSON 정본입니다.
- `native/taide-native-app/src/keymap.rs`, `keybinding-editor.rs`, `keybinding-search.rs`, 원본 test-body 두 파일과 `lib.rs`입니다.
- `native/taide-remote-web/src/browser-editor.rs`, `browser-workbench.rs`, `canvas.rs`, 공용 `settings-controls.rs` 및 관련 검사·probe·export 도구입니다.
- shared UI·native App·remote web·browser probe의 manifest/lock 의존성 경계입니다.

## 리포트

기존 native 편집기와 입력 코어를 동일 Rust 생산 소스로 공유했습니다. browser에 별도 편집기·JS fallback·시험 전용 생산 소비자를 만들지 않았습니다. 실제 Settings 버튼은 일회성 출력 소비 후 같은 편집기를 열며, 실제 Canvas 전역 후처리에서 modal·Tooltip·Toast를 표시하고 기존 typed 설정 쓰기 및 종료 drain을 사용합니다.

## 상세

1. native 생산 경로는 re-export입니다. private 회귀검사만 test-only same-source include를 사용합니다. 원래 Keymap/Editor test-body는 바이트 동일 대조 PASS이며 Catalog/Search fixture는 기존 native 파일을 그대로 참조합니다. 두 JSON 값도 이동 전과 동일합니다. 두 export 도구는 공용 JSON 정본을 가리킵니다.
2. 기존 `web-time`의 공식 설치 원천에서 browser `Instant::now`와 native `std::time` re-export 계약을 확인했습니다. native tests에는 동일 std Instant를 전달하며 browser 생산 코드는 Web Instant를 사용합니다. 입력 route는 설치된 egui `Context.os()`와 eframe user-agent 설정을 사용합니다. macOS/Windows의 정·반대 modifier를 새 단일 검사로 확인했습니다.
3. ICU normalizer/locale core 2.3.0 및 collator 2.3.1은 이미 native가 사용하는 exact 버전입니다. 공용 구현의 기존 Unicode·정렬 동작 때문에 그 의존성을 공유했습니다. 새로운 라이브러리 선택·버전 업그레이드 작업이 아닙니다. shared UI lock은 기존 ICU 2.2 하위 그래프에서 필요한 2.3 그래프로 정합하며 관련 tinystr/writeable/zerovec 등의 변경이 있습니다. remote/probe lock에는 필요한 기존 ICU 그래프가 추가됐습니다. native의 기존 버전과 MSRV는 유지합니다.
4. 실제 BrowserEditor는 편집기 상태를 소유하고 system-language collation·host 플랫폼·현재 preview theme를 전달합니다. Canvas가 frame 시작/끝을 소유하며 하위 Settings/file 화면은 modal 동안 disabled, underlying raw-input hook도 차단합니다. 기존 편집기의 raw capture/검색/행/저장 출력을 사용합니다. 연결 종료/Failed 결과·Cancel·명시 재시도는 기존 PreferenceWrites/close 경계를 사용하며 자동 replay하지 않습니다.
5. 읽기 inspection은 feature-gated이며 상태·현재 controls/실제 pointer hit만 제공합니다. 쓰기·입력 주입 우회 API를 추가하지 않습니다. 브라우저의 전체 전역 명령 dispatch·모든 window/terminal 소비자는 이번 세부 완료와 구분합니다.

## 검증

- [x] 이동 후 native lib/bins/tests check10.20초 exit0입니다. 이동 때문에 생산 아이콘 alias의 unused 경고가 생겨 test-only로 고쳤습니다. 이후 Change/Canvas/inspection 영향 native 최종 check7.26초 exit0이며 기존 Wry17경고 외 새 경고가 없습니다.
- [x] 기존 native keybinding 영향21건 PASS(build24.81초/suite0.71초)입니다. 전체 등록순서/override/충돌/NFC·UTF16·3언어 정렬/capture/modal/동적 row focus/전체 Tab/Tooltip/실제 host 쓰기를 확인합니다. 성공한 같은 검사를 다시 실행하지 않았습니다.
- [x] 새 portable host-platform route1 PASS(13.75초/0.01초), 새 typed preference/ack/Closed1 PASS(14.24초/0.00초)입니다. native root의 전체 기존 입력 검사 성공을 주장하지 않습니다.
- [x] 최초 공용 portable Wasm check13.58초, 실제 normal Canvas Wasm check26.23초, probe build26.15초 및 최종 pointer-hit inspection 영향 build2.95초 exit0입니다. 최신 바인딩 생성 완료 뒤 최종 연속 검사를 실행했습니다.
- [x] 새 실제 Chrome 연속1 GREEN: 요청 seq1~15 고유·연속, 설정 쓰기4회입니다. 실제 목차/버튼→검색 자동 focus→quick-open 검색→Cmd+Shift+Y 캡처/확정→ack→Reset→ack→Unbind 보류→종료 Pending→거절/Failed 및 기존 `[]` 보존/Toast→Cancel→명시 Unbind 재시도→ack drain→Ready/socket0입니다. 마지막 설정은 quick-open의 빈 바인딩이며 종료 후1.1초 동안 snapshot/요청 불변·page errors0·실패 누출0·panic false입니다.
- [x] TS strict/대상 도구3개 Prettier exit0입니다. Rust18개 exactfmt 중 새 모듈 선언 순서1건만 수정 후 실패 lib1개 검사 exit0이며 성공한17개는 재사용합니다. 두 native test-body/JSON 동등 대조와 `git diff --check`를 확인했습니다.

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --lib keybinding --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-native-ui/Cargo.toml --locked --offline --no-default-features --test keymap-platform --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --test preferences 키바인딩_변경 --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
/Users/hyunseokbyun/development/js/bun/bin/bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built settings-keybindings
```

최종 원자료는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/settings-keybindings-result.json`입니다. 첫 연속 실행은 목차 클릭 직후 화면 밖 초기 버튼 좌표로 click timeout, 두 번째는 Reset ack 직후 이전 controls 좌표로 Unbind가 클릭되지 않아 pending timeout이었습니다. 둘 다 실제 bounds·pointer hit 확인으로 fixture를 수정했으며 제품 쓰기 동작을 바꾸지 않았습니다. 첫 실행의 쓰기0/앱 오류0과 두 번째의 assign/reset 두 쓰기를 최종 완전 성공과 혼동하지 않습니다. 첫 바인딩 생성 호출은 비동기 handle을 반환한 뒤 최초 실패 검사를 시작했으므로 그 실행을 최신 asset 증거로 사용하지 않습니다. 최종 실행은 생성 handle exit0 후 시작했습니다. 중간 apply_patch 한 건은 TS 포맷이 다른 문맥으로 실패했으며 파일 상태를 확인하고 정확한 문맥으로 적용했습니다.

## 남은 범위·상태

- [ ] 전체 browser App의 전역 명령/단축키 dispatch 및 모든 file·terminal·window 입력 소비자는 남습니다. 이번 공유/실제 modal 동작을 그 전체 완료로 쓰지 않습니다.
- [ ] 원본 전체 시각/AX/DPI/theme/Presence·ordered mixed 입력·native 실제 App 왕복·실제 CJK/VoiceOver는 남습니다. 합의대로 OS 설정을 건드리지 않고 실기는 최후 순위에 둡니다.
- [ ] Snippets/LSP/AI/Plugins/Sync/Remote/조건부 Performance와 전체 App/assets/cutover/Rust99%·최종 N1~N8은 남습니다.

이번 세부4/4(100%)·Editor/Terminal3/4(75%)·provider2/4(50%)·전체363/433(83.83%, 부모 집계에 중복 합산하지 않음)·최종0/8·전체 ETA 산정 보류입니다. goal active·main 직접·전체완료 전 Git 없음·live 검사 없음입니다. 제품 TypeScript·보호 앱·OS·사용자 프로젝트·Keychain·원격 저장소는 변경하지 않았습니다. 다음은 같은 N5-S1의 실제 Snippets/나머지 Settings renderer와 소비자입니다.
