# M8 공용 설정 컨트롤·실제 브라우저 쓰기 소유자

## 대상 파일

- `native/taide-native-ui/src/settings-controls.rs`, `src/lib.rs`
- `native/taide-native-app/src/settings-controls.rs`, `src/settings-view.rs`
- `native/taide-remote-web/src/preferences.rs`, `src/shell.rs`, `src/browser-shell.rs`, `src/browser-workbench.rs`, `src/browser-editor.rs`, `src/browser-application.rs`, `src/close.rs`, `src/lib.rs`
- `native/taide-remote-web/tests/preferences.rs`, `tests/browser-probe/src/application-probe.rs`, `application.html`
- `tools/m8-remote-rust-file-probe.ts`

## 리포트

원본 Rust Settings의 Section/Switch20/Position9/Numeric2/NumericDraft/Change를 순수 shared UI로 이동하고 native 공개 경로를 re-export로 유지했습니다. 실제 BrowserEditor→BrowserWorkbench가 typed settings_update/settings_set_theme와 seq 응답·오류/Closed·캐시/표시 갱신을 소유합니다. 신규 portable2와 변경된 native 컨트롤 영향1, 새 Chrome-Wasm 연속1을 검증했습니다. 원본 전체 Settings 화면의 브라우저 이식 완료는 아닙니다.

## 원본 대조

1. 원본 `features/settings/numeric-field.tsx`/`shared/lib/numeric-field-commit.ts`의 blur 시 저장값 복원·NaN/no-change·clamp, 기존 native의 HTML number 파싱/정수 서버값·범위를 유지했습니다. 원본 switch/position/번역 키/SettingsPatch 단일 필드도 변경하지 않았습니다. Section::BASIC/title은 native settings-view의 별도 inherent impl이었으므로 타입과 함께 shared로 이동했습니다.
2. 원본 `entities/settings/settings.query.ts`는 성공 응답의 Settings를 캐시에 적용하고 실패는 저장 실패 피드백을 표시합니다. 서버 명령은 native `remote-preferences.rs`와 Tauri gateway의 기존 settings_update/settings_set_theme이며 전체 Settings를 응답합니다. 새 브라우저 소유자는 동일 DTO/명령을 사용하고 오류 값을 지우지 않습니다. toast 실제 consumer는 아직 전체 화면 provider의 다음 연결입니다.
3. 기존 원본은 여러 독립 mutation을 허용합니다. 변경을 임의 coalesce·자동 재전송하거나 새 사용자 확인을 추가하지 않습니다. 재연결은 조회만 새로 하고 mutation은 명시 현재 값으로만 다시 제출합니다.

## 상세 계약

1. `change_call`은 Theme만 settings_set_theme/themeId이고 나머지는 동일 공용 Change.patch의 settings_update/patch입니다. 같은 serializer의 null optional 필드와 실제 단일 non-null 필드를 유지합니다. 원격 보호/입력·권한은 기존 서버 policy에 남기며 클라이언트 판단으로 서버 검증을 대체하지 않습니다.
2. PreferenceWrites는 제출된 seq/change/settings 사건 generation을 저장합니다. matched JSON은 실제 Settings로 deserialize하고 binary/null·서버 오류는 typed Failure입니다. 미등록/중복 응답은 소유하지 않으며 disconnected는 pending 전체를 한 번 Closed로 반환한 뒤 비웁니다. mutation 자동 재시도 큐는 없습니다.
3. BrowserWorkbench는 연결된 실제 Client만 사용해 제출하고, 응답을 한 번 소비한 후 mandatory consumer가 `take_preference_results`로 결과를 받습니다. 실패는 기존 cache를 바꾸지 않습니다. 성공은 동일 ShellState의 Settings를 갱신하며 오래된 settings_get pending을 제거합니다. PresentationState의 기존 설정 비교가 영향받는 theme/locale만 갱신합니다. 별도 서버 상태 복제 store는 만들지 않았습니다.
4. settings generation은 실제 `settings:changed` 사건에서만 증가합니다. 제출 뒤 새 서버 사건이 있었다면 그 이전 쓰기 응답으로 최신 Settings를 덮지 않습니다. 사건 없는 여러 정상 응답은 generation을 바꾸지 않아 계속 적용됩니다. ShellState가 배치 사건을 먼저 처리해도 새 사건이 우선입니다. generation 없는 서버 응답의 인과 시각을 새로 추정하지 않습니다.
5. BrowserApplication의 명시 close는 설정 mutation pending도 기다립니다. 오류는 결과를 consumer가 가져가기 전 관찰해 Failed(Preference)로 공개하고 연결/현재 설정을 유지합니다. cancel은 Open 복귀이며 사용자가 기존 API로 다시 제출합니다. Ready 뒤 같은 소유자 dispose이며 Drop/강제 dispose는 abort입니다. pending format/auto-save/LSP/scoped handshake/무응답 deadline까지 확장하지 않습니다.

## 검증

- [x] 새 `tests/preferences.rs` 2건 첫 PASS(build1.94초/suite .00)입니다. 공유 Change wire/값·matched/duplicate/unknown 응답·Remote/binary/null/Closed·pending 회수, 최신 서버 사건/늦은 settings_get 덮기 차단·사건 없는 연속 정상 응답을 확인했습니다. test 이름의 Closed 대문자 경고는 lowercase로 정정하고 성공 실행은 반복하지 않았습니다.
- [x] native 이동 영향 `tests/settings-controls.rs::native_settings_controls는_단일필드와_numeric저장값을_보존한다` 1건 PASS(build16.35초/suite .00/다른 host1 filtered)입니다. 최초 E0116/E0599는 Section 타입 이동 후 다른 crate에 남은 impl 원인이며 해당 순수 impl도 함께 옮긴 뒤 실패한 명령만 재실행했습니다. vendor Wry deprecation/unsafe 17경고·기존 linker unwind 경고는 요청 밖이며 숨기지 않았습니다.
- [x] production Wasm canvas strict1.12초, 새 portable lib/test strict .59초, 최신 probe/production 포함 Wasm strict .39초 exit0입니다. 최초 probe .98초와 build3.68초 뒤 runtime 실패 fixture만 수정했고 최종 probe build1.24초·공식 wasm-bindgen .2.129 생성 exit0입니다. 같은 source의 성공 검사는 다시 실행하지 않았습니다.
- [x] `bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built preferences`는 실패한 새 연속 검사 1회 정정 후 PASS입니다. 최초 최종 responses 검사에서 정상 settings:changed event를 응답 누출로 오분류했습니다. 제품에서 사건 전달을 숨기지 않고 시험 mandatory consumer가 해당 사건을 별도 count로 소비하도록 정정했습니다. 단언을 없애지 않았고 실제 event1/response 누출0을 확인했습니다.
- [x] 도구 strict TS exit0이며 최초 throw-only callback의 never 추론 TS2322는 optional 등록 callback과 존재 가드로 정정했습니다. 대상 TS/HTML Prettier와 authored Rust14 exactfmt·tracked diff whitespace exit0입니다. 새 의존성/버전/lock/MSRV·제품 TS·OS 설정/사용자 데이터/보호 앱/Git은 이 설정 경계에서 변경하지 않았습니다. 앞선 canvas 전이 lock 변경과 구분합니다.

Cargo는 기존 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`/같은 bin/cargo·`--locked --offline --target-dir /private/tmp/taide-m8-menu-build.j6Efnw`를 사용하고 직렬 실행했습니다. 최소 native 검사는 위 정확한 이름의 `--test settings-controls -- --exact --nocapture`이며 기존 host/전체 회귀 검사를 재실행하지 않았습니다. 새 synthetic localhost/격리 headless Chrome만 정확한 승격 범위에서 실행했습니다.

### 실제 Chrome-Wasm 결과

1. interval/RAF throw 조건에서 같은 BrowserApplication/Editor/Workbench가 ShowSystemUsage true→false를 한 번 제출·응답 cache에 적용했습니다. UI 값은 서버 응답 전에 optimistic 변경하지 않았습니다.
2. Language ja의 ack를 보류하고 더 늦은 외부 서버 사건 en을 보냈습니다. 같은 locale/current Settings는 en이고 옛 ja ack가 도착해도 en을 유지했습니다. mutation 성공 결과는 버리지 않고 소비했으며 실제 SettingsChanged 사건1도 집계했습니다.
3. theme 실패 응답 보류 중 명시 close는 Pending/socket1입니다. 오류를 풀자 Failed(Preference/Remote/SYNTHETIC_SETTINGS)·연결 true/socket1이며 결과 오류를 공개했습니다. cancel→명시 정상 theme 제출은 성공했습니다.
4. 응답 보류된 switch는 서버에서 이미 적용됐지만 connection cut으로 Closed를 반환했습니다. recovery1 뒤 settings 조회로 서버 최신 값을 얻었고 자동 settings_update는 0회였습니다. 사용자의 명시 새 toggle만 현재 값 반대로 제출했습니다. 응답 전 단절의 서버 적용 여부가 불확실할 수 있다는 계약을 성공/자동 replay로 덮지 않았습니다.
5. 마지막 theme 쓰기 ack를 보류하면 file pending false라도 close Pending/설정 pending true/socket1입니다. ack 후만 Ready/disposed/socket0이며 종료 후1.1초 snapshot/pump/change/요청이 불변입니다. seq1~26 고유·연속, update4/theme3·upgrade2/recovery1, 최종 pump31/change23·SettingsChanged1·응답 누출/page 오류/다른 파일·mirror 오류0입니다. 의도적 Preference 오류 2건은 별도 preferences 결과 배열에 있으며 모든 오류0이라고 주장하지 않습니다.

실제 결과는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/preferences-result.json`입니다. 최신 bindings는 canvas feature를 포함한 설정 소유자 빌드이고 선행 canvas/close/pump 결과는 당시 source의 독립 증거입니다. 새 설정 연속 검사에서 canvas 렌더를 다시 검사했다고 쓰지 않습니다.

## 미완료

- 원본 전체 Settings 화면/Catalog/theme editor/모든 설정 필드·실제 toast·tab/mount/닫기 provider의 브라우저 소비는 다음 이식입니다. 현재 test-only buttons/단일 합성 파일을 제품 UI로 세지 않습니다.
- 다른 ShellSurfaces/전체 CanvasContents·폰트/terminal/preview/전체 consumer·Rust 제품 public assets/패키징·최종 gate는 미완료입니다.
- 모든 close/scoped handshake/deadline/format/autosave/LSP/외부 writer 복구와 GUI/IME/VoiceOver/성능/security/beta/cutover/Rust99/TS제거는 기존 pending에 유지합니다.

현재 설정 소유자 경계3/3(100%), 후속47 2/4(50%), M8 active363/433(83.83%, 공수비 아님), N1~N8 0/8입니다. 전체 ETA는 남은 화면/패키징/최종 gate 공수 미확정으로 산정 보류합니다. goal active·main 직접·전체완료 전 commit/push 없음·live handle 없음입니다.

종료 직전 도구 상태 관찰: 마지막 get_goal은 blocked로 표시됐습니다. 이 작업에서 update_goal/차단 판정을 호출하지 않았으며 코드 작업은 progress이고 실행 차단은 없습니다. 위 active 표기는 해당 기록 당시 상태와 구분하며 M8 완료를 뜻하지 않습니다.
