# 탭 창 이동·복귀 runtime 분리

상태: 이 정책 단위 이전과 자동 검증을 완료했습니다. M6 전체·M7·M8은 미완료입니다.

## 대상 파일과 리포트

`src-tauri/src/lib.rs`의 root command `layout_move_tab_to_window`와 `plan_return_of_auxiliary_window_tabs` 안의 UI 비의존 정책을 `crates/taide-runtime/src/layout_actions.rs`로 이전했습니다. 빈 보조 창 layout 정리도 runtime이 수행하고 실제 OS 창 생성/close는 Tauri callback에 남습니다. 기존 `WindowRegistry`와 `EventSink`를 소비하며 의존성은 추가하지 않았습니다.

`crates/taide-runtime/tests/layout_window_actions.rs`에 synthetic integration 8건을 추가했습니다. 기존 root의 순서 unit 2건은 runtime 정책과 실제 adapter를 각각 확인하도록 옮겼으며 Specta 등록·창 닫힘 이벤트 2개·registry 회수 검사를 유지했습니다. 공개 IPC 시그니처와 영어 문서 3줄, 실제 창 adapter, 순수 layout service는 변경하지 않았습니다.

## 상세

창 이동은 guard→layout clone→대상 탭/프로젝트 선정 순서입니다. 새 창 대상이면 callback을 await해 OS 창을 먼저 열고 local layout을 수정합니다. 순수 이동의 오류에는 받은 label로 close callback을 호출하고 state/dirty/event를 기록하지 않습니다. 성공 뒤 빈 auxiliary layout 슬롯을 먼저 제거하고 registry에 label이 있는 창만 close callback을 호출합니다. `finish_mutation`의 dirty/event 뒤 state write라는 기존 순서를 유지하며 runtime에서 registry를 forget하지 않습니다.

보조 창 복귀는 같은 mutation guard 안에서 기존 mirror 조회를 수행합니다. 조회 실패를 `unwrap_or_default`로 빈 목록으로 처리하는 정책도 그대로입니다. mirror 없는 file tab의 phantom dirty를 정리한 뒤 탭을 main에 복귀시키고, state write→dirty→LayoutChanged 순서로 기록합니다. 존재하지 않는 프로젝트나 슬롯은 기존 debug 로그와 무이벤트 처리를 유지합니다. 실제 창 닫힘과 registry 제거, `auxiliary-tab-return` 감독 등록은 기존 Tauri 조립부에 남습니다.

원본 창 이동·복귀·빈 창 정리 본문은 참조/callback 치환 및 공백 정규화 뒤 동일했습니다. 기존 공통 layout action과 command/helper/unit 전체 본문도 추가 구간을 제외하면 불변입니다. 실제 창 생성 adapter와 순수 service diff가 없는 것을 확인했습니다.

## 검증

1. 초안 RED는 새 runtime API 부재 E0425와 비공개 fixture helper 호출 E0603으로 실패했습니다. fixture를 공개 PaneNode 구조로 수정한 뒤 새 API 부재 E0425만 남는 RED(exit 101)를 확인했습니다. 구현 후 `cargo test -p taide-runtime --test layout_window_actions --quiet` 8건이 통과했습니다.
2. `cargo test -p taide --test layout_commands_runtime --test layout_service_runtime --test rust_native_phase0_contract --test domain_boundaries --quiet`: command 4건·공통 service 5건·IPC 7건·도메인 3건이 통과했습니다. `cargo test -p taide --lib 'tests::탭_창_이동은_조립부에서_창_생성과_rollback을_순서대로_수행한다' -- --exact`와 복귀의 `tests::보조_창_닫힘은_조립부에서_미러_정리와_탭_복귀를_순서대로_수행한다`도 각 1건 통과했습니다. 변경 후 서로 다른 검사 합계는 29건이며 각 명령 exit 0입니다.
3. `cargo clippy -p taide-runtime -p taide --all-targets -- -D warnings`, `RUSTDOCFLAGS='-D warnings' cargo doc -p taide-runtime --no-deps --quiet`, `cargo fmt --all -- --check`, `git diff --check`는 exit 0입니다. strict 문서 성공은 runtime 범위이며 Tauri 전체 rustdoc 성공으로 확대하지 않습니다.
4. bindings·manifest·의존 목록은 diff가 없고 SHA-256은 `f874269e742c9ac204c0b35085a41815743211b18ee33aabd48384f353a5975a`로 불변입니다. 공개 IPC 문서/타입을 유지했으므로 생성 검사를 생략했습니다. 의존 그래프도 변경이 없어 직전 normal runtime의 Tauri 미의존 근거를 재사용했습니다. 불변 순수 layout service 104건의 기존 성공 결과도 재사용했습니다.

UUID 임시 fixture만 사용했습니다. 실제 앱/창·사용자 mirror·시크릿·키링·외부 설치기/LSP/PTY process는 실행하지 않았습니다. synthetic 검사와 source contract는 실제 OS rollback 성공을 증명하지 않습니다. 정상 layout에서 선검증 뒤 방어적 이동 실패를 유도하기 위한 product hook은 추가하지 않았고, 순수 service 슬롯 rollback 검사와 향후 실기/정책 변경 시 재현 조건을 `docs/quality-assurance/2026-09-27-layout-window-actions.md`에 기록했습니다.

새 결과 문서만 Prettier로 검사하고 기존 대형 PROCESS/architecture는 minimal diff를 유지합니다. 전체 workspace/frontend/GUI, 잔여 application 정책·LSP 설치/reader·PTY 수명 소유권과 command 전체 body 판정은 미완료입니다. 일반 push는 승인된 GitHub B-HS/TAIDE의 to_rust_native 브랜치에 M6 전체 완료 후 수행합니다.
