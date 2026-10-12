# 배치 23 커서·스크롤 상태 복원

현재 상태: editor-67의 원본 저장/복원과 현재 native 소비 경계를 읽었습니다. 구현/실패 재현은 아직 실행하지 않았으며 기능표의 unwired 판정을 유지합니다. 기능 체크리스트는 298/588(50.7%)·미완료 290이고 전체 전환율/잔여 시간은 미산정입니다. workflow/서브에이전트 없이 메인이 진행합니다.

- [x] a. 원본 저장 형식·native 모델/표시·facade/종료 경계 대조
- [ ] b. 임시 App의 초기 복원 실패 재현·검증된 저장 형식과 복원 연결
- [ ] c. 탭/뷰 전환·닫힘·창/프로젝트/앱 종료의 최신 상태 저장 연결
- [ ] d. 실제 App/재시작·Unicode/scroll/mirror/readonly·잘못된 값/소유 수명 검증
- [ ] e. 관련 회귀/형식/경계·기능표/PROCESS·선별 Git 저장

## 원본 계약

`src/widgets/editor-pane/use-editor-view-state.ts`는 Tab.viewState 문자열을 구독하며 JSON 객체의 cursorState 배열과 viewState 객체를 확인합니다. 잘못된 JSON/최상위 형식은 복원을 생략합니다. 살아 있는 모델에만 초기 복원을 적용하고 같은 탭의 후속 layout/dirty/자기 저장 갱신은 커서를 다시 이동시키지 않습니다. 저장은 editor/tab 전환의 layout cleanup과 앱/창 종료의 별도 view-state flush registry에서 수행합니다. 탭별 마지막 전송 값과 같은 상태는 중복 전송하지 않습니다. disposed editor의 null 결과는 저장하지 않습니다.

설치된 Monaco의 `cursor/cursor.js`는 각 선택의 selectionStart와 position에 1부터 시작하는 모델 줄/UTF-16 열을 저장합니다. 역방향 선택의 anchor/head를 보존하며 복원 시 누락된 위치 축은 기본값/현재 위치로 보완하고 모델에서 유효한 좌표로 처리합니다. `codeEditorWidget.js`는 cursorState/viewState/contributionsState를 반환하고 실제 모델/뷰가 있을 때만 복원합니다.

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
