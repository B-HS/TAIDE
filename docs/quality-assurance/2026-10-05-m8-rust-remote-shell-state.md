# M8 공유 Rust renderer와 실제 원격 셸 데이터 연결

## 결과와 범위

이후 theme/locale의 typed consumer·필수 system 인자·read-only listener·late/retry/recovery/dispose 실측은 `2026-10-05-m8-rust-remote-presentation.md`가 최신 정본입니다. 이 문서의 BrowserShell/UUID/shared presentation 성공은 재사용합니다. 아래 남은 항목 중 테마·언어 통신 경계는 완료했지만 실제 제품 App/canvas/provider UI·전체 consumer·자산/패키징은 여전히 미완료입니다.

후속 실제 Chrome/Wasm ShellProbe의 연속 검사는 PASS입니다. 현재 bootstrap/read/RPC/event/recovery/Drop/UUID runtime과 아래 shared presentation 이동이 정본이며, 기존 transport 성공은 반복하지 않았습니다. 제품 canvas/App/원본 모든 화면과 native/remote handoff·최종 자산 게이트는 여전히 미완료입니다.

기존 `taide-native-ui`의 shell/editor renderer를 서버 실행 의존성 없이 Wasm으로 컴파일할 수 있게 분리했습니다. 같은 ShellMutation 15종을 기존 backend RPC에 매핑하고, 실제 BrowserClient가 project/group/session/settings/layout 조회·응답·변경 이벤트·재연결을 처리해 기존 ShellSnapshot을 공급하도록 BrowserShell을 연결했습니다. 제품 브라우저 App·각 화면 consumer·canvas/GPU·최종 자산·패키징 완료가 아닙니다.

후속47의 기존 Rust public UI 준비 항목 안에서 main이 process/verify/save-docs 스킬을 직접 사용했습니다. subagent/workflow/Git·사용자 앱/데이터/OS 설정 접근은 없습니다.

## 대상과 보존 계약

- `native/taide-native-ui/Cargo.toml`, `src/{lib.rs,commands.rs,snapshot.rs}`: native-host 기본 feature는 기존 controller·dispatch·AppState read·document admission을 그대로 제공합니다. runtime/layout/project/infra/file/Tokio는 optional로 분리했습니다. renderer와 순수 intent/snapshot은 feature 없이 사용합니다. native-host가 필요한 기존 controller/workbench/document_admission 통합 테스트만 required-features로 표시했습니다.
- `crates/taide-model/{Cargo.toml,src/layout.rs}`, `crates/taide-layout/src/service.rs`: 원본 find_leaf의 DFS 본문을 모델로 이동하고 기존 서비스 경로는 re-export했습니다. 브라우저에서 host 서비스를 반입하거나 함수 복제본을 만들지 않았습니다. wasm32-unknown-unknown에서만 UUID의 js 난수 공급자를 활성화했습니다. native 난수 경로는 유지하며 실제 브라우저 UUID entropy는 아직 실측하지 않았습니다.
- `native/taide-remote-web/src/{shell.rs,browser-shell.rs,browser.rs,lib.rs}`, manifest/lock: 실제 BrowserClient에 ShellState를 연결했습니다. 명령은 원본 remote-projects/remote-layout의 인자명을 사용하고 partial patch의 null 및 KeepTab의 preview=false를 유지합니다. BrowserShell은 동일 Client로 다른 화면 RPC를 호출할 수 있고, 자신이 소유한 read response만 소비하며 mutation 응답·channel/event/lifecycle는 다른 consumer에 반환합니다.
- `crates/taide-remote-wire/src/client.rs`: is_open 조회만 추가해 동기 send 실패 뒤 같은 poll에서 조회를 disconnected 큐에 새로 쌓지 않게 했습니다. 초기 FIFO·disconnect 폐기·mutation 재전송 금지·epoch·4001·timer 정책은 바꾸지 않았습니다.
- browser/probe 루트는 기존 `native/taide-native-app/vendor/egui-input` patch를 그대로 재사용합니다. vendor 자체는 수정하지 않았습니다. probe manifest/lock은 새 library graph를 반영했고 이전 성공한 transport 브라우저 실측은 재실행하지 않았습니다.

## 실제 데이터 수명

- [x] initial Connected와 recovered Connected에서 기존 project_list/project_group_list/session_get_shell_state/settings_get과 열린 프로젝트의 layout_get을 읽습니다. 같은 read는 하나만 in-flight이고, 그동안 들어온 invalidation은 dirty 하나로 합쳐 완료 후 다시 읽습니다. 아직 없는 필수 snapshot을 빈 프로젝트 화면으로 가장하지 않습니다.
- [x] 원본 ipc-sync-provider/event-relay를 대조해 project/group/session 이벤트는 재조회하고 settings 이벤트는 전체 Settings DTO를 역직렬화해 적용합니다. settings 이벤트 뒤 늦게 오는 이전 settings read는 덮어쓰지 못합니다. layout 이벤트는 현재 revision보다 새로울 때만 재조회하며 뒤늦은 구 revision은 캐시를 덮어쓰지 않습니다.
- [x] 프로젝트 제거 시 layout·관련 dirty/error/pending 소유권을 정리해 늦은 응답을 버립니다. disconnect 시 read pending/dirty를 폐기하고 새 연결에서 새 seq로 읽습니다. 오류는 유형별로 노출하며 무한 자동 재시도하지 않습니다. explicit refresh 또는 새 event/recovery에서 재시도합니다. dispose는 실제 Client의 socket/timer/listener 종료를 사용합니다.
- [x] native tree/slot/zen/auxiliary/editor/controller의 기존 동작을 변경하지 않았습니다. 원격 browser가 native process·PTY·filesystem·Tokio runtime을 가져오지 않는 정상 Wasm 의존 그래프를 확인했습니다.

## 최소 실행 근거

CARGO_HOME=`/Users/hyunseokbyun/development/rust/cargo`, cargo=`/Users/hyunseokbyun/development/rust/cargo/bin/cargo`, target=`/private/tmp/taide-m8-menu-build.j6Efnw`입니다. 아래 최종 명령은 `--locked --offline --target-dir <target>`이며 소켓/GUI 실측 명령은 이번 경계에서 실행하지 않았습니다.

- [x] UI 최초 `check --target wasm32-unknown-unknown --no-default-features --lib`는 UUID의 Wasm 난수 feature 부재 compile_error로 exit101이었습니다. target 전용 js feature 추가 뒤 `clippy --manifest-path native/taide-native-ui/Cargo.toml --target wasm32-unknown-unknown --no-default-features --lib -- -D warnings`는 exit0·7.87초입니다. 성공한 단독 UI 검사는 반복하지 않았습니다.
- [x] `test --manifest-path native/taide-native-ui/Cargo.toml --test workbench --test controller --test editor_surface -- --exact`와 아래 세 이름:3 PASS, build14.95초·controller0.00초/editor0.02초/workbench0.07초입니다.
  - 실제_레이아웃_렌더는_두_프로젝트와_zen_보조창_범위를_분리한다
  - 실제_controller는_runtime_명령과_외부_이벤트를_갱신하고_소유_worker를_회수한다
  - 실제_editor_surface는_입력_선택_ime_commit과_stale_거절을_연결한다
- [x] `test --manifest-path native/taide-remote-web/Cargo.toml --test shell`:최초 build8.60초에 mutation 매핑1 PASS·공통 fixture의 snapshot3 FAIL입니다. 합성 PaneNode/ShellSlotTree JSON이 실제 계약의 node 대신 kind를 사용해 역직렬화가 거절된 fixture 오류이며, 제품 코드를 느슨하게 바꾸지 않았습니다. fixture의 두 필드와 실패 경계 assertion만 수정한 뒤 실패한 세 이름만 --exact로 실행해 build0.36초·suite0.00초·3 PASS입니다. 이미 통과한 mutation 검사는 반복하지 않았습니다.
- [x] production Wasm `clippy --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --lib -- -D warnings`:exit0·0.85초입니다. 이후 BrowserShell이 raw Client를 노출하지 않고 invoke만 제공해 response/event 소비 소유권을 유지하고 is_connected가 실제 open phase도 확인하도록 공개 경계를 정리했습니다. 해당 변경의 최종 같은 Wasm strict는 exit0·0.11초입니다. 변경 전 성공을 무조건 반복한 것이 아니라 새 browser-only API를 컴파일한 검사입니다.
- [x] `clippy --manifest-path native/taide-remote-web/Cargo.toml --lib --tests -- -D warnings`:exit0·4.51초입니다. 이후 테스트의 동일값 상수명 분리와 원본 schema 상수 사용은 의미 동등하며 성공을 재사용하고 Rust fmt만 확인했습니다.
- [x] 실제 native App의 `clippy --manifest-path native/taide-native-app/Cargo.toml --lib --bins --tests -- -D warnings`:exit0·9.85초입니다. 기본 native-host와 기존 공개 서비스 경로의 전체 caller를 확인했습니다. 기존 Wry17개 경고는 억제하지 않았습니다. 원본 constructor/Exit/server 성공 실측은 반복하지 않았습니다.
- [x] browser Wasm `cargo tree -e normal --prefix none`의 runtime/infra/file/project/layout/Tokio/PTY/terminal/IDE/reqwest 의존성은0입니다. egui는 같은 local input patch, wasm-bindgen0.2.129/web-sys0.3.106·순수 model/editor/UI/wire만 연결됩니다. 실제 renderer가 사용하는 라이브러리를 제외하거나 가짜 의존성으로 치환하지 않았습니다.
- [x] 직접 수정한 Rust9파일 exact rustfmt --check는 exit0입니다. root model/service의 기존 대규모 포맷은 유지하고 본문 이동만 diff로 확인했습니다.
- [x] 최종 tracked 관련 diff --check는 exit0입니다. 미추적 관련18파일의 개별 no-index --check는 출력 없이exit1(새 diff 있음)이며 whitespace 문제0입니다. 실행 중 Cargo/브라우저 handle은 없습니다.

root/native App의 기존 package 버전과 lock/MSRV는 이번 경계에서 변경하지 않았습니다. 독립 browser/probe lock에는 실제 공유 renderer/editor/model과 그 전이 의존 package69개를 추가했습니다. 해당 새 그래프의 UUID1.26.1 등은 offline 캐시에서 Rust1.95 호환으로 해석됐으며 기존 App의 UUID1.24.0을 업그레이드한 것이 아닙니다. standalone UI lock에는 선행 공유 wire의 local edge만 추가했습니다.

Cargo feature와 UUID Wasm 설정은 [Cargo 공식 기능 문서](https://doc.rust-lang.org/cargo/reference/features.html), [UUID 공식 문서](https://docs.rs/uuid/latest/uuid/), DTO 경계는 [serde_json 공식 from_value 문서](https://docs.rs/serde_json/latest/serde_json/fn.from_value.html)를 확인했습니다.

## 남은 검증과 제품 경계

- [x] 새 BrowserShell의 실제 browser socket→DTO/snapshot·새 UUID 공급자 runtime을 격리 환경에서 확인했습니다. 이전 BrowserClient의 실측을 대신 사용하는 것이 아니라 아래 신규 실제 consumer 연속 검사입니다.
- [ ] 실제 browser App/canvas·각 원본 surface의 remote read/write/channel/event/error/lifetime·locale/theme/font·기존 terminal parser/query handoff를 연결합니다. 현재 ShellState는 열린 project layout을 읽고 성공 응답 때 snapshot을 복제하므로 전체 제품 크기/고빈도 layout pressure 성능 증거가 아닙니다. 필요 시 원본 mount 범위를 기준으로 조정하며 임의 숨김 policy/별도 byte cap을 추가하지 않습니다.
- [ ] 실제 Rust 제품 UI·generated binding/bundle manifest·Catalog/packaging과 GUI·failed-close 재연결·전체 성능/beta/cutover/Rust99%/설치/rollback을 완료합니다. 이번 단계에 no-op ShellSurfaces나 test-only renderer를 제품 UI로 넣지 않았습니다.

후속47은2/4(50%)·전체363/433(83.83%, 비가중·공수비 아님)·최종 N1~N8은0/8입니다. 실제 제품 UI 게이트는 아직 체크하지 않았습니다. 전체 ETA는 남은 화면/실기/배포 공수 미확정으로 산정 보류·goal active·전체 완료 전 commit/push 없음입니다. 보호 egui spike bundle·OS/입력기/VoiceOver/Keychain/사용자 데이터·제품 TS·vendor는 보존합니다.

## 실제 BrowserShell 연속 runtime (후속 정본)

대상은 test-only `native/taide-remote-web/tests/browser-probe/src/shell-probe.rs`, `shell.html`과 `tools/m8-remote-rust-shell-probe.ts`입니다. probe는 실제 BrowserShell을 소유하고 model Settings::default를 직렬화해 합성 서버에 공급하며, same PinTab mutation과 실제 ProjectId::new를 호출합니다. 제품 serializer/Client/소비자를 fixture 대역으로 바꾸지 않았습니다. server의 payload/재시작은 합성 계약 상대방이고 실제 auth/정책/backend/GUI 검증을 주장하지 않습니다.

- [x] `cargo build --manifest-path native/taide-remote-web/tests/browser-probe/Cargo.toml --offline --target-dir <target> --target wasm32-unknown-unknown --lib`:최초9.36초 exit0입니다. 같은 공식CLI0.2.129로 `--target web --no-typescript --out-name probe --out-dir /private/tmp/taide-m8-wasm-tools.h2xBQB/shell-built <wasm>`를 생성했습니다. 기존 transport의 `/built` 결과를 덮어쓰지 않았으며 제품 자산에 넣지 않았습니다. 최초 probe Wasm strict는0.45초 exit0입니다.
- [x] 최초 연속 검사에서 bootstrap·pin·settings stale read 방어·recovery 조건은 확인됐으나 마지막 총 요청 예상17/실제18 mismatch로 실패했습니다. ShellState::refresh가 기존 cached project layout도 재연결에서 조회하는 원본 전체 invalidation 계약을 간과한 fixture 기대값 오류였습니다. 실제 소스와 seq1~18 흐름을 대조했고 제품 코드/갱신을 바꾸지 않았습니다. 총수만18로 바꾸지 않고 명령18개 순서를 명시했으며 close 뒤 layouts0/revision null/pinned null 확인을 추가했습니다. 그때 변경된 probe만 build0.43초 exit0·binding 재생성·TS strict exit0 후 실패한 연속 경계를1회 재실행해 PASS입니다. 통과한 이전 transport/core/native/state 검사는 다시 실행하지 않았습니다.
- [x] `bun tools/m8-remote-rust-shell-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/shell-built`:최종 exit0·연속1 PASS입니다. 전용 localhost와 새 headless Chrome context·mock Keychain·service worker 차단·downloads 차단이며 기존 앱/브라우저 profile/사용자 데이터/OS 설정은 사용하지 않았습니다. 이미 확인된 sandbox localhost EPERM에 대해 해당 검사만 좁은 실행 권한을 사용했습니다.
- [x] boot는 connected/ready true·project1/layout1·focused project·revision1·pinned false·hideStatus true·failures0입니다. same PinTab RPC1회 후 revision2/pinned true이며 SettingsChanged로 hideStatus false가 된 뒤 지연된 이전 Settings 응답(seq11)이 이를 덮어쓰지 않습니다. UUID는 실제 js 공급자를 통한 v4 형식입니다. 난수 품질 전체 통계/보안 entropy를 별도로 측정한 주장은 없습니다.
- [x] restart 후 recoveries1·project/layout0·focus/revision/pinned null·hideStatus false·failures0·wake22입니다. seq18의 옛 project layout 응답이 새 snapshot을 복원하지 않습니다. 명령18개·seq 재사용0, 최종 Rust owner `.free()` Drop 뒤 활성 socket0·1.1초 quiet에서 upgrade2/요청18 불변·JS page error0입니다. 명시 dispose의 transport 성공은 기존 근거를 재사용하며 이번 경계는 실제 BrowserShell Drop입니다.

결과 JSON은 `/private/tmp/taide-m8-wasm-tools.h2xBQB/shell-built/shell-result.json`입니다. boot/change/recovery는 상태 전환이 다른 한 세션의 검사이며 같은 성능을3회 계측한 것이 아닙니다. 총 browser 실행 초는 따로 계측하지 않았으므로 tool poll 시간/빌드 시간에서 추정하지 않았습니다. Rust2개 exactfmt·TS/HTML Prettier·최종 단독 strict TS는 exit0입니다. 최초 정정 전 파일은 실패였으므로 최종 성공 근거와 구분합니다.

wasm-bindgen의 struct/static method/free는 [공식 exported Rust struct 문서](https://wasm-bindgen.github.io/wasm-bindgen/contributing/design/exporting-rust-struct.html), fixture Pub/Sub는 [Bun 공식 WebSocket 문서](https://bun.sh/docs/runtime/http/websockets), 격리 context는 [Playwright Browser 문서](https://playwright.dev/docs/api/class-browser)를 확인했습니다.

## 같은 화면의 presentation 공유 (후속 정본)

native `src/presentation.rs`의 색상/테마 token·EditorAppearance·번역 문자열 치환·font step/clamp 본문을 `native/taide-native-ui/src/presentation.rs`로 옮겼습니다. native public/crate 경로는 re-export하고 기존 native 테스트는 원래 위치에서 그대로 shared 구현을 호출합니다. 테스트가 사용하는 font/line-height/padding 상수도 같은 원본을 가져오며 테스트용 복제값을 만들지 않았습니다. 오류 문자열·투명/short/RGB/RGBA 범위·font/indent·fallback key와 치환 의미를 유지했습니다. OS/FS/runtime 의존성과 새 패키지/버전/manifest 변경은 없습니다.

- [x] 변경된 shared presentation의 browser `cargo clippy --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir <target> --target wasm32-unknown-unknown --lib -- -D warnings`:0.41초 exit0입니다. 같은 egui-input/DTO/renderer 그래프에 실제 구현을 컴파일하며 native UI를 browser용 별도 함수로 복제하지 않았습니다.
- [x] actual native `cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir <target> --lib -- --exact theme_color_picker::tests::native_theme_color_picker는_hex_hsv와_shared_color_범위를_보존한다 presentation::tests::font_keymap은_전역단계_저장실패_이벤트와_editor표시만_변경한다`:2 PASS·build25.20초/suite0.09초입니다. 해당 이동의 parse 범위와 실제 font keymap/Settings/실패 보존/editor 표시만 덮었습니다. 기존 다른 browser/core/native 성공은 반복하지 않았습니다. 기존 Wry17 경고와 lib-test __eh_frame 16MiB linker 경고는 유지했습니다.
- [x] 이동 후 실제 native App `clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir <target> --lib --bins --tests -- -D warnings`:4.82초 exit0입니다. 이번 이동의 전체 caller 영향이며 같은 변경 상태를 반복한 검사가 아닙니다. authored Rust5파일 exactfmt와 해당 TS/HTML Prettier check도 exit0입니다.
- [x] 해당 경계의 최종 tracked docs diff --check는exit0·미추적 관련11파일의 no-index whitespace 오류0(exit1은 새 diff 있음)입니다. 실행 중 Cargo/Chrome/server handle은 없고 전체 M8 goal은active입니다.
- [ ] 실제 remote theme/locale consumer를 연결합니다. 원본 theme/locale query/provider·system-appearance와 backend를 대조해 `theme_get_current`의 필수 `systemTheme`, `locale_get_current`의 필수 `systemLanguage`를 확인했습니다. browser의 matchMedia/navigator 읽기·change listener 수명, settings/theme/event/recovery·stale/pending 소유권, 로딩/오류·원본 retry/locale/theme 적용이 다음입니다. null args/임의 내장 theme나 번역 fallback으로 readiness를 가장하지 않습니다. 현재 공유 함수가 준비됐다는 이유로 실제 모든 AppSurfaces/ThemeProvider/canvas/UI 완료를 주장하지 않습니다.
