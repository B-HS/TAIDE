# 레이아웃 공통 service runtime 분리

상태: 공통 service 이전과 이 단위 자동 검증 완료. layout command 전체와 M6·M7·M8은 미완료입니다.

## 대상과 보존한 순서

`crates/taide-runtime/src/layout_actions.rs`로 flush_dirty_layouts·finish_mutation·open_tab_and_finish·close_tab_and_finish 및 기존 flush unit 4개를 옮겼습니다. `src-tauri/src/domain/layout/service.rs`는 finish 재수출과 기존 crate-visible flush 재수출을 제공하고 open/close의 기존 공개 시그니처·문서를 보존합니다. 공통 EventSink와 FnOnce(&Tab) 닫기 callback을 받으며 실제 AppHandle·LayoutTabClosedObservers의 IDE→PTY 후처리 조회/등록은 adapter와 기존 조립부에 유지합니다. 이미 workspace에 있는 taide-layout local path와 기존 log 0.4만 runtime 소비 목록에 추가했고 새 외부 패키지/버전은 없습니다.

flush는 dirty 집합 drain→현재 layout snapshot→동기 저장 순서입니다. 재표시된 dirty는 다음 flush에 남고 layout이 없으면 기존 경고/생략, 저장 실패도 원본처럼 경고하며 이미 drain한 ID를 복구하지 않습니다. 저장 실패 재시도 정책을 새로 도입하지 않았습니다. 주기 flush의 awaited blocking 실행과 shutdown의 동기 호출은 기존 lib.rs 경로를 유지합니다. 원래 설명 주석의 serialized fsync·flush 비중첩·재표시 보존 근거를 이 문서로 옮깁니다.

finish는 pane focus invariant를 고친 뒤 snapshot·dirty 표시·LayoutChanged 발행을 수행하며 revision을 추가로 올리지 않습니다. open은 guard→clone→preview 설정/target 결정→open→finish 이벤트→state write 순서입니다. close는 guard→clone→close→finish 이벤트→state write→observer→guard 해제를 유지합니다. 이벤트가 state write보다 앞인 기존 순서를 이번 이전에서 변경하지 않았습니다. observer는 최신 state와 이벤트를 보고 기존 잠금을 계속 취득한 상태여야 합니다. command와 IDE 호출자의 파일 경계 선검증은 이번 공통 service 이전에 포함하지 않습니다.

layout command 19개와 composition root의 창 이동은 별도 경계이며 이번 변경으로 전체 application body 분리를 완료했다고 기록하지 않습니다.

## 검증 근거

- 변경 전 `cargo test -p taide-layout --quiet`: 정책 104건 통과. `cargo test -p taide --lib domain::layout:: --quiet`: 기존 command/service unit 12건 통과. service 구현은 불변이므로 104건의 성공 증거를 재사용합니다.
- 새 테스트 초안은 없는 collector E0425와 runtime module 부재 E0432로 실패했습니다. 실제 PaneNode API로 초안을 수정한 다음 E0432만 재확인했습니다. 첫 구현 실행은 3/5 통과했으며 기본 layout이 빈 탭 목록이라는 잘못된 기대 2건이 실패했습니다. 실제 기본 Welcome·Terminal 탭을 유지하고 추가한 fixture 탭만 선택하도록 테스트를 수정했습니다. 제품 정책은 변경하지 않았습니다.
- `cargo test -p taide-runtime --quiet`: 55건 통과, exit 0입니다. 기존 51건에 이전한 flush unit 4건이 추가됐으며 원본 unit 본문은 import/포맷 외에 불변입니다. flush·finish의 본문도 문서 제거와 포맷 외에 불변임을 비교했습니다. `cargo test -p taide --lib layout --quiet`: command 및 조립부 회귀 10건 통과, exit 0입니다.
- `cargo test -p taide --test layout_service_runtime --test taide_layout_service_extraction --test platform_event_sink --test rust_native_phase0_contract --test domain_boundaries --quiet`: 새 공통 경계 5건·기존 공개 service 경로 1건·이벤트 포트 29건·IPC 7건·도메인 3건 통과, exit 0입니다. 최초 실행 명령의 잘못된 test target 이름은 Cargo가 실행 전에 거절했고 실제 파일명의 taide_layout_service_extraction으로 수정했습니다. 수정한 action target의 1회 재실행을 이 묶음에 포함했습니다. runtime 및 lib 검사와 합쳐 변경 후 110건이 통과했습니다.
- runtime/Tauri all-target clippy와 strict runtime rustdoc, fmt·diff 검사는 exit 0입니다. 테스트 기대값 수정 후 `cargo clippy -p taide --test layout_service_runtime -- -D warnings`도 exit 0입니다. normal runtime dependency graph에 taide-layout이 포함되고 Tauri는 없습니다. runtime 엄격 rustdoc 성공을 Tauri 전체 엄격 rustdoc 성공으로 확대하지 않습니다.

공개 IPC command/type/문서는 변경하지 않았고 bindings/manifest diff는 없습니다. UUID 임시 디렉터리와 synthetic 상태/파일만 사용했으며 실제 사용자 파일·설정·시크릿·키링·앱 실행에는 접근하지 않았습니다. 전체 workspace·frontend·GUI 실기는 이번 단위에서 실행하지 않았습니다. 일반 push는 승인된 저장소·브랜치에 M6 전체 완료 후 수행합니다.
