# 배치 23 커서·스크롤 상태 복원

현재 상태: 실제 셸 전환과 Host 새 preview 파일 열기의 이전 위치 None을 재현해 선행 저장을 연결했습니다. 기존 닫힘/재열기·초기/종료 복원과 현재 App14/Host9/종료2·controller3이 통과했습니다. 변경 없는 Core306/UI88을 재사용한 서로 다른422건과 소유 fmt/diff를 확인했습니다. 실제 preview 제거의 모델 캐시와 project/slot/aux·batch close/IDE/remote·folding contribution·전체 재시작은 남아 editor-67 partial을 유지합니다. 기능 체크리스트는 298/588(50.7%)·미완료 290이고 전체 전환율/잔여 시간은 미산정입니다. workflow/서브에이전트 없이 메인이 진행합니다.

- [x] a. 원본 저장 형식·native 모델/표시·facade/종료 경계 대조
- [x] b. 임시 App의 초기 복원 실패 재현·검증된 저장 형식과 복원 연결
- [ ] c. 탭/뷰 전환·닫힘·창/프로젝트/앱 종료의 최신 상태 저장 연결
- [x] c1. 실제 geometry/live 선택 캡처·정상/직접 앱 종료의 layout 저장과 본문 우선 오류 처리
- [x] c2. 기본 Host typed CloseTab의 최신 캡처 선행 적용·실제 최근 닫힘/재열기와 요청 수명 검증
- [x] c3. 셸 optional 포트와 Host 파일 열기의 선행 캡처·실제 두 전환과 typed 실패/guard·관련 회귀
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

초기 복원 코드/검사·QA/PROCESS 상단의 12경로는 `2ee11a21`로 선별 커밋했습니다. PROCESS 하단의 기존 두 hunk와 HANDOFF/architecture/이전 결정/피드백·memory 변경은 스테이징하지 않았습니다. 기능표는 599행·391근거 경로 정합성을 확인하며 editor-67만 partial로 정정합니다. 완료 298/미완료 290·배치 23 잔여 23/최종 33과 전체 목표를 유지하고 다음은 실제 종료의 최신 위치 저장 실패입니다.

## 종료 직전 최신 위치 저장

`2ee11a21`과 판정 기록 `339fc46a`는 일반 push·upstream 0/0을 확인했습니다. 실제 종료 검사는 소유 임시 App의 Unicode 역방향 선택을 실제 Shift+Right로 움직인 뒤 on_exit와 정상 close를 각각 확인합니다. 첫 `view-state-exit-red.log`는 직접 종료 후 디스크 layout의 Tab.view_state가 None이라 실패했습니다. 0통과/1실패·21.10초·exit 101(컴파일 21.00초)이며 정상 close 사례는 아직 도달하지 않았습니다. 원문/revision/dirty/디스크/작업 회수의 후기 단언은 통과로 세지 않습니다.

기존 실제 EditorGeometry의 첫 가시 표시 행과 원점은 wrap/접기·추가 zone이 반영돼 있습니다. 모델 byte와 화면 위쪽 대비 delta를 이 geometry에서 얻고 live ViewState의 UTF-16 선택과 함께 캡처합니다. display/document revision·scroll이 맞지 않으면 첫 모델 위치를 추측하지 않고 검증된 raw scroll 형식을 사용합니다. 닫힌 탭/프로젝트나 교체된 문서/TabKind는 저장하지 않으며 기존 contribution/알 수 없는 최상위 JSON 필드는 보존합니다. 종료에서는 Host reply의 PreparedDocument guard가 회수된 뒤 기존 mutation/dirty-layout flush 순서에 캡처를 연결합니다. 전환/닫힘/원격 요청의 fresh capture와 folding contribution 소비·실제 전체 재시작은 별도 미완료입니다.

첫 연결의 `view-state-exit-green.log`는 실제 App 1건의 정상/직접 종료 두 사례가 1통과·20.90초·exit 0(컴파일 34.43초)입니다. 실제 Shift+Right의 마지막 Unicode 역방향 선택과 가로 scroll이 디스크 layout에 도달하고 원문/revision/dirty/디스크 원문·작업 0을 보존했습니다. 이어서 디스크 값을 새 ViewId에 복원하고 실제 첫 모델 anchor/delta가 같은 세로 scroll을 만드는 단언과 순수 상태의 primary/readonly·최신 선택/소유 교체 및 async 저장의 중복/contribution/닫힌 프로젝트·탭 검사를 추가했습니다. 전체 App 재시작이나 전체 editor-67 완료로 확대하지 않습니다.

첫 관련 `view-state-exit-related.log`는 authored async fixture의 Project.root에 PathBuf를 넣어 E0308 2건·exit 101로 컴파일 실패했습니다. 실제 모델의 String 타입에 맞춰 소유 임시 경로를 변환했습니다. 검사 실행 0건을 성공으로 세지 않으며 같은 관련 10건을 선별 실행합니다. 원본이 읽지 않는 inSelectionMode는 Deserialize에서 소비하지 않고 live 선택 저장에서만 정확한 Boolean으로 작성합니다.

`view-state-exit-related-fixed.log`는 실제 App 5/상태 5의 서로 다른 10통과/0실패·23.69초·exit 0(컴파일 26.52초)입니다. 기존 7건을 중복 합산하지 않으며 최초 복원 단위의 Core 306/UI 87을 합치면 서로 다른 403건입니다. 정상/직접 종료 두 사례의 디스크 layout 값→새 ViewId 선택/실제 모델 anchor·세로 scroll roundtrip과 원문/revision/dirty/디스크·작업 0, readonly raw scroll/primary 우선·observe 뒤 최신 선택·교체된 문서 종류/해제, 알 수 없는 최상위 필드/contribution 보존·중복 revision/event/dirty 억제·닫힌 프로젝트/탭과 종류 교체를 확인했습니다. 새 ViewId roundtrip은 전체 App/process 재시작 게이트를 대신하지 않습니다.

직전 전체 로그 `taide-batch23-ide-selection-app-all-targets.log:165`의 기존 application::exit_tests는 통과했습니다. 그 경로는 빈 캡처의 apply가 즉시 반환하고 기존 draft/종료 순서가 같으며 호출의 새 인자만 Ok(Vec::new())입니다. 기존 성공을 재사용하고 마지막 제품의 App 전체 성공으로 바꾸지 않습니다. 추가 geometry API는 native-host에만 컴파일되고 App 저장은 frozen에 의존하지 않으므로 기존 frozen host/Wasm 성공도 재사용합니다.

`view-state-virtual-anchor-check.log`의 UI 1건은 1통과·0.02초·exit 0(컴파일 9.70초)입니다. 실제 다중 줄 completion preview를 그린 geometry의 모델 행에서는 anchor가 있고 가상 행만 남는 clipping에서는 None입니다. 우려한 가상 행의 잘못된 anchor는 재현되지 않았으며 검사만 추가하고 제품 경계를 더 바꾸지 않았습니다. 실제 preview 원문도 보존합니다. App 10 + Core 306 + UI 87/새 geometry 1의 서로 다른 404건이며 이전 App 종료 1·초기 App/상태 7을 다시 합산하지 않습니다.

`view-state-exit-app-check.log`는 App tests 포함 check·8분 49초·exit 0입니다. 기존 Wry 경고와 editor-code-actions의 test-only is_applying에 대한 lib dead_code 경고를 보존합니다. 검사 중 오래 살아 있던 Cargo와 최근 시작된 rustc를 읽었고 정상 종료·성공을 확인했습니다. 별도 Cargo 프로세스는 조작하지 않았으며 완료 시간/CPU 대기를 앱 동작 실패의 근거로 바꾸지 않습니다.

위치 캡처 실패가 유효한 본문 draft의 mirror까지 건너뛰지 않는지 종료 오류 경계를 확인합니다. 기존 소유 fixture와 실제 untitled mirror 서비스에 유효한 draft와 위치 오류를 함께 전달하고 직접 종료의 원래 오류·저장된 draft·작업 회수를 단언합니다. 검사 입력은 소유 임시 데이터뿐이며 다른 종료 실패나 원본 버그의 강제 재현으로 범위를 넓히지 않습니다.

`view-state-exit-error-red.log`는 위치 오류를 보고했지만 유효한 draft의 mirror가 0개라 기대한 1개와 달라 실패했습니다. 0통과/1실패·0.03초·exit 101입니다. 위치 저장을 본문 mirror 뒤로 옮기고 위치 결과와 기존 layout flush를 분리했습니다. 위치 캡처/apply가 실패해도 유효한 본문과 기존 layout을 먼저 저장하며 위치 오류는 그대로 반환합니다. 정상 닫기는 기존 실패 처리로 돌아가고 직접 종료는 기존 drain 경계에서 작업을 회수합니다. 오류를 무시하거나 성공으로 바꾸지 않으며 변경한 종료 함수의 관련 두 검사만 확인합니다.

`view-state-exit-error-fixed.log`는 1통과/1실패·0.05초·exit 101(컴파일 20.48초)입니다. 새 위치 오류/draft mirror/오류 반환/작업 0 검사는 통과했습니다. 기존 서버 종료 검사는 IDE 서버 포트를 열 때 Operation not permitted로 실패했으며 종료 동작까지 도달하지 않았습니다. 소유 임시 데이터와 자체 loopback 서버의 실패 한 건만 권한 허용으로 실행하고 성공한 새 검사는 반복하지 않습니다.

`view-state-exit-server-permitted.log`의 실패 한 건은 권한 허용·exit 0으로 통과했습니다. 원래 서버/세션/pending/lockfile·감독 종료와 직접 오류 drain을 확인했습니다. 관련 종료 2건을 포함한 서로 다른 성공은 총 406건입니다. API/타입이 같은 private shutdown 내부의 본문 우선 순서만 바뀌었으므로 App tests 포함 compile 성공을 재사용하고 변경한 함수의 두 검사를 직접 확인했습니다. 마지막 소유 7Rust fmt/diff는 exit 0이고 변경하지 않은 Core/UI 본문·native-host 외의 frozen 성공은 재사용합니다. 전체 App/실기/전체 재시작이나 editor-67 전체 완료를 뜻하지 않습니다.

종료 저장 코드/검사·QA/현재 PROCESS의 9경로는 `f1805287`로 선별 커밋했습니다. 현재 기능표는 599행·392근거 경로·588대상·complete 298/partial 93/unwired 110/missing 87입니다. editor-67 partial·미완료 290·배치 23 잔여 23/최종 33을 유지합니다. 이전 사용자 문서/PROCESS 하단 변경은 보존하고 다음은 d3b2의 실제 탭 닫힘/최근 닫힘과 fresh capture 순서입니다.

## 탭 닫힘의 최신 위치 전달

이전 goal turn은 초기 복원/종료 저장의 실제 코드·검증·커밋/일반 push로 진척했습니다. 현재 f1805287/e7709ae1의 source와 보존된 작업 트리를 대조했고 d3b2를 이어갑니다. 첫 `view-state-tab-close-red.log`는 실제 Shift+Right의 마지막 선택을 만든 뒤 생산용 request_tab_close를 실행했습니다. 닫힘 자체는 완료됐지만 ClosedTab.tab.view_state가 None이라 실패했습니다. 0통과/1실패·21.16초·exit 101(컴파일 21.80초)입니다. 재열기/복원·내용/디스크의 후기 단언은 아직 통과로 세지 않습니다.

UI의 최신 Store를 요청하는 oneshot 경계를 App 포트가 공유하고 Host의 typed 닫힘 result 안에서 이를 먼저 적용합니다. UI는 mutation guard를 동기로 기다리지 않고 기존 poll로 PreparedDocument reply를 회수한 뒤 요청을 처리합니다. 소유자가 없는 기존 headless 포트는 빈 캡처를 사용하고 정상/직접 앱 종료는 최종 캡처를 대기 요청에도 전달해 UI 종료 뒤 대기를 남기지 않습니다. 기존 셸/IDE/remote·프로젝트/보조 창·preview 교체 경계는 d3b2 전체 범위로 남기며 기본 Host 한 경로를 전체 완료로 올리지 않습니다.

[Tokio oneshot 공식 계약](https://docs.rs/tokio/latest/tokio/sync/oneshot/fn.channel.html)의 태스크 간 단일 값 전달과 sender/receiver 수명, [Rust Arc 공식 계약](https://doc.rust-lang.org/std/sync/struct.Arc.html)의 공유 소유를 확인했습니다. 기존 sync feature와 std만 쓰고 새 의존성/RPC를 추가하지 않습니다. 최종 캡처는 Arc slice로 공유하며 mutation 적용은 slice를 빌려 같은 캡처를 불필요하게 deep clone하지 않습니다.

`view-state-tab-close-green.log`의 같은 실제 App은 1통과·21.26초·exit 0(컴파일 29.31초)입니다. 최근 닫힘의 저장값·같은 탭 재열기의 마지막 선택/실제 scroll·원문/dirty/디스크 보존이 통과했습니다. 초기 성공을 이후 관련 검사 수에 다시 합산하지 않습니다. 동시 요청·소유자 없는 포트·취소/최종 오류 회수·종료 취소 뒤 live 요청과 기존 관련 App/상태를 함께 확인 중입니다.

첫 `view-state-tab-close-related.log`는 authored sync 검사에서 빈 Arc::from([])의 slice/allocator 타입을 추론할 수 없어 E0282 1건·exit 101로 컴파일 실패했습니다. Arc<[Captured]>의 타입을 명시해 필수 추론 경계를 좁혔고 같은 관련 13건을 확인합니다. 실행 0건을 성공으로 세지 않습니다. 실제 lock은 Tokio 1.53.1이며 공식 최신 1.53.2와 기존 repo의 oneshot 동일 API를 대조했고 lock/의존성은 수정하지 않았습니다.

`view-state-tab-close-related-fixed.log`는 App 6/기존 상태 5/sync 2의 서로 다른 13통과/0실패·24.02초·exit 0(컴파일 17.84초)입니다. 실제 최근 닫힘/재열기와 초기/종료 복원·readonly·metadata·명시 이동 회귀, 소유자 없는 포트·동시 요청의 같은 Arc 공유·취소 회수·최종 오류 전달·종료 취소 뒤 live 재개를 확인했습니다. 이전 10건과 첫 기본 App 성공은 중복 합산하지 않습니다. 현재 Host 경계와 종료 입력의 Arc slice 변경을 관련 검사로 확인하며 Core/UI/frozen은 변경하지 않아 기존 성공을 재사용합니다.

`view-state-tab-close-host.log`는 전체 Host 9통과/0실패·0.13초·exit 0입니다. 기존 공개 연결 포트·파일/프로젝트 결과·PreparedDocument 미소비 회수·실제 mirror/저장·종료 layout flush·watcher 선행 대기 회귀를 확인했습니다. `connect_with_application_ports`만 공유 캡처를 공급하며 기존 headless 연결 경로는 기존 동작을 보존합니다. 소유자 없는 공유 포트도 UI 대기를 만들지 않습니다.

`view-state-tab-close-exit.log`는 권한 허용의 소유 종료 2건·exit 0입니다. 위치 캡처 실패의 실제 draft mirror/오류 반환·작업 회수와 서버/세션/pending/lockfile 종료를 확인했습니다. 현재 App 13/Host 9/종료 2와 변경 없는 Core 306/UI 88의 서로 다른 418건을 유지하며 기존 같은 성공을 추가 합산하지 않습니다. 소유 9Rust fmt/diff는 exit 0이고 manifest/lock/Core/UI/Syntax/엔진/frozen 변경은 없습니다. 기본 Host CloseTab만 연결했으며 셸/IDE/remote·프로젝트/slot/aux·새 파일/preview·folding contribution/전체 재시작과 전체 배치 게이트는 남습니다.

기본 Host 선행 저장과 요청 수명 코드/검사·QA/현재 PROCESS의 11경로는 `056ad927`로 선별 커밋했습니다. 기능표는 599행·393근거 경로·588대상·complete298/partial93/unwired110/missing87이며 editor-67 partial·미완료290·배치23 잔여23/최종33을 유지합니다. 다음은 셸 탭/프로젝트/창 전환과 나머지 Host/IDE/remote의 동일한 선행 저장입니다. 사용자 기존 문서와 PROCESS 하단 두 hunk는 보존합니다.

## 셸 전환과 Host 파일 열기

이전 turn은 기본 탭 닫힘의 코드/실제 검증/일반 push로 진척했습니다. 현재 056ad927/97035bb3와 실제 포트/셸 controller를 대조했습니다. 공유 캡처는 Host CloseTab에만 공급되고 셸 mutation과 Host OpenFileTab에는 없습니다. 실제 원본 hook은 editor/tab 전환의 layout cleanup에 위치를 저장하므로 전환 전에 같은 live 캡처가 필요합니다. 추가 앱 검사는 영구 본문에서 다른 셸 탭 또는 Host의 새 preview 파일로 이동한 뒤 이전 본문 Tab.view_state의 마지막 UTF-16 선택을 단언합니다. 기존 preview 슬롯 교체의 제거된 탭/모델 캐시는 별도 대조가 필요하며 이 검사를 해당 경계의 완료로 확대하지 않습니다.

첫 `view-state-transition-red.log`는 authored 검사에서 PaneNode import 누락과 연쇄 pattern 타입 오류로 E0433 2/E0277 1·exit101의 컴파일 실패입니다. 실제 모델의 PaneNode를 import하고 같은 재현을 확인합니다. 실행 0건을 기능 실패나 성공으로 세지 않습니다.

`view-state-transition-red-fixed.log`는 실제 셸 ActivateTab 완료 뒤 이전 본문의 view_state None에서 0통과/1실패·21.18초·exit101(컴파일20.68초)입니다. Host preview 사례는 앞선 실패 때문에 아직 도달하지 않았습니다. UI controller의 optional async port를 실제 App의 공유 캡처에 연결했습니다. 탭/프로젝트/slot 전환·닫힘/이동/분할만 캡처하며 focus-only·resize 등 뷰를 교체하지 않는 작업은 기존 경로를 사용합니다. mutation guard 전에 요청/적용을 마치고 실패하면 기존 errors 경계로 반환하며 해당 mutation은 실행하지 않습니다. App 전용 소비자가 UI crate에 역참조되지 않도록 AppState/EventSink·std Future 타입 계약을 사용합니다.

`view-state-transition-shell-fixed.log`는 이전 셸 사례를 지나갔지만 같은 실패 단언에 mode가 없어 어느 사례인지 명확히 구분하지 못했습니다. 진단에 is_host를 추가한 `view-state-transition-mode.log`는 is_host=true의 view_state None으로 0통과/1실패·21.34초·exit101(컴파일15.88초)입니다. 셸 저장 단언을 통과한 뒤 Host OpenFileTab의 실제 누락에 도달했습니다. 두 사례를 한 검사에 묶어 통과한 셸 사례를 재실행한 한계도 보존하며 새로운 성공으로 중복 집계하지 않습니다. Host OpenFileTab/팔레트/옆 열기/새 untitled를 기존 typed 결과의 async 경계에서 같은 선행 캡처에 연결했습니다. 읽기 전용 문서 load는 캡처를 기다리지 않아 PreparedDocument reply의 회수 경계를 유지합니다.

`view-state-transition-green.log`는 실제 셸 전환과 Host의 새 preview 파일 열기 두 사례가 1통과·21.62초·exit0(컴파일23.45초)입니다. 이전 영구 본문의 마지막 UTF-16 anchor/head가 Tab.view_state에 도달하고 원문/디스크를 보존했습니다. 제거되는 기존 preview 탭의 모델 캐시나 project/slot/aux·IDE/remote·전체 재시작까지 완료한 것으로 확대하지 않습니다. generic controller의 capture 실패/guard 밖 적용/focus-only·기존 controller 회귀와 현재 App/Host 소비를 확인합니다.

`view-state-transition-controller.log`는 전체 controller 3통과·0.00초·exit0(컴파일10.28초)입니다. 선행 캡처 실패 때 ActivateTab이 실행되지 않고 errors 경계로 전달됨, 성공한 캡처 안의 기존 mutation guard 획득/저장이 완료된 뒤 전환됨, focus-only의 캡처 생략과 정상 worker 회수·기존 외부 이벤트/snapshot/revision/닫힌 프로젝트를 확인했습니다. authored 검사에서 read guard의 수명을 다음 쓰기까지 늘리지 않도록 비교 대상 layout을 clone한 뒤 read guard를 즉시 회수했고 실행 전 정리한 내용은 실패 재현으로 세지 않습니다.

`view-state-transition-related.log`는 actual App 7/상태 5/sync 2의 서로 다른 14통과/0실패·20.80초·exit0입니다. 새 셸/Host 전환과 기존 실제 탭 닫힘/재열기·초기/정상·직접 종료 복원·readonly·잘못된 상태/metadata·명시 이동·동시 요청/취소/최종 오류/live 재개를 확인했습니다. 초기에 따로 통과한 동일 검사나 이전 13건을 추가 합산하지 않습니다. 현재 Host와 종료 회귀를 확인하고 변경이 native-host controller에 한정된 frozen과 변경 없는 Core/UI 본문 성공은 재사용합니다.

`view-state-transition-host.log`는 전체 Host 9통과·0.13초·exit0입니다. 변경한 파일/팔레트/옆 열기의 결과와 기존 document lease·mirror/저장/retarget symlink·종료 flush·watcher 회귀를 확인했습니다. 기존 생성 포트/헤드리스 경로와 요청/실패 결과를 보존하며 새 공개 RPC/의존성은 없습니다.

`view-state-transition-exit.log`는 소유 loopback 서버 권한 허용의 종료 2건·exit0입니다. AppState/EventSink로 분리한 동일 mutation 적용과 기존 draft 우선 오류 처리·서버/세션/pending/lockfile·작업 회수를 확인했습니다. 현재 App14/Host9/종료2/UI controller3과 변경 없는 Core306/UI본문·geometry88의 서로 다른422건입니다. 소유7Rust fmt/diff·manifest/lock/Core/구문/엔진/frozen 불변을 확인했고 controller는 기존 native-host cfg 내부이므로 frozen 성공을 재사용합니다. 본문/geometry renderer도 변경하지 않았습니다. 전체 App/OS/배치 통과나 editor-67 전체 완료로 올리지 않습니다.

셸/Host 전환 코드·검사와 QA/현재 PROCESS의 9경로는 `1874334e`로 선별 커밋했습니다. 기능표는 599행/393근거 경로·588대상·complete298/partial93/unwired110/missing87·미완료290이며 editor-67 partial·배치23 잔여23/최종33을 유지합니다. 다음은 제거되는 preview 모델 캐시·batch close·project/slot/aux·IDE/remote의 같은 선행 저장과 소유 수명입니다. 사용자 기존 변경과 PROCESS 하단 두 hunk는 보존합니다.

## 교체된 preview 모델의 메모리 위치

실제 TS `model-registry.ts`는 경로별 모델과 viewState를 보관하고 `code-editor.tsx`는 모델 교체 때 이전 위치를 저장한 뒤 같은 모델을 다시 붙일 때 복원합니다. 명시적 닫힘의 모델 disposal과 달리 layout 서비스의 preview 교체는 이전 TabId를 새 TabId로 대체하며 최근 닫힘 이력도 만들지 않습니다. native는 문서 모델을 보관하지만 새 ViewId의 위치가 기본값이라 복원이 누락됐습니다.

`view-state-preview-reopen-red.log`는 실제 Shift+Right와 preview 교체 두 번 뒤 같은 DocumentId임을 확인하고 선택 비교에서 실패했습니다. 관찰값은 anchor/head 0/0, 기대값은 18/12입니다. 0통과/1실패·20.99초·exit101(컴파일20.98초)이며 후기 scroll/원문/디스크 단언은 실패 단계의 성공으로 세지 않습니다.

기존 선행 캡처에서 preview의 typed 위치를 창/pane/document별로 보관합니다. 새 탭의 유효한 저장값을 우선하고 누락/잘못된 값에만 같은 revision의 메모리 위치를 한 번 소비합니다. 문서 revision 변경/명시 disposal 또는 캡처 때 제거된 pane은 캐시를 회수합니다. 분리된 mirror 위치가 섞이지 않도록 pane을 구분하며 원본의 전역 경로 캐시를 새 pane에 그대로 복제하지 않습니다. 기존 복원/실제 geometry 앵커 경계를 재사용하며 새 의존성이나 모델 복제는 없습니다.

`view-state-preview-reopen-green.log`의 같은 실제 App은 1통과/0실패·20.46초·exit0(컴파일29.88초)입니다. 새 TabId와 같은 DocumentId, 마지막 역방향 선택과 실제 scroll, 원문/revision/dirty/디스크 보존을 확인했습니다. 캐시 수명/우선순위와 기존 App/상태/sync 회귀는 현재 관련 검사에서 확인 중입니다. project/slot/aux·batch/IDE/remote와 folding/전체 재시작, editor-67 전체 완료는 아직 남습니다.

`view-state-preview-related.log`는 실제 App8/상태6/sync2의 서로 다른16통과/0실패·26.39초·exit0(컴파일19.07초)입니다. 새 재열기와 분리된 뷰/창·pane 격리·정상 저장값 우선/잘못된 값 fallback·같은 revision/명시 disposal, 기존 초기/전환/닫힘/정상·직접 종료·UTF-16/readonly/명시 이동·공유 요청 수명을 확인했습니다. 앞선 단독 성공이나 기존14건은 중복 합산하지 않습니다. `view-state-preview-exit.log`는 소유 loopback 허용의 기존 종료2건·0.06초·exit0입니다. 변경 없는Host9/controller3/Core306/UI88을 재사용한 서로 다른424건입니다. 소유4Rust fmt/diff가 통과했고 manifest/lock/renderer/Core/engine/frozen은 변경하지 않았습니다. 전체 diff의 사용자 architecture 말미 공백 경고는 기존 변경으로 보존하고 소유 경로 diff는 통과했습니다. 전체App/실기/재시작·외부 포트/종료·folding과 editor-67 전체 완료로 확대하지 않습니다.

preview 재열기 코드/검사·QA/소유 PROCESS의6경로는 `b05c1c2d`로 선별 커밋했습니다. 기능표는599행/393근거 경로·588대상·complete298/partial93/unwired110/missing87·미완료290이며 editor-67 partial·배치23 잔여23/최종33을 유지합니다. 다음은 IDE/remote와 project/slot/aux 종료의 동일한 선행 캡처/소유 수명입니다.

## IDE 명령의 최신 위치

실제 App의 application ports는 IDE LayoutActions를 구성하지만 같은 view_states 공유 포트를 전달하지 않습니다. IDE openFile은 경로/열린 프로젝트/실제 파일 검증 뒤 직접 layout을 변경하고 close_tab/closeAllDiffTabs도 직접 닫습니다. 기존 Headless LayoutActions와 RPC 입력/응답·서버 인증 경계는 보존합니다. 실제 App 검사에서 조립된 IDE action을 사용해 입력 직후 openFile/close_tab의 이전 탭과 최근 닫힘 위치를 확인합니다.

첫 `view-state-ide-red.log`는 작성한 검사에서 fixture_with_view_state의 필수 text 인자를 누락해 E0061 1건·exit101로 컴파일 실패했습니다. 실제 시그니처의 text를 공급했으며 실행 0건을 제품 실패/성공으로 세지 않습니다.

`view-state-ide-red-fixed.log`는 실제 App 조립 포트의 IDE openFile이 완료된 뒤 이전 Tab.view_state None에서 0통과/1실패·27.95초·exit101(컴파일33.43초)입니다. 뒤의 IDE 닫힘 단언에는 아직 도달하지 않았습니다. 같은 공유 포트를 App constructor가 IDE LayoutActions에 전달하며 openFile의 파일 경계 검증 뒤, 실제 닫힘 대상이 있는 close_tab/closeAllDiffTabs 앞에서 최신 캡처를 적용합니다. 캡처 오류는 기존 ToolError로 반환하고 해당 mutation/닫힘 이벤트를 실행하지 않습니다. 잘못된 입력/읽기 도구/닫힐 대상 없음은 캡처를 기다리지 않습니다. 기존 headless 포트는 unbound 공유 포트를 사용하고 기존 fn pointer/open·close 동작/RPC·서버 인증은 유지합니다.

`view-state-ide-green.log`의 실제 App은1통과/0실패·20.30초·exit0(컴파일32.54초)입니다. 입력 직후 openFile 이전 탭의 UTF-16 역방향 선택과 비활성 본문의 추가 이동 직후 close_tab 최근 닫힘 위치·원문/dirty/디스크 보존을 확인했습니다. `view-state-ide-tools.log`는 새 실패 경계와 기존 IDE 도구/RPC/실제 responder·소유 PTY의5통과·0.05초·exit0(컴파일18.64초)입니다. 필수 값/경계 밖 입력·읽기/대상 없음은 캡처 요청0, 캡처 실패 때 open/close/일괄 diff close 모두 동일 layout/이벤트0·대기0·작업 회수를 확인했습니다.

`view-state-ide-server.log`는 소유 loopback 허용의 기존 인증/MCP/RPC/동기 알림·pending/lockfile·지연 owner 회수4통과·0.07초·exit0입니다. `view-state-ide-ports.log`는 실제 조립/dispatch/persist/event/lazy ports·소유 PTY agent 회수2통과·0.05초·exit0입니다. 변경 없는App/상태/sync16·종료2·Host9/controller3/Core306/UI88을 재사용한 서로 다른436건이며 이전 단독 성공을 추가 합산하지 않습니다. 소유7Rust fmt/diff는 exit0이고 manifest/lock/renderer/Core/engine/frozen 불변과 사용자 기존 변경을 보존합니다. 전체App/실기·remote/project/slot/aux/batch·folding/전체 재시작과 editor-67 전체 완료는 별도 미완료입니다.

IDE 선행 저장 코드/검사·QA/소유 PROCESS의9경로는 `958dbf87`로 선별 커밋했습니다. 기능표는588대상·complete298/partial93/unwired110/missing87·미완료290이며 editor-67 partial·배치23 잔여23/최종33을 유지합니다. 다음은 remote 명령과 project/slot/aux·batch 종료에서 최신 위치가 누락되지 않는지 실제 경로별 대조/재현입니다.

## 원격 레이아웃과 프로젝트의 최신 위치

이전 turn은 preview 메모리 재열기와 IDE 최신 위치의 구현/실제 검사·문서/배치 갱신·일반 push로 진척했습니다. 현재6906347b/958dbf87의 실제 원격 조립을 확인했고 remote_layout/remote_projects에는 동일 캡처 포트가 없습니다. 기존 outer command policy 뒤 typed argument와 파일/프로젝트 경계 검증을 유지하며 탭 열기/닫힘·전환/분할·프로젝트/슬롯 수명 앞에서 선행 저장합니다. metadata/명시 viewState/읽기/resize 등은 기존 경로를 유지합니다. 실제 App 조립 dispatch로 이전 본문 위치/최근 닫힘과 프로젝트 detach 전 디스크 layout을 검증합니다.

첫 `view-state-remote-red.log`는 작성한 fixture의 TabKind 태그를 실제 `kind` 대신 `type`으로 지정해 열기 결과 오류에서0통과/1실패·18.71초·exit101(컴파일23.87초)입니다. 실제 모델의 serde 계약을 읽고 TabKind 자체를 직렬화하도록 수정했습니다. 입력 오류를 제품의 위치 실패로 세지 않으며, 후기 위치/닫힘/프로젝트 종료 단언에는 도달하지 않았습니다.

`view-state-remote-red-fixed.log`는 실제 원격 파일 열기가 끝난 뒤 이전 Tab.view_state None에서0통과/1실패·20.79초·exit101(컴파일30.26초)입니다. App 조립 dispatch가 같은 공유 캡처를 remote layout/project 포트에 전달하도록 연결했습니다. mutation future를 만들 때 typed 인자를 먼저 검증하며 실제 mutation을 poll하기 전에 캡처를 적용합니다. 파일 열기의 기존 root guard와 프로젝트 root/슬롯 검증도 캡처 전에 수행하며 원래 서비스는 실제 변경 시 다시 검증합니다. 명시 viewState/metadata/읽기/resize는 캡처하지 않고 기존 outer policy·typed 오류/async 작업·close flush 순서를 유지합니다.

첫 `view-state-remote-green.log`는 native App의 직접 의존성에 taide-project가 없는데 직접 import한 작성 오류로 E0432/E0433·실행0건·exit101입니다. 실제 manifest를 확인하고 기존 taide-runtime project_actions 경계가 필요한 원본 검증 함수만 재노출하도록 수정했습니다. 새 의존성/lock 변경은 없으며 프로젝트 경로 검증은 기존 localized 오류/홈 경로/읽기 가능 디렉터리 확인을 동일 함수로 공유합니다. Host OpenProject도 같은 경계 뒤 선행 저장하며 실제 앱 검사와 관련 포트/서비스 회귀를 확인 중입니다.

`view-state-remote-green-fixed.log`의 실제 App은1통과/0실패·22.02초·exit0(컴파일39.34초)입니다. 원격 파일 열기 전 마지막 선택·비활성 본문 추가 이동 뒤 최근 닫힘·프로젝트 종료 직전 peer 위치가 디스크 layout에 도달한 뒤 실제 project/layout detach가 완료되는 순서와 원문/디스크 보존을 확인했습니다. 이후 프로젝트 닫힘의 lifecycle factory 생성도 캡처 성공 뒤로 옮겨 실패 때 생성/부수 효과를 막았습니다. 성공 경로는 동일하며 새 오류 경계 검사가 이를 직접 확인합니다.

`view-state-remote-boundaries.log`는7통과·0.10초·exit0(컴파일23.12초)입니다. 새 레이아웃/프로젝트 두 경계는 typed 값·파일/프로젝트 root·슬롯 중복/존재/권한 거절과 읽기/명시 viewState의 캡처 요청0, 캡처 실패의 레이아웃10/프로젝트7 명령의 mutation/이벤트0·lifecycle 생성/trace0·대기/작업 회수를 확인했습니다. 넓은 한국어 필터로 원격 URI·설정 보호·테마 owner·터미널 query의 불변4건도 실행한 한계를 기록하고 반복하지 않습니다. 기존 소유 PTY 닫힘1건의 성공은 뒤의 모듈 검사에서 제외해 재사용했습니다.

`view-state-remote-layout.log`는 이전 성공2건을 제외한 나머지 레이아웃3통과·0.01초·exit0입니다. `view-state-remote-projects.log`는2통과/1실패·0.66초·exit101입니다. 새 사전 검증이 작업 감독 종료의 Forbidden보다 먼저 슬롯/그룹 NotFound를 반환하는 회귀를 확인했습니다. 열기3진입점은 typed parsing 뒤 기존 nonabortable 작업을 접수하고 그 작업 안에서 경계 검증/캡처/서비스를 수행하도록 수정했습니다. 캡처 대기 중 호출자가 취소돼도 접수한 작업의 소유가 유지되며, 종료 거절/파일·슬롯 검증과 캡처 실패의 mutation 억제를 함께 보존합니다. `view-state-remote-projects-fixed.log`는 관련 프로젝트4통과·0.65초·exit0(컴파일14.53초)입니다. 변경된 성공2건과 실패 영향/new 경계를 확인한 재검사는 중복 집계하지 않습니다.

`view-state-remote-cancel.log`는 캡처 대기 중 원격 호출자를 취소하고 같은 요청에 빈 합성 캡처를 전달한 뒤 실제 프로젝트 등록/레이아웃/소유 watcher와 작업0 회수를 확인한 새1통과·0.19초·exit0(컴파일23.28초)입니다. 위치 데이터 자체는 앞선 실제 App으로 검증하며 이 검사는 요청/감독 수명 경계의 근거입니다. `view-state-remote-dispatch.log`는 전체177명령 routing/default deny·동일 services/파일/raw/search/설정·소유 PTY의3통과·0.16초·exit0입니다. `view-state-remote-host.log`는 전체Host9통과·0.06초·exit0이며 기존 미소비 문서 lease·mirror/저장·초기 watcher 대기·실제 종료 flush 오류를 확인했습니다.

추가 실제 App 검사에서는 두 프로젝트/슬롯의 원격 focus/close/activate와 Host OpenProject의 최신 위치·본문/디스크 보존을 확인합니다. 첫 `view-state-remote-slots.log`는 작성한 검사에서 Path import 누락으로 E0433 1건·실행0건·exit101입니다. std::path::Path를 import하고 같은 검사를 확인 중이며, 미실행 후기 단언을 성공으로 세지 않습니다.

`view-state-remote-slots-fixed.log`는 실제 원격 슬롯 focus/close·project_activate와 Host OpenProject의 네 전환에서 본문 최신 위치·원문/dirty/디스크 보존이1통과·77.12초·exit0(컴파일26.19초)입니다. 두 프로젝트/슬롯과 실제 본문 Shift+Right 뒤 위치를 사용하며 headless mutation 성공이나 단순 코드 연결을 실제 App 검증으로 대체하지 않습니다. 현재 compile/준비 시간 변동의 원인을 이 성공만으로 해결했다고 보고하지 않습니다.

첫 `view-state-remote-project-core.log`는 native 독립 workspace의 dependency package가 dev-dependency를 가진 taide-project 검사를 허용하지 않아 실행0건·exit101입니다. 실제 taide-project가 속한 기존 root workspace manifest와 lock을 사용해 동일 소유 target에서 기존 프로젝트 서비스 전체를 확인합니다. 프로젝트 원본 함수의 본문과 localized 오류/홈 확장·읽기 가능 디렉터리 규칙은 그대로이며 별도 검증 명령을 임의 통과로 대체하지 않습니다.

`view-state-remote-project-core-workspace.log`는 실제 기존 workspace의 프로젝트서비스/그룹/슬롯 전체98통과·0.51초·exit0(컴파일2분01초)입니다. localized missing root·디렉터리/홈 경로·재열기/복원·프로젝트와 슬롯/그룹 저장/수명을 확인했습니다. `view-state-remote-ports.log`는 조립/dispatch/persist/event/lazy ports·소유 agent 회수2통과·0.10초·exit0, 소유 loopback 허용의 `view-state-remote-exit.log`는 실제 종료 저장/오류·서버/세션/pending/lockfile·감독 회수2통과·0.05초·exit0입니다.

현재 실제 App2·원격레이아웃5/프로젝트5/dispatch3·넓은 필터의 불변4·Host9/포트2/종료2·프로젝트서비스98은 로그의 package/test 이름을 합쳐 서로 다른130건입니다. 기존436건에서 다시 실행한Host9/포트2/종료2를 제외한423건을 재사용해 서로 다른553건이며 같은 성공을 추가 합산하지 않습니다. 소유12Rust fmt/diff는 exit0이고 manifest/lock/UI renderer/Core editor/engine/frozen은 변경하지 않았습니다. 런타임의 파일 검증 함수 공개와 프로젝트 검증 함수의 제한된 재노출은 기존 서비스 경계를 재사용하며 새 의존성/RPC가 없습니다. editor-67 partial·complete298/partial93/unwired110/missing87·미완료290·배치23 잔여23/최종33·전체율/잔여시간 미산정을 유지합니다. 실제 batch/aux·folding contribution/전체App 재시작과 전체App/실기/신뢰성/출시 게이트는 아직 남습니다.

원격 선행 저장 코드/검사·QA/소유 PROCESS의14경로는 `fe9cf0fc`로 선별 커밋했습니다. 기능표의 상태와 전체 배치 ID 배정을 실제 JSON으로 대조했고 미완료290·배정누락/중복/미확인ID0·최종33을 유지합니다. 다음은 실제batch/aux의 나머지 경로와 folding contribution·디스크에서 새 App으로 재시작하는 복원입니다.

## 접기 contribution 저장과 복원

이전 turn은 원격 탭/프로젝트/슬롯의 실제 저장·요청/감독 수명 구현과 관련130건·선별 커밋/일반 push로 진척했습니다. 현재 실제 파일을 대조했고 editor-67은 partial입니다. 원본 Monaco folding controller/model은 collapsedRegions의 start/endLineNumber·isCollapsed·source·UTF-16 signed checksum과 lineCount/provider를 저장합니다. provider가 아직 준비되지 않아도 저장 영역을 복원하며 source가 provider 이외면 펼친 수동 영역도 보관합니다. native ViewState의 folds/manual_folds는 있으나 기존 위치 복원이 contribution을 소비하지 않았습니다.

`view-state-folding-red.log`는 실제 본문 최초 복원에서 folds가 빈 배열이고 기대값6..12라0통과/1실패·24.55초·exit101(컴파일27.08초)입니다. 후기 수동 명령/종료 저장은 아직 성공으로 세지 않습니다. private 접기 adapter를 기존 위치 저장/복원에 연결했습니다. malformed contribution/개별 영역을 cursor/scroll과 독립적으로 좁히고 원본 signed checksum/UTF-16·CRLF를 지원하며 범위/source·기존5000영역 상한과 native tier 게이트를 적용합니다. 기존 folding 모델이 중복/교차 영역을 정리하고 수동/펼친 수동 영역을 보존합니다.

새 native 저장에는 전체 접기 본문의 versioned hash를 추가해 첫/끝 줄이 같은 중간 변경도 거절합니다. 문서의 UTF-16 줄 hash/prefix를 한 번 계산해 각 영역의 검증은 일정 비용이고 CRLF/LF 변환과 영역 밖 변경은 허용합니다. 기존 Monaco checksum이 있는 legacy 값은 해당 계약으로 대조하며 hash가 없는 원본 상태도 읽습니다. 기본 checksum의 약한 중간 검증을 새 native 저장에 강제 복제하지 않습니다. unknown contribution과 같은 folding 객체의 unknown 필드는 nested merge로 보존하며 별도 라이브러리/공개 RPC는 없습니다.

`view-state-folding-green.log`는 같은 실제 App의 복원·숨긴 본문 표시·UnfoldAll/CreateFromSelection·펼친 수동 영역 종료 저장·원문/revision/dirty/디스크·future contribution 보존이1통과·30.15초·exit0(컴파일35.01초)입니다. 설치 Monaco hash.js를 직접 실행해 Unicode/CRLF 샘플의 signed checksum -646167을 확인했고 `view-state-folding-state.log`는 이 값의 호환성과 열린 수동/legacy 기본값·중간 변경 거절/영역 밖 및 EOL 변경 허용·잘못된 contribution/범위·readonly/기존 상태 회귀9통과·0.01초·exit0입니다. 단독 성공을 중복 합산하지 않습니다. 추가 실제 App 회귀와 nested future 필드 보존을 확인 중이며 실제batch/aux와 새 App 재시작은 별도 미완료입니다.

`view-state-folding-app.log`는 실제 본문 위치 관련 전체12통과·39.07초·exit0(컴파일19.06초)입니다. 추가한 같은 folding 객체의 futureField 보존도 확인했습니다. `view-state-folding-exit.log`는 기존 종료/저장 오류·본문 mirror 보존·서버/세션/감독 회수2통과·0.05초·exit0입니다. checksum 검사에 같은 문서의 다른 창/뷰를 추가해 접힌 일반 영역과 펼친 수동 영역이 서로 덮어쓰지 않는 실제 State 복원을 확인했고 `view-state-folding-mirror.log`는1통과·0.01초·exit0(컴파일16.24초)입니다. 이 한 건은 상태9건에 이미 포함되어 중복 집계하지 않습니다.

현재 관련 서로 다른23건에서 새 App1/상태3만 이전 누계553에 추가하므로 성공 근거는 서로 다른557건입니다. 소유4Rust fmt는 exit0입니다. 전체 working diff 검사는 기존 사용자 `docs/architecture.md:955`의 EOF 빈 줄로 exit2이며 해당 파일은 수정하지 않았습니다. 커밋 전 소유 경로/선별 staged diff만 따로 확인합니다. 새 dependency/manifest/lock/Core/UI/frozen 변경은 없으며 editor-67 partial·298/588(50.7%)·잔여290/배치23 잔여23/최종33을 유지합니다. 실제 batch/aux와 새 App 재시작·전체App 준비 실패/실기/성능/신뢰성/출시 게이트는 미완료입니다.
