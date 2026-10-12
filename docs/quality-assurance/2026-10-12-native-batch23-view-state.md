# 배치 23 커서·스크롤 상태 복원

현재 상태: 실제 App의 초기 선택 미복원·명시 이동의 scroll 덮임·누락된 선택 시작 축 오류를 재현해 수정했습니다. 실제 App 4·상태 3건, Core 전체 306·UI editor_surface 87의 서로 다른 400건과 frozen host/Wasm 제품 compile·소유 Rust fmt가 통과했습니다. 최신 상태 저장/닫힘/종료 소비는 아직 미연결이며 editor-67은 partial로 정정하고 전체 완료로 올리지 않습니다. 기능 체크리스트는 298/588(50.7%)·미완료 290이고 전체 전환율/잔여 시간은 미산정입니다. workflow/서브에이전트 없이 메인이 진행합니다.

- [x] a. 원본 저장 형식·native 모델/표시·facade/종료 경계 대조
- [x] b. 임시 App의 초기 복원 실패 재현·검증된 저장 형식과 복원 연결
- [ ] c. 탭/뷰 전환·닫힘·창/프로젝트/앱 종료의 최신 상태 저장 연결
- [ ] d. 실제 App/재시작·Unicode/scroll/mirror/readonly·잘못된 값/소유 수명 검증
- [ ] e. 관련 회귀/형식/경계·기능표/PROCESS·선별 Git 저장
- [x] e1. 초기 복원 단위의 실제 App/상태·Core 전체·UI 회귀·frozen 제품 compile·소유 fmt 확인

## 원본 계약

`src/widgets/editor-pane/use-editor-view-state.ts`는 Tab.viewState 문자열을 구독하며 JSON 객체의 cursorState 배열과 viewState 객체를 확인합니다. 잘못된 JSON/최상위 형식은 복원을 생략합니다. 살아 있는 모델에만 초기 복원을 적용하고 같은 탭의 후속 layout/dirty/자기 저장 갱신은 커서를 다시 이동시키지 않습니다. 저장은 editor/tab 전환의 layout cleanup과 앱/창 종료의 별도 view-state flush registry에서 수행합니다. 탭별 마지막 전송 값과 같은 상태는 중복 전송하지 않습니다. disposed editor의 null 결과는 저장하지 않습니다.

설치된 Monaco의 `cursor/cursor.js`는 각 선택의 selectionStart와 position에 1부터 시작하는 모델 줄/UTF-16 열을 저장합니다. 역방향 선택의 anchor/head를 보존하며 복원 시 누락된 위치 축은 기본값/현재 위치로 보완하고 모델에서 유효한 좌표로 처리합니다. `codeEditorWidget.js`는 cursorState/viewState/contributionsState를 반환하고 실제 모델/뷰가 있을 때만 복원합니다.

추가 대조에서 `cursor.js:139-176`은 selectionStart의 누락된 축을 position의 같은 축으로 보완합니다. 복원 DTO의 각 위치에 일괄 기본 1을 적용하면 빈/부분 selectionStart가 잘못된 선택을 만들므로 해당 상태 검사를 추가했습니다. `folding.js:118-155`와 `foldingModel.js:132-183`은 contributionsState에 접힌/수동 범위를 보관하고 유효한 줄 범위·본문 checksum을 확인한 뒤 복원합니다. 현재 첫 복원 단위는 이 contribution을 소비하지 않으며 전체 editor-67 완료 전에 저장/재시작과 함께 대조해야 합니다.

`viewModel/viewModelImpl.js`의 scroll 상태는 scrollLeft·firstPosition·firstPositionDeltaTop입니다. 첫 가시 줄의 모델 위치와 그 줄 위쪽/scrollTop 사이의 차이를 사용하므로 저장된 화면 위치와 커서 위치는 독립적입니다. firstPosition이 없는 이전 형식은 scrollTopWithoutViewZones를 읽습니다. native의 pixel/내부 행 수를 강제로 Monaco와 맞추지 않되 줄바꿈/접기/폰트/추가 zone에서 사용자가 보던 모델 위치를 보존해야 합니다.

## 현재 native 경계

`native/taide-native-app/src/application.rs`의 show_document는 본문에 EditorStore.attach_view를 호출하지만 Tab.viewState를 읽지 않습니다. `native/taide-native-editor/src/store.rs`의 새 뷰는 기본 선택·ScrollPosition으로 시작합니다. 숨긴 기존 뷰는 메모리에 남을 수 있으므로 메모리 유지와 앱 재시작 후 복원을 구분합니다. peek는 별도 조립 경로이며 본문 탭의 저장 대상으로 공급하지 않습니다.

`native/taide-native-ui/src/editor_surface.rs`는 실제 DisplayMap과 VerticalLayout을 만든 뒤 wrap/접기·code lens/preview zone을 합쳐 scroll 범위를 결정합니다. DisplayMap만으로 실제 세로 위치를 추정하지 않으며 저장된 첫 모델 위치의 scroll 복원은 해당 표시가 준비된 단계에서 적용해야 합니다. 커서 이동/reveal과 저장된 scroll 복원이 서로 덮어쓰지 않는지 실제 App으로 확인합니다.

`crates/taide-runtime/src/layout_actions.rs`의 layout_set_view_state는 기존 mutation guard와 layout locate·finish_mutation·dirty layout 저장을 사용합니다. Tab/ClosedTab은 같은 view_state 필드를 보존합니다. 기존 DTO/저장 경로와 개인 파일 원문을 유지하며 새 RPC/의존성·엔진·frozen 계약 변경은 필요성을 먼저 확인합니다.

셸 mutation은 `taide-native-ui/src/controller.rs`의 별도 worker에서, 파일 탭 닫기/열기는 `taide-native-app/src/host.rs`에서 처리됩니다. 각각에 저장과 닫기 요청을 보내는 것만으로 두 큐 사이의 실행 순서가 보장되지 않습니다. NativeApplication UI를 mutation guard의 동기 대기로 막으면 PreparedDocument를 UI가 회수해야 하는 현재 경계와 충돌할 수 있으므로 기존 async facade/소유와 닫기/flush 안의 순서를 검토합니다. 공유 캡처/변경 요청을 어느 기존 경계에 연결할지는 실제 재현 뒤 결정합니다.

현재 앱 shutdown은 호스트/LSP/web 종료를 기다리고 텍스트 draft를 mirror에 저장한 뒤 host.flush_layouts를 호출합니다. 커서/스크롤의 최신 상태 캡처는 없습니다. NativeProjects.await_project_flush도 동일 layout flush를 사용합니다. 정상 종료/직접 on_exit·보조 창 반환·탭 닫기/최근 닫힘·프로젝트 닫기의 상태 보존을 각각 확인해야 합니다.

## 다음 실제 재현과 완료 조건

임시 App과 소유 데이터에서 저장된 Unicode/역방향 선택을 가진 살아 있는 본문을 표시하고 기본 위치 대신 해당 선택을 복원하는지 먼저 단언합니다. 실제 scroll은 충분한 줄 수/가로 폭과 표시 geometry를 사용하며 잘못된 값·범위를 벗어난 좌표·빈 선택 목록·후속 metadata/layout 갱신·동일 파일의 독립 탭/보조 창을 확인합니다. 모델/뷰 교체의 복원 수명은 native의 실제 ViewId/DocumentId와 원본의 메모리 상태 재사용을 함께 대조합니다.

사용자 실제 앱 데이터/OS 설정/클립보드/Keychain/Trash·보호 앱·합성 OS 입력 없이 임시 저장/재시작과 egui 이벤트를 사용합니다. 커서/스크롤 저장이 원문/revision/dirty/undo·mirror를 변경하지 않고 현재 Tab.viewState와 디스크 layout에 도달해야 합니다. 같은 값 중복 저장/종료 직전 최신 값·실패/닫힌 소유의 늦은 요청·readonly를 검증하며 editor-67의 전체 기능 묶음이 연결되기 전에는 complete로 올리지 않습니다.

## 실제 초기 복원 실패와 첫 연결

`application-view-state-tests.rs`는 공용 임시 fixture의 Tab.viewState를 첫 본문 생성 전에 설정합니다. 저장된 Monaco (2:11→2:3) 선택은 native UTF-8 byte (18→8)의 역방향 선택이어야 합니다. 첫 실행 `/private/tmp/taide-batch23-view-state-red.log`는 문서 준비 뒤 이 단언에서 실제 (0→0)으로 실패했습니다. 0통과/1실패·24.76초·exit 101이고 컴파일은 6분 09초입니다. 실제 Shift+Right와 후속 layout 변경·원문/revision/dirty/디스크 보존은 이 첫 실패 이후라 아직 검증한 것으로 올리지 않습니다.

App 전용 editor-view-state 상태는 live ViewId/DocumentId의 최초 복원을 기억합니다. UTF-16 좌표를 현재 줄/문자 경계에 맞추며 이미 복원한 모델의 후속 layout 갱신은 무시합니다. Core의 typed scroll restore 요청은 renderer에서 실제 DisplayMap/VerticalLayout이 준비된 뒤 적용합니다. 기존 wrap/접기·추가 zone을 사용하고 원본의 커서와 첫 가시 모델 위치를 분리하며 명시 이동/사용자 입력은 이후에 반영합니다. 읽기 전용 문서에서도 문서 내용/revision/dirty를 바꾸지 않습니다.

기존 Serde 의존성의 [derive 공식 문서](https://serde.rs/derive.html)와 [serde_json from_str 공식 계약](https://docs.rs/serde_json/latest/serde_json/fn.from_str.html)을 확인했습니다. 새 의존성·RPC·엔진·환경 설정은 추가하지 않습니다. `view-state-green.log`의 같은 actual App 검증은 진행 중이고 parser/scroll·추가 범위 검증과 저장/flush·전체 게이트는 미완료입니다.

첫 연결의 `view-state-green.log`는 같은 실제 App 1통과·25.53초·exit 0(컴파일 1분 13초)입니다. 저장된 역방향 선택과 실제 Shift+Right, 후속 metadata 갱신의 재복원 방지·원문/revision/dirty/디스크 불변이 통과했습니다. 최초 및 중간 성공은 후속 검사 수에 중복 합산하지 않습니다.

raw legacy scroll도 초기 커서 reveal에 덮이지 않도록 Core 요청에 모델 anchor 또는 raw offset의 두 형태를 보존했습니다. JSON은 최상위/scroll 객체·cursor 배열과 중첩 위치 객체를 확인하고 typed Serde로 좁힙니다. 배열을 struct의 위치 필드처럼 받아들이지 않습니다. [Ropey 1.6.1의 UTF-16 변환 계약](https://docs.rs/ropey/1.6.1/ropey/struct.RopeSlice.html#method.utf16_cu_to_char)에 따라 범위를 먼저 제한하고 문자/바이트 경계로 변환합니다. surrogate 중간·CRLF·큰 좌표를 확인했고 설치 Monaco 기본 multiCursorLimit 10000으로 복원 cursor 수를 제한합니다. 임의 의존성/환경 우회는 없습니다.

`view-state-related.log`는 실제 선택/metadata·두 scroll 형식과 상태의 잘못된 JSON/배열·CRLF/UTF-16 범위/readonly/mirror·해제까지 서로 다른 4통과·21.99초·exit 0(컴파일 52.26초)입니다. 실제 scroll은 첫 모델 줄 45와 delta -7, cursor 줄 1을 독립적으로 복원하고 16프레임에서 즉시 안정됐습니다. legacy scrollTopWithoutViewZones와 가로 위치도 확인했습니다.

## 명시 이동 우선순위와 실제 읽기 전용 본문

첫 본문 생성 전에 명시 줄 100 이동을 큐에 넣은 `view-state-navigation-red.log`는 커서가 목적 줄로 이동했지만 scroll이 저장 위치 887로 덮였습니다. 0통과/1실패·20.52초·exit 101(컴파일 20.39초)입니다. Core의 명시 reveal_position/selection reveal이 이전 pending scroll 복원을 취소하도록 연결했습니다. 원본의 내부 수치를 강제로 재현하지 않고 명시 이동의 실제 목적을 보존합니다.

추가 actual App은 임시 파일 끝에 유효하지 않은 UTF-8 byte를 넣어 생산용 open_file의 lossy/read_only를 적용합니다. 새 live view의 초기 선택 복원·편집 거절·실제 Shift+Right·원문/revision/dirty/디스크 byte 불변을 확인합니다. 첫 `view-state-final-related.log`는 5통과/1실패·32.40초·exit 101(컴파일 35.35초)이며 authored test의 memory_mut 안에서 body lookup을 호출해 같은 egui Context의 read/write 잠금을 중첩했습니다. epaint의 10초 DEBUG PANIC으로 실제 실패했습니다. 본문 ID를 잠금 밖에서 읽도록 고쳤습니다. 종료 요청을 시도했을 때 프로세스는 이미 자연 종료했으며 실제 종료 성공으로 기록하지 않습니다.

`view-state-final-related-fixed.log`는 서로 다른 6통과/0실패·22.11초·exit 0(컴파일 15.14초)입니다. 이전 4건과 첫 성공을 다시 합산하지 않습니다. 첫 6건 중 통과한 5건은 test-only 수정 뒤 재실행이 불필요했고 readonly exact만 선택했어야 했습니다. 검사 반복을 제품 개선으로 세지 않습니다. 저장/탭 전환/닫힘/앱 종료의 최신 상태 전달은 아직 연결하지 않았으며 editor-67 전체 완료로 바꾸지 않습니다.

10개 소유 Rust fmt와 diff check는 exit 0입니다. 변경 Core의 전체 대상과 UI editor_surface 회귀·동결 host/Wasm 제품 compile을 직렬로 확인하며 같은 소스의 관련 성공을 재사용합니다. 전체 App의 이전 877/22 결과와 준비 실패의 원인 미확정, 실기/성능/신뢰성/출시 게이트는 별도 유지합니다.

`view-state-core-all-targets.log`의 Core 전체 대상은 39대상·306통과/0실패·기존 성능 ignored 1·exit 0입니다. 공개 scroll restore 요청과 기존 명시 이동/선택·편집/undo·저장·mirror/readonly 경계의 회귀를 확인했습니다. App 6건과 겹치는 조기 성공은 다시 합산하지 않습니다.

`view-state-ui-editor-surface.log`는 UI editor_surface 87통과/0실패·1.81초·exit 0(컴파일 2분 07초)입니다. 변경 renderer의 본문 입력/IME·명시 cursor/selection reveal·smooth scroll/scrollbar·접기/wrap·미리보기/zone·읽기 전용 회귀를 확인했습니다. 실제 App 6 + Core 306 + UI 87의 서로 다른 399건이며 UI 전체/App 전체/실기 전체 통과로 확대하지 않습니다.

최초 `view-state-frozen-host-check.log`는 native-host 없이 빌드할 때 공통 reveal 조건이 native 전용 `last_editor_event`를 두 곳에서 참조해 E0425 2건·exit 101로 실패했습니다. 공통 조건과 native 입력 추가 조건을 cfg로 분리했습니다. native-host의 최종 조건은 이전과 같고 웹에서는 pending 복원이 없으므로 기존 reveal을 유지합니다. 동결 소스/manifest/lock 수정이나 검사 제외로 우회하지 않았으며 같은 native 동작의 App/Core/UI 성공은 재사용합니다. 수정한 host compile과 Wasm 제품 compile은 별도로 확인합니다.

수정 뒤 `view-state-frozen-host-check-fixed.log`는 host의 tests 포함 check·38.42초·exit 0, `view-state-frozen-wasm-check.log`는 canvas의 wasm32-unknown-unknown lib check·1분 52초·exit 0입니다. 기존 Wasm tests의 std/web Instant 불일치 7건을 제품 compile 실패나 통과로 바꾸지 않았습니다. manifest/lock/frozen source와 기존 경고를 보존했습니다.

`view-state-missing-axis-red.log`는 빈 selectionStart에서 anchor 0/head 9를 만들고 기대한 빈 선택 9/9와 달라 실제 실패했습니다. 0통과/1실패·0.03초·exit 101(컴파일 1분 15초)입니다. selectionStart의 누락 축만 저장된 position의 같은 축으로 보완했습니다. 명시 좌표/범위 clamp·readonly/scroll은 보존하며 이 경로가 바뀐 상태 검사 3건을 선별 확인합니다. 모델 내용/화면 경계가 변하지 않은 기존 성공은 반복하지 않습니다.

`view-state-missing-axis-fixed.log`의 상태 3건은 3통과/0실패·0.00초·exit 0(컴파일 15.85초)입니다. 빈/줄만/열만 selectionStart와 기존 CRLF/UTF-16 clamp·잘못된 형식/readonly/mirror를 확인했습니다. native reveal 조건은 cfg 분리 전과 같고 App parser 변경이 frozen 제품에는 의존하지 않으므로 이전 App 4/Core 306/UI 87·frozen 성공을 재사용합니다. 서로 다른 400건이며 마지막 소유 10Rust fmt check·관련 diff check는 exit 0입니다. 기존 Wry 17경고와 App linker의 unwind-table 크기 경고는 검사 실패로 바꾸거나 제거하지 않았습니다.
