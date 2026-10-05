# M8 Editor·Terminal: 셸 경로·Picker 접근성·키 소유

## 대상·범위

`native/taide-native-ui/src/settings-code-view.rs`, `font-families.rs`, `lib.rs`, `native/taide-native-app/src/ui-fonts.rs`, 기존 vendored egui `src/memory/mod.rs`, `native/taide-remote-web/tests/settings-resources.rs`, `tools/m8-remote-rust-file-probe.ts`입니다. 기존 Editor·Terminal 화면 재현 체크의 세부 구현이며 부모 완료 수를 늘리지 않습니다.

원본 `src/features/settings/{font-picker,option-picker,shell-profile-list}.tsx`, `src/shared/ui/{command,popover,button}.tsx`, `src/shared/styles/global.css` 및 설치된 cmdk/egui/AccessKit 소스를 근거로 구현했습니다. 이전 글꼴 픽셀·목록 캐시·설정 ack 성공은 `2026-10-05-m8-settings-font-preview.md`와 선행 QA의 당시 소스로 재사용합니다.

## 구현

셸 행의 고정 44px를 제거했습니다. 원본의 border1/padding12·6에 따른 inset13·7, 12px/16px 줄 높이, 이름 truncate·medium family, 경로 break-all, 활성 행에서만 check16/gap8 공간을 예약합니다. 실제 galley 높이로 행을 배치하고 행·viewport로 그림을 clip합니다. List/ListItem/Button 관계, 이름과 전체 경로가 포함된 accessible name, pressed 상태를 연결했습니다. 기존 medium family 선택 함수를 shared UI로 이동하고 native는 같은 함수를 재노출합니다. browser에 medium family가 없으면 기존 proportional fallback이며 모든 플랫폼의 실제 500 weight 일치 완료가 아닙니다.

Picker는 ComboBox의 expanded/Dialog controls, Dialog/ListBox/EditableComboBox/ListBoxOption 역할, autocomplete·selected·active-descendant 관계를 현재 pass의 최종 cursor로 구성합니다. check 표시의 현재 설정값과 키보드 선택을 구분합니다. option ID는 번역/검색 순서 대신 원래 옵션·font identity에 연결합니다. 비활성 부모 상태를 Popup 자식에 전달하고 Escape 뒤 새 pass의 목록/입력 관계를 제거합니다. cmdk의 실제 기본 목록 label은 Suggestions입니다.

원본 outline 버튼의 hover=list.hoverBackground/list.foreground·inset11/아이콘 gap6 및 검색 구분선=popover.separator를 적용했습니다. 조합 키·최초 포커스 필터의 수정 근거는 `docs/bug/2026-10-05-settings-picker-key-ownership.md`입니다.

## 실제 검증

- [x] 신규 셸/AccessKit 최소 재현: 첫 `새_` 필터는 신규2뿐 아니라 기존 `새_pass` 포커스 검사도 매칭해 3건 실패했습니다(1.32초/.12초). 실제 제품 실패는 고정 높이44·expanded=None이고 포커스는 캡처 시점 문제였습니다. 혼입 실행을 신규 독립 성공으로 세지 않습니다.
- [x] 최종 포커스 메모리 검사 1 PASS: .42초/.11초. 화면 pass의 검색 ID가 최종 focused ID와 같았습니다. 이 시점의 Trace는 request 전 값입니다.
- [x] 구현 뒤 신규 AccessKit 1 PASS: 1.94초 빌드/.13초의 세 검사 중 해당1건입니다. expanded/controls·option 선택·disabled End/Enter 차단·Escape 회수를 확인했습니다. 같은 실행의 셸 실패는 path 입력란을 그림으로 고른 fixture 오류이고 포커스 영향1도 PASS였습니다.
- [x] 셸 fixture 정정 뒤 해당 실패 1 PASS: .40초/.12초. 실제 행의 긴 경로 다중 줄·가로/세로 inset·pressed/full name/ListItem을 확인했습니다. 성공한 AccessKit/포커스는 이 실행에 포함하지 않았습니다.
- [x] 합성 IME 1 RED→GREEN: 첫 fixture는 Preedit 필드명 오류로 compile 실패했습니다. 실제 active_range_chars로 수정한 뒤 Escape 닫힘을 재현(.43초/.12초). 첫 guard 수정은 포커스가 pass 시작에서 사라져 같은 오류(1.40초/.14초). focus filter/사전 Escape 캡처 수정 뒤 1.08초/.13초 PASS입니다. OS 입력기는 변경하지 않았습니다.
- [x] controlled Popup 변경의 기존 AccessKit 영향 1 PASS: .06초/.09초입니다. Escape·disabled를 직접 덮는 검사만 실행했습니다.
- [x] 첫 focus frame의 ArrowLeft를 확장한 portable 검사 1 PASS: 2.05초/.11초입니다. 같은 자동 포커스 검사이지만 Chrome의 새 첫 키 실패를 추가한 소스입니다. 새 공개 SDK API의 최초 missing_docs 경고는 영어 API 문서를 추가해 해결했으며 lint를 끄지 않았습니다.
- [x] native lib/bins/tests check: 테스트 모듈의 egui import 제거로 E0433와 연쇄 E0308 실패가 있었고 test 전용 import를 복원해 5.34초 exit0입니다. 기존 Wry 경고17은 유지합니다. 이 검사는 medium helper·초기 AX/셸 소스이며 이후 조합/첫 프레임 필터는 portable/Wasm이 컴파일합니다.
- [x] 최종 SDK의 native lib/bins 소비 check5.72초 exit0입니다. 공개 API 문서 뒤 missing_docs 경고가 없으며 기존 Wry 경고17은 그대로입니다. 바로 뒤 disabled trigger의 ArrowDown guard를 추가한 최종 portable 소스는 마지막 Wasm build로 컴파일합니다. 통과한 UI 시나리오는 enabled 상태이므로 이 guard 추가가 해당 성공 입력을 바꾸지 않습니다.
- [x] Wasm probe build: 초기 AX/셸3.41초, 조합 guard3.06초, 최종 첫 프레임 필터2.94초 exit0입니다. 최종 SDK API의 missing_docs 경고는 없습니다. bindings를 같은 최종 소스로 생성했습니다.
- [x] 변경된 시험 TS strict exit0·Prettier write unchanged입니다. Rust format은 변경한 파일에만 적용했습니다.
- [x] 마지막 disabled trigger ArrowDown guard의 Wasm build1.98초 exit0·같은 bindings 생성, 변경 Rust6 exactfmt·PROCESS/HANDOFF/재개 프롬프트 diff exit0입니다. 최종 enabled Chrome 성공은 이 guard가 바꾸지 않는 입력 경계로 재사용합니다.
- [x] 실제 Chrome 새 `settings-picker-lifecycle` 연속 검사 GREEN: 첫 ArrowLeft의 font-search→settings.terminal 이동 RED를 원자적 필터로 수정했습니다. 두 번째 실행은 UI 검색/일반 Escape 복귀·옵션 End/Escape·재개/바깥 클릭까지 통과했으나 시험 코드가 Drop 뒤 null probe.snapshot을 호출해 실패했습니다. 해제 뒤에는 socket/요청 수만 관찰하도록 시험 코드를 수정한 최종 연속1은 seq11/write0·font-search 유지/빈 검색·trigger 복귀·옵션 닫기·바깥 클릭·Drop/socket0·quiet1.1초 요청 불변·catalog/page/panic/누출 오류0으로 exit0입니다. Drop 뒤 Rust 상태를 읽지 않으며 Ready/프레임·pump 불변 검증으로 과장하지 않습니다. 세 번의 실행은 제품 첫 키 수정/fixture 회수 오류 정정의 실패 경계이며 같은 성공의 반복 계측이 아닙니다.
- [ ] 원본 전체 Popup Presence/motion/Tab focus·IME 플랫폼 실기·동적 대량 목록/모든 viewport·전체 theme/DPI/시각 일치·나머지8개 Settings UI·전체 App/assets/최종 gate는 남습니다.

## 명령·원자료

```sh
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test settings-resources 새_ -- --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test settings-resources 조합중_검색의_enter_escape는_설정선택이나_popup_닫기로_소비되지_않는다 -- --exact --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo test --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --features inspection --target-dir /private/tmp/taide-m8-menu-build.j6Efnw --test settings-resources 폰트_popup은_클릭_후_새_pass에서_검색_입력을_자동으로_포커스한다 -- --exact --nocapture
CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo /Users/hyunseokbyun/development/rust/cargo/bin/cargo build --manifest-path native/taide-remote-web/tests/browser-probe/Cargo.toml --locked --offline --features canvas --target wasm32-unknown-unknown --target-dir /private/tmp/taide-m8-menu-build.j6Efnw
/Users/hyunseokbyun/development/js/bun/bin/bun tools/m8-remote-rust-file-probe.ts /private/tmp/taide-m8-wasm-tools.h2xBQB/file-built settings-picker-lifecycle
```

신규 browser 원자료는 `/private/tmp/taide-m8-wasm-tools.h2xBQB/file-built/settings-picker-lifecycle-{result,failure}.json`입니다. 합성 localhost·mock Keychain·격리 Chrome만 사용합니다. 기존 style 결과의 실제 focused=[font-search]/seq11/write0을 파일로 확인하고 같은 성공 실행은 반복하지 않았습니다. 최신 bindings와 이전 font/style/캐시 성공의 source 시점은 구분합니다.

세부3/4(75%)·provider2/4(50%)·전체363/433(83.83%)·최종0/8·ETA 산정 보류·goal active·main 직접·전체 M8 완료 전 Git 없음입니다. 실제 사용자 프로젝트/키체인·보호 앱·OS 설정·의존성 버전·manifest/lock/MSRV·제품 TS를 보존했습니다.
