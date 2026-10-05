# M8 원격 ThemeEditor 명령·쓰기 종료 대기

## 대상과 결과

대상은 `crates/taide-model/src/theme.rs`, `native/taide-native-app/src/remote-preferences.rs`, `native/taide-native-ui/src/theme-edit.rs`, `native/taide-remote-web/src/{theme-operations,browser-workbench,browser-editor,close}.rs`와 합성 browser probe입니다. 기존 후속47 Settings provider 이식의 하위 경계이며 전체 M8 완료가 아닙니다.

- [x] 실제 native dispatcher의 중복 Create 덮어쓰기 RED → 기존 owner/guard 경로 연결 후 GREEN 1건
- [x] portable ThemeOperations 2건: typed 목록/source/base·요청 identity·중복 억제·쓰기 응답·오류·tombstone·Closed/no replay
- [x] 실제 Chrome/Wasm 연속 1건: 원본 Canvas 버튼으로 생성·쓰기 실패·명시 retry·편집·삭제·pending close·Ready
- [x] 최신 probe Wasm 정적 검사 exit0, 5.03초
- [ ] 전체 provider: Tooltip/toast·preview 적용·다른 Settings 명령·전체 tab/surface caller

## 계약

같은 `theme_save`/`theme_delete` 명령에 선택적 typed `ThemeEditorContext`를 추가했습니다. `editor`가 없으면 legacy 동작을 유지하고, null/잘못된 형식은 거절합니다. 새 명령이나 allowlist 확장은 없습니다. context는 서버 권한을 대신하지 않으며 실제 native AppState owner와 원본 supervised mutation guard에서 다시 검증합니다. 중복 Create·source 불일치·owner 변경을 거절하고 active theme 삭제의 원본 fallback/settings 사건과 follow-system 설정을 보존합니다.

Load는 목록/source 두 조회가 완료된 뒤 catalog의 builtin 출처에 따라 동일 source를 base로 재사용합니다. custom은 type의 기본 builtin을 따로 조회합니다. Save/Delete는 private 요청 identity와 owner를 유지하며 실패를 모델 AppError로 돌려줍니다. 제출된 쓰기의 응답을 정상 close가 기다리고, 오류는 consumer 전후에 관찰하여 Failed와 연결/초안을 유지합니다. 명시 cancel/retry만 허용하며 자동 replay하지 않습니다. 서버에 이미 승인된 쓰기를 browser Session Drop으로 취소했다고 주장하지 않습니다.

## 실행 근거

Cargo는 `/Users/hyunseokbyun/development/rust/cargo/bin/cargo`, CARGO_HOME은 같은 `cargo`, target은 `/private/tmp/taide-m8-menu-build.j6Efnw`이며 locked/offline으로 직렬 실행했습니다.

- native exact `원격_theme_editor는_실제_owner_중복guard와_삭제fallback을_보존한다`: 기존 일반 쓰기가 두 번째 Create를 덮어쓴 RED(20.71초/.05초), 수정 후 GREEN(27.84초/.07초)
- portable `--manifest-path native/taide-remote-web/Cargo.toml --test theme-operations --no-default-features`: 최종 영향 2 PASS(.70초/.00초)
- `check --manifest-path native/taide-remote-web/tests/browser-probe/Cargo.toml --target wasm32-unknown-unknown --locked --offline`: exit0 5.03초. 최신 canvas probe build는 직전 6.86초 성공을 재사용했습니다.
- Bun `tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built theme-operations`: 첫 실행 PASS. 실제 요청 seq1–27, save3(의도 실패1 포함)/delete1·upgrade1·최종 active socket0·Ready·frame95/pump96·quiet1.1초 추가 요청/연결/상태 변화0·page error/panic/consumer 누출0입니다.

결과는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/theme-operations-result.json`과 `theme-operations.png`입니다. 실제 화면의 이름 입력·Save/Delete·색상 목록·LivePreview를 확인했습니다. 합성 locale message-key fallback과 preview의 좁은 폭 잘림을 원본 pixel parity 완료로 세지 않습니다. 의도된 저장 오류1은 별도 `themeErrors`와 Failed(Theme) 상태에 공개됩니다. runtime 서버는 합성 in-memory fixture이며 실제 서버 권한/디스크/fallback 증거는 위 native 검사입니다.

portable fixture가 Create ID를 새 이름 slug로 추정했던 실패는 실제 Draft의 source-name slug 계약에 맞춰 정정했습니다. 잘못된 exact 필터의 실행0건은 PASS로 세지 않았습니다. builtin source 재사용 정책 변경 뒤 영향2만 실행했고 같은 성공 runtime은 반복하지 않았습니다. TS strict/Prettier·Rust exact format 성공은 직전 출력으로 재사용합니다.

## 남은 범위

전체 theme draft hot-exit 확인·scoped server handshake·무응답 deadline·전체 App/asset/package/GUI/final gates는 미완료입니다. 현재 provider2/4(50%)·기존 부모363/433(83.83%, 공수비 아님)·최종0/8·전체 ETA 보류·goal active입니다. 보호 앱·OS 설정·사용자 데이터·제품 TypeScript·기존 의존성 버전/MSRV는 유지했고 Git은 전체 M8 완료 전 수행하지 않았습니다.
