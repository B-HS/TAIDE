# 레이아웃 command runtime 분리

상태: action 이전과 이 단위 자동 검증 완료. M6 전체·M7·M8은 미완료입니다.

## 대상 파일과 분리 범위

`src-tauri/src/domain/layout/commands.rs`의 command 19개 중 18개 정책을 `crates/taide-runtime/src/layout_actions.rs`로 이전했습니다. 닫기 command는 이미 공통 runtime close를 호출하는 기존 service adapter를 사용하므로 중복 action을 만들지 않았습니다. Tauri AppHandle/State·IPC 시그니처·실제 IDE→PTY 닫기 observer 등록은 기존 adapter/조립부에 남습니다. EventSink와 AppState 참조를 받으며 새 의존성은 없습니다. 실제 보조 창 생성/이동은 이번 범위가 아닙니다.

command의 mutation helper는 tab/pane/project ID로 layout을 찾습니다. 원래 private 설명의 13개가 아니라 실제 소비자는 14개입니다. guard→전체 layout clone→대상 선정→정책 실행→finish 이벤트→state write 순서를 유지합니다. common open/close·finish·flush 4개와 닫기 adapter 본문도 변경하지 않았습니다. 18개 action·helper 3개와 기존 unit 8개를 참조 치환/포맷 정규화 후 원본과 비교했습니다. 비교 도구 초안은 줄 단독 여는 중괄호를 찾지 못해 close 대신 다음 함수를 잡았으며, 실제 첫 중괄호 기준으로 고친 뒤 동일성을 확인했습니다. 제품 코드 차이가 아니었습니다.

## 보존한 정책과 private 설명 이관

파일 열기와 새 split은 stale quick-open 인덱스가 건네는 삭제된 파일/디렉터리를 탭 생성 전에 거절합니다. root guard의 Forbidden과 공통 ensure_existing_file의 error.file.notFound/경로 인수를 유지하며 파일이 아닌 탭은 검사하지 않습니다. CLI에서 명시 허용한 프로젝트 밖 파일은 허용하지만 IDE caller와 untitled 변환은 기존 strict 경계를 유지합니다. 거절 시 기존 warn·경로·사유 로그도 동일하며 로그 범위를 추가하지 않았습니다. 순수 layout service는 디스크를 알지 않으므로 runtime command와 IDE 호출자가 각각 기존 선검증을 수행하고 공통 open helper는 이를 우회하거나 중복하지 않습니다.

열기와 split은 기존 enable_preview_tabs 설정을 적용하되 일반 set_preview에 새 설정 gate를 추가하지 않습니다. untitled 변환은 guard를 취득한 뒤 strict owning-project와 파일명을 확인합니다. 경로 변경은 파일 도메인의 rename/delete 성공 뒤 입력을 받으므로 이미 사라진 from/path를 다시 canonicalize하지 않습니다. 기록된 canonical project root에 대해 Path 성분 단위 starts_with로 경계를 확인해 '/'와 이름 접두사가 같은 형제 디렉터리를 거절합니다. 닫힌 탭 stack만 개명한 결과는 dirty/state write를 수행하되 revision bump·LayoutChanged를 만들지 않습니다.

공개 split 문서의 private helper rustdoc 링크만 일반 코드 표기로 바꾸며 나머지 공개 문서는 유지합니다. 생성 bindings와 IPC manifest는 실제 생성 결과의 digest로 동기화합니다.

## 검증

- 새 layout_commands_runtime은 action 부재 E0425(exit 101)로 먼저 실패했습니다. 구현 후 새 4건과 runtime 63건이 통과했고 runtime에는 이전한 기존 unit 8개가 포함됩니다. 기존 taide-layout service 정책 104건과 이전 전 command unit의 성공 결과는 구현이 불변이므로 재사용합니다.
- 최초 묶음 명령의 잘못된 test target 이름은 Cargo가 검사 시작 전에 거절했습니다. 실제 파일명 rust_native_phase0_contract·domain_boundaries로 고쳐 실행했습니다.
- all-target clippy가 테스트의 explicit drop 이후 MutexGuard를 await까지 보유한다고 판정했습니다. 명시 lexical scope로 바꾸고 같은 all-target 검사 1회 재실행은 exit 0입니다. 억제 지시나 async Mutex 도입 없이 해소했으며 `cargo test -p taide --test layout_commands_runtime --quiet`의 관련 4건 재실행도 통과했습니다.
- strict runtime rustdoc은 exit 0입니다. runtime normal 의존 그래프에 Tauri가 없으며 Tauri 전체 strict rustdoc 성공으로 확대하지 않습니다.

- `cargo test -p taide --test layout_commands_runtime --test layout_service_runtime --test taide_layout_service_extraction --test platform_event_sink --test rust_native_phase0_contract --test domain_boundaries`: 새 4건·공통 5건·기존 공개 경로 1건·이벤트 29건·IPC 7건·도메인 3건 통과, exit 0입니다. `cargo test -p taide-runtime`: 63건, `cargo test -p taide --lib layout --quiet`: 남은 조립부 2건 통과입니다. 생성 bindings 검사 1건을 포함해 변경 후 서로 다른 검사 115건이 통과했습니다.
- 공개 split 문서의 private helper 링크 한 줄만 생성 bindings에서 변경됐습니다. `cargo test -p taide --lib typescript_바인딩을_생성한다 -- --exact tests::typescript_바인딩을_생성한다`: 1건 통과이며 SHA-256은 e69d19c6da72dd6695fcb29dc54a4a75c123f8ef015c539b75404dee1194f1d6입니다. manifest를 동기화한 최종 입력에서 `cargo test -p taide --test rust_native_phase0_contract --quiet` 7건도 통과했습니다. IPC 인수·응답·이벤트·등록 수는 불변입니다.
- fmt·diff·변경 문서 Prettier 검사는 exit 0입니다. 새 테스트는 잠금 대기 중 상태 불변, 이벤트 발행 시점의 이전 state snapshot, 미존재/외부 파일과 CLI 허용 split, preview 설정, strict untitled 변환 거절, closed stack만의 개명 저장과 무이벤트를 확인합니다.

UUID 임시 fixture와 합성 파일만 사용했으며 사용자 설정·시크릿·키링·앱 실행은 건드리지 않았습니다. 전체 workspace·frontend·GUI와 M6 나머지 경계는 미완료이고 일반 push는 승인된 저장소·브랜치에 M6 전체 완료 후 수행합니다.
