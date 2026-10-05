# M8 Editor·Terminal 실제 화면·목록 소비

이 문서는 최초 실제 UI/목록·공유 캐시 연결의 당시 source 정본입니다. 이후 glyph/선택 trigger·원본 라벨/목록/검색 스타일·browser autofocus의 최신 결과는 `2026-10-05-m8-settings-font-preview.md`입니다. 아래 glyph/정렬 미완료 표시는 그 이전 이력이며 전체 시각/접근성/부모 gate는 계속 미완료입니다.

## 대상 파일

- `native/taide-native-ui/src/settings-code-view.rs`, `settings-view.rs`, `settings-controls.rs`, `settings-code-controls.rs`, `settings-resources.rs`, `command-score.rs`, `lib.rs`, `license-command-score.txt`
- `native/taide-remote-web/src/settings-resources.rs`, `browser-workbench.rs`, `browser-editor.rs`, `Cargo.toml`, `tests/settings-resources.rs`
- `native/taide-native-app/src/host.rs`, `application.rs`
- `native/taide-remote-web/tests/browser-probe/src/settings-probe.rs`, `tools/m8-remote-rust-file-probe.ts`

## 리포트

공유 Section::BASIC에 Editor·Terminal을 원본 순서로 추가해 실제 화면은 6개 섹션입니다. 원본의 26개 스위치·5개 숫자 입력·눈금/셸 초안·4개 typed 선택 목록·2개 독립 폰트 Picker·셸 profile 선택을 기존 Change→SettingsPatch→동일 설정 쓰기 경로에 연결했습니다. 다른 저장 경로와 셸 실행 동작을 추가하지 않습니다. 아직 원본 전체 픽셀·폰트·접근성 일치 완료가 아닙니다.

font_list와 shell_profiles는 Theme/Locale Catalog와 독립적으로 로드합니다. native는 기존 폰트 서비스의 캐시된 목록과 기존 terminal_actions::shell_profiles를 사용하며, 폰트 스캔은 감독된 blocking worker에 남습니다. portable/Wasm에 OS/PTY/폰트 스캔 의존성을 추가하지 않습니다. browser는 같은 socket의 seq를 정확히 소비해 typed 결과·중복/unknown 응답·binary 오류·Closed를 처리하며 읽기를 자동 재전송하지 않습니다. 실패 목록은 원본 query의 기본 data=[]처럼 빈 목록이고 raw 오류는 진단에 보존합니다.

설치된 원본 font/terminal query는 staleTime Infinity이고 QueryClient는 gcTime 600000ms·retry0입니다. query-core의 optionalRemove/removeObserver/shouldLoadOnMount/fetch finally와 실제 IPC의 AbortSignal 미소비를 확인했습니다. 같은 Views 안의 마운트들이 목록과 진행 요청을 공유하고, 마지막 화면이 해제돼도 진행 응답을 캐시에 받아 다음 화면이 사용합니다. 성공한 목록은 재마운트 시 다시 조회하지 않으며 실패한 목록만 새 마운트에서 조회합니다. ThemeEditor 뒤로 원본 섹션이 숨는 동안은 목록 observer로 세지 않고 재진입 시 Picker·초안을 초기화합니다.

폰트/셸 목록 데이터는 Arc slice로 공유해 매 paint에서 전체 문자열/목록을 복사하지 않습니다. 각 종류의 비활성 10분 만료를 독립적으로 판정하며 진행 요청은 만료시키지 않고 정산 뒤 만료 기준을 다시 잡습니다. 만료 판정은 다음 관찰/프레임에서 수행하는 lazy 방식입니다. 관찰값 재사용 경계는 검사했으나 10분 시점의 물리적 메모리 회수 타이머를 구현하거나 실측했다고 주장하지 않습니다.

이전 구현의 native Settings owner 활성 guard는 이 두 전역 읽기만 application shutdown guard로 정정했습니다. 시작한 목록 읽기의 결과는 첫 화면이 닫혀도 공유 캐시로 갈 수 있어야 하기 때문입니다. 설정 쓰기·theme 변경·폴더·remote allowlist/인가·browser 요청 생성 시 active owner 검증은 그대로 유지했습니다. 닫힌 owner에 변이를 실행하는 권한을 추가하지 않았습니다.

폰트 검색은 원본 설치된 cmdk1.1.1 command-score의 UTF-16·case/gap/boundary/transposition 점수와 정렬 규칙을 포팅했습니다. 실제 설치본 `node_modules/cmdk/dist/chunk-NZJY6EH4.mjs`와 LICENSE.md가 근거이며 14개 점수 사례를 비교했습니다. attribution/MIT 전문은 `license-command-score.txt`에 보존합니다. 배포 third-party notice 통합은 최종 패키징 게이트가 남습니다. system-default도 원본 필터 대상이고 검색어 sm은 Synthetic Mono와 system-default 두 항목을 모두 매치합니다.

## 검증

- [x] 실제 공유 Views/ResourceReads 통합 단일 검사: 새 UI 첫 최종 PASS(.36초/.15초)와 캐시 변경 영향 PASS(.49초/.14초, 같은 명령에 신규 캐시 검사 포함해 2 passed/0 failed). 실제 ToC 이동·스위치·폰트 검색→Enter·옵션 End→Enter·셸 profile 선택, 두 독립 응답·중복/unknown·잘못된 종류·binary 오류·Closed를 확인했습니다.
- [x] 신규 캐시 결정 검사: 동시 마운트 중복 요청0, 진행 응답 보존, 성공 재사용, 실패 새 마운트 조회, 종류별 만료·진행 조회 만료 방지·refresh 뒤 오래된 결과 거절을 확인했습니다. Arc 공유 보강 뒤 해당 검사만 1회 영향 PASS(1.84초/.00초, 1 passed/0 failed/1 filtered out)입니다.
- [x] 실제 Chrome settings-code 연속1 GREEN: seq16·설정 쓰기5·font_list/shell_profiles 각1. actual hit/focus→스위치 저장·폰트 검색/선택·FontSize18 blur 저장·셸 profile 선택·TerminalCursor End→Enter·held ack 전 Pending/socket1→응답 뒤 Ready/socket0·quiet1.1초 요청/상태 불변·page error/panic/응답 누출0입니다. 단일 합성 socket이며 제품 배포 자산 전체 검사가 아닙니다.
- [x] 별도 캐시 영향만 실제 Chrome settings-resource-cache 첫 PASS: seq14·font1/shell2·설정 쓰기0. 목록을 held 상태로 두어도 Theme/Locale가 준비됨→unmount→late 응답/의도 셸 실패→remount의 성공 폰트 재사용·셸만 재조회→Ready/socket0/quiet1.1초·page error/panic/누출0입니다. 기존 설정 입력 성공 시나리오는 반복하지 않았습니다.
- [x] native lib/bins/tests check10.98초 exit0: 변경된 host/app/shutdown guard를 포함하며 기존 Wry 경고17개 외 새 제품 경고는 없습니다. 최신 Wasm probe build3.38초 exit0은 Arc/공유 캐시를 포함합니다. 앞선 normal production Wasm/canvas check.48초 성공은 당시 UI 소스로 재사용합니다.
- [x] 시험 TS strict exit0·Prettier exit0·Rust14 exactfmt. 마지막 Arc 포인터 검사의 포맷 한 줄만 rustfmt로 정정해 해당 파일 fmt만 재검사했습니다. 기존 성공 검사는 반복하지 않습니다.
- [ ] 실제 per-font glyph/선택된 폰트 trigger 표시·원본 라벨/목록 정렬·popup/disabled/전체 접근성·full App/다중 viewport·전체 폰트 성능/배포 notice·제품 자산/최종 gate는 남습니다.

## 시험 실패와 제품 수정 구분

처음 portable 화면 검사는 ToC animation state offset1499.5와 동일 pass field rect1698.1을 혼동해 offscreen 클릭했습니다. 설치된 ScrollArea는 content UI geometry를 만든 뒤 animated offset을 갱신합니다. fixture에서 원본 animation deadline 뒤 state 적용/새 geometry 프레임을 각각 처리했고 제품 scroll animation을 끄거나 변경하지 않았습니다. 이어서 검색 sm의 결과를 1개로 예상한 오류는 원본 점수(Synthetic Mono .8908218089100001, system-default .1676278091269683)에 따라 2개로 정정했습니다. 이를 제품 RED나 점수 변경 근거로 쓰지 않습니다.

실제 Chrome의 첫 시도는 검색창이 그려졌다는 사실만 보고 자동 focus 적용 전 insertText/Enter를 보내 폰트가 null로 남았습니다. 실패 원자료/스크린샷에는 검색창이 빈 상태였고 제품 오류/panic/설정 추가 쓰기는 없었습니다. fixture에서 actual 검색창 hit→클릭→같은 trace ID focus 확인→실제 keyboard text 순서로 정정해 실패 시나리오만 한 번 재실행했습니다. 이 수정으로 제품 Picker를 고쳤다고 주장하지 않습니다.

TS 배열 응답을 요청 인자용 object/null helper에 넣은 타입 오류와 optional patch에 nullable 초기 Settings를 합친 오류는 시험 서버에서 직접 typed JSON 응답 및 Settings schema 파싱으로 수정했습니다. 새 캐시 검사에서 전이 의존성 web_time을 직접 참조한 compile 오류는 비Wasm의 원래 std::time::Instant alias를 사용해 의존성 추가 없이 수정했습니다. 검사 억제·any/as·새 의존성·원본 제품 TS 수정은 없습니다.

## 명령과 원자료

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test settings-resources -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test settings-resources 목록은_전체 -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo check --manifest-path native/taide-native-app/Cargo.toml --locked --offline --lib --bins --tests --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo build --manifest-path native/taide-remote-web/tests/browser-probe/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
/Users/hyunseokbyun/development/js/bun/bin/bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built settings-code
/Users/hyunseokbyun/development/js/bun/bin/bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built settings-resource-cache
```

원자료는 같은 tmp directory의 `settings-code-result.json`, `settings-code-failure.json`, `settings-code.png`, `settings-code-failure.png`, `settings-resource-cache-result.json`입니다. code 성공은 공유 캐시 추가 전 source이며 cache 성공/최신 bindings는 공유 캐시·Arc source입니다. 제품 화면 전체·실기·성능 baseline이라고 주장하지 않습니다. Chrome은 mock Keychain·합성 localhost·service workers blocked/download false이며 검사 뒤 browser/server/socket을 회수했습니다.

세부3/4(75%)·provider2/4(50%)·전체363/433(83.83%)·최종0/8·ETA 산정 보류·goal active·main 직접입니다. 다음은 두 섹션의 원본 글꼴/정렬 일치입니다. 보호 앱/실제 OS 설정/사용자 프로젝트/키체인/제품 TS/의존성 버전/lock/MSRV/Git을 보존했습니다. 전체 M8 완료 전 commit/push는 하지 않습니다.
