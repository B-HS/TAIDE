# 배치 20 — 정의·선언·타입/구현 이동과 참조·peek

현재 상태: batch20의 구현·실제 앱 수명 통합과 전체 게이트까지 체크리스트 6/7을 마쳤습니다. 변경 editor/UI/app/LSP SDK의 전체 121대상을 직접 1회 실행했고 최초 실패 3건은 영향 검사로 해소했습니다. 현재 서로 다른 성공은 editor 212·UI 375·app 689·SDK 82, 총 1358건입니다. frozen host/Wasm·4크레이트 포맷/diff exit 0, manifest/lock/engine 경계 유지·디스크 651GiB/65%입니다. 현재 기능 대응표는 완료 282/588(48.0%)이며 판정 갱신 전 수치입니다. 전체 출시 전환율/잔여 시간은 미산정입니다. 서브에이전트·workflow 없이 main이 직접 수행하고 Cargo/fmt는 실제 앞 process 종료를 확인한 뒤 하나씩 실행합니다. 아래 진행 로그의 당시 미완료 상태는 최신 결과로 대체합니다.

## 범위와 확인한 기준

TS `src/shared/lib/lsp/adapters/definition.ts`의 공용 location adapter와 declaration/type-definition/implementation, references adapter는 위치 요청·취소·Location/LocationLink 변환과 peek target preload를 연결합니다. 참조 요청은 includeDeclaration을 보존합니다. `src/shared/lib/lsp/peek-model-preload.ts`는 기존 모델을 우선하고 새 normal 모델의 파일 수/TTL을 제한하며 실제 편집 탭이 인수한 모델을 자동 폐기하지 않습니다. 실패하거나 대형/읽기 전용/refused인 파일은 새 peek 모델로 미리 불러오지 않습니다.

현재 native의 typed SDK·LSP 문서 mirror/owner/세대/취소, 문제 이동·workspace 심볼의 현재 창 preview/reveal와 기존 EditorStore/표시 view-zone·원본 명령 registry/keymap을 재사용했습니다. 정의/선언/타입/구현·참조의 실제 요청 및 본문/peek 소비자와 preview 편집/찾기/치환/접기/저장·미리보기 인수를 연결했습니다. 전체 게이트가 남았으며 순수 preview의 별도 LSP 공급과 일반 hover/완성 등은 후속 LSP 범위로 보존합니다.

원본 Monaco의 goToCommands/goToSymbol·link gesture·referencesModel·peek widget/controller/tree/CSS와 실제 TS의 openCodeEditor 경계를 더 읽고 단일/복수/없음·이동/옆 그룹/peek/순환·Ctrl/Meta+클릭의 동작을 확정합니다. 새 기능/디자인·원본 버그/내부 수치 강제 재현·engine/의존성·동결 browser 변경·OS 합성 입력은 추가하지 않습니다. 실제 데이터/OS 설정/clipboard/Keychain/Trash·보호 app bundle을 유지합니다.

## 체크리스트

- [x] a. 실제 TS/Monaco·공식 typed API·현재 native 공급/선택/열기/원본 화면 경계 대조
- [x] b. Location/LocationLink·UTF-16·정렬/중복/그룹·현재 요청/선택·peek 모델 수명 검증
- [x] c. 실제 typed LSP 5종·준비/미지원/빈/오류·취소/편집/닫힘/프로젝트/재시작 공급
- [x] d. native 명령/기본 키·Ctrl/Meta link gesture·단일/복수 이동·원본 peek 표시/입력 소비
- [x] e. 실제 앱의 현재 pane/기존/새 탭·옆 그룹·peek target loading/인수·dirty/readonly/reveal 통합
- [x] f. 변경 크레이트 전체 대상·동결 host/Wasm·포맷/diff·디스크/보호 확인
- [ ] g. 실제 QA/기능표/완료 근거/PROCESS·선별 커밋·일반 푸시·다음 범위

## 확인한 동작 계약

1. TS `position.ts`는 LSP의 0-based UTF-16 좌표를 그대로 보존해 Monaco에 +1로 전달합니다. LocationLink의 targetRange는 전체 선언 범위, targetSelectionRange는 이동/참조 표시 위치, originSelectionRange는 원본 링크 밑줄입니다. 5종 adapter는 취소를 요청 뒤와 preload 뒤 모두 확인합니다. 설치된 공식 lsp-types 0.97.0의 request.rs/lib.rs/references.rs 및 기존 SDK feature.rs의 typed 계약을 읽었습니다. 공식 웹 명세 조회는 목차 25줄만 반환했으므로 명세 본문을 읽었다고 판정하지 않습니다.
2. Monaco `goToSymbol.js`는 provider별 실패를 격리해 등록 우선순위대로 병합합니다. Go to References는 선언 포함 결과가 정확히 2개일 때 선언 제외를 다시 요청하고 1개이면 축약합니다. 명시 Peek References는 축약하지 않습니다. `referencesModel.js`는 URI/전체 targetRange로 정렬·중복 제거하고 원본 첫 provider 위치를 별도로 보존합니다. UI 선택/이동에는 targetSelectionRange를 사용하며 파일 그룹·현재 위치 근접 선택·끝에서 순환을 보존합니다.
3. 원본 기본 복수 결과 처리 방식은 5종 모두 peek입니다. 정의/선언/타입의 단일 결과가 현재 위치이면 goToReferences를 대체 실행하며 구현/참조는 대체 명령이 없습니다. 이동은 선택을 시작 위치로 접고 화면 밖이면 상단 근처로 드러냅니다. 명시 peek는 결과 개수와 무관하게 열립니다. F12/Shift+F12/Mod+F12/Mod+Shift+F12, Alt+F12(Linux Mod+Shift+F10), Mod+K 뒤 F12 옆 열기를 보존합니다.
4. `referencesWidget.js`/controller/tree/CSS의 원본 peek는 기본 18줄·미리보기 70%/트리 30%이며 크기/비율을 바꿀 수 있습니다. 파일 하나는 그룹 제목을 생략하고 참조 줄을 표시하며 여러 파일은 경로·개수/23px 줄로 묶습니다. 미리보기는 원본 embedded editor로 편집 가능하고 같은 파일의 미저장 내용을 공유합니다. 트리 단일 클릭은 미리보기, Enter/더블 클릭은 이동, modifier는 옆 열기, F4/F12·Shift 변형은 이전/다음, Mod+K F2는 포커스 교대, Escape는 닫기입니다. 새 디자인으로 교체하지 않습니다.
5. `peek-model-preload.ts`는 기존 모델을 건드리지 않고 새로운 normal 파일만 중복 제거 후 최대 8개 불러옵니다. 60초 TTL이며 편집 탭이 인수하거나 attached editor가 있으면 자동 폐기하지 않습니다. native에서도 기존 dirty 본문을 우선하고 순수 미리보기 파일 읽기와 hot-exit mirror 복원/실제 편집 탭 인수를 구분해야 합니다. 대형/읽기 전용/refused의 새 preload는 제외하며 선택한 추가 파일은 resolver 경로로 필요할 때 읽습니다.
6. TS opener bridge는 동일 URI이면 정확한 원본 editor가 처리하도록 넘기고 untitled는 기존 editor만 선택합니다. 다른 file은 현재 window의 focused pane에 preview 탭/시작 위치로 전달합니다. 옆 열기 인자를 bridge가 전달하지 않는 관찰된 누락은 정상 옆 그룹 기능으로 연결하며 오류를 강제로 재현하지 않습니다. TS의 file_open backend와 native file worker 모두 열린 프로젝트/CLI 허용 경로 root guard를 사용하므로 임의 외부 파일 읽기 권한을 넓히지 않습니다. 원본 링크 gesture는 기본 mac Meta/그 외 Ctrl, Alt 옆 열기이며 눌렀던 줄/수정키·원본 선택/스크롤/문서 버전이 변하면 취소합니다.

## 현재 검증과 잔여 게이트

`cargo test --test symbol-locations --manifest-path native/taide-native-editor/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`는 exit 0, 새 6건 통과입니다. 로그는 `/private/tmp/taide-batch20-locations-model.log`입니다. scalar/array/link, targetRange와 선택/origin, 전체 범위 기준 정렬/중복과 provider 첫 위치·파일 그룹, 반전/탈출 범위 거절, 파일 우선 근접 선택/양방향 순환, UTF-16 보조 단위 문맥과 다중 줄 미리보기를 확인했습니다. 앱 요청/미리보기 수명과 실제 LSP/표면 연결은 아직 완료로 판정하지 않습니다.

`cargo test --lib --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target editor_locations::tests`는 exit 0·4통과(427 filtered), `/private/tmp/taide-batch20-request-model.log`입니다. 요청 교체·현재 선택/문서 버전·편집·뷰 닫힘·프로젝트 종료·provider 교체·늦은 응답과 file URI의 퍼센트 경로/원격·쿼리·조각/NUL 거절을 확인했습니다. 같은 옵션의 `peek_models::tests`는 exit 0·4통과(427 filtered), `/private/tmp/taide-batch20-peek-models.log`입니다. 기존/중복 제외·첫8개·60초 TTL·attached preview와 실제 탭 인수·기존 dirty 본문·대형/readonly/lossy 제외를 확인했습니다. TTL 만료 시 dirty 미리보기는 폐기하지 않고 편집 탭이 인수할 때까지 보존해 원본의 잠재적 내용 손실을 강제로 재현하지 않습니다. 실제 파일 worker/본문/앱 연결은 c~e 미완료입니다.

`cargo test --manifest-path crates/taide-lsp/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target 문서별_기능_공급`의 최초 실행은 새 테스트에서 DocumentMirror.version 누락·typed 등록/해제와 change 인자를 잘못 사용해 컴파일 exit 101입니다. 실제 기존 API에 맞춘 영향 재검사는 exit 0·1통과(81 filtered), `/private/tmp/taide-batch20-sdk-feature-support-after-api.log`입니다. 정적/동적 언어·scheme/패턴 selector·등록 해제·닫힘·채널 종료와 같은 기능/문서에서 Arc 공급 재사용을 확인했습니다. 런타임의 새 의존성/engine은 추가하지 않았습니다.

`cargo build --example native-lsp-mock --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target`의 최초 실행은 서로 다른 요청의 TypedReply<R>를 동일 match 결과로 직접 합친 타입 오류로 exit 101입니다. 공용 `.value`로 각 typed 응답을 변환한 재검사는 exit 0, `/private/tmp/taide-batch20-locations-mock-build-after-typed.log`입니다. 새로운 typed 요청·linkSupport 4종/references 초기화·프로젝트/문서/세대/capability 확인과 별도 취소 worker를 연결했습니다.

`cargo test --lib --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target lsp::symbol_location_tests` 최초 실행은 5건 모두 15초 timeout으로 exit 101입니다. 대표 1건의 readiness/상태/stderr 진단에서 TransportClosed와 `fixture requires valid roots and client capabilities`를 확인했습니다. 새 locations 모드가 유효한 native root 대신 null root를 요구하던 fixture 조건을 수정했습니다. 실제 mock 재빌드 exit 0(`/private/tmp/taide-batch20-locations-mock-build-after-roots.log`) 후 같은 영향 모듈은 exit 0·5통과·431 filtered, `/private/tmp/taide-batch20-lsp-locations-after-roots.log`입니다. 초기 실패를 합격에 포함하지 않습니다. 검증 서버는 App의 edition 2024 example으로 빌드했고 standalone prototype lock/edition 부채는 우회하지 않았습니다.

실제 child 5건은 5종 scalar/array/link·전체/선택/origin·참조 includeDeclaration 축약과 명시 peek, 빈/null/오류/잘못된 범위·미지원, 보류 중 다른 문서 동기화·편집/뷰 닫힘의 cancel 전송, basedPyright/ruff 두 provider의 역 등록 우선순위/부분 오류와 다른 프로젝트 owner 격리, 서버 재시작 뒤 mirror 재생과 이전 요청 만료를 확인했습니다. 모든 fixture task 종료 뒤 tracked_count 0을 확인했으며 OS 입력/실제 사용자 데이터는 사용하지 않았습니다. native UI·실제 앱 파일 열기/미리보기 소비는 d~e 미완료입니다.

변경 없는 경계만 batch19의 editor 206·UI 362·app 663·LSP SDK 81 및 개별 이전 syntax/egui SDK 근거를 재사용합니다. 변경한 크레이트는 f에서 전체 대상을 직접 1회 실행하고 실패 영향만 재검사합니다.

보호 Trash 3·기존 ignored 5·실제 OS/IME/접근성/pixel·대형 성능/soak/패키징/출시·테마의 7자리 HEX 결정은 미완료입니다. experiments/lsp-coordinator-spike standalone lock 불일치와 edition 2021/2024 포맷 차이는 batch19 QA에 기록한 전체 workspace/CI 부채이며 frozen browser 또는 lockfile을 우회하여 해결하지 않습니다.

## 표시·입력과 실제 파일 host 진행 근거

앱 테스트 코드 전체 컴파일은 `/private/tmp/taide-batch20-link-app-check.log`에서 exit 0입니다. 최초 투영 borrow, 경로 async 수명, callback lifetime, 새 Reply exhaustive 오류는 API/소유 경계에 맞춰 수정했습니다. 실제 파일 host 회귀 3건은 `/private/tmp/taide-batch20-location-host.log` exit 0·3통과(437 filtered)입니다. 주창/보조창의 현재 pane·UTF-16·preview, 오른쪽 그룹·기존 탭 보존과 cancel·프로젝트/탭/layout 변경·root 밖 대상 거절을 확인했습니다.

메모리 UI의 참조 zone/제목/tree/편집 가능 미리보기·readonly 원본문 격리·연속 tree 키/옆 열기/Escape 뒤 본문 입력·빈 결과 4건은 입력 소유 fallback 수정 뒤 통과했습니다(`/private/tmp/taide-batch20-ui-locations-after-focus.log`). 플랫폼 Meta/Ctrl+Alt 클릭·눌렀던 줄·나중에 추가한 modifier/다른 줄/빈 여백/미지원/스크롤·편집 변경 거절을 더한 최신 대상은 `/private/tmp/taide-batch20-definition-link-after-api.log` exit 0·6통과입니다. 첫 추가 회귀 컴파일에서 Scroll 타입/RawInput modifiers 인자를 잘못 사용한 실패는 실제 ScrollPosition/Event::ModifiersChanged API로 수정했습니다. OS 합성 입력은 사용하지 않았습니다.

미리보기 명령의 본문 오배달을 피하도록 store를 받는 직렬 editor keymap 경계와 preview view별 focus/단축키를 연결하고 있습니다. preview 편집 명령/저장은 실제 대상 모델을 소비하며 peek F12/닫기/포커스 키는 창의 기본 정의 명령보다 먼저 처리합니다. 현재 연결 컴파일 `/private/tmp/taide-batch20-preview-keymap-check.log` exit 0이며 실제 preview 명령·저장/앱 수명 검증은 아직 남았습니다. d/e/f는 완료로 표시하지 않습니다. hover 요청은 열린 peek와 분리하고 최신 token의 reply만 close하며 파일 preload reply도 현재 root 소유자를 다시 확인합니다.

## 앱의 실제 정의·미리보기 파일 경로

`/private/tmp/taide-batch20-navigation-reference-tests.log`는 exit 0·8통과(435 filtered)입니다. preview view의 원본 언어/직렬 편집 명령·대소문자 변환 재지정·실제 대상 Save snapshot, 독립 hover 교체/늦은 close·열린 peek 보존, source anchor와 참조 targetRange/선택의 줄 삽입 추적, 상단 reveal 요청과 350ms 강조 만료를 확인했습니다. 첫 preview 변환 실패는 fixture가 mac_cmd 없이 command만 전달한 것을 실제 플랫폼 modifier로 수정했습니다. 진단 출력은 제거했습니다. location 키 최초 1실패의 IME 기대를 기존 보조 키 허용 계약에 맞춘 최신 검사 `/private/tmp/taide-batch20-location-keys-after-ime-exact.log`는 exit 0·1통과(6 filtered)입니다. 잘못된 이름으로 0건 실행된 두 로그는 합격 근거에 포함하지 않습니다.

실제 mac modifier로 확대한 UI 클릭 취소 검사에서는 미지원 provider인데 hover 요청이 발생하는 실패를 재현했습니다(`/private/tmp/taide-batch20-definition-link-real-modifiers.log`, 5통과·1실패). 입력과 paint의 지원/IME/disabled gate를 통일하고 drag 거리도 기존 egui 입력 옵션으로 확인한 영향 검사 `/private/tmp/taide-batch20-definition-link-after-provider-gate.log`는 exit 0·6통과입니다. 참조 tree의 제목·경로/클릭·Codicon 닫기·원본 행 강조/preview 2px 테두리, 보이는 행만 미리보기 읽기와 원본문/preview/tree 휠 분리를 이어 연결했습니다. 최신 UI 확장 검증은 아직 진행 중입니다.

격리된 NativeApplication constructor 회귀의 실제 child definition→peek→같은 문서 공유→F12 본문 이동, 새 peek 파일 host 읽기→preview 입력→실제 Save→다시 dirty 편집→참조 파일 열기→같은 모델/미저장 본문 인수→정상 종료는 `/private/tmp/taide-batch20-actual-app-locations-after-ownership.log` exit 0·1통과(442 filtered)입니다. 최초 실행은 테스트에서 이미 이동한 PaneId를 다시 사용한 컴파일 exit 101이며 실제 참조 열기 명령을 소비하는 검증으로 수정했습니다. 현재 pane의 실제 파일 이동/reveal와 기존 dirty 모델 우선, 원본문 revision 보존·peek 모델 인수·task 종료 0을 확인했습니다. 사용자 앱 데이터/OS 설정/클립보드/Keychain/Trash는 사용하지 않았습니다.

최신 메모리 UI 확장 대상 `/private/tmp/taide-batch20-peek-render-navigation-after-phase.log`는 exit 0·9통과입니다. 화면 밖 이동의 상단 간격/이미 보이는 위치 유지, 1000개 참조의 실제 보이는 행만 읽기와 tree 휠이 source scroll을 변경하지 않는 동작, 닫기 Enter/Space 뒤 본문 입력을 포함합니다. 앞선 확장 컴파일의 MouseWheel phase 누락은 실제 TouchPhase API로 수정했으며 0건 실행은 검증으로 세지 않습니다.

실제 앱의 파일 간 순환/명시 peek 유지 최신 검사 `/private/tmp/taide-batch20-actual-app-cycle-peek.log`는 exit 0·1통과(442 filtered)입니다. Enter 이동 뒤 새 본문 뷰에서 같은 참조 모델을 인계하고 F4/Shift+F4로 두 파일을 오가며 실제 현재 탭/소스 ViewKey·원본 revision·dirty 내용을 보존했습니다. 해결된 peek는 provider mirror가 새 대상에 아직 준비되지 않았다는 이유로 폐기하지 않으며, 새 요청과 hover는 기존 provider/문서 버전 gate를 유지합니다.

연속 참조 이동의 첫 회귀 `/private/tmp/taide-batch20-latest-open-first.log`는 exit 101·1실패입니다. 같은 source 파일로 마지막 이동을 끝냈는데 앞서 대기한 다른 파일 열기가 실행됐습니다. 이동마다 독립 token/watch 취소를 두고 같은 파일 이동도 앞선 열기를 취소하며 host는 mutation guard 뒤 취소를 확인합니다. 앱은 현재 open token·원본문 선택/버전을 확인한 응답만 소비합니다. 영향 host 모듈 `/private/tmp/taide-batch20-latest-open-after-cancel.log`는 exit 0·4통과(441 filtered)입니다. 해결된 peek 모델을 보존하면서 원본문 편집 시 보류 열기만 취소하는 추가 상태 회귀는 아직 실행 전입니다.

hover code 색·포커스 인계·preview 명령 범위·project close/late host와 syntax snapshot 수명의 마지막 통합은 남아 있습니다. d/e와 전체 f/g는 계속 미완료입니다. 마지막 디스크 확인은 661GiB 여유·64%이며 manifest/lockfile/동결 browser diff가 없습니다.

임시 preview 탭의 dirty 제출 회귀 `/private/tmp/taide-batch20-preview-dirty-first.log`는 exit 101·1실패입니다. preview의 임시 TabId가 실제 layout dirty 큐로 전달됐습니다. 실제 layout의 모든 주창/보조창 root에 존재하는 탭만 제출/flush하도록 수정했습니다. 영향 실제 앱 검사 `/private/tmp/taide-batch20-preview-dirty-after-layout.log`에서는 이 검사를 통과한 뒤 새 대상 caret가 0/기대 14로 실패했습니다. 진단 검사 `/private/tmp/taide-batch20-open-reply-diagnostic.log`는 exit 0·1통과였고 정상 응답 3개의 current/source/cancel gate가 유효했습니다. 임시 진단 출력은 제거했습니다. 앱 검사의 완료 조건도 파일 loading만 확인하던 것에서 caret/peek 인계 완료까지 확인하도록 보강했습니다.

같은 순서를 의도적으로 분리한 상태 회귀 `/private/tmp/taide-batch20-layout-reply-first.log`는 exit 101·1실패입니다. 파일 열기의 레이아웃 이벤트가 앞서 도착하면 이전 active tab 조건이 open 요청을 취소했습니다. 보류 중인 파일 열기는 같은 창/프로젝트에서 응답 또는 5초 만료까지 유지하고 최신 선택/문서 및 host layout/root 검증을 보존합니다. 잘못된 일본어 이름 필터로 0건 실행된 `/private/tmp/taide-batch20-layout-reply-filter.log`는 증거에 포함하지 않습니다. 수정 뒤 상태 모듈은 현재 검증 중입니다.

수정 뒤 상태 모듈 `/private/tmp/taide-batch20-open-state-after-retention.log`는 exit 0·10통과(436 filtered)입니다. 보류 열기를 최신 원본문 선택/편집에서 취소하되 해결된 peek는 보존하며 레이아웃 이벤트가 응답보다 먼저 도착해도 요청을 유지합니다. 인계/회수 회귀를 더한 최초 `/private/tmp/taide-batch20-pending-peek-first.log`는 exit 101·10통과/1실패로 종료 프로젝트의 pending peek가 남았습니다. reconcile에서 현재 프로젝트 집합을 인계에도 적용한 `/private/tmp/taide-batch20-pending-peek-after-project.log`는 exit 0·11통과(437 filtered)입니다. 새 대상 뷰/preview/포커스 인계·잘못된 path 거절·프로젝트 종료·탭 변경·만료의 실제 회수를 확인했습니다.

포커스 인계 UI `/private/tmp/taide-batch20-focus-transfer-ui.log`는 exit 0·10통과입니다. F4가 본문 또는 preview에서 실행됐을 때 새 widget에서도 같은 영역에 포커스를 전달하고 사용자의 다음 트리 포커스를 반복해서 빼앗지 않습니다. 키보드 정의 hover/코드 표시는 `/private/tmp/taide-batch20-keyboard-hover-ui.log`에서 첫 frame의 popup sizing pass를 완료 조건으로 사용한 1실패(10통과)를 확인했습니다. sizing 뒤 실제 텍스트를 확인하는 영향 검사 `/private/tmp/taide-batch20-keyboard-hover-after-sizing.log`는 exit 0·11통과입니다. 포인터 없이 code LayoutJob 표시·fallback 텍스트 미사용·다음 키의 닫힘과 원본문 커서 이동을 확인했습니다.

`/private/tmp/taide-batch20-hover-preview-check.log`는 앱 테스트 코드 check exit 0입니다. hover는 키보드/포인터 요청을 구분하고 기존 TextMate의 최대 8줄 코드 토큰 색·bold/italic/underline/strike를 표시합니다. preview와 hover의 보이는 줄을 원래 syntax worker에 전달합니다. `/private/tmp/taide-batch20-peek-syntax-cache.log`는 exit 0·1통과(446 filtered)이며 같은 doc/revision/언어/토큰 generation/style에서 Arc를 재사용하고 숨은 peek 모델·즉시 편집·테마·같은 revision의 언어 변경·문서 폐기의 캐시 수명을 확인했습니다. 새 engine/의존성/동결 변경은 없습니다. 최신 실제 앱 hover/dirty/인수/순환/종료 검사는 진행 중입니다.

## 최종 연결 회귀와 전체 게이트 진행

트리의 파일 그룹/접힌 자손과 좌우·상하/Home/End·F4 순환 검사 최초 `/private/tmp/taide-batch20-tree-navigation-first.log`는 1실패였고, 계층 탐색 구현 뒤 `/private/tmp/taide-batch20-tree-navigation-after-hierarchy.log`도 11통과/1실패였습니다. 진단 `/private/tmp/taide-batch20-tree-focus-diagnostic.log`에서 첫 ArrowLeft가 tree에서 sash로 포커스를 넘긴 것을 관찰했습니다. egui의 `set_focus_lock_filter`는 이전 프레임의 포커스가 필요하므로 새 tree에는 `request_focus_with_filter`로 포커스와 필터를 함께 설치했습니다. 영향 검사 `/private/tmp/taide-batch20-tree-navigation-after-focus-filter.log`는 exit 0·12통과입니다. 진단 출력은 제거했으며 OS 입력을 사용하지 않았습니다.

추적 이력 만료와 preview 접기 최초 `/private/tmp/taide-batch20-fold-lag-first.log`는 exit 101·10통과/2실패입니다. 추적 실패 시 오래된 session을 닫지 않던 경계를 수정해 이전 참조 이동을 거절하고 preview만 회수합니다. 접기 fixture는 기본 false인 presentation.folding을 true로 설정했습니다. `/private/tmp/taide-batch20-fold-lag-after-tracking.log`는 exit 0·12통과(437 filtered)입니다. 원본문 dirty 모델을 보존하고 preview 접기/직렬 편집/재지정/저장이 대상 문서만 소비하는 것을 확인했습니다.

preview 찾기/접기 최초 정적 검사 `/private/tmp/taide-batch20-preview-find-fold-check.log`의 private UI FoldCommand 경로 오류는 공개 core 타입 경로로 수정했습니다. 실제 앱의 preview 찾기 열기/Escape 닫기·peek 유지 검사는 `/private/tmp/taide-batch20-actual-preview-find.log` exit 0·1통과(447 filtered)입니다. 치환을 더한 `/private/tmp/taide-batch20-actual-preview-replace.log`는 잘못된 모듈 이름으로 0건 실행돼 근거에서 제외합니다. 실제 모듈을 지정한 `/private/tmp/taide-batch20-actual-preview-replace-exact.log`는 exit 0·1통과(448 filtered)입니다. 실제 창 keymap→preview Find/Replace→대상 본문 치환→실제 저장과 원본문 revision 보존, dirty 모델 인수·파일 간 순환·정상 종료의 task 0을 확인했습니다.

4크레이트 포맷 중 UI 최초 `/private/tmp/taide-batch20-ui-fmt.log`는 잘못된 필드 삽입 위치 두 곳 때문에 exit 1입니다. EditorRequest 필드를 if 내부에서 밖으로 옮기고 무관한 workbench struct의 잘못된 삽입을 제거한 `/private/tmp/taide-batch20-ui-fmt-after-fields.log`는 exit 0입니다. editor/app/LSP SDK 포맷도 각 로그에서 exit 0이며 전체 테스트를 직렬로 시작했습니다. 전체 결과는 아직 완료로 표시하지 않습니다.

일반 LSP hover/완성/signature/rename/actions/semantic/inlay/code-lens 및 순수 미리보기의 별도 LSP mirror·공급/명령 소비는 잔여 전체 목표입니다. 미리보기의 찾기·편집·접기/저장과 부모 참조 순환을 전체 embedded LSP 구현으로 확대 판정하지 않습니다. 실기 pixel/IME/OS 접근성/성능/soak/패키징과 보호 검사·standalone prototype lock/edition 부채도 보존합니다.

editor 전체 `test --no-fail-fast`는 `/private/tmp/taide-batch20-editor-full.log` exit 0·31대상·212통과·0실패·1ignored입니다. UI inspection 전체 같은 옵션은 `/private/tmp/taide-batch20-ui-full.log` exit 101·21대상·374통과/1실패입니다. 위치 명령 11종을 실제 지원 gate로 연결했는데 기존 검사가 무조건 활성화 목록 21개로 기대했습니다. 플랫폼별 실제 location 지원을 제외하도록 기대값을 고친 영향 단위 검사 `/private/tmp/taide-batch20-ui-registry-after-support.log`는 exit 0·1통과(121 filtered)입니다. 현재 서로 다른 UI 375건이 통과했고 전체 실행을 반복하지 않았습니다.

app 전체 `test --no-fail-fast`는 `/private/tmp/taide-batch20-app-full.log` exit 101·67대상·687통과/2실패·보호 Trash 3제외입니다. 새 위치 LSP 검사 2건의 초기화 대기가 4초를 넘겼으며 관찰 상태는 Initializing/pending 1/failure None/빈 stderr였습니다. 병렬 전체 단위 검사에서 child 시작을 기다리는 한도만 10초로 조정하고 테스트 전체 15초 상한과 정상 응답·취소·owner/세대·프로젝트 격리·task 회수 단언은 유지했습니다. 영향 모듈 `/private/tmp/taide-batch20-lsp-after-readiness.log`는 exit 0·5통과(444 filtered), 2.97초입니다. 현재 서로 다른 app 689건이 통과했습니다. 전체 병렬 부하를 다시 실행한 근거로 확대하지 않으며 최초 전체 실패와 수정 뒤 영향 검사를 구분합니다.

## 전체 게이트 결과와 보존 경계

LSP SDK 전체 `test --no-fail-fast`는 `/private/tmp/taide-batch20-sdk-full.log` exit 0·2대상·82통과·0실패/ignored입니다. 변경 4크레이트의 전체 31+21+67+2=121대상을 직접 실행했고 현재 서로 다른 212+375+689+82=1358건이 통과했습니다. 최초 UI 1/app 2실패와 해당 영향 재검사만 보존했으며 전체 반복 실행의 성공으로 바꾸어 쓰지 않습니다. editor 성능 1ignored, 보호 Trash 3과 변경 없는 syntax 3/egui SDK 문서 1ignored·실기 부채를 이번 성공에 더하지 않습니다.

공통 테스트 옵션은 `--locked --offline --target-dir experiments/native-shell-spike/target --no-fail-fast`이고 각 manifest에 `cargo test`를 실행했습니다. UI는 `--features inspection`이며 app 인자에서 다음 3건만 `--skip`했습니다.

- `remote_files::tests::실제_파일14명령과_raw는_plugin_overlay_저장_복사_이동_삭제_mirror를_보존한다`
- `실제_탐색기_삭제는_확정한_dirty를_회수하고_선택프로젝트만_닫으며_공유초안을_보존한다`
- `실제_workspace_delete는_dirty_mirror_root를_거절하고_모든_프로젝트_tab과_document를_회수한다`

동결 browser의 host `cargo check --tests`는 `/private/tmp/taide-batch20-web-host.log` exit 0(7.83초), Wasm `cargo check --features canvas --target wasm32-unknown-unknown`은 `/private/tmp/taide-batch20-web-wasm.log` exit 0(2.43초)입니다. 둘 모두 공통 locked/offline/target-dir를 유지했습니다. source/features/manifest/lock/dependency graph를 변경하지 않았습니다. editor/UI/app/SDK의 `cargo fmt -- --check`도 해당 `/private/tmp/taide-batch20-*-fmt-check.log` 4개에서 exit 0입니다. 변경 source/QA/license의 diff check exit 0, manifest/lockfile diff 0이며 `df -h .`는 651GiB 여유·65%입니다. 기존 Wry 경고 17개와 큰 __eh_frame 링크 경고는 남아 있고 검사 실패로 바꾸어 기록하지 않습니다.

새 코드의 Monaco gotoSymbol/link/references derivation 경로와 기존 MIT 고지 위치를 `THIRD_PARTY_LICENSES.md`에 기록했습니다. 기존 TS/Tauri/Monaco/xterm과 실제 앱 데이터·OS 설정·clipboard/Keychain/Trash·보호 app bundle을 유지했습니다. 일반 hover/signature/완성·순수 preview의 별도 LSP 공급·고정 줄 Ctrl/Meta 정의 링크와 실제 pixel/접근성/IME·성능/출시, standalone prototype lock/edition·7자리 HEX 결정은 잔여 전체 목표입니다.
