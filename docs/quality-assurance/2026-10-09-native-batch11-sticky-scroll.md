# 배치 11 — 편집기 상단 고정 줄 (2026-10-09)

상태: 실제 TS·Monaco와 native 접기/표시/입력 경계를 대조해 구현과 통합 검증을 마쳤습니다. 구현 `17a0f903`을 선별 커밋·일반 푸시했고 로컬/원격 차이 0/0입니다. 서브에이전트·workflow 없이 메인이 직렬 수행했습니다. 전체 목표는 미완료이며 배치 12 미니맵으로 계속 진행합니다.

## 원본과 공급 경계

- TS `code-editor.tsx:279`와 `shared/lib/code-editor-settings.ts`: sticky scroll 기본 true입니다. 대형 제한은 원본 `StickyLineCandidateProvider.updateStickyModel`의 too-large-for-tokenization 경계이며 native 문서 tier에서 같은 기능 제한을 적용합니다.
- Monaco 0.56.0 `contrib/stickyScroll/browser/stickyScroll{ModelProvider,Provider,Controller,Widget,Actions}.js`: outline→syntax folding→indentation folding 순으로 실제 공급을 선택합니다. 같은 시작 줄을 중복 표시하지 않고 숨긴 접기 영역 안의 후보를 제외합니다. 최대 5줄과 화면 높이의 25% 반올림 중 작은 수를 사용하며 마지막 scope 끝에서 마지막 줄만 위로 밀립니다. 작은 화면에서 상한 0을 무시하는 원본 결함은 복제하지 않습니다.
- 위젯은 각 시작 줄의 첫 표시 줄을 기존 토큰·인라인 장식·탭/폭으로 렌더하고 가로 스크롤을 따릅니다. 배경/gutter `editor.widgetBackground`, border `editor.widgetBorder`는 실제 TS theme 매핑입니다. hover·shadow는 원본 기본 색 경로를 확인합니다.
- 일반 클릭은 실제 글자 열로 이동하고 Shift 클릭은 scope 종료 줄 1열을 화면 밖이면 중앙으로 이동합니다. Shift hover는 종료 줄로 바꿉니다. 접기 버튼은 기존 접기 모델을 토글하고 header 높이를 보존하도록 스크롤을 조정합니다. 포커스 상태의 위/아래·Enter·Escape와 토글 명령은 원본 행동입니다. Ctrl/Meta 정의 이동은 실제 LSP 정의 provider 연결의 후속 범위입니다.
- native `InputState.fold_regions`·`maintain_folds`는 revision/tab/language에 따른 들여쓰기·언어 표식 캐시이며 기존 앱 LSP typed request에 DocumentSymbol/FoldingRange 사용자 공급 경로는 없습니다. 프로토콜은 이를 지원하지만 공급을 추측으로 완성 처리하지 않습니다. 문서/revision을 확인하는 모델 경계를 제공해 현재는 접기 fallback을 사용하고 LSP/outline 배치에서 실제 provider를 연결합니다.

원본 후보/상태 API를 CLI에서 처음 직접 import하면 `window is not defined`로 실패했습니다. 프로젝트에 이미 있는 happy-dom GlobalRegistrator로 메모리 DOM을 등록한 뒤 원본 `StickyLineCandidateProvider`·`StickyScrollController` prototype API import가 exit 0입니다. 새 의존성·OS 입력·실제 브라우저 조작은 없습니다. 실제 원본 함수를 기준값으로 실행해 후보·밀림 상태를 검증합니다.

## 구현과 실패 재현 근거

- 원본 prototype API로 생성한 357표본의 후보·중첩/최대·밀림·접기 숨김·wrap을 `tests/sticky-model.rs`와 비교했습니다. interval 검색의 종료 경계 1줄 차이를 실제 실패로 확인한 뒤 수정해 357표본 전체 일치입니다. 문서 버전·작은/비정상 화면·4096단계 중첩 검사도 선별 통과했습니다. 언어 태그 확인을 추가했으며 최종 전체 대상에서 재확인합니다.
- 메모리 egui 고정 줄 클릭 뒤 같은 묶음의 Text가 누락되는 회귀가 수정 전 실패했습니다(`/private/tmp/taide-batch11-sticky-click-before-20261009.log`, exit 101). 고정 줄의 선언된 포커스 route를 이벤트를 꺼내기 전에 수집하고 클릭/키를 순서대로 처리하도록 수정한 뒤 해당 검사가 통과했습니다. 초기 harness의 RawInput 필드·미처리 textures 오류와 필터 오타의 0건 실행은 통과 근거에서 제외합니다.
- 일반 클릭만 있는 입력도 본문 포커스가 없어서 이동하지 않는 문제가 Shift/읽기 전용 검사에서 재현됐습니다. 고정 줄의 최종 포커스를 처리 조건에 포함한 뒤 클릭+타이핑·위/아래/Enter/Escape+뒤따르는 Text·Shift 종료 줄·최대/밀림/clip·folding 독립 fallback/설정 종료·읽기 전용/대형·고정 줄 휠 7건 모두 통과했습니다(`/private/tmp/taide-batch11-sticky-interaction-click-only-after-20261009.log`, exit 0).
- 다른 입력창 클릭 뒤 본문이 포커스를 되찾는 문제와 편집 후 이전 고정 줄 byte 위치가 UTF-8 경계에서 벗어나는 문제를 실패로 재현했습니다. 실제 SDK 포커스 route를 사용하고 최종 포커스 적용을 이벤트 묶음 뒤로 미루며, 기존 DecorationLayer와 문서 저널로 클릭 위치를 추적해 수정했습니다. 접기 명시 offset이 일반 캐럿 reveal에 덮이는 실패도 재현 후 적용 순서를 수정했습니다. 최종 UI 전체 대상의 고정 줄 15건에서 함께 통과했습니다.
- `standaloneServices.js`의 StandaloneConfigurationService.updateValues는 ConfigurationTarget.MEMORY를 사용합니다. `code-editor.tsx`의 옵션 공급에는 이 변경을 영구 설정에 저장하는 경로가 없습니다. 따라서 명령·문맥 메뉴는 현재 창의 세션 상태를 토글하고 실제 설정 값 변경을 동기화하며 새 세션은 설정 값으로 초기화합니다. 중간의 영구 저장 연결은 제거했습니다. 실제 설정 UI의 기존 저장 경로는 그대로 사용합니다. 다른 editor 마운트가 메모리 옵션을 덮는 원본 초기화 특이 동작은 복제하지 않습니다.
- 최종 회귀는 실제 원본 357표본/core 4건, egui 메모리 UI 15건입니다. UI는 클릭+Text·키/종료·Shift 종료 줄·최대/밀림·fallback/설정 종료·읽기 전용/대형·휠·다른 입력창 포커스·토큰/인라인 장식/테마·wrap/탭/Unicode/가로 clip·접기 offset·revision 추적·문맥 메뉴·접기 아이콘 fade·세션 토글 수명을 검증합니다. OS 합성 입력은 사용하지 않았습니다.
- app --tests 초기 연결 컴파일 exit 0, 14.89초입니다(`/private/tmp/taide-batch11-app-connect-check-20261009.log`). 이후 최종 앱 전체 대상에서 세션 상태·테마·명령 연결을 재확인했습니다. 기존 wry vendor 경고 17개는 유지됩니다.

## 최종 통합 게이트

변경 editor/UI/app의 `cargo test --all-targets --no-fail-fast --manifest-path native/<crate>/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`를 각 한 번 직접 실행했습니다. 앱은 아래 보호 검사만 정확히 제외했습니다. Cargo는 한 번에 하나씩 실행했습니다.

| 대상 | 실행 결과 | 근거 로그 |
| --- | --- | --- |
| editor 전체 26대상 | exit 0, 192 통과·0 실패·1 ignored, 빌드 4.44초 | `/private/tmp/taide-batch11-editor-full-20261009.log` |
| UI 전체 17대상 | exit 101, 309 통과·1 실패, 빌드 7.50초. 새 고정 줄 15건은 모두 통과 | `/private/tmp/taide-batch11-ui-full-20261009.log` |
| UI 실패 영향 1건 | 기존 명령 지원 기대 목록에 토글을 반영한 뒤 exit 0, 1 통과. 서로 다른 UI 310건의 성공 근거이며 전체 실행의 exit 101을 숨기지 않음 | `/private/tmp/taide-batch11-ui-registry-after-20261009.log` |
| app 전체 67대상 | exit 0, 612 통과·0 실패·3 filtered out, 빌드 39.86초 | `/private/tmp/taide-batch11-app-full-20261009.log` |
| remote-web host | `cargo check --tests --manifest-path native/taide-remote-web/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target` exit 0, 4.48초 | `/private/tmp/taide-batch11-browser-host-20261009.log` |
| remote-web Wasm | `cargo check --manifest-path native/taide-remote-web/Cargo.toml --target wasm32-unknown-unknown --features canvas --locked --offline --target-dir experiments/native-shell-spike/target` exit 0, 2.35초 | `/private/tmp/taide-batch11-browser-wasm-20261009.log` |

보호 제외는 `remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다`, `실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다`, `실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다`입니다. 실제 Trash를 건드리는 검사이므로 성공으로 집계하지 않습니다.

변경 3개 패키지를 각각 지정한 Cargo fmt 검사, generator의 프로젝트 Prettier 명시 ignore-path 검사, 이번 변경의 diff 공백 검사가 exit 0입니다. 기준 `8864800e` 대비 remote-web 전체·모든 Cargo.toml/Cargo.lock diff exit 0입니다. 의존성·SDK vendor 변경이 없으므로 batch10 syntax 158건(ignored 3건)과 기존 egui 단위 50·문서 167건(ignored 1건)의 성공 근거를 재사용합니다. 디스크 여유 675GiB·사용률 64%이며 캐시 정리를 실행하지 않았습니다.

원본 sticky scroll 후보/상태·위젯의 사용 범위와 357표본 fixture를 THIRD_PARTY_LICENSES.md의 기존 Monaco MIT 고지에 추가했습니다. 라이선스 전문은 기존 두 고지 파일에 유지됩니다.

## 게이트와 잔여 범위

순수 모델과 메모리 egui의 실제 문자/shape/입력으로 중첩·최대·밀림·접기/숨김·wrap·옵션/테마·토큰/장식·뷰 수명·클릭/키·대형 제한을 확인했습니다. 설정 14필드 중 고정 줄까지 13필드의 공급·소비 근거가 연결됐으며 전체 기능 완료율을 뜻하지 않습니다.

실제 UI/OS 입력·IME/RTL/접근성·대형 실기·soak, 기존 Trash 보호 3건·ignored 성능 부채는 미검증입니다. 보호 앱 번들·실제 데이터·클립보드·Keychain·OS 설정·Trash를 건드리지 않습니다. minimap·LSP provider·나머지 전체 목표를 후속으로 계속 진행하며 전체 목표를 완료로 표시하지 않습니다.
