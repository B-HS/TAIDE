# M8 native Problems — 후속46 진행 중

## 대상 파일·원본 계약

`native/taide-native-app/src/diagnostics.rs`, `diagnostics-tests.rs`, `lsp.rs`, `lsp-diagnostics.rs`, `lsp-diagnostics-tests.rs`, `lsp-status.rs`, `problems.rs`, `problems-tests.rs`, `problems-icons.rs`, `problems-icons-tests.rs`, `resources/problems/`, `application.rs`, `host.rs`, `presentation.rs`, `presentation-refresh.rs`, `lib.rs`, `tests/lsp-recovery.rs`, `tests/host.rs`와 `native/taide-native-ui/src/shell.rs`, `tests/workbench.rs`, `Cargo.lock`이 대상입니다.

원본 근거는 `src/shared/lib/lsp/adapters/diagnostics.ts`, `src/shared/lib/lsp/client.ts`, `src/entities/lsp/lsp-session-registry.ts`, `src/shared/hooks/use-monaco-markers.ts`, `src/features/problems/`, `src/widgets/problems-panel/problems-panel-container.tsx`, `src/shared/ui/file-group-header.tsx`, `src/widgets/editor-area/editor-area.tsx`입니다. 원본 재초기화는 같은 client와 diagnostics subscription을 재사용합니다. generation 변경만으로 owner를 바꾸거나 진단을 비우지 않습니다. 실제 새 SessionClient에만 UUID owner를 만들고 recovery·format clone에는 유지했습니다.

공식 pinned crate의 lsp-types0.97.0 Diagnostic/severity, Tokio1.53.1 watch의 has_changed/borrow_and_update/closed, egui0.36.2 ScrollArea/Context interaction, epaint0.36.2 LayoutJob/TextWrapping 문서를 읽었습니다. docs.rs 조회 2건은 접근 실패여서 로컬 registry 공식 소스를 사용했습니다. 후속에서는 egui Image의 tint/rotation/paint_at, AccessKit button/toggled/expanded/group/splitter, Memory focus lock, emath GUI_ROUNDING과 원본 react-resizable-panels의 키보드 5%/Home/End·상대 크기 유지 소스를 확인했습니다.

## 구현·검증 범위

- [x] `(owner, DocumentId)`별 교체·빈 batch·폐기, 다른 owner 보존, 닫힌 모델과 폐기된 owner의 늦은 재발행 거절입니다. missing/unknown severity→error와 raw code/data/source를 보존하며 unchanged batch에는 같은 revision identity를 유지합니다.
- [x] 실제 producer에서 같은 session의 두 문서는 같은 owner, crash/replay는 같은 owner, dispose/reacquire와 같은 서버의 별도 Cargo workspace는 서로 다른 owner입니다. 변경된 단일 registry로 project summary·usage labels·종료 task0/owner 해제도 확인했습니다.
- [x] App에 bound-marker error counter, 슬롯별 열기·4 severity 필터/count·파일 그룹/행·열 정렬·collapse·20px 가상행/overscan12·빈 상태를 연결했습니다. 전역 counts는 필터와 무관하며 store revision/filter 변경 때 cache를 재구축합니다. 재열기는 필터/collapse를 초기화하고 닫힌 slot을 회수합니다.
- [x] headless 실제 포인터 입력으로 상태바 열기·error 필터·1000개 진단의 가상행·warning row의 1-based 파일 위치 요청·접기를 확인했습니다. 이것은 성능 계측이나 실제 GUI 검증이 아닙니다.
- [x] typed OpenProblem은 기존 `layout_open_tab`의 preview/root guard를 유지하고 실제 tab/pane/layout을 기존 Reveals에 전달합니다. 실제 HostBridge 검사에서 동일 tab 재사용·위치·경계 밖 경로/0 좌표 거절·layout 불변·task0/owner 해제를 확인했습니다. Problems 오류는 터미널 전용 문구와 구분합니다.
- [x] 원본 `file-icon.ts`의 특수 이름6개·README/LICENSE/LICENCE/.env 접두사·확장자25개·기본값과8개 theme color key를 Rust로 옮겼습니다. .env는 이름 분류만 하며 파일 내용을 읽지 않습니다. 원본 Lucide1.28.0의 실제 SVG29개·ISC/Feather MIT 고지를 고정 자산으로 저장했습니다. FileJson은 원본 file-braces 별칭이며 생산 실행에 TypeScript/JS가 필요하지 않습니다.
- [x] 실제 SVG를 12/14/20px와 DPI에 맞춰 지연 raster/cache하며 theme tint 변경에는 재업로드하지 않습니다. 외부 이미지 resolver는 거절하고 오류는 App 상태에 전달합니다. 파일 그룹에도 실제 FileTypeIcon과90도 회전한 원본 ChevronRight를 연결했고 근사 severity/empty/close vector를 제거했습니다.
- [x] 실제 headless AccessKit 결과에서 severity 버튼의 pressed true/false·필터 group·파일 그룹 expanded·진단 이름의 severity/source/1-based 위치를 확인했습니다. tooltip은 원본 bottom이며 헤더 medium/tabular font·전체 AX는 아직 완료가 아닙니다.
- [x] 실제 UI 분할에서 resizerThickness5px·슬롯 격리·상단30%/하단120px·220px 기본·ArrowUp5%/Home/End·pointer drag60px·닫기/재열기 초기화를 확인했습니다. 상대 크기 state와 hover의 app.focusBorder, splitter orientation/value·focus lock·repaint를 연결했습니다.
- [x] 별도 실제 headless 이벤트 검사에서 그룹 Space 접기·반복 Space 무시·Enter 펼치기, 진단 Enter의1-based 위치 요청, 실제 wheel 스크롤, Close 포인터 입력, 새 mount/scroll ID와 offset0·첫 행 복귀, emptyFiltered 문구·전역 count 유지·실제 AX Focus 뒤 필터 Enter 복귀를 확인했습니다. 직접 스크롤 상태를 주입하지 않았으며 필터의 포인터 클릭 후 키보드 포커스를 자동으로 가정하지 않습니다.
- [x] 실제 편집기 자식 UI를 접근성 Group으로 렌더하고 separator controls가 그 실제 node를 가리키게 연결했습니다. Group의 실측 영역·자식과 window height800→1100에서 상대 비율 유지, F6/Shift+F6 소비·포커스 유지를 확인했습니다. 원본 F6는 같은 Group의 separator만 순환하며 Problems Group은1개이므로 다른 슬롯/내부 pane 선으로 이동시키지 않습니다.
- [x] 상태바·필터·Close 버튼은 Space 최초 누름/반복에는 실행하지 않고 같은 버튼에서 해제할 때 실행합니다. Enter 누름/반복과 실제 포인터·AX Click을 유지하고 포커스 이탈 시 pending Space를 취소합니다. pending은 소유자 안의 viewport별 실제 response ID이며 소유자 폐기와 함께 회수합니다.
- [x] 같은 프레임의 최초 Space+반복, 여러 Enter, 여러 Space 누름/해제와 실제 AX Click의 활성화 횟수를 보존합니다. 상태바의 짝수 토글도 닫기/재열기 수명을 거치며 필터를 초기화합니다. 행의 진단 Open은 Vec으로 반환해 actual AppSurfaces가 사건별 HostCommand를 모두 전달합니다.
- [x] 1000개 진단을 실제 wheel로 스크롤한 뒤 필터 off/on 및 한 프레임 두 Enter의 off/on에서 새 scroll ID·offset0·첫 그룹 복귀·이전 persisted State 제거를 확인했습니다. 빈 목록/닫기 때 scroll 소유자를 폐기하며 직접 scroll State를 주입하지 않았습니다.

marker Store는 실제 EditorStore의 전체 live 모델 ID를 유지합니다. producer의 활성 문서 binding과 session이 marker를 발행한 모델 ID를 분리해 비활성 진단도 session 폐기까지 보존합니다. 미바인딩 URI의 raw 단일 출처와 실제 모델 폐기 기록/App caller도 연결했습니다. 기본5초 grace·inactive 진단은 아래 실제 child로 확인했으며 전체 provider/Monaco 수명·실제 App GUI parity는 아직 완료되지 않았습니다.

## 실행 결과·실패 구분

공통 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`, app manifest `native/taide-native-app/Cargo.toml`, `--locked --offline --target-dir experiments/native-shell-spike/target`이며 Cargo는 직렬 실행했습니다.

| 명령 범위                                                            | 실제 결과                                                                                                                                                                                                                                                                                                                       |
| -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `cargo test … --lib diagnostics::tests`                              | 존재하지 않는 EditorLimits default fixture compile 실패를 실제 필드로 정정한 뒤 1 PASS, compile14.51초/suite0.00초입니다. 이후 store는 불변이며 성공을 재사용합니다.                                                                                                                                                            |
| `cargo test … --test lsp-recovery`                                   | 두 번째 sync 처리 전에 summary를 완료 신호로 오인한 fixture 실패를 실제 Diagnostics 응답 대기로 정정한 뒤 1 PASS, compile5.56초/suite0.64초입니다.                                                                                                                                                                              |
| `cargo test … --lib problems::tests`                                 | compile7.25초/suite0.05초에서 행 Open이 없었습니다. trace 실행6.54초/0.05초는 selectable Label/RTL child 입력·경계 문제를 출력했습니다. bounded Painter/단일 row interaction으로 제품 수정 후 1 PASS, compile7.54초/suite0.05초입니다. 이후 테스트명 한국어 정정과 색상 수정은 strict로 확인했고 행 성공은 반복하지 않았습니다. |
| `cargo test … --test host 실제_problems_open`                        | compile10.20초/suite0.01초에서 Localized Forbidden을 직접 enum으로 기대한 fixture 실패입니다. 원본 AppError.kind으로 정정한 뒤 1 PASS, compile1.32초/suite0.01초입니다. 거절 정책을 완화하지 않았습니다.                                                                                                                        |
| `cargo clippy … --lib --bin taide-native-app --tests -- -D warnings` | exit0, 17.03초입니다. 이후 추가한 host fixture만 `--test host` strict exit0, 0.64초로 확인했습니다. 기존 Wry dependency 경고17건은 별도입니다.                                                                                                                                                                                  |

이전 UI 검사는 stale lock/workspace 경계 때문에 미실행이었습니다. 이번에는 실제 UI own manifest/lock을 대조하고 정상 `--offline`으로 실행해 해결했습니다. UI lock 변경은 기존 taide-runtime의 sysinfo와 taide-sync의 reqwest/serde/taide-infra edge4개뿐이며 package/version 변경은 없습니다. 최초 기본 분할 검사는 compile11.51초/suite0.06초에1 PASS입니다. 이후 설정 두께와 실제 resize 입력을 추가한 검사는 별개의 위험을 확인하며 아래 실패·최종 성공을 기록합니다.

## 원본 glyph·접근성·분할 후속 결과

| 명령 범위                                                                | 실제 결과                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| app `cargo test … --lib problems`                                        | compile14.46초/suite0.05초에서 SVG/파일 분류/cache1 PASS입니다. UI 검사는 클릭 직후 이전 프레임의 pressed 상태를 읽어 FAIL이었으며 API가 요구하는 다음 렌더 프레임으로 fixture를 정정했습니다.                                                                                                                                                                                                                                                                     |
| app `cargo test … --lib problems::tests`                                 | 실패한 UI만 compile6.44초/suite0.05초에1 PASS입니다. 상태바 열기/filter/collapse/virtual row/open과 actual AccessKit을 확인했습니다. 성공한 SVG 검사는 반복하지 않았습니다.                                                                                                                                                                                                                                                                                        |
| UI `cargo test … --test workbench problems_panel은`                      | 두께5px 기대/실제1px RED입니다. 두께 수정 후 키보드 fixture는 35.95px 기대/35.9375px 실측을0.01보다 작게 비교해 실패했습니다. 공식 GUI_ROUNDING=1/32와 실제 focus settle 경계로 정정했으며 자의적인 오차 확대가 아닙니다. 이후 pointer에는 NaN/Flags(1), 최대·드래그 높이 모두503.3125px인 RED를 관찰했습니다. source input 순서를 자식 뒤로 옮긴 뒤 최종 compile1.36초/suite0.07초1 PASS입니다. 입력 사건을 재발행하지 않고 다음 렌더 프레임의 치수를 확인합니다. |
| app `cargo clippy … --lib --bin taide-native-app --tests -- -D warnings` | exit0,17.53초입니다. 이후 바뀐 UI private splitter/input 코드와 UI fixture는 다음 UI strict로 확인하며 동일 App 성공을 재사용합니다. Wry dependency의 기존17 warning은 별도입니다.                                                                                                                                                                                                                                                                                 |
| UI `cargo clippy … --lib --test workbench -- -D warnings`                | 최종 exit0,8.32초입니다. 실제 UI dev lock을 사용하는 검사이며 app 대신 workspace 경계를 우회한 실행이 아닙니다.                                                                                                                                                                                                                                                                                                                                                    |

SVG 추출 최초 in-memory generator는 file-json 모듈의 __iconNode가 별칭 export에 없어서 중단됐습니다. 원본 file-braces re-export를 확인한 뒤 정확한 node로 생성했으며 실패 당시 파일 쓰기는 없었습니다. 생성 결과는 apply_patch로만 저장했습니다. Cargo는 직렬 실행했고 같은 성공 계측을3회 반복하지 않았습니다. 분할 실패의 상세는 `docs/bug/2026-10-04-native-problems-separator.md`입니다.

## 실제 키보드·닫기·스크롤 후속 결과

app `cargo test … --lib problems::tests::problems_키보드`는 최초 compile12.70초/suite0.05초에 반복 Space가 접힌1행을1001행으로 다시 펼치는 RED였습니다. 원본 `createActivationKeyDownHandler`는 반복 Space를 무시하지만 egui 기본 `key_pressed`는 반복도 클릭으로 처리합니다. 행의 반복 Space만 거절하고 Enter·실제 포인터·AccessKit Click은 유지하며 소비하는 키를 원본에 맞췄습니다.

제품 수정 뒤 compile5.44초/suite0.07초 실패는 scroll fixture가 ScrollArea의 IdSalt 재해시 경계를 빠뜨린 오류입니다. 공식 `ScrollArea::begin`·`Ui::make_persistent_id`·`IdSalt` 소스로 ID를 정정했습니다. 다음 compile5.40초/suite0.08초 실패는 마지막 Enter에 버튼 포커스가 없었던 fixture 오류이며, 실제 AX Focus 이벤트로 정정했습니다. 최종 신규 검사1 PASS(compile5.38초/suite0.09초)·app `cargo clippy … --lib --tests -- -D warnings` exit0(17.46초)입니다. Wry의 기존 dependency warning17건은 별도이고 앞선 glyph/cache·UI·분할 성공은 반복하지 않았습니다. 반복 Space bug는 row-input 문서에도 기록합니다.

## 창 resize·분할선 controls/F6 후속 결과

UI `cargo test … --test workbench problems_separator는`는 controls0개/기대1개 RED(compile0.95초/suite0.04초)입니다. 실제 편집기 영역을 별도 bounded child UI로 감싸고 그 실제 AX Group node에 연결했습니다. 최초 source compile의 Option 이동 오류는 as_ref borrow로 정정했습니다. 다음 실행(compile1.40초/suite0.04초)은 Group bounds가 없었으며, 공식 egui가 non-focusable Ui에 bounds를 자동으로 쓰지 않는 경계를 확인하고 실제 편집기 rect를 설정했습니다.

최종 신규 검사1 PASS(compile1.30초/suite0.04초), UI 구조 변경의 영향을 받는 기존 두께/키보드/실제 drag/close/reopen 검사1 PASS(compile0.10초/suite0.05초), UI lib/workbench strict exit0(0.92초)입니다. 이전 성공을 변경 없이 재계측한 것이 아니라 실제 editor UI 구조가 변경된 영향 검사입니다. 낮은 창의 상충 최소 제약·다른 종류의 pane separator 전체 정책·실제 GUI/VoiceOver는 완료로 주장하지 않습니다.

## 헤더 button Space 해제 후속 결과

원본 `IconButton`의 HTML button과 [W3C APG의 공식 button 입력 소스](https://github.com/w3c/aria-practices/blob/main/content/patterns/button/examples/js/button.js)를 대조했습니다. DOM button은 Space 해제에 활성화하지만 기존 egui 기본 클릭은 누름에 실행했습니다. app `cargo test … --lib problems::tests::problems_헤더버튼`에서 상태바가 최초 Space 누름에 열리는 RED(compile8.35초/suite0.02초)를 재현했습니다.

세 버튼에서 같은 활성화 판정을 재사용합니다. Space pending·Enter/AX/실제 포인터를 분리하고 egui의 Space 가짜 클릭은 사용하지 않습니다. mount와 슬롯별 pending·포커스 이탈 취소도 연결했습니다. 최종 신규1 PASS(compile6.44초/suite0.03초)입니다. 실제 헤더 동작 변경의 영향을 받는 기존 포인터/filter/open 검사1 PASS(compile0.22초/suite0.03초), 앞선 실제 키보드/Close/emptyFiltered/scroll 검사1 PASS(compile0.22초/suite0.06초), 최종 app lib/tests strict exit0(15.04초)입니다. UI 최종 strict0.92초와 새 separator/창 resize 성공은 재사용하며 Wry의 기존 dependency warning17건은 별도입니다.

## 프레임 batch·빈 필터 viewport 수명 후속 결과

최초 fixture는 존재하지 않는 EditorLimits 필드와 Result unwrap 누락으로 compile 실패했습니다. 실제 타입을 확인해 정정했으며 제품 RED로 세지 않습니다. batch 신규2건은 compile6.57초/suite0.03초에 RED였습니다. 최초 Space+반복이 그룹을 접지 않았고 상태바 두 Enter가 패널을 열었습니다. boolean 집계를 사건 수로 교체한 뒤 compile10.52초/suite0.04초에2 PASS입니다. 짝수 토글을 parity no-op으로 생략하지 않아 닫기/재열기의 로컬 상태 초기화를 유지합니다.

빈 필터 왕복 신규1건은 compile6.53초/suite0.04초에 실제 offset1000/기대0 RED였습니다. 원본은 빈 목록에서 scrolling element를 제거하지만 native는 scroll state를 유지했습니다. 목록 viewport를 별도 소유자로 만들고 Drop에서 실제 egui persisted State를 제거했습니다. live viewport pruning은 Context input lock 밖에서 수행해 Drop의 data lock 재진입을 피하며 매 프레임 동일 guard를 교체하지 않습니다. Space pending도 Context 임시 UUID state 대신 소유자 안의 viewport map으로 옮겼습니다. 관련 전체 Problems6건은 compile11.22초/suite0.09초에 PASS입니다.

후속 strict는 미사용 Panel.mount와 fixture의 &PathBuf 때문에 실패했습니다. 억제하지 않고 mount 필드 제거·&Path로 정정했습니다. 별도 source 검토에서 진단 Open의 Option이 여러 요청을 한 개로 합치는 경계를 확인해 Vec 및 actual Application caller의 루프로 교체했습니다. 이 API 변경은 실행 RED를 주장하지 않습니다. 확장된 batch 행 검사는 두 Enter/두 actual AX Click의 두 위치 요청을 확인합니다. 최종 관련 입력/출력3건만 compile6.96초/suite0.09초에 PASS이며 변경되지 않은 헤더/빈 필터3건 성공은 재사용합니다. 명령은 `cargo test … --lib problems::tests -- --skip problems_프레임_batch_헤더 --skip problems_헤더버튼 --skip problems_빈필터`입니다. 최종 app lib/bin/tests strict exit0(17.21초), authored Rust3파일 exact rustfmt입니다. 이전 UI strict0.92초는 그대로 재사용하고 기존 Wry dependency warning17건은 별도입니다.

공식 pinned egui Context root Ui의 ID는 `(viewport_id, "__top_ui")`를 포함하므로 Ui.make_persistent_id는 이미 viewport별로 분리됩니다. 불필요한 viewport salt를 추가하지 않았습니다. Context equality는 Arc identity, ScrollAreaOutput.id는 실제 stored ID, IdTypeMap.remove는 persisted State에도 적용되는 공식 소스를 확인했습니다.

실제 두 viewport의 검사1 PASS(compile7.29초/suite0.03초)입니다. 같은 슬롯·Context에서 root의1000px wheel과 child의500px wheel이 서로 다른 ID/offset을 가지며 다른 viewport 렌더가 root offset을 바꾸지 않습니다. child 제거 시 persisted State·소유자 map entry를 회수하고 동일 ID 복귀 때 offset0·첫 그룹을 확인했습니다. 슬롯 reconcile(None)은 양 State를 회수합니다. 이후 변경한 테스트에만 app lib/tests strict exit0(1.99초)이며 기존 app lib/bin strict 성공은 재사용합니다. 실제 OS 창이나 전체 DPI/glyph/분할 cache 검증으로 세지 않습니다.

## 낮은 창의 상충 제약 후속 결과

원본 EditorArea는 editor 최소30%, Problems 기본220px/최소120px를 전달합니다. 설치된 react-resizable-panels4.12.2의 실제 `We/K/Z/le` 함수와 Panel의 flexBasis0/flexGrow를 대조했습니다. 함수 본문을 in-memory로 호출한 진단은 최초 `$t` 비교 함수 누락으로 실패했으며 누락을 포함해 정정한 뒤 실제 원본 결과를 확인했습니다. 유효 높이100px은 논리 layout30/100·Problems76.923px,150px은30/80·109.091px,200px은30/70·140px입니다. 두 낮은 경우의 keyboard5%는 원본 le가 layout을 유지합니다. [W3C CSS flex 길이 배분](https://www.w3.org/TR/css-flexbox-1/#resolve-flexible-lengths)에 따라 논리 weight의 합이100을 넘으면 실제 영역은 비율로 배분됩니다. 이는 source/표준 기반 비교이며 원본 DOM의 픽셀 실측은 아닙니다.

새 actual headless workbench fixture는 run_ui closure가 ShellIntent Vec을 반환해 compile 실패했으며 unit closure로 정정했습니다. 이후 compile0.79초/suite0.04초에 유효 높이99px·native Problems69.3125px/기대76.15385px RED를 재현했습니다. 상충 최소 제약에서 원본의3자리 percentage 정규화·100% max clamp·flex ratio를 적용하고 geometry minimum/maximum을 동일하게 고정했습니다. 충분한 높이로 복귀하면 상대 fraction을 원본의 정상 제약에 재적용합니다. AX value/min/max는 실제 픽셀 비율 대신 원본 논리 editor weight30을 유지합니다.

변경된 UI의 낮은 창/controls·창 resize/F6/정상 두께·keyboard·pointer·닫기/재열기 관련3건 PASS(compile1.43초/suite0.07초)·UI lib/workbench strict exit0(0.87초)입니다. 높이180/220에서 실제 두 영역·AX·ArrowUp/Down/Home/End 고정과 정상800 복귀를 확인했습니다. source2파일 exact rustfmt이며 같은 성공을 재실행하지 않았습니다. 보호 bundle·OS·제품 TS/manifest/lock은 변경하지 않았습니다.

## 서로 다른 진단 행의 사건 순서 후속 결과

실제 AX Click 두 번째 행→첫 번째 행→두 번째 행에서 요청 위치 `[1,2,2]`/기대 `[2,1,2]` RED(compile6.67초/suite0.03초)를 재현했습니다. 미소비 input snapshot의 사건 인덱스를 보존해 행 iteration 순서 대신 사건 순서로 요청을 반환합니다. actual App caller는 기존 Vec 순서를 그대로 전달하며 활성화 횟수/반복 정책/소비 경계를 유지했습니다.

영향받는 입력7 PASS(compile6.88초/suite0.08초), 생산 코드 app lib/bin/tests strict exit0(17.68초)입니다. 이후 새 혼합 fixture만1 PASS(compile6.93초/suite0.04초)로 pointer release/AX `[1,2,1]`·Enter/AX/반복 Enter `[2,1,2]`와 소비 후 미잔류를 확인했습니다. 성공한7건/strict·두 viewport scroll·UI splitter·font 검사는 재실행하지 않았습니다. 상세는 [진단 열기 순서 bug](../bug/2026-10-04-native-problems-open-order.md)입니다. 중간 Focus/Tab·filter/collapse/Close까지 섞인 전체 사건 적용 순서는 별도 미완료입니다.

## 명시적 AX Focus·상태 변경 사건 순서 후속 결과

AX Focus 두 번째 진단→Enter 눌림/해제→AX Focus 첫 번째 진단→Enter 눌림/해제가 요청 `[1,1]`/기대 `[2,1]`인 RED(compile6.78초/suite0.04초)를 재현했습니다. header/행이 공유하는 원래 snapshot/소비 인덱스와 Memory의 프레임 시작 focus를 사용해 사건별 Focus·키를 배정합니다. 같은 형태의 Enter가 다른 target에서 처리돼도 사건을 구별하며 기존 Space owner/포커스 이탈 취소를 유지합니다. 영향9 PASS(compile11.13초/suite0.09초)·당시 strict17.50초입니다. 상세는 [focus-order bug](../bug/2026-10-04-native-problems-focus-order.md)입니다.

이후 진단 Click 뒤 필터 Click인데 Open0/기대1 RED(compile6.94초/suite0.04초)를 재현했습니다. Filter/Collapse/Open/Close를 같은 사건 순서로 적용하고 각 단계의 실제 가시 진단/그룹을 확인합니다. Close 뒤 동작은 실행하지 않으며 앞선 Open은 유지합니다. filter/collapse/close 상태 변경 뒤 명시적 repaint를 예약합니다.

관련10건 실행은9 PASS/1 fixture FAIL(compile6.80초/suite0.10초)입니다. 신규6조합과 Focus/기존 입력9건은 성공했습니다. 실패는 필터 정착 프레임의 warning font 텍스처2개를 처리하지 않은 TexturesDelta Drop이며 fixture에 clear를 추가해 실패한1건만 PASS(compile5.37초/suite0.06초)입니다. 같은 성공9건은 반복하지 않았으며 최종 strict는 아래에 기록합니다. 상세는 [action-order bug](../bug/2026-10-04-native-problems-action-order.md)입니다.

최종 app `cargo clippy … --lib --bin taide-native-app --tests -- -D warnings`는 exit0,15.42초입니다. Cargo는 직렬 실행했으며 이전 font/UI/viewport 성공을 반복하지 않았습니다. Wry의 기존 dependency warning17건은 별도입니다. 변경된 Rust2파일 exact rustfmt와 문서 포맷·tracked diff check를 확인했습니다. untracked source/QA/bug의 no-index check는 빈 출력/exit1(새 파일 diff 존재)입니다. live Cargo handle은 없습니다.

## 필터 변경 뒤 같은 행 ID의 새 진단 후속 결과

원본 `problem:${path}:${index}` node 계약을 대조하고 error 행 Click→filter off→같은 index Click이 요청 `[(1,1)]`/기대 `[(1,1),(2,4)]`인 RED(compile6.84초/suite0.04초)를 재현했습니다. 캡처한 Diagnostic Arc 대신 사건 적용 시점의 실제 path/index를 조회해 같은 node의 새 진단을 열고 사라진 index는 거절합니다. collapsed/Close·HostBridge preview/path/1-based 좌표 경계는 유지합니다.

영향7 PASS(compile5.90초/suite0.10초)로 새3조합·행/Focus/키/pointer/AX·filter/Close를 확인했습니다. 변경하지 않은 header·빈 필터·viewport 성공은 skip으로 재사용했습니다. app lib/tests strict exit0(15.33초)·Rust2fmt이며 공개 Open/actual caller는 불변이라 기존 bin strict15.42초는 재사용합니다. 상세는 [row-retarget bug](../bug/2026-10-04-native-problems-row-retarget.md)입니다.

Tab 구현 경계도 pinned egui Memory::begin_pass/interested_in_focus와 Context 소스로 확인했습니다. 단일 프레임의 여러 Tab은 focus_direction 하나로 집계되고 Shift+Tab은 id_next_frame을 통해 이전 widget으로 이동합니다. 전체 workbench의 다른 focusable widgets와 scope 경계를 모르는 panel 내부 wrap/강제 request_focus로 이를 우회하지 않습니다. 현재 source 조사만으로 Tab 또는 coalesced Tab/Enter를 구현·통과로 기록하지 않았습니다.

기존 `taide-native-retained`는 메모리 방문/예산 기능이며 글꼴 shaping renderer가 아닙니다. native editor model에도 별도 shaping API가 없어 tnum 구현으로 재사용할 수 없습니다. 기존 epaint OpenType feature API 한계와 UI font QA의 미완료 경계를 유지합니다.

## 버튼 표시 후속 결과

원본 status-bar의 text-[11px]/rounded-sm/side=top·severity filter의 text-xs/rounded-sm/side=bottom·Close의 size-5/rounded-sm/side=bottom을 대조했습니다. global.css의 --radius0.375rem/--radius-sm=radius-2px와 설치된 Tailwind preflight의 html line-height1.5를 기준으로 radius4px·status height16.5px를 구현했습니다. 이는 원본 소스/CSS 기반 값이며 원본 DOM 픽셀 실측은 아닙니다.

actual headless renderer의 RectShape/AX bounds/실제 hover tooltip 검사에서 radius0/기대4 RED(compile6.63초/suite0.03초)를 재현했습니다. 수정 뒤 status 높이16.5px는 맞았지만 왼쪽 버튼의 tooltip 텍스트 중심이145.5px로 버튼 아래인 RED(compile6.30초/suite0.03초)였습니다. pinned egui의 popup이 가로 overflow에도 default MENU_ALIGNS의 bottom-start를 먼저 선택하는 원인을 확인했습니다. 설치된 Radix Popper의 shift→flip 계약과 대조해 동일 방향 start/end 후보를 반대 방향보다 먼저 선택하도록 수정했습니다.

신규1 PASS(compile5.32초/suite0.06초)는 status/error filter/Close3개 실제 버튼의4px 배경·16.5/20/20px 높이·위/아래/아래 tooltip을 확인합니다. 시간은 RawInput의 합성 시계만 전진시키고 기본 tooltip delay를 읽었습니다. 전역 설정·OS 앱을 바꾸거나 tooltip을 always-open으로 강제하지 않았습니다. 성공한 검사는 반복하지 않았으며 app lib/tests strict exit0(15.17초)·Rust2fmt입니다. 이전 row-retarget7/입력/font/viewport/bin strict15.42초는 변경 위험에 맞춰 재사용합니다. 기존 Wry dependency17경고는 별도입니다. 상세는 [button-display bug](../bug/2026-10-04-native-problems-button-display.md)입니다.

tooltip의 전체 style/delay/arrow·Radix의 정확한 collision shift·CSS opacity 그룹 합성·브라우저 focus ring·full GUI/font/tnum/CJK는 아직 미완료입니다. 방향 후보 수정은 전체 collision 또는 전체 시각 parity의 증거가 아닙니다.

## 진단 발행자 watch와 marker store 수명 후속 결과

실제 `LspBridge::diagnostic_bindings`의 tokio watch receiver와 실제 marker `Store`를 같은 합성 검사에 연결했습니다. 신규1 PASS(compile8.39초/suite0.00초)입니다. 두 owner의 error/warning 발행→첫 owner 폐기 뒤 warning 보존→폐기 owner의 늦은 발행 거절→모델 제거와 늦은 발행 거절→retained 모델 복귀 뒤 현재 owner 발행→미확인 상태 갱신이 남은 채 publisher Drop→빈 bindings로 전체 marker 제거→늦은 발행 거절을 확인했습니다. 변경 없는 poll은 None이고 폐기 후 반복 reconcile은 revision identity를 유지합니다.

이는 실제 watch·binding/Store 결합 검사이며 실제 OS LSP child 종료 또는 NativeApplication 전체 모델 disposal을 실행한 증거가 아닙니다. 기존 실제 child/recovery 검사 결과는 재사용하고 변경하지 않았습니다. 현재 slice의 생산 코드는 추가로 바꾸지 않았습니다. 기존 generic Store 검사를 반복하지 않고 publisher watch와의 연결 경계만 검사했습니다.

변경된 test fixture의 app lib/tests strict exit0(16.65초)·lsp-status exact rustfmt입니다. 앞선 버튼 생산 코드의 strict15.17초와 렌더1 PASS는 재사용하며 성공 동작 검사를 다시 실행하지 않았습니다. 기존 Wry17 dependency 경고는 별도입니다. 두 신규 검사는 Cargo를 직렬로 실행했으며 live handle은 없습니다.

이 검사 당시 native `lsp.rs`는 Session.documents에서 URI를 찾는 단계에서 미바인딩 raw를 폐기했습니다. 아래 raw 후속 구현에서 이 차이를 수정했습니다. 당시 watch/marker 검사 성공을 raw 구현의 근거로 대신하지 않습니다.

## 미바인딩 URI raw 단일 출처 후속 결과

`lsp-diagnostics.rs`의 세션별 raw Store를 실제 알림 처리에 연결했습니다. 유효한 generation의 알림은 bound 문서 조회 전에 normalized URI별 원본 Diagnostic을 저장하고, marker만 기존 version/문서 binding gate를 유지합니다. raw `code`의 숫자/문자·`data`·`source`를 marker로 재구성하지 않으며 저장 전 code action은 이 Store에서 읽습니다. Session.Document 중복 필드와 Format의 수동 Session 복제를 제거하고 Clone으로 같은 snapshot 계약을 유지했습니다.

URI key는 원본 Monaco URI.parse/toString의 component percent decoding·file/http/https 기본 루트·Windows drive 소문자·userinfo 보존/host 소문자·query/fragment 구분·잘못된 UTF-8 escape의 graceful 보존을 따릅니다. 경로 dot segment를 제거하거나 localhost/다른 authority를 파일 접근용으로 합치지 않습니다. URI 비교만 하며 실제 파일 열기/root guard는 우회하지 않습니다. lsp-types0.97/fluent-uri0.1.4와 설치된 Monaco uri.js의 공식 소스를 읽었고 Bun으로 합성5개 URI의 실제 Monaco 출력도 확인했습니다. Bun 비교 프로세스는 출력 후 event loop가 남아 해당 진단 세션에 Ctrl-C로 종료해 exit130이었으므로 명령 PASS로 세지 않습니다. 출력이 확인된 사실만 기록합니다.

raw Store 신규1 PASS(compile12.86초/suite0.00초)는 미바인딩 입력·같은 세션 덮어쓰기/동일 batch Arc 보존·다른 세션/clone snapshot 독립·빈 batch·명시 URI remove·encoded 괄호/쉼표/CJK·drive/authority/query/fragment/encoded slash/invalid UTF-8 escape·dot segment 비합침을 확인합니다. 명령은 `cargo test … --lib 원본_raw는`입니다.

실제 child 신규1 PASS(compile12.13초/suite0.63초) 뒤, 문서 구독 해제와 모델 폐기를 분리하는 생산 코드 변경의 영향만 확장한 동일 검사가 PASS(compile10.17초/suite0.42초)입니다. `--native-raw-diagnostics` 합성 child가 wire 괄호/쉼표 URI의 미바인딩 raw와 bound 진단을 발행합니다. 실제 notices/bindings·raw code42/data/source·bound marker1·다른 문서가 살아 있는 세션에서 첫 문서 unbind 뒤 raw 유지·마지막 문서 닫기/owner 제거·reacquire의 새 UUID·child/task0을 확인했습니다. 같은 성공 상태를 반복한 것이 아니라 실제 lifetime 변경과 두 문서 입력을 검사했습니다. 명령은 `cargo test … --lib 실제_child의_미바인딩_raw`입니다. 초기 mock example build는13.90초입니다.

저장 전 code action의 출처가 바뀐 영향 검사1 PASS(compile2.19초/suite0.34초)는 실제 codeAction/resolve/executeCommand/server applyEdit→imports→format→cleanup→disk save를 확인합니다. 명령은 `cargo test … --test lsp 실제_저장은_fix_all`입니다. 원본 mode는 그대로이고 신규 mock flag에서만 추가 raw를 발행합니다. 성공한 Store/저장/기존 viewport/font/입력/watch 검사는 재사용합니다.

raw는 session 수명과 함께 회수하고 단순 didClose/unbind만으로 삭제하지 않습니다. URI 변경은 이전 key를 제거하고 같은 URI의 language 재연결은 유지합니다. 아래 모델 폐기·기본 inactive/grace 후속에서 App caller를 연결했습니다. full diagnostics consumer·전체 provider 수명은 미완료입니다. source와 같은 누적 raw 입력의 aggregate URI/heap 정책도 M8 최종 보안·메모리 gate에서 판정해야 합니다. 기존 frame/incoming bound는 개별 전송 제한이지 누적 raw quota가 아니며 임의 cap/eviction으로 원본 기능을 조용히 바꾸지 않았습니다.

초기 app strict는 constant chunks_exact 지적2건 때문에 실패했습니다. 동일 고정 배열 iterator인 as_chunks로 정정해 strict exit0(17.61초)였습니다. 이후 didClose/모델 폐기를 분리한 실제 생산 코드 변경과 영향 검사를 반영한 최종 `cargo clippy … --lib --bin taide-native-app --tests --example native-lsp-mock -- -D warnings`는 exit0(16.05초)입니다. 성공 동작 검사는 반복하지 않았으며 Wry dependency의 기존17경고는 별도입니다. authored Rust4파일 exact rustfmt·문서 포맷·tracked/untracked whitespace 검사를 확인했습니다. Cargo/Bun은 모두 종료됐고 live handle은 없습니다.

## 모델 폐기 기록·전송·실제 child 후속 결과

대상은 `native/taide-native-editor/src/store.rs`, 직접 core 검사3파일과 `native/taide-native-app/src/{application,lsp,lsp-diagnostics-tests}.rs`입니다. 원본 `model-registry.ts`의 실제 model dispose와 `adapters/diagnostics.ts`의 각 session disposal listener를 기준으로 구현했습니다. 실행 전 RED가 아니라 실제 코드 대조에서 폐기 알림 미연결을 확인했습니다.

EditorStore에서 opt-in 폐기 기록을 제공합니다. clean release·revision-gated discard·파일 retarget의 이전 URI·displaced target·Untitled 저장 합류가 기록되며 View detach, 실패한 삭제·전환, 같은 경로의 metadata 전환은 기록되지 않습니다. 프레임 시작/끝 snapshot 비교를 쓰지 않아 같은 프레임에 생성·삭제된 모델도 놓치지 않습니다. opt-in하지 않은 기존 Store에는 기록이 누적되지 않습니다.

NativeApplication의 실제 Store 생성에서 tracking을 켜고 `reconcile_lsp`에서 visible Sync보다 먼저 폐기 기록을 전송합니다. 큐 포화/폐쇄에는 기록을 그대로 남기고 그 프레임의 Sync를 중단합니다. 성공적으로 큐에 들어간 뒤에만 확인 처리합니다. [Tokio try_send 공식 계약](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Sender.html#method.try_send)에 따라 성공은 큐 입장이며 처리 완료 ACK라고 주장하지 않습니다. worker 종료에는 session raw 전체도 Drop됩니다.

worker의 DisposeModels는 normalized URI를 모든 live session raw에서 제거하고 해당 URI에 바인딩된 모델만 닫습니다. 같은 DocumentId가 다른 URI로 이동한 provider는 URI 필터로 보호합니다. File만 현재 native LSP가 입장시키는 모델이며 Untitled/AppFile과 UTF-8이 아닌 File 경로는 LSP URI로 입장할 수 없어 전송에서 제외합니다. 이 제외를 전체 Untitled/provider 지원 완료로 해석하지 않습니다. 원본처럼 dispose 이후 새 알림의 raw 저장 자체를 tombstone으로 막지는 않습니다.

- [x] core release/discard 신규1·retarget 영향1·Untitled 영향2, 총4 PASS(compile1.51초, 각 suite0.00초)입니다. `cargo test --manifest-path native/taide-native-editor/Cargo.toml --test model-disposal --test file-retarget --test untitled --locked --offline --target-dir experiments/native-shell-spike/target`입니다. attached/dirty/stale 거절·detach·실제 폐기·재열기·ack/재enable·old/target/Untitled key·same-path 무기록을 확인했습니다.
- [x] bridge 큐 신규1·실제 child 입력 확장1, 총2 PASS(compile13.94초/suite0.39초)입니다. `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib diagnostics_tests --locked --offline --target-dir experiments/native-shell-spike/target`입니다. 큐 포화/폐쇄 시 pending 보존·성공1회 전송/확인·File-only 전송과 실제 session의 unbind 보존→모델 폐기 raw 삭제·다른 문서/owner 유지·같은 프레임 생성/삭제된 미바인딩 URI raw 삭제·마지막 session/child/task0을 확인했습니다. 이전 성공의 단순 재실행이 아니라 바뀐 폐기 경로를 확장한 입력입니다.
- [x] core lib/직접3test strict exit0(1.27초)·app lib/bin/tests/mock example strict exit0(17.23초)입니다. Rust7파일 exactfmt exit0이며 Wry의 기존 dependency warning17건은 별도입니다. Cargo 공통 환경은 위 결과와 같고 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo`이며 직렬 실행했습니다. 성공한 raw Store·저장 code action·기존 watch/입력/font 검사는 재사용했습니다. 마지막 assert의 동등한 slice 표현만 clippy 관례로 정리했고 동작 검사를 반복하지 않았습니다.

실제 App caller의 소스 연결과 실제 core→bridge→child 경계는 확인했지만 NativeApplication 창을 구동한 disposal·모든 provider의 동시 retarget·전체 모델 heap은 실행하지 않았습니다. 같은 ID/다른 URI 보호의 branch는 현재 코드 근거이며 실제 다중 provider 재현의 대체 증거가 아닙니다. 폐쇄된 큐에서 장기간 새 URI가 계속 폐기되는 pending journal 누적도 최종 메모리 gate에 남깁니다. 이 단계에서 남았던 원본5000ms 유예와 inactive marker 차이는 아래 후속으로 구현했습니다.

PROCESS/HANDOFF/재개 프롬프트/QA/bug 문서5개 Prettier exit0·tracked diff whitespace exit0입니다. 신규 소스7개/QA/bug의 no-index whitespace 검사는 출력 없이 exit1로 파일 차이만 표시했으며 whitespace 오류가 아닙니다. 실행 중인 Cargo/Bun handle은 없습니다.

## 비활성 모델 진단·세션5초 유예 후속 결과

대상은 `native/taide-native-app/src/{application,lsp,lsp-idle,lsp-status,lsp-diagnostics-tests}.rs`, `tests/lsp-recovery.rs`와 mock server입니다. 원본 `lsp-session-registry.ts`의 `LSP_SESSION_DISPOSE_GRACE_MS=5000`, release/acquire·프로젝트 강제 폐기와 `adapters/diagnostics.ts`의 모델 기준 marker 수명을 기준으로 구현했습니다.

App은 core의 전체 File 모델 목록에서 변경·제거된 snapshot만 worker에 전송하며 queue 입장 실패에는 identity cache를 변경하지 않습니다. journal→모델 목록→프로젝트 목록→활성 Sync 순서입니다. marker Store의 retain은 visible 파일이 아니라 실제 전체 모델 수명에 맞추고 전송 실패보다 먼저 수행합니다. worker의 session은 활성 mirrors와 이미 발행한 marker 모델을 분리합니다. 알림의 raw를 먼저 보존하고 전체 live 모델 URI에서 marker 대상을 찾으므로 never-bound/inactive 모델도 진단을 받습니다. 모델 폐기에는 marker ID도 지우고 session 폐기에는 owner 전체를 회수합니다.

마지막 활성 문서 close는 didClose만 전송하고5초 idle deadline을 설정합니다. 중복 release가 deadline을 미루지 않고 성공적인 acquire가 취소합니다. worker의 기존 select loop가 가장 가까운 deadline을 기다리므로 session별 별도 timer task를 만들지 않습니다. 종료/기한을 일반 큐보다 먼저 확인하며 [Tokio sleep_until](https://docs.rs/tokio/latest/tokio/time/fn.sleep_until.html)과 [select의 취소·우선순위 계약](https://docs.rs/tokio/latest/tokio/macro.select.html)을 따릅니다. project 목록에서 제거된 session은 유예 없이 stop하고 해당 project의 Sync cache도 제거합니다. App Exit의 기존 bridge disconnect도 전체 session을 즉시 stop합니다. 현재 native lock의 Tokio는1.53.1이며 의존성/lock을 변경하지 않았습니다.

활성 문서 알림은 기존 protocol version gate와 그 mirror snapshot의 revision을 사용합니다. inactive 모델은 전체 모델 snapshot의 revision을 사용합니다. 이 구분이 없으면 모델 목록만 먼저 revision1로 갱신되고 server mirror는0인 사이에 옛 진단이1로 표시돼 App의 stale gate를 통과합니다. 실제 child의 didSave/version0 입력에서 기대0/실제1 RED를 확인하고 active revision 출처를 수정했습니다. 테스트는 실제 marker Store/bridge watch 결합을 사용하며 모델 목록이 먼저 바뀌었다고 server 진단 revision을 임의로 올리지 않습니다.

- [x] 원래 실제 child 검사에 unbind 후 marker owner 보존 기대를 추가해 RED(compile6.73초/suite0.32초, false/기대true)를 확인했습니다. 구현 뒤 큐/기존 child/새 lifetime 고유3 PASS(12.72초/6.05초)입니다. 큐에는 모델 목록 중복 전송 없음·제거 전송 포화→cache 보존→다음 입장 성공도 추가했습니다. 명령은 `cargo test … --lib lsp::diagnostics_tests`입니다.
- [x] 실제 lifetime 검사에 앞선 revision1/기존 mirror0 입력을 추가해 RED(6.84초/0.51초)를 확인했습니다. 수정 뒤 해당1건만 PASS(6.67초/5.12초)입니다. 처음 성공했던5초 검사와 입력/생산 코드가 달라진 영향 확인이며 동일 상태의 단순 반복이 아닙니다. never-bound 실제 모델 marker·inactive didClose 알림 교체·같은 owner/PID 재사용·deadline 취소·실제5초 이상 후 session/marker0·disconnect/child/task0을 확인했습니다. 명령은 `cargo test … --lib 실제_child는_비활성_모델_marker와_5초`입니다. mock build는 최초11.57초, 새 revision 입력 mode 영향1.91초이며 별도 `--native-idle-diagnostics`에서만 close/save/change 진단을 발행합니다.
- [x] virtual clock Idle 정책1 PASS(compile0.24초/suite0.00초), crash/replay/명시 project stop 영향1 PASS(6.86초/0.84초)입니다. 전자는4999ms·취소·중복 release·새5초 deadline을 확인하며 후자는 기존 backoff 종료 지점에 explicit project stop을 사용합니다. 각각 `cargo test … --lib 세션_유예는_5초이며`, `cargo test … --test lsp-recovery 실제_editor_bridge는_child_crash후`입니다. 총5개 고유 검사이며 이전 raw/저장/core/font/입력 검사는 재사용했습니다.
- [x] 최종 app lib/bin/tests/mock example strict exit0(17.19초)입니다. 초기 strict는 동등한 let-else를 `?`로 단순화하라는1규칙으로 exit101이었고 `?`로 정리한 뒤 관련 strict만 재실행했습니다. 동작 검사는 반복하지 않았습니다. Rust7파일 exactfmt exit0이며 Wry dependency warning17건은 별도입니다. Cargo 공통 환경/직렬 정책은 위 결과와 같습니다.

실제 App GUI의 비활성 편집/폐기·여러 provider/다른 URI로 부분 retarget·모든 root 합류/handshake·full diagnostics consumer·raw/marker/catalogue/journal 전체 heap은 아직 검증하지 않았습니다. 기본 inactive/grace가 통과했다고 이 범위를 완료로 세지 않습니다. 다음은 다중 session의 폐기/retarget 경계이며 quota/RSS는 최종 메모리·보안 계약으로 남깁니다.

이번 후속의 문서7개 Prettier·tracked whitespace는 exit0입니다. 새 소스7개/QA/bug3개의 no-index whitespace는 출력 없이 exit1로 파일 차이만 표시했습니다. live Cargo/Bun handle은 없고 전체 M8 N1~N8은0/8·목표 active이며 전체 완료 전 commit/push하지 않습니다.

## 두 provider 부분 URI 전환·이전 marker 소유권 후속 결과

대상은 `native/taide-native-app/src/lsp.rs`와 `lsp-diagnostics-tests.rs`입니다. 이전 marker 기록은 DocumentId만 보존했습니다. 모델 catalogue가 같은 ID의 새 URI로 갱신되고 한 provider만 전환된 뒤 다른 provider가 이전 URI의 inactive marker를 보존하면 DisposeModels가 옛 모델 ID를 찾지 못해 marker가4개/기대3개로 남았습니다. 실제 두 합성 child에서 RED(compile0.23초/suite0.38초, exit101)를 확인했습니다.

Session의 marker 기록을 `(DocumentId, URI)` map으로 바꿨습니다. URI 전환에는 해당 ID의 옛 기록을 지우고 모델 폐기에는 normalized URI가 같은 기록을 모든 session에서 제거합니다. 실제 sync/dispose 처리를 private helper로 분리해 worker와 두 branch 검사가 동일 생산 경로를 사용합니다. 공개 binding은 그대로 모델 ID를 투영합니다. 다른 URI로 전환된 같은 ID의 활성 mirror는 기존 URI filter로 보호합니다.

- [x] 비활성 이전 URI branch1 PASS(compile11.41초/suite0.20초), `cargo test … --lib 실제_두_provider의_이전_uri_폐기는`입니다. RED 뒤 수정에 영향받는 해당1건만 재실행했습니다.
- [x] 활성 이전 URI branch 신규1 PASS(compile7.38초/suite0.48초), `cargo test … --lib 실제_두_provider의_활성_이전_uri_폐기는`입니다. 두 branch의 공통 본문을 helper로 옮겼고 앞서 성공한 비활성 입력은 반복하지 않았습니다.
- [x] app lib/bin/tests/mock example strict exit0(16.34초), `cargo clippy … --lib --bin taide-native-app --tests --example native-lsp-mock -- -D warnings`입니다. Wry dependency의 기존17경고는 별도이며 끄지 않았습니다. 공통 환경은 위와 같은 locked/offline/CARGO_HOME/target-dir이며 Cargo는 직렬입니다.

두 검사는 builtin Python provider discovery의 basedPyright/ruff spec을 실제 합성 executable wrapper로 연결합니다. 실제 Python 언어 도구를 검증한 것이 아닙니다. 실제 File service/core rename·encoded 괄호/쉼표 URI 폐기 journal·소유한 Session map·생산 sync/dispose/notice·registry watch와 실제 marker Store를 사용합니다. 이전 URI owner만 제거해 marker3개, 새 URI/다른 문서2개의 raw/bindings·미바인딩 raw·동일 PID/owner·idle 해제·감독 child/task0을 확인했습니다. full worker dispatcher/NativeApplication GUI·전체 provider/root 합류를 구동한 검사는 아니며 큐와 기존 recovery 성공은 재사용했습니다.

처음 잘못 입력한 Unicode filter `実际_两_provider`는 compile11.58초 뒤0tests로 exit0이었으므로 PASS에 포함하지 않습니다. 올바른 filter로 실제 RED부터 확인했습니다. 이전 raw/저장/core/inactive/grace/입력/font/viewport 성공을 반복하지 않았습니다. manifest/lock·제품 TS·보호 bundle·사용자 앱·OS·Git은 변경하지 않았습니다.

## 공유 provider의 여러 root 합류 후속 결과

대상은 `native/taide-native-app/src/lsp.rs`, `lsp-diagnostics-tests.rs`, `crates/taide-lsp/src/native/session.rs`와 합성 mock server입니다. 원본 `src/entities/lsp/lsp-session-registry.ts`의 공유 group roots·한 initialize, 기존 Rust `should_reuse_session`/`lsp_actions`의 동일 project/server 공유와 root 추가 알림을 따랐습니다. 기존 native는 SessionKey에 root를 포함하고 항상 별도 child를 만들어 실제 두 Python workspace에서2개/기대1개 RED(exit101, compile7.68초/suite0.02초)를 확인했습니다.

같은 project/server의 공유 가능한 session을 canonical key로 선택하고 최초 root·owner·PID를 유지합니다. 새 root는 기존 initialize가 Running에 도달한 뒤에만 actor에 추가합니다. 종료된 session은 합류 대상에서 제외하고 nonshare spec/다른 project는 분리합니다. root 목록은 문서 close에 지우지 않고 group/session 종료까지 유지하는 원본 계약입니다. 서버 workspaceFolders 응답과 applyEdit 허용 root scope에도 전체 목록을 연결했습니다. 실제 applyEdit 왕복은 이 검사에 포함하지 않습니다.

actor는 기존 workspace-folder serializer로 추가 알림을 만들고 duplicate root는 무변경 처리합니다. 후보 initialize에 aggregate heap/wire budget을 검사하고 outgoing enqueue 성공 뒤에만 저장하므로 실패가 root 목록만 바꾸지 않습니다. 다음 restart의 initialize는 저장된 전체 roots를 사용하며 기존 primary root는 유지합니다. [LSP3.17 root 변경 알림](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/workspace/didChangeWorkspaceFolders.md)의 added/removed 형식을 확인했습니다. 발행 조건은 기존 제품 backend의 추가 root 정책을 유지했으며 전체 dynamic workspace 등록 parity를 새로 완료했다고 주장하지 않습니다.

- [x] 공유 root 실제 합성 child1 PASS(compile7.24초/suite0.13초), `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib 공유_provider의_두_root는 --locked --offline --target-dir experiments/native-shell-spike/target`입니다. 실제 pyproject 두 root discovery·단일 child/owner·marker2개·encoded 괄호/쉼표 URI·추가 알림·서버 root 조회 응답·중복 Sync 무추가·다음 generation 새 PID/같은 owner·단일 initialize/두 roots·다른 문서 close 생존·child/task0을 확인했습니다.
- [x] nonshare spec/다른 project 실제 합성 child1 PASS(9.11초/0.41초), 같은 manifest에서 `--lib root_비공유_spec과`입니다. Rust 두 Cargo root의 독립 child/owner와 공유 Python spec의 project 분리를 서로 다른 입력으로 확인했습니다. 실제 rust-analyzer/basedpyright 도구가 아니라 builtin spec을 통한 합성 executable wrapper입니다.
- [x] actor phase/duplicate/aggregate budget·공개 command 입장 byte 거절의 원자적 보존1 PASS(compile2.99초/suite0.00초), `cargo test --manifest-path Cargo.toml -p taide-lsp --lib workspace_root_추가는 --locked --offline --target-dir experiments/native-shell-spike/target`입니다. 실제 actor handler/queue와 기존 prepare fixture를 사용하며 파일/OS child를 띄우지 않는 독립 용량 경계입니다.
- [x] root LSP lib/tests strict exit0(1.21초)·app lib/bin/tests/mock strict exit0(18.09초), Rust4파일 exactfmt/check exit0입니다. 명령은 각각 root manifest `cargo clippy … -p taide-lsp --lib --tests -- -D warnings`, native app manifest `cargo clippy … --lib --bin taide-native-app --tests --example native-lsp-mock -- -D warnings`이며 공통 CARGO_HOME/locked/offline/target-dir는 위와 같습니다. Cargo는 직렬이고 기존 Wry17경고는 별도입니다.

실패를 구분합니다. 첫 fixture에서 확인하지 않은 cleanup API 이름/시그니처를 써 컴파일 exit101이었고 task count 이름을 다시 잘못 써 두 번째 컴파일 exit101이었습니다. 실제 기존 fixture와 TaskSupervisor/LspStore 정의로 정정한 뒤에만 위 RED를 얻었습니다. 생산 수정 뒤 marker0/기대2(exit101, compile12.52초/suite0.42초)는 새 fixture가 실제 Store의 retain_documents를 등록하지 않은 원인이었으며 fixture를 정정한 뒤 해당1건만 PASS했습니다. 이를 생산 marker 회귀로 기록하지 않습니다. mock 최초 build10.12초·mock의 root mode에만 telemetry를 한정한 수정 뒤 build12.48초입니다. native 별도 workspace에서 dependency `-p taide-lsp` test 선택은 dev-dependency/member 제약으로 exit101이어서 실제 root workspace 명령으로 실행했습니다. 성공한 검사는 반복하지 않았고 URI 소유권2건과 이전 inactive/grace/raw/저장/입력/font 성공은 재사용했습니다.

이번3검사는 full worker dispatcher/NativeApplication GUI가 아니며 root 합류와 실제 protocol/marker 경계만 확인합니다. 동일 generation handshake3회/2초 재시도·실패 후 dispose/reacquire·전체 provider/consumer/GUI·누적 heap/M8 gates는 미완료입니다. manifest/lock/MSRV·제품 TS·보호 bundle·사용자 앱·OS·Git은 변경하지 않았습니다.

## 동일 child/generation 재초기화 후속 결과

대상은 `crates/taide-lsp/src/native.rs`, `native/session.rs`, `native/taide-native-app/src/lsp.rs`, `lsp-recovery.rs`, `lsp-diagnostics-tests.rs`와 합성 mock입니다. 원본 `lsp-session-registry.ts`의 같은 generation에서 최대3회·2초 간격·각15초 initialize deadline·live 문서 replay·소진 후 group 폐기를 따랐습니다. [LSP3.17 initialize](https://raw.githubusercontent.com/microsoft/language-server-protocol/gh-pages/_specifications/lsp/3.17/general/initialize.md)의 성공 전 메시지 경계와 InitializeError.retry도 확인했습니다. 자동3회 정책은 기존 제품 호환 계약이며 표준이3회를 요구한다고 주장하지 않습니다. fixture 오류에는 retry:true를 보냅니다.

native는 첫 initialize 오류에 child를 회수해 실제 Running 대기가 `ServerError(-32002)`로 실패했습니다(exit101, compile7.68초/suite0.63초). 재시작 generation만 same-child retry를 예약하고 내부 Degraded backoff를 외부 Initializing/failure 없음으로 표시해 기존 process-crash recovery가 조기에 다른 child를 띄우지 않도록 했습니다. timer가 만료되면 새 request ID와 기존 request timeout으로 initialize를 보내고 최신 문서·전체 root initialize는 유지합니다. initial generation/transport exit는 별도 기존 계약입니다. Stop/reap/restart는 timer를 지우며 retry 소진은 ReinitializeExhausted로 표시합니다.

회복 monitor는 소진을 process crash로 재시작하지 않습니다. 실제 worker는 channel·generation·phase·failure가 현재 소유자와 같을 때만 session을 회수하고 marker 상태를 발행하며 Failed를 반환합니다. 매 프레임 같은 visible snapshot으로 자동 연결 루프를 만들지 않으며 실제 close/open의 synced cache 제거 뒤 새 owner로 연결합니다.

- [x] 실제 child success1 PASS, `cargo test --manifest-path native/taide-native-app/Cargo.toml --lib 재초기화는_같은_child_generation에서 --locked --offline --target-dir experiments/native-shell-spike/target`입니다. compile17.71초/suite4.13초이며 첫2회 오류/3번째 성공·2초씩 간격·같은 PID/generation·initializeCount3·문서 hover 내용·child/task0을 확인했습니다.
- [x] 실제 worker 소진/reacquire1 PASS, 동일 manifest의 `--lib 재초기화_소진은_worker_owner`입니다. compile4.85초/suite4.15초이며3회 오류 뒤 같은 generation/Stopped/PID 없음·session binding/summary/marker0·동일 visible sync 무재생성·close/open 뒤 새로운 owner/generation0/marker1·종료 task0입니다. 실제 LspBridge dispatcher/recovery/notice를 사용하지만 NativeApplication GUI 검증은 아닙니다.
- [x] coordinator latest/deadline/late 응답/Stop/원자성1 PASS, `cargo test --manifest-path Cargo.toml -p taide-lsp --lib reinitialize_retry --locked --offline --target-dir experiments/native-shell-spike/target`입니다. compile0.91초/suite0.00초이며 initial generation retry 거절·timer 만료 뒤 새 request deadline/ID·이전 request 및 이전 generation 응답 무시·Degraded 때 변경한 최신 text/version replay·Stop 이후 retry 거절을 확인했습니다. 실제4초 중 동시 editor 편집을 구동한 검사는 아닙니다.
- [x] actor backoff Stop1 PASS, 같은 root manifest의 `--lib reinitialize_backoff`입니다. compile1.25초/suite0.00초이며 actual handler·외부 Initializing/failure 없음·예약 deadline·Stop 뒤 timer/pending/outgoing 없음·Stopped/같은 generation·retry 거절을 확인했습니다. OS child 없는 독립 취소 경계입니다.
- [x] root LSP lib/tests strict exit0(1.06초), native app lib/bin/tests/mock strict exit0(18.24초), Rust6 exactfmt/check·tracked whitespace exit0입니다. 명령은 위 root/app strict와 동일하며 기존 Wry dependency17경고는 별도입니다. Cargo는 직렬이고 성공 검사는 반복하지 않았습니다.

실패를 구분합니다. worker 검사의 첫 marker0/기대1(exit101, compile7.90초/suite0.64초)은 fixture에서 actual Store의 live 모델 retain 등록이 없었던 원인입니다. fixture 정정 뒤 해당 검사만1회 실행했습니다. mock 소진 mode build는 exit0/11.73초입니다. coordinator 검사 시작의 잘못된 filter `reinitialize_retryは`는 compile2.02초/0tests로 성공에 포함하지 않습니다. 올바른 filter에서 counter overflow 뒤 Detected/기대Degraded RED(exit101, compile0.11초/suite0.00초)를 재현했고 retry begin 실패의 phase를 복원해 해당1건만 PASS했습니다. actor test의 대문자 Stop snake_case warning은 소문자로 이름만 정정해 final strict로 확인했으며 동작 검사를 재계측하지 않았습니다. earlier mock/production apply_patch hunk 실패2건은 파일 변경 없이 정확한 실제 본문으로 재적용했습니다.

이전 marker URI/공유 roots/비활성 grace/저장/raw/입력/font 성공은 재사용했습니다. 전체 언어 도구·provider/consumer·dynamic root 등록/applyEdit 왕복·실제 App GUI/OS·누적 quota/RSS와 전체 M8 gates는 남아 있습니다. manifest/lock/MSRV·제품 TS·보호 bundle·사용자 앱·OS·Git 불변이며 live Cargo handle은 없습니다.

## 비활성 hover opacity·tooltip 기본 표시 후속 결과

원본 `problem-severity-filter.tsx`의 opacity-60은 비활성 버튼 배경에도 적용되지만 native hover RectShape는 alpha255를 유지했습니다. 신규 실제 렌더 검사에서 `#2A2D2EFF`/기대 `#191B1C99` RED(exit101, compile10.26초/suite0.04초)를 확인했습니다. CountButton에 opacity를 전달해 배경·icon tint·text에 같은 값으로 적용하고 status/선택 상태는1을 유지했습니다. 수정 뒤 신규1 PASS(compile4.56초/suite0.04초), `cargo test … --lib problems_비활성_filter의_hover`입니다. 실제 AX Click으로 필터 off/on·hover 배경·text color·선택 복귀를 확인했습니다. 이 검사는 CSS opacity의 offscreen group 합성/겹친 픽셀 전체 동등성을 확인한 것이 아닙니다. 당시 app lib/tests strict exit0(16.13초)는 다음 tooltip 변경 이전 결과입니다.

원본 `shared/ui/tooltip.tsx`와 pinned egui Tooltip/Popup/Frame/RichText 공식 소스로 tooltip 표시를 비교했습니다. actual status의 popup에 resolved tooltip.background가 없는 RED(exit101, compile7.50초/suite0.03초)였습니다. Appearance의 기존 theme resolver에 tooltip.background/border를 연결하고3곳 tooltip에 같은 Frame을 사용합니다. 원본 radius6px·border1px·가로12px/세로6px padding·12px UI font/line-height18px·app.foreground를 유지하고 별도 shadow를 넣지 않습니다. 기존 align/collision 방향을 바꾸지 않았습니다.

- [x] tooltip style 신규1 PASS(compile7.59초/suite0.04초), `cargo test … --lib problems_tooltip은_테마색`입니다. 실제 dark/light theme 해석·RectShape fill/stroke/radius·galley font/color/line-height·border/padding을 포함한 실제 popup 치수입니다. fixture color assertion은 RichText의 실제 section.color를 읽으며, 쓰이지 않는 fallback_color로 색을 판단하지 않습니다.
- [x] tooltip 치수/표시 변경 영향1 PASS(compile0.23초/suite0.03초), `cargo test … --lib problems_버튼은_원본_모서리`입니다. status/filter/Close의4px 버튼/16.5px status·위/아래 방향을 확인했습니다. 이전 성공을 그대로 재계측한 것이 아니라 실제 tooltip renderer 변경의 영향 검사입니다.
- [x] 최종 app lib/tests strict exit0(16.09초)·이번 Rust8 exactfmt/check·tracked whitespace exit0입니다. root LSP strict1.06초·app bin/mock strict18.24초는 LSP 경계에서 성공한 결과를 재사용합니다. Cargo는 직렬이고 같은 상태의 성공 검사를 반복하지 않았습니다. 기존 Wry dependency17경고는 별도입니다.

원본 wrapper의 기본0ms만으로 제품 지연을 판단하지 않았습니다. 실제 `AppProviders`는400ms를 지정하지만 pinned egui 기본은500ms입니다. 당시 full hover/focus/skip-delay 수명은 미완료였으며 아래 후속에서 기본 provider와 Problems/IDE caller를 연결했습니다. arrow/animation/text-balance/정확한 collision shift·CSS group opacity 픽셀·전체 GUI는 아직 구현/검증 완료가 아닙니다. tnum 기능은 pinned TextFormat에 feature 선택 필드가 없고 shaper가 기본 ShapeOptions를 사용해 여전히 미완료이며 mono 대체/임의 폭 조정을 추가하지 않았습니다. manifest/lock·제품 TS·보호 bundle·사용자 앱·OS·Git 불변이며 live Cargo handle은 없습니다.

## 공용 tooltip 지연·키보드·hoverable content 후속 결과

NativeApplication의 Provider를 Problems/IDE status에 공유하고 viewport별400ms deadline·300ms skip·focus/blur/Escape·실제 mouse click·외부 down·content transit·widget/viewport/context 회수를 연결했습니다. source는 실제 AppProviders와 설치된 Radix Tooltip이며, geometry의 WorkOS MIT 고지는 LICENSE-RADIX-TOOLTIP에 보존했습니다. global egui style·OS 시계를 바꾸지 않았습니다. 자세한 원인은 [tooltip-lifetime bug](../bug/2026-10-04-native-tooltip-lifetime.md)입니다.

아래 명령의 공통 부분은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib`입니다. RawInput의 합성 시각과 실제 egui Response/Popup/paint 결과를 사용합니다.

- [x] `problems_tooltip은_키보드_focus에_즉시_열리고_escape와_blur에_닫힌다`: 실제 AX Focus/paint/Escape/지속 focus/다른 버튼 blur 고유1 PASS(compile7.89초/suite0.04초)입니다. 원래 focus RED는 compile9.18초/suite0.02초, wiring 후 첫 paint RED는12.93초/0.03초였습니다. egui의 첫 sizing pass를 같은 시각의 다음 paint로 구분했습니다. 이후 새로운 혼합 입력 Focus(error)→Enter→Focus(warning)을 추가해 RED(7.05초/0.04초)→확장한 같은 검사1 PASS(0.23초/0.02초)입니다. 입력 조건이 바뀐 관련 검증이며 동일 성공 재실행이 아닙니다.
- [x] `tooltip_provider는` 최초2검사 PASS(compile7.36초/suite0.02초)입니다.400ms 직전/정확한 기한·hover 이동에도 deadline 연장 없음·300ms skip 안/경계·실제 paint/Escape 소비와 trigger→gap→content→outside·렌더 후 widget 회수를 확인했습니다. 뒤에 추가한 같은 접두사 검사와 구별하며 최초 성공2건은 다시 실행하지 않았습니다.
- [x] `tooltip_provider는_짝없는`: 짝없는 up에서 [false,false]/기대[true,false] RED(6.72초/0.02초)→수정한1 PASS(5.72초/0.02초)입니다. 실제 down·held·release에서 닫힌 상태 유지도 확인했습니다.
- [x] `tooltip_ -- --skip tooltip_provider는 --skip problems_tooltip --nocapture`: touch/폐기된 pending 재마운트·viewport별 독립/제거·같은 ID의 다른 Context·실제 Problems→IDE skip 공유3 PASS(compile9.00초/suite0.03초)입니다. 같은 실행의 content-down 검사1만 RED였으므로 전체 실행 PASS라고 쓰지 않습니다. 실제 caller의 resolved locale/theme/port·텍스트·prior tooltip 폐기를 확인했습니다.
- [x] `tooltip_content의_pointer_down`: content 내부 down을 닫던 RED(9.00초/0.03초의 위 실행)→수정한1 PASS(11.51초/0.01초)입니다. content release 뒤 외부 down은 닫습니다.
- [x] `tooltip_keyboard_space`: 짝없는 release RED(4.19초/0.01초)와 실제 keys_down 없이 repeat=true를 넣은 fixture 실패(9.78초/0.01초)를 구분했습니다. 실제 press→held repeat→release로 정정한1 PASS(6.18초/0.02초)입니다. armed Space와 raw Focus 순서 수정 뒤 이 검사만 재실행했습니다.
- [x] `tooltip_pointer_down_ref`: down 뒤 다른 trigger focus→원래 trigger focus에서 [true,false]/기대[false,false] RED(compile7.10초/suite0.02초)→widget별 held ref 수정1 PASS(8.00초/0.02초)입니다. actual PointerButton·Focus/memory·Response에서 up까지 억제, 실제 release 뒤 새 keyboard focus 왕복은 다시 열리는 조건을 확인했습니다.

최초 touch/viewport 검사는 새 hit-test 등록 프레임 누락으로 각 최종 기대에서 [false,false] RED(compile7.30초/suite0.02초)였습니다. egui의 실제 이전-pass interaction 등록을 반영해 mount/replacement layout을 먼저 렌더했습니다. 동시에 새 actual IDE fixture의 `client_count` 누락으로 compile E0063(exit101)2회가 있었고 실제 IdeStatus의4필드를 확인해1로 정정했습니다. 두 번의 동일 compile 실패는 중복이었으며 성공 증거로 사용하지 않습니다. Touch Start/End/PointerGone 경계를 fixture에 포함했습니다.

held pointer ref 수정 이전의 `cargo clippy --manifest-path native/taide-native-app/Cargo.toml --locked --offline --target-dir experiments/native-shell-spike/target --lib --bin taide-native-app --tests --example native-lsp-mock -- -D warnings` exit0(18.20초)입니다. 기존 Wry dependency17경고는 새 코드 검사 성공과 구분합니다. 기존 LSP/marker/버튼/테마/font/geometry 성공은 재사용하며 전체 M8 suite green으로 주장하지 않습니다.

그 뒤 held pointer ref만 변경해 app lib/tests strict를1회 실행한 최종 결과는 exit0(16.22초)입니다. bin/mock18.20초는 재사용하며 고유10 성공을 다시 일괄 재실행하지 않았습니다. authored7 Rust exactfmt와 tracked/untracked whitespace 검사에 출력 오류는 없었고 docs5 Prettier exit0입니다. no-index의 파일 차이 exit1은 whitespace 오류와 구분합니다.

마지막으로 준비된 동일 pass 여부를 raw snapshot clone 전에 확인해 소비자별 Event/Text의 반복 복사를 없앴습니다.400ms/skip/입력 정책은 바꾸지 않았고 해당 변경은 app `--lib` clippy `-D warnings` 단일 검사 exit0(5.25초)와 해당 파일 format/whitespace로 확인했습니다. 이전 runtime 성공·lib/tests16.22초/bin/mock18.20초는 재사용하며 전체 RSS/CPU benchmark를 했다고 주장하지 않습니다.

- [ ] 모든 tooltip consumer 연결과 style 통일·원본 arrow/animation/text-balance/collision shift·content AX role/described-by·content hit-test/modal gate입니다.
- [ ] 모든 pointer/touch/keyboard의 한 프레임 혼합·동일 프레임 down/up/focus 전체 순서·PointerGone·조상 scroll과 실제 App/aux GUI·DOM 픽셀입니다. 현재 단일-event 및 명시한 AX/keyboard/held pointer sequence 성공으로 전체 입력 graph를 증명하지 않습니다.
- [ ] 이전 full LSP/provider/consumer/quota/RSS·Tab/full AX·tnum/CJK·viewport/glyph/cache·전체 App/remote UI·213views/41actions/Monaco21·cutover/TS제거/성능/보안/배포 gate입니다. N1~N8 0/8·목표active이며 전체 완료 전 commit/push는 없습니다.

서브에이전트/workflow·제품 TS·manifest/lock·MSRV·보호 bundle·사용자 앱·OS·Git은 이 후속에서 유지했고 새 의존성은 없습니다. 검증 성공은 입력·코드 위험별 재사용하며 고유10건을 다시 한꺼번에 실행하지 않았습니다.

## 공용 tooltip renderer·AX·상태바 소비자 후속 결과

대상은 `native/taide-native-app/src/tooltips.rs`, `problems.rs`, `problems-tests.rs`, `status-ide.rs`, `status-editor.rs`, `system-usage-view.rs`, `system-usage-view-tests.rs`, `application.rs`입니다. 원본 `src/shared/ui/tooltip.tsx`, `src/features/window/status-bar.tsx`, `font-size-stepper.tsx`와 pinned egui0.36.2의 Tooltip/Popup/Area/Frame/Response/Ui/Context 공식 소스를 직접 대조했습니다. 새로운 의존성은 없습니다.

Provider가 공용 theme renderer를 소유하며 Problems·IDE status·글꼴 icon/button8곳·system usage button이 같은 App owner를 사용합니다. 원본 tooltip.background/border·app.foreground·12px/18px line·radius6·border1·padding12×6을 재사용합니다. 글꼴 icon은 viewport/target별 실제 stable Response ID를 사용하며 클릭 동작·설정 저장 경로는 그대로 유지합니다. system usage detail의 HTML title 대응 기본 tooltip은 이 공용 소비자로 세지 않습니다.

Tooltip의 실제 area ID에 Role::Tooltip·라벨·최종 rect를 연결하고 열린 동안만 trigger의 described_by를 설정합니다. Button role은 보존하고 IDE span은 자식 생성 전에 실제 unique UI ID의 GenericContainer를 생성합니다. Tooltip content의 이름과 자식이 실제 AX tree에 존재하며 Tooltip에는 Focus action을 추가하지 않습니다. Area::end가 반환하는 move Response ID와 실제 area ID를 혼동한 중간 구현은 Role::Tooltip이2개인 RED였고, 실제 area ID로 수정했습니다.

실패를 구분합니다. 최초 Problems role 검사0/기대1(compile7.72초/suite0.03초) 뒤 중간 구현2/기대1(11.45초/0.03초)을 확인했습니다. area ID 수정 후의 직접 자식 label assertion 실패(7.22초/0.03초), Role::Label.value로 바꾼 뒤 실패(7.28초/0.03초)는 fixture의 직접 자식 가정이 틀린 원인이었습니다. 진단 출력(6.42초/0.03초)과 공식 Frame::begin으로 실제 Tooltip→GenericContainer→Label.value 계층을 확인해 해당 계층을 검증하도록 정정했습니다. Role/연결 검사를 삭제하거나 tree를 임의로 평탄화하지 않았습니다.

통합 상태바 검사에서 안정 색상 assertion이 두 번 실패(7.42초/0.03초,9.40초/0.02초)했습니다. 첫 visible pass의 Area fade-in으로 실제 alpha21인 것을 확인했고 실제 시각을 전진한 안정 paint에서 테마를 검사했습니다. 원본 animation/transform parity를 구현한 결과로 주장하지 않습니다. 같은 fixture의 case별 시각도 단조 증가하도록 정정했습니다. 이어 disabled Editor Font Size의 Role::Tooltip 잔존 RED(9.30초/0.02초)는 제품 오류였으며 `contains_pointer()`가 disabled에서도 true인 공식 계약이 원인입니다. disabled Response의 열린 상태·deadline·pointer/focus/cache를 회수하고 실제 re-enable 뒤 오래된 예약이 되살아나지 않게 했습니다.

이번 후속 고유7검사의 성공 근거입니다. 공통 명령은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml --lib <filter> -- --nocapture`이며 각 filter를 한 번씩 실행하고 성공은 재사용했습니다.

- [x] `tooltip은_ax_role`:1 PASS, compile6.25초/suite0.03초입니다. 실제 Problems AX Focus·Tooltip1개·이름/양의 bounds·non-focusable·Frame 자식 text·Button described_by·Escape 후 role/설명 제거입니다.
- [x] `ide_tooltip은`:1 PASS,0.25초/0.02초입니다. 실제 dark/light·source theme/frame/font/line/top 방향·span bounds/GenericContainer·Connected Label.value 자식을 확인합니다. 이전 style-only1 PASS 뒤 AX 입력을 확장한 결과이며 그 이전 성공을 새 AX 성공으로 세지 않습니다.
- [x] `problems_tooltip은_테마색`: 변경 renderer 영향1 PASS,0.22초/0.02초입니다. 실제 dark/light Frame·12px/18px·여백/테두리/색상을 확인합니다.
- [x] `실제_글꼴_버튼은`: 변경 caller 영향1 PASS,7.80초/0.08초입니다. 기존6개 버튼의 실제 Settings 저장/이벤트 roundtrip과 독립 두 크기 변경입니다.
- [x] `system_usage_상태버튼`: 변경 caller 영향1 PASS,0.23초/0.03초입니다. 기존 실제 status click/수치/texture·상세 목록·닫기 경로입니다.
- [x] `status_tooltip은`: 신규 통합1 PASS, disabled 수정 뒤7.84초/0.06초입니다. 이후 pending 취소와 described_by 제거 입력을 추가한 같은 검사1 PASS(9.02초/0.06초)입니다. dark/light 각각 글꼴 icon2/button6/system button1의 실제 Response·TOP/12px·테마·AX role/설명·disabled 즉시 회수·re-enable 시 pending 무재생성을 확인합니다. 실제 OS VoiceOver 검사는 아닙니다.
- [x] `problems_버튼은_원본_모서리`: 변경 renderer의3곳 방향 영향1 PASS,0.28초/0.04초입니다. status/filter/Close의 실제 상하 배치를 확인합니다.
- [x] app lib/tests strict exit0(24.17초), `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml --lib --tests -- -D warnings`입니다. 신규 IDE test의 불필요한 비교 참조 clippy::op_ref로 최초 exit101이었고 해당 비교만 정정해 관련 strict1회로 확인했습니다. 동작 입력이 같아 위 성공 검사를 다시 실행하지 않습니다. Wry dependency의 기존17경고는 새 authored 코드 검사와 구분하며 억제하지 않았습니다. 변경 Rust8개 exact rustfmt check와 tracked diff whitespace exit0입니다.

이전10개 기본 수명/입력 성공은 재사용했습니다. 모든 tooltip 소비자·HTML title/forced validation popup 구분·mixed pointer/touch graph·원본 arrow/motion/text-balance/collision shift·content/modal hit-test·GUI/DOM 픽셀·tnum/CJK·전체 M8 gates는 여전히 미완료입니다. 실제 OS/IME/VoiceOver·보호 bundle·제품 TS·manifest/lock/MSRV·Git은 변경하지 않았습니다.

## 탐색기 toolbar·controlled validation tooltip 후속

대상은 `native/taide-native-app/src/explorer_toolbar.rs`, `explorer.rs`, `explorer-tooltip-tests.rs`, `tooltips.rs`, `presentation-refresh.rs`, `application.rs`입니다. 원본 `file-tree-toolbar.tsx`의4개 bottom tooltip과 `file-tree-draft-row.tsx`의 `open={error !== null}`·`aria-invalid`를 대조했습니다. 설치된 Radix Tooltip1.2.16과 use-controllable-state1.2.6 공식 구현에서 controlled prop의 외부 변경은 onChange를 호출하지 않는 것을 확인했습니다.

툴바는 실제 Explorer 반환 Response를 실제 AppSurfaces의 공용 Provider·현재 theme Appearance에 연결합니다. Appearance는 초기 구성과 두 실제 테마 갱신 경로에 연결됩니다. 네 개 버튼의 원본 순서·BOTTOM·기존 클릭 액션은 유지하며 public API에 내부 Provider를 노출하거나 Context.data에 강한 소유권을 저장하지 않습니다.

오류 tooltip은 공용 paint/AX renderer만 재사용하고 hover/focus Provider의 is_open gate를 거치지 않습니다. Explorer가 commit/cancel 처리 뒤 실제 입력 ID와 현재 create/rename owner가 일치할 때만 오류 metadata를 전달합니다. 오류가 있으면 TextInput의 Invalid::True와 실제 Tooltip area의 described_by를 추가하고 BOTTOM으로 표시합니다. 오류 해제나 Escape 취소 뒤 같은 rendered output에는 오류 tooltip이 남지 않습니다. 전체 controlled close-attempt/Provider 사건 graph까지 구현했다는 뜻은 아닙니다.

공통 명령은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml <선택> -- --nocapture`입니다.

- [x] `--lib explorer_tooltip은`: 실제 dark/light·4버튼·AX Focus·Role::Tooltip1개/이름·Button described_by·테마/12px/BOTTOM·Escape 제거1 PASS(compile7.49초/suite0.07초)입니다. 최초 실제 role0/기대1 RED(7.57초/0.02초) 뒤 수정했습니다. TreeRowPage에 없는 revision 필드의 fixture compile 오류는 실제 필드 rows/total로 정정했습니다.
- [x] `--test explorer 탐색기_생성은_toolbar`: 기존 실제 생성/새 폴더/refresh/collapse·draft 검증/IME/확인 액션 영향1 PASS(13.70초/0.03초)입니다. 오류 metadata 추가는 액션/키 처리 구현을 변경하지 않았으므로 성공을 재사용했습니다.
- [x] `--lib explorer_오류_tooltip은`: 실제 create/rename×dark/light·TextInput invalid/설명·Tooltip role/name/테마/12px/BOTTOM·포인터 이탈 유지·오류 해제·Escape 취소1 PASS(11.79초/0.07초)입니다. 최초 실제 Tooltip role 없음 RED(6.87초/0.03초) 뒤 수정했습니다. sibling fixture의 Explorer struct update가 private 필드를 요구한 E0451은 Default 생성 뒤 public root 설정으로 정정했습니다.

Provider 기본 수명10건과 prior renderer/caller7건의 성공은 재사용합니다. 공용 paint 함수 추출은 기존 색상/Frame/라벨/ID/AX 동작을 그대로 옮겼으며 같은 동작 입력을 재계측하지 않습니다. 전체 Tooltip/HTML title 소비자 구분·arrow/motion/text-balance/collision·혼합 입력/modal·실제 GUI와 전체 M8 gates는 미완료입니다. 새 의존성/manifest/lock/MSRV/제품 TS/보호 bundle/OS/Git은 변경하지 않았습니다.

## 탐색기 후속 정적 검사

## 단축키 편집기 modal tooltip 후속

대상은 `native/taide-native-app/src/keybinding-editor.rs`, `keybinding-tooltip-tests.rs`, `application.rs`입니다. 원본 `src/features/settings/keybinding-row.tsx`의 실제 reset/unbind Button·aria-label·BOTTOM RadixTooltip를 대조했습니다. 두 실제 버튼 Response를 Output의 내부 metadata로 반환하고 App이 같은 Provider와 현재 theme Appearance로 렌더합니다. Editor 공개 API에 내부 Provider를 노출하지 않습니다. modal이 닫힌 rendered output의 metadata는 비워 오래된 trigger를 재생성하지 않습니다.

기존 App의 finish_frame은 shell 렌더 직후이므로 나중에 생성되는 modal trigger를 다음 pass에 회수할 수 있었습니다. App begin_frame은 그대로 유지하고 finish_frame을 실제 keybinding modal/render 뒤로 옮겼습니다. 첫 Escape는 Provider가 소비하여 tooltip만 닫고 modal은 유지하며 다음 Escape는 기존 Editor가 modal을 닫습니다. 실제 AX Focus·Button role/설명·Tooltip role/name·theme/12px/BOTTOM·두 Escape를 합성 headless actual Modal에서 확인했습니다.

- [x] `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml --lib keybinding_tooltip은 -- --nocapture`: 최초 실제 Tooltip role0/기대1 RED(compile8.33초/suite0.08초) 뒤 actual reset/unbind×dark/light1 PASS(6.90초/0.26초)입니다. 검증 편의를 위해 임의 버튼을 만들거나 실제 액션/초기화/저장 구현을 바꾸지 않았습니다.
- [x] 같은 환경의 `cargo clippy ... --lib --tests -- -D warnings`: exit0(16.93초)입니다. Wry dependency17경고는 별도이며 억제하지 않았습니다. 이전 탐색기/공용 상태바/수명 성공은 반복하지 않습니다.

원본 Tooltip/HTML title 구분도 실제 코드로 확인했습니다. keybinding reset/unbind는 BOTTOM, theme live-preview terminal token은 TOP, color-picker 색상 버튼은 BOTTOM, PDF previous/next/zoom4개와 HWP previous/next2개는 BOTTOM RadixTooltip입니다. 당시 keybinding만 연결 완료였으며 theme picker/ANSI의 추가 연결 결과는 아래 절이 정본입니다. 나머지 actual App owner 연결·preview external action/settings position/theme editor의 공용 IconButton/span 경계·전체 mixed input/modal hit-test·GUI·arrow/motion/collision·전체 M8 gates는 미완료입니다.

탐색기 후속의 최종 native lib/tests clippy `-D warnings`는 exit0(17.56초)입니다. 최초 collapsible_if exit101은 오류 metadata 두 Option의 중첩 if만 tuple pattern으로 정정했고 동작 성공은 재사용했습니다. 기존 Wry dependency17경고는 별도이며 억제하지 않았습니다.

## 테마 색상 선택·ANSI tooltip 후속

대상은 `native/taide-native-app/src/tooltips.rs`, `theme-color-picker.rs`, `theme-live-preview.rs`, `theme-editor.rs`, `settings-view.rs`, `application.rs`, `theme-tooltip-tests.rs`입니다. 원본 color-picker는 스포이트가 아니라 실제 Popover 색상 버튼의 BOTTOM Tooltip이고 ANSI16개 span은 TOP입니다. 실제 공용 `src/shared/ui/icon-button.tsx`를 추가로 읽어 token reset·bold·italic·theme duplicate/edit도 RadixTooltip이며 span 래퍼를 사용함을 확인했습니다. 이들은 HTML title로 분류하지 않으며 해당 래퍼/호출부는 다음 미완료 경계입니다. `toast-position-picker.tsx`9개는 TOP RadixTooltip입니다.

actual Response/label/align의 내부 Trigger metadata를 여러 실제 소비자에 재사용합니다. Picker는 public show 시 실제 trigger를 기록하고 Editor가 take하여 해당 pass의 Output에 포함합니다. LivePreview는 실제16개 ANSI Response를 반환하고 Editor→Settings Output→AppSurfaces가 같은 Provider/현재 theme로 렌더합니다. Settings가 theme editor를 닫는 pass에는 폐기된 Editor metadata를 전달하지 않습니다. UI 선언 자체를 별도 위치에 복제하거나 Context.data에 owner를 저장하지 않습니다. pinned Context::read_response는 이전 pass widget도 반환하므로 사라진 위젯을 실제 현재 소비자로 등록하는 용도로 쓰지 않았습니다.

실패와 진단을 구분합니다. 최초 actual Picker Tooltip role 없음 RED(compile8.28초/suite0.05초) 뒤 renderer를 연결했습니다. 이어 인접 ANSI에서 black/기대red 실패(7.39초/0.06초)와 좌표 진단(7.26초/0.06초)은 렌더 theme/ID 오류가 아니었습니다. target/current rect가 모두[57,359]-[69,371]이고 hovered=true/실제 pointer63,365인데 원본과 같은 transit grace가 다음 trigger를 억제했습니다. 설치된 Radix1.2.16의 trigger pointermove·global isPointerInTransit·getExitSideFromRect·document grace tracking을 대조했고, fixture를 grace 바깥에서 각 표본으로 진입하도록 정정했습니다. source transit 정책을 지우거나 같은 실패를 성공으로 세지 않습니다. 실제 browser 인접 전환의 전체 이벤트 graph까지 검증한 것은 아닙니다.

공통 환경은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml --lib <filter> -- --nocapture`입니다.

- [x] `theme_tooltip은`: 실제 Picker/ANSI16개×dark/light·AX Focus/실제 pointer·Button/GenericContainer·Tooltip role/name·described_by·테마/12px·BOTTOM/TOP·Escape 신규1 PASS(5.65초/0.15초)입니다. 합성 시각을 전진했으며 실제 OS 시간 대기는 아닙니다.
- [x] `native_theme_editor는_picker_hex_blur와_reset_순서를_보존한다`: 변경된 실제 Picker/Editor caller의 기존 hex blur/reset 동작 영향1 PASS(0.22초/0.07초)입니다. 이전 성공들을 일괄 반복하지 않습니다.

최종 native lib/tests clippy `-D warnings`는 exit0(16.52초)입니다. Wry dependency 기존17경고는 별도이며 억제하지 않았습니다. 기존 탐색기/단축키/상태바/수명 성공은 재사용하고 같은 성공 동작은 재실행하지 않았습니다.

전체 IconButton span/disabled focus·theme duplicate/edit/reset/bold/italic·Settings 위치9·PDF4/HWP2/preview external·mixed pointer/scroll/modal·arrow/motion/collision·전체 GUI/M8 gates는 미완료입니다. public Picker API·제품 TS·manifest/lock/MSRV·보호 bundle·OS·Git은 변경하지 않았습니다.

## Settings 토스트 위치9 tooltip 후속

대상은 `native/taide-native-app/src/settings-view.rs`, `settings-tooltip-tests.rs`와 앞 절의 actual App Output renderer입니다. 원본 `src/features/settings/toast-position-picker.tsx`9개 Button의 TOP Tooltip·aria-label·aria-pressed를 대조했습니다. 실제 interface section의9개 Response/label/TOP metadata를 Settings Output에 추가해 같은 App Provider/current theme로 렌더합니다. 클릭 시 Change::Position을 만드는 기존 코드와 선택 상태·실제 hit geometry는 변경하지 않았습니다.

- [x] `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml --lib settings_tooltip은 -- --nocapture`: actual 첫 top-left의 Tooltip role0/기대1 RED(compile7.14초/suite0.07초) 뒤 dark/light×9버튼·actual Views/AX Focus·Button/Tooltip role/name/설명·theme/12px/TOP·Escape 후 제거1 PASS(6.04초/0.18초)입니다. 현재 section의 실제9개 metadata를 확인하고 가짜 버튼/별도 renderer를 쓰지 않았습니다.
- [x] 기존 실제 numeric/position 클릭·blur 성공은 [native-settings-surface QA](2026-10-03-m8-native-settings-surface.md)의31번 근거(suite0.07초/compile4.13초)와 해당 source를 확인해 재사용합니다. 이번 변경은 클릭/저장/숫자 처리 구현을 바꾸지 않았으며 동일 성공 입력은 재실행하지 않았습니다.

최종 native lib/tests clippy `-D warnings`는 exit0(16.23초)입니다. 직전 theme 단계의 strict16.52초 뒤 이번 Settings renderer/test source가 추가되어 관련 정적 검사만1회 실행했고 이미 성공한 동작 검사는 반복하지 않았습니다. 기존 Wry dependency17경고는 별도이며 억제하지 않았습니다.

전체 IconButton span/disabled focus·theme token/reset/duplicate/edit/bold/italic·preview controls·mixed/modal/scroll·arrow/motion/collision·전체 GUI/M8 gates는 미완료입니다. 사용자 설정 값·제품 TS·manifest/lock/MSRV·보호 bundle·OS·Git은 변경하지 않았습니다.

## Theme·Settings IconButton span 래퍼 후속

대상은 `tooltips.rs`, `theme-editor.rs`, `settings-view.rs`, `icon-tooltip-tests.rs`이며 source `src/shared/ui/icon-button.tsx`를 대조했습니다. span은 enabled에서 hover-only, disabled에서 focusable_noninteractive이고 Button child는 원래 이름/클릭을 유지합니다. Tooltip 설명은 span에만 연결합니다. enabled Button의 실제 focus/AX Focus·Click을 span owner로 전달하며 disabled child는 focus alias에 넣지 않습니다. 바깥 UI disabled는 기존 Provider 회수 gate를 유지합니다.

원본이 HTML title이라는 이전 분류는 잘못이었고 RadixTooltip span으로 정정했습니다. 실제 최초 Tooltip role 누락 RED(6.96초/0.02초) 뒤 wrapper를 연결했습니다. 크기 지정용 allocate_ui_with_layout이 별도 GenericContainer를 만들어 직접 Button 자식 assertion이 RED(8.27초/0.03초)였습니다. pinned egui0.36.2 new_child/accesskit parent 코드를 확인하고 Button 자체 min_size24px로 구조를 수정했습니다. public API·의존성은 추가하지 않았습니다.

공통 명령은 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml --lib <filter>`입니다.

- [x] `icon_wrapper_tooltip은`: dark/light×ThemeReset/Copy/Pencil×disabled2 actual icon helper·AX role/name·span 설명/직접 Button child·BOTTOM/font12·Escape·disabled hover/Focus·child click 차단1 PASS(6.74초/0.10초)입니다.
- [x] `native_theme_editor는_picker_hex_blur와_reset_순서를_보존한다`: 변경 wrapper geometry/action 영향1 PASS(0.22초/0.07초)입니다.
- [x] `native_settings_theme는_실제_create_host와_목록갱신_닫힘을_연결한다`: 변경 Settings icon caller/ID 영향1 PASS(5.26초/0.29초)입니다.
- [x] native `cargo clippy --lib --tests -- -D warnings` 동일 prefix/locked/offline strict exit0(16.59초), 변경 Rust6파일 exact rustfmt exit0입니다. 같은 성공 입력을 재실행하지 않았고 Wry 기존17dependency경고는 억제하지 않았습니다.

PDF4/HWP2 실제 owner·preview external source 분류·모든 consumer/GUI·arrow/motion/collision·전체 M8는 미완료입니다. OS/IME/VoiceOver·보호 bundle·제품 TS·manifest/lock/MSRV·Git 불변입니다.

## PDF4·HWP2 직접 Button tooltip 후속

대상은 `preview_pdf_surface.rs`, `preview_hwp_surface.rs`, `preview_status.rs`, `application.rs`, `preview-tooltip-tests.rs`입니다. source `pdf-preview.tsx`, `hwp-preview.tsx`는 span IconButton이 아닌 Button 직접 TooltipTrigger·BOTTOM·disabled이고 `preview-status.tsx`, `unsupported-preview.tsx`의 external은 Tooltip/title 없는 일반 Button입니다. PDF/HWP actual Button Response/label/align만 내부 Output으로 반환하고 App의 현재 Provider/Appearance가 소비합니다. public show7인자/bool·appearance literal·control ID·page/zoom/paint/cache는 유지하며 Context read_response의 previous pass를 production consumer 조회에 사용하지 않습니다. external의 native-only on_hover_text는 제거했습니다.

공통 prefix는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo test --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml`입니다.

- [x] `--lib preview_tooltip은`: 잘못된 test color 키 editorWidget.background fixture 실패(8.02초/0.01초)를 actual editor.widgetBackground/widgetBorder로 정정했습니다. 이어 actual 첫 enabled PDF Button의 Tooltip role 없음 RED(4.88초/0.02초) 뒤 dark/light×PDF4/HWP2×disabled2 actual cache/UI·AX 역할/이름/설명·24px/Button geometry·BOTTOM/font12/theme·Escape/hover/화면 제거1 PASS(7.37초/0.16초)입니다. 합성 Page/Raster를 public Cache.accept에 주입했고 실제 파일/OS renderer는 실행하지 않았습니다.
- [x] `--test preview-pdf pdf_control의_키보드_페이지_zoom_상한과_실패단계_및_cache_admission을_검사한다`: 기존 caller 액션/disabled/page/zoom/실패/external/admission 영향1 PASS(11.81초/0.05초)입니다.
- [x] `--test preview-hwp-host hwp_surface의_locale_키보드_빈_page와_cache_상한을_검사한다`: HWP locale/keyboard/empty/cache 영향1 PASS(2.85초/0.05초)입니다.
- [x] native lib/tests `cargo clippy -- -D warnings` 동일 locked/offline prefix strict exit0(16.71초)·Rust6파일 exact rustfmt exit0입니다. Wry 기존17dependency경고를 억제하지 않았고 이전 성공은 재사용했습니다.

Tooltip arrow·원본 Radix flip/shift/motion/text balance·모든 consumer/혼합 입력/실제 App GUI·OS/IME/VoiceOver·전체 M8 gates는 미완료입니다. deps/manifest/lock/MSRV/제품TS/보호bundle/OS/Git 불변입니다.

## 원본 Tooltip DOM 실측·arrow/배치 후속

대상은 `tooltips.rs`, `tooltip-placement.rs`, `tooltip-placement-tests.rs`, `problems-tests.rs`, `experiments/native-tooltip-reference/`, `tools/m8-tooltip-source-measure.ts`와 두 third-party license입니다. 제품 TooltipContent/global.css와 설치된 Radix Popper/Floating UI의 공개 소스, pinned egui0.36.2 Area/Painter/RectShape를 확인했습니다. 신규 dependency·manifest·lock·제품 TS 변경은 없습니다.

원본 reference는 실제 TooltipContent/global.css/React Compiler로 단일 빌드했습니다. `envDir:false`로 환경 파일을 읽지 않고 합성 텍스트만 사용합니다. fresh ephemeral Chrome profile·service worker 차단·fake `.invalid` origin의 로컬 생성 HTML/JS/CSS route만 허용했고 외부 요청은 거절합니다. 기존 브라우저 profile/사용자 앱·보호 bundle·OS 설정은 조작하지 않았습니다.

`/private/tmp/taide-tooltip-source.J9DufD`의 첫 Vite 빌드는 105초 성공했습니다. Tailwind plugin이 105.2초를 차지했으며 실패한 브라우저 시작 때문에 빌드를 반복하지 않았습니다. sandbox Chrome 시작은 EPERM/SIGABRT였고 동일 build를 재사용한 격리 Chrome 실행만 승격했습니다. 첫 screenshot은 body readiness attribute 누락으로 전부 hidden이어서 시각 PASS로 세지 않았습니다. source reference와 생성 HTML에 `data-theme-ready/data-locale-ready`를 명시하고 visible predicate를 추가한 뒤 실제 visible DOM 측정 exit0(0.75초)·screenshot을 확인했습니다. 별도 제품 ready 로직은 바꾸지 않았습니다.

실측 원본은 font12px·line16px·body30px·padding12×6px·border1px·radius6px, body와 trigger 간격10px입니다. arrow는10×10px·rotate45deg·radius2px·translateY(-50%-2px)이며 center-offset으로 실제 중심을 맞출 수 없으면 숨깁니다. TOP/BOTTOM/LEFT/RIGHT·좌우 shift·상하 flip 8개 좌표/transform은 [실측 JSON](assets/2026-10-04-tooltip-source.json)과 [실제 screenshot](assets/2026-10-04-tooltip-source.png)에 저장했습니다. 이전 절의 line18px와 egui gap4/일반 Popup collision 기록은 당시 소스 추론값이며 이번 실제 DOM의 16px/10px로 정정합니다. 원본 font의 전체 glyph/pixel 동등성은 이 geometry 표본으로 증명하지 않습니다.

공용 renderer는 자체 Tooltip Area에서 중앙 정렬·교차축 sticky shift·반대 방향 flip·실측 arrow를 사용합니다. Label의 native-only320px 폭 제한을 제거했고 공개 controlled API/AX 연결은 유지합니다. arrow의 painter를 Area 내부 본문 painter에서 파생해 첫 sizing invisible과 fade opacity를 공유합니다. Provider의 content hover/outside-down은 body 또는 viewport에 clip된 회전 rounded arrow의 실제 면적을 검사합니다. arrow 외곽 bounding square의 빈 모서리를 content로 처리하지 않습니다. 라벨 크기 변경 뒤 잘못된 배치가 입력 없이 고착되지 않도록 실제 Frame 치수 변경 시 repaint를 예약합니다.

공통 Cargo prefix는 앞선 절과 동일한 locked/offline native app·spike target입니다.

- [x] `--lib tooltip_원본_dom은_16px_line과_10px_gap_arrow를_실제_renderer에_보존한다`: 기존 height32/기대30 RED(37.04초/0.02초) 뒤 actual renderer 네 방향·line/font·gap·shape·Tooltip AX/trigger description1 PASS(12.59초/0.03초)입니다. baseline 대문자 DOM 이름 경고는 소문자 이름으로 수정했습니다. 이 안정 상태 성공은 동일 위험 검사 반복 없이 재사용합니다.
- [x] `--lib tooltips::placement_tests::tooltip_arrow는_첫_sizing과_fade에서_본문의_표시상태를_공유한다 -- --exact`: 첫 단독 arrow RED(0.22초/0.01초)를 재현했습니다. 앞선 짧은 filter+exact 실행은0tests(8.07초)이며 PASS가 아닙니다. 원인은 새 layer_painter의 opacity1/visible 기본값이었고 Area 내부 ui.painter로 수정했습니다.
- [x] `--lib tooltip_arrow -- --nocapture`: 원본8개 좌표/flip/shift·first sizing/fade 공유·실제 Provider arrow tip hover/down 보존/빈 모서리 down 닫기/화면 제거 회수의 서로 다른 신규3건 PASS(6.76초/0.02초)입니다. DOM 좌표 비교 허용 오차는1/32px이고 원본 width/clientWidth의 subpixel 경계를 유지합니다. 모든 viewport/oversized/detached/best-fit 조건을 확인한 것은 아닙니다.
- [x] `--lib tooltip_geometry는_라벨_변경후_재배치를_예약한다`: 안정 상태의 라벨 증가 뒤 repaint_delay=Duration::MAX/기대0 RED(6.99초/0.02초)→실제 크기 변경 repaint 예약·다음 프레임 중앙 정렬/gap10 보존1 PASS(6.02초/0.02초)입니다. 즉시 같은 프레임 layout 완료나 browser ResizeObserver 전체 동등성을 주장하지 않습니다.
- [x] 공용 Area 전환/line16의 영향 검사 `--lib problems_tooltip은_테마색`1 PASS(0.22초/0.02초), `--lib preview_tooltip은`1 PASS(0.21초/0.13초)입니다. 기존 provider controller10·PDF/HWP 액션/cache·IconButton 등 변경 없는 성공은 재사용하며 전체 suite는 반복하지 않았습니다.

선택 TS typecheck `bunx --no-install tsc --noEmit -p experiments/native-tooltip-reference/tsconfig.json` exit0(0.47초)를 재사용합니다. 최종 동일 prefix `cargo clippy --lib --tests -- -D warnings` exit0(57.49초)이며 Wry dependency17경고는 억제하지 않았습니다. `rustfmt --edition 2024 --config skip_children=true`의 tooltips/tooltip-placement/tooltip-placement-tests/problems-tests4파일 exit0입니다. 이 정적 성공은 변경 없는 상태에서 다시 실행하지 않습니다.

남은 관련 경계는 source150ms/ease의 scale95/slide8/transform-origin과 closed Presence, multiline text-balance, viewport/DPI·oversized/detached/best-fit 전체, mixed keyboard/touch/scroll, 실제 underlying/modal hit-test와 GUI입니다. 이번 arrow tip 검사에는 뒤에 겹친 실제 Button이 없으므로 click-through 차단까지 검증했다고 쓰지 않습니다. OS/IME/VoiceOver는 사용자-last이며 후속46·M8 N1~N8 0/8·목표active, 전체 완료 전 commit/push 없음입니다.

## 원본 Tooltip motion·닫힘 Presence 후속

대상은 `tooltip-motion.rs`, `tooltip-motion-tests.rs`, `tooltips.rs`, `tooltip-placement.rs`, `css-motion.rs`, `toast-motion.rs`와 격리 motion reference입니다. 실제 Radix Tooltip/Presence/Popper·tw-animate CSS·pinned egui0.36.2 Area/layer transform/hit-test 소스를 확인했습니다. 기존 Toast의 CSS ease 계산을 공용 모듈로 옮겼으며 수식은 변경하지 않았습니다. 새 의존성이나 제품 TypeScript 변경은 없습니다.

`tools/m8-tooltip-motion-source-measure.ts`는 앞선 성공 빌드의 source CSS를 SHA256과 함께 재사용했습니다. `/private/tmp/taide-tooltip-motion.aUce4s`의 React Compiler 격리 빌드는192ms·Chrome 측정 exit0입니다. 환경 파일을 읽지 않는 설정·fresh ephemeral profile·service worker 차단·fake.invalid 로컬 route만 허용하는 경계는 유지했고 기존 브라우저/사용자 앱을 조작하지 않았습니다. native 전용 근삿값이 아니라 실제 CSSAnimation을 pause하고 currentTime=0/30/75/149ms에서 네 방향의 open/closed 32시점을 한 번씩 측정했습니다. 닫힘 finish 이후 실제 DOM 제거도 확인했습니다. 결과는 [motion 실측 JSON](assets/2026-10-04-tooltip-motion-source.json)입니다.

원본은150ms/ease(.25,.1,.25,1)·open opacity0→1/scale.95→1/side별 slide8→0이며 closed opacity1→0/scale1→.95/slide 없음입니다. CSS animation 방향 전환은 새 keyframe에서 시작합니다. closed 동안 Tooltip role은 남지만 trigger aria-describedby는 즉시 제거됩니다. transform-origin은 Popper arrow 중심/10px arrow 높이를 반영하며 이번 측정의 초기/재열림 원점에는1px 차이가 있었습니다. native 수식 테스트는 실측 origin을 입력으로 사용하므로 이1px의 동적 배치까지 동일하다고 주장하지 않습니다.

- [x] 첫 closed Presence 테스트는 role 즉시 제거 RED(compile9.62초/suite0.02초)였습니다. viewport별 Widget Motion에 clock과150ms 종료 수명을 추가하고 즉시 trigger 설명 해제·닫힘 중 Escape 소유권을 연결했습니다. Explorer 오류처럼 source가 TooltipContent 자체를 조건부 제거하는 소비자는 기존 즉시 unmount 경계를 유지합니다.
- [x] 구현 중 graphics_mut 안의 content_rect 호출은 실제 debug deadlock panic(compile12.52초/suite10.02초·exit101)이었습니다. clip을 graphics guard 밖에서 먼저 읽도록 수정했습니다. 테스트의 Vec2.distance API 오류는 공식 타입의 length로 정정했습니다. 이후 실측32시점 수식·실제 Provider closed Role/설명/중간 alpha/150ms 제거의 신규2건 PASS(compile7.91초/suite0.02초)를 재사용합니다.
- [x] renderer 영향 단일 실행은5 PASS/11 실패(compile0.22초/suite0.06초)였습니다. 실패는 이전 fixture의 즉시0-opacity paint 기대·즉시 closed role 제거·18px line 기대·scale 중 정수 corner_radius 비교였습니다. AX 즉시 열림과150ms paint/제거를 구분하고 actual source16px line·변환된 shape 식별로 수정했습니다. 탐색기 controlled 오류 해제의 즉시 unmount 기대는 그대로 유지합니다.
- [x] 수정된 실패11개와 공용 easing의 기존 Toast 영향1개만 선택해12 PASS(suite0.30초)입니다. `cargo test --lib --no-run` compile4.26초 이후 libtest의 여러 filter를 사용했으며 이미 성공한5개와 신규2개는 반복하지 않았습니다. Problems→IDE skip은 close0.6초/다음 진입0.7초로300ms 안의 즉시 AX 열림을 유지하고 별도로150ms 뒤 paint를 검사합니다. 단축키 modal은 closed75ms의 두 번째 Escape도 tooltip이 차단하고 기한 뒤 Escape는 modal을 닫습니다.

선택 TS typecheck `bunx --no-install tsc --noEmit -p experiments/native-tooltip-reference/tsconfig.json`과 해당 fixture/tool 포맷은 exit0입니다. 앞선 native strict57.49초는 motion 변경 전 증거이므로 변경 후 최종 strict를 대신하지 않습니다. CSSAnimation source 측정과32시점 성공은 반복하지 않습니다.

- [x] `--lib tooltip_motion은_이동한_본문`: animation-only 본문 위치를 실제 뒤쪽 Button에 겹친 재현의 layer_id_at=Background RED(7.25초/0.02초)였습니다. pinned egui의 set_transform_layer로 그림과 입력을 함께 변환하고 viewport clip은 inverse transform으로 고정했습니다. 이후 layer 판정은 맞지만 release Button 클릭이 통과하는 별개 RED(5.68초/0.02초)를 확인했습니다. hover-only Area를 포커스 없는 CLICK으로 바꿔 actual press/release 비실행1 PASS(5.21초/0.02초)입니다. 뒤쪽 Button 없는 기존 arrow tip 성공과 혼동하지 않습니다.
- [x] `--lib tooltip_motion은_화면_제거시`: 화면에서 trigger를 제거한 뒤 sticky layer transform 잔존 RED(7.16초/0.02초)→Provider finish/stale/content 종료의 guard 밖 identity 복원으로1 PASS(6.24초/0.02초)입니다. controlled validation의 별도 owner graph/모든 Context·viewport 교체 경계는 아직 완료로 세지 않습니다.
- [x] 최종 동일 prefix native `cargo clippy --lib --tests -- -D warnings` exit0(20.39초)입니다. 기존 Wry17dependency경고는 억제하지 않았습니다. 이전 성공 검사는 같은 입력으로 반복하지 않았습니다.

arrow/rounded corner의 실제 source hit·자식 AX/global bounds·전체 provider graph/controlled owner·viewport/DPI/collision/multiline/GUI는 미완료입니다. 후속46·N1~N8 0/8·목표active·전체완료 전 commit/push 없음이며 deps/제품TS/manifest/lock/MSRV/보호bundle/OS/Git 불변입니다.

## Tooltip 자식 AX 좌표·원본 hit 영역 후속

layer 입력/그림 변환 뒤 Label의 AX 좌표는 여전히 local rect였습니다. 앞선 closed Presence 테스트에 실제 fade TextShape의 위치/크기와 Label bounds 비교를 추가해 RED(compile5.94초/suite0.02초)를 확인했습니다. 관찰값은 AX `[166,67]-[258.4375,83]`, 실제 그림 약 `[167.8,68.3]-[256.5,83.7]`입니다. Frame 안의 실제 Label Response를 반환받고 부모와 같은 transform을 Label bounds에 적용했습니다. 수정된 검사1 PASS(compile6.94초/suite0.02초)·layer CLICK/AX 변경의 actual 단축키 modal 영향1 PASS(suite0.27초)입니다. 기본 motion32시점·다른 소비자의12 PASS를 반복하지 않았습니다. 이 단계의 최종 native lib/tests strict exit0(16.63초)이며 직전20.39초는 Label 수정 전 기록입니다.

`tools/m8-tooltip-hit-source-measure.ts`는 이미 성공한 `/private/tmp/taide-tooltip-motion.aUce4s/built`를 재사용하고 빌드나32시점 재계측 없이 source `document.elementFromPoint`만 네 방향×4위치에서 측정했습니다. selective TS typecheck/선택 포맷 exit0, 격리 Chrome 측정 exit0(0.704초)입니다. sandbox 시작 실패가 이미 확인되어 정확한 격리 명령만 승격했습니다. profile/service worker/local-only route 경계는 앞선 reference와 동일하며 결과는 [source hit JSON](assets/2026-10-04-tooltip-hit-source.json)입니다.

source는 네 방향 모두 arrow 중심/실제 tip을 Tooltip 자식으로 hit하며 arrow의 빈 bounding corner와 body 왼쪽 위0.25px 둥근 모서리는 Tooltip 자식으로 hit하지 않습니다. native Provider의 회전 arrow membership은 기본 표본과 맞지만 body는 아직 단순 Rect이고 egui Area 입력은 사각형입니다. 따라서 이번 측정은 native rounded body/arrow의 실제 뒤 Button 차단 또는 투과까지 완료한 증거가 아닙니다. raw 그림과 analytic membership만 맞추고 실제 input router의 사각 hit를 그대로 두는 해결은 채택하지 않습니다. pinned egui hit-test는 WidgetRect 기반이고 pass 시작에 이전 위젯의 rect로 입력을 결정하므로 UI callback에서 뒤늦게 pointer 좌표만 바꾸는 우회도 하지 않습니다.

다음 경계는 source rounded body/arrow의 정확한 실제 input hit·controlled owner의 sticky transform·전체 viewport/Context 교체와 혼합 입력입니다. source 실측 성공은 그대로 재사용하고 해당 native 재현·수정만 수행합니다. 실기 GUI/CJK/VoiceOver·후속46·M8 N1~N8 0/8은 미완료·목표active입니다.

## Tooltip 정확한 입력 영역·viewport/pass 소유권 후속

대상은 `native/taide-native-app/src/{tooltips.rs,tooltip-placement.rs,tooltip-hit-tests.rs}`, native manifest/lock과 `vendor/egui-input/src/{context.rs,hit_test.rs,pass_state.rs,memory/mod.rs}`입니다. 앞 절의 사각 input 미완료 상태는 이 수정 전 기록입니다. source hit JSON16위치와 기존 성공 브라우저 빌드를 재사용하고 재계측하지 않았습니다.

pinned egui0.36.2는 이전 pass의 WidgetRect로 UI 렌더 전에 입력을 판정하며 공개 Area/sense/Plugin API만으로 둥근 body·바깥 arrow의 정확한 영역을 전달할 수 없었습니다. 같은 pinned 소스에 native 한정 `[patch.crates-io]`를 적용했습니다. `set_layer_input_region`은 layer-local bounds/predicate를 현재 viewport/pass에 저장하고 hit-test와 Context layer 판정에 연결합니다. 입력 영역이 없는 layer의 사각 동작·modal 순서는 유지합니다. 다음 pass 전환에서 이전 callback을 즉시 회수하며 Context를 callback에 캡처하지 않습니다. Tooltip은 실제 transform/clip·rounded body·rounded 회전 arrow를 같은 predicate에 사용하고 포커스 없는 CLICK proxy로 뒤 Button을 차단합니다.

vendor는 공식 egui commit `49682f8baa058bf49e011035cfbd6e825f88a5ef`의0.36.2 소스입니다. 변경하지 않은111개 파일은 cache 원본과 byte-for-byte 비교 PASS이며 생성 patch의 여분 EOF newline은 apply_patch로 제거했습니다. [공식 MIT license](https://raw.githubusercontent.com/emilk/egui/49682f8baa058bf49e011035cfbd6e825f88a5ef/LICENSE-MIT)를 보존합니다. 새 버전/외부 package는 추가하지 않았으며 native lock은 egui의 registry source/checksum만 path patch로 전환했습니다. `cargo generate-lockfile --offline`은 기존 libssh2-sys0.3.2 yanked 때문에 exit101이어서 기존 lock graph를 보존했고 이후 locked/offline 검사로 확인했습니다. root/Tauri manifest·MSRV·제품TS·보호 bundle·OS·Git은 이 수정에서 불변입니다. vendored Rust는 authored migration/Rust99% 산정에 포함하지 않습니다.

- [x] `--lib tooltip_hit은_source16위치의_rounded_arrow와_뒤_button_투과를_보존한다`: 실제 behind Button/네 방향×4위치에서 top arrow-tip layer 판정 RED(compile8.00초/suite0.02초)→정확한 input region 연결 후1 PASS(21.38초/0.08초)입니다. source arrow center/tip은 차단하고 빈 arrow corner/rounded body corner는 실제 Button 클릭이 통과합니다.
- [x] `--lib layer_input_region은_이동후_최신_영역과_프레임_viewport_회수를_보존한다`: 같은 LayerId를 쓰는 두 viewport의 독립 영역·이동 후 최신 영역·Root/child 화면 제거의 Arc 회수1 PASS(7.47초/0.01초)입니다. fixture의 closure bool 추론 컴파일 실패·Area.show 완료 전 lookup·첫 sizing pass의 non-interactable 기대를 정정했습니다. 이후 child를 parent에서 등록하지 않아 실제 egui가 child viewport를 제거한 Arc2/기대3 실패는 실제 deferred child 등록으로 수정했습니다. 소유권 assertion을 낮추지 않았습니다.
- [x] 변경된 engine의 독립 영향4건은 기존 keybinding modal Escape, Problems pointer/key/AX 사건 순서, motion body 뒤 Button 차단, closed150ms role/Label AX입니다. 정확한 네 필터를 compiled binary에서 각각1회 실행해4 PASS(suite 합계0.27초)입니다. 이 engine 변경과 무관한 source motion32시점·소비자12 성공은 재사용합니다.
- [x] `cargo clippy --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml -p egui -p taide-native-app --lib --tests -- -D warnings`: exit0(22.70초)입니다. 기존 Wry17 dependency warning은 억제하지 않았고 engine/app 신규 warning은 없습니다.

controlled validation owner/disabled 중 sticky transform·Context 및 전체 viewport 수명·전체 mixed touch/modal/GUI·fractional radius/DPI·collision/text balance는 아직 미완료입니다. 위16위치와 실제 Button 성공을 전체 입력·픽셀 parity로 확대하지 않습니다. 후속46·N1~N8 0/8·목표active·전체 완료 전 commit/push 없음입니다.

## Tooltip controlled·disabled·Context 변환 회수 후속

대상은 `tooltips.rs`, `explorer_toolbar.rs`, `tooltip-motion-tests.rs`입니다. [egui0.36.2 공식 Context 계약](https://docs.rs/egui/0.36.2/egui/struct.Context.html#method.set_transform_layer)과 pinned 구현의 `Memory.to_global`을 대조했습니다. `set_transform_layer`는 입력/그림에 함께 적용되는 sticky 설정이며 Area 또는 Widget 제거만으로 자동 회수되지 않습니다. controlled validation은 Appearance renderer만 호출해 Provider의 회수 대상이 아니었고 일반 disabled 경로는 기존 Widget을 먼저 덮어써 이전 content 기록을 잃었습니다.

공용 Provider가 viewport별 실제 Tooltip LayerId와 마지막 렌더 pass를 별도로 소유합니다. 일반/controlled 렌더를 모두 등록하고 finish_frame에서 그리지 않은 layer를 회수합니다. viewport 제거·Context 교체는 이전 Context의 실제 소유 layer를 lock 밖에서 identity로 복원합니다. 새 Context에 대한 오래된 finish 호출은 새 owner를 변경하지 않습니다. Explorer controlled 오류는 hover/focus의 open/skip/motion graph에 넣지 않고 레이어 소유권만 공유하므로 Explorer Escape 취소·즉시 오류 unmount를 유지합니다. Appearance의 owner 없는 show_controlled wrapper는 geometry fixture 전용 cfg(test)이며 제품 호출부는 Provider API입니다.

- [x] `--lib tooltip_motion은_disabled_및_context_교체`: disabled 변환 잔존 RED(compile8.48초/suite0.02초)입니다. 수정 뒤 disabled 회수·재열림 실제 변환·새 Context 전환 후 이전 Context identity를 확인했습니다.
- [x] `--lib tooltip_motion은_controlled_오류_제거시`: controlled 오류 제거 후 변환 잔존 RED(6.94초/0.02초)입니다. 수정 뒤 제거 즉시 transform/AX role 회수와 Escape 비소비를 확인했습니다.
- [x] 수정 뒤 `--lib tooltip_motion은_`는6 PASS(8.24초/0.02초)입니다. 신규2건과 기존 motion body 차단/화면 제거/closed role·AX3건의 영향 검사는 필요했지만 넓은 필터가 변경하지 않은 source32시점 수식1건도 포함했습니다. 해당1건의 추가 실행을 새로운 증거로 세지 않으며 다음 검사는 exact 필터로 제한합니다.
- [x] actual Explorer `explorer_toolbar::tests::explorer_오류_tooltip은_create_rename의_controlled_open과_ax_invalid를_보존한다 --exact`: create/rename·dark/light·오류 해제/재열림/취소/AX1 PASS(suite0.05초)입니다.
- [x] `--lib tooltip_motion은_제거된_viewport의_layer만`: 실제 deferred child viewport 제거 뒤 child transform만 회수하고 살아 있는 Root transform은 유지하는 신규1 PASS(7.20초/0.02초)입니다. Root/child는 서로 다른 widget/layer ID 표본이며 같은 ID의 서로 다른 transform까지 확인했다는 뜻은 아닙니다.
- [x] native app lib/tests strict `-D warnings` exit0(17.62초), 변경 Rust9파일 exact rustfmt 및 `git diff --check` exit0입니다. engine 소스는 이 owner 수정에서 바꾸지 않았으므로 앞선 engine/app strict22.70초 중 engine 증거와 source16위치 성공을 재사용합니다. Wry17 dependency warning은 계속 보존합니다.

남은 경계는 같은 LayerId를 여러 viewport에서 쓰는 transform 격리, controlled close-attempt와 전체 provider graph·mixed pointer/touch/modal·전체 DPI/GUI입니다. engine의 viewport-owned exact region 성공과 서로 다른 ID의 transform 회수를 같은ID transform 격리 성공으로 확대하지 않습니다. OS/제품TS/dependency/manifest/lock/보호 bundle·Git은 이 owner 수정에서 바꾸지 않았고 M8 전체는 미완료입니다.

## Tooltip 동일 ID viewport 변환 격리 후속

위 owner 절의 같은ID transform 미완료 상태는 다음 수정 전 기록입니다. 같은 widget ID를 Root/child에서 사용하는 actual controlled renderer에 기존 child 회수 검사를 확장했습니다. 같은 LayerId가 `Memory.to_global`을 공유해 child가 제거돼도 Root의 새 transform이 child layer lookup에 남는 RED(compile6.35초/suite0.02초)였습니다. 동일ID의 input region이 viewport별이라는 앞선 성공은 이 global transform 충돌을 해결하지 못했습니다.

native Tooltip의 Area/AX/content layer ID를 보조 viewport에서 `trigger.with(viewport)`로 namespace하고 root ID는 그대로 유지했습니다. track/render/clear가 같은 ID 함수를 사용합니다. engine 전체의 persistent transform API를 바꾸지 않고 제품 Tooltip 소유 경계에서 충돌을 제거했습니다. 수정1 PASS(6.04초/0.02초) 뒤 서로 다른 실제 trigger 위치의 transform 비교를 추가해 강화 검사1 PASS(5.96초/0.02초)입니다. Root transform이 child 렌더 뒤 바뀌지 않고 두 변환이 서로 다르며 child 제거 후 Root만 살아 있습니다. namespace 변경의 서로 다른ID child 회수 영향은 exact filter1 PASS(suite0.01초)입니다. patch의 test 줄 위치 불일치는 적용 전 검증에서 거절되어 대상 줄을 확인하고 재적용했습니다.

최종 앱 lib/tests strict exit0(16.98초)·tooltips/motion exact rustfmt·diff exit0입니다. engine/source16/Root motion 및 Explorer 성공은 재사용했습니다. protected bundle/OS/제품TS·의존성/manifest/lock/Git은 이 namespace 수정에서 불변입니다. 이 검사는 두 실제 native viewport의 geometry/transform/제거 경계이며 전체 modal/touch/controlled close-attempt·DPI/GUI 검증으로 확대하지 않습니다. 후속46/N1~N8 0/8·목표active·전체완료 전 commit/push 없음입니다.

## controlled graph의 다음 코드 경계

원본 `src/features/explorer/file-tree-draft-row.tsx`는 오류가 없을 때도 `Tooltip open={false}`와 Trigger를 유지하고 Content만 조건부로 제거합니다. 입력 Escape는 stopPropagation/preventDefault 뒤 Explorer cancel이며 blur도 cancel입니다. 설치된 Radix Tooltip1.2.16의 onChange는 외부 controlled prop 변경이 아닌 setOpen 시도에서만 Provider onOpen/onClose 및 전역 tooltip.open을 호출합니다. use-controllable-state1.2.6은 controlled setter의 요청값이 prop과 다르면 onChange를 호출하되 prop 자체는 변경하지 않습니다. 따라서 오류가 계속 있는 동안의 닫기 시도와 오류가 없는 Trigger의 열기 시도에도 Provider skip/global 효과가 있을 수 있습니다. 현재 native controlled 경로는 실제 오류 Content의 소유권만 관리하므로 해당 전체 graph는 아직 구현·검증하지 않았습니다. source의 `node_modules/@radix-ui/react-tooltip/dist/index.mjs`와 use-controllable-state 구현을 대조한 이 경계부터 이어갑니다. 실제 source 오류 파일은 `features/explorer`이며 존재하지 않는 `features/file` 경로 조회 실패는 구현/검증 성공이 아닙니다.

## controlled 상태 요청·공용 지연·pass 최종 렌더 후속

대상은 `native/taide-native-app/src/{tooltips.rs,explorer_toolbar.rs,tooltip-controlled-tests.rs,tooltip-motion-tests.rs}`입니다. [Radix 공식 Tooltip 계약](https://www.radix-ui.com/primitives/docs/components/tooltip)과 설치된 Tooltip1.2.16/use-controllable-state1.2.6 코드를 대조했습니다. 공식 페이지의 표기 버전1.2.13을 제품 버전으로 바꾸지 않았으며 실제 이벤트 근거는 기존 설치된 구현입니다. 앞 절의 controlled Trigger 미연결 상태는 이번 수정 전 기록입니다.

Explorer는 오류 유무와 관계없이 실제 input을 공용 Provider의 controlled Trigger로 전달하고, 오류가 있을 때만 Content/Invalid를 표시합니다. Widget의 실제 controlled prop과 setOpen 요청을 분리합니다. 오류 없는 input의 focus/hover 열기 요청도 전역 기존 Tooltip 닫기와 Provider 지연 해제를 수행하지만 빈 Content를 만들지 않습니다. 오류가 남아 있는 상태의 닫기 요청은 actual prop을 바꾸지 않고 skip timer만 갱신합니다. 외부 오류 prop 변경은 onChange를 발생시키지 않습니다. 명시적 is_delayed와 skip deadline을 분리해 첫 controlled 오류의 onClose만으로 초기 지연이 건너뛰어지는 차이도 막았습니다. controlled input의 Enter/Space는 버튼 activation으로 처리하지 않으며 Explorer의 keydown 소유권을 유지합니다.

전역 닫기 요청이 나중 consumer에서 발생하면 앞서 그린 normal Trigger의 described_by가 같은 pass에 남는 실제 RED였습니다. Provider.show는 현재 pass의 실제 Response/label/appearance를 pending에 등록하고 finish_frame에서 모든 상태 요청 적용 뒤 최신 open/motion으로 한 번 렌더합니다. pending은 finish에서 take해 즉시 회수하며 새 pass나 Context 교체에서도 버립니다. 오래된 Response를 다음 pass에 소비자로 재사용하지 않습니다. controlled 오류의 조건부 즉시 unmount와 normal150ms closed Presence는 그대로 유지합니다.

- [x] 신규 실제 Explorer metadata 호출부의 `controlled_tooltip은_오류없는_input의_focus_열기시도로_기존_tooltip을_닫는다`: 초기 fixture의 WidgetInfo::text_edit 인자3/실제4 누락 E0061은 제품 RED가 아닙니다. fixture 시그니처를 pinned source와 맞춘 뒤 전역 닫기 상태1 PASS(compile8.20초/suite0.02초)였습니다. 같은 pass의 normal described_by 검사 추가는 실제 RED(6.00초/0.02초)이며 최종 렌더 queue 적용 후1 PASS(13.93초/0.02초)입니다.
- [x] `controlled_tooltip은_prop_변경과_닫기_시도를_분리하고_skip_타이머를_보존한다`: 외부 false→true/true→false prop, input Enter, 반복 outside down, 오류 유지·AX/제거, skip 재예약/만료·pending0입니다. `controlled_tooltip은_초기_오류의_닫기시도로_기본_지연을_건너뛰지_않는다`는 초기 controlled true→닫기 시도 뒤 is_delayed=true를 확인합니다. 신규2건을 추가한 필터는3 PASS(7.32초/0.02초)였으나 앞선 성공 focus1건도 포함했습니다. 해당 추가 재실행은 새 증거로 세지 않습니다. 이후 영향 검사는 libtest의 FILTERS.../--exact로 범위를 고정했습니다.
- [x] 변경된 공용 request/is_delayed 경계의 `tooltips::tests::` 기본 지연·skip·hoverable content·pointer down/up·Space·touch·viewport8건은1회8 PASS(suite0.01초)입니다.
- [x] 최신 pass 최종 렌더의 actual source16 뒤 Button·motion body 차단/closed AX·disabled/Context/같은ID viewport·controlled 제거/Escape·actual Explorer create/rename·actual keybinding modal8건은 정확한8필터 단일 실행8 PASS(suite0.28초)입니다. motion32시점·변경 없는 engine 검사/다른 소비자 성공은 재사용했습니다.
- [x] native app lib/tests `cargo clippy --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml -p taide-native-app --lib --tests -- -D warnings`: exit0(19.13초)입니다. authored Rust4 exactfmt·git diff --check exit0이며 기존 Wry17 dependency warning은 억제하지 않았습니다.

이번 범위는 controlled 기본 prop/request와 pass 최종 렌더입니다. AX Focus·pointer/touch·keyboard 여러 사건의 전체 순서, controlled top layer의 modal Escape·조상 scroll, 실제 source callback 전체 실측·전체 viewport/DPI/GUI는 아직 미완료입니다. 이 완료 범위를 full controlled graph로 확대하지 않습니다. 제품TS/engine/dependency/manifest/lock/MSRV·보호 bundle/OS/Git은 이번 수정에서 불변이며 후속46·M8 N1~N8 0/8·목표active·전체완료 전 commit/push 없음·live Cargo handle 없음입니다.

## Tooltip source focus callback·닫힘 Presence 재진입 후속

대상은 `experiments/native-tooltip-reference/{events.html,app/events.tsx,tsconfig.json}`, `tools/m8-tooltip-events-source-measure.ts`, `native/taide-native-app/src/{tooltips.rs,tooltip-motion-tests.rs}`입니다. 원본 shared Tooltip과 설치된 Radix1.2.16의 실제 `onOpenChange`/document `tooltip.open`/Content listener를 사용했습니다. [공식 Tooltip 계약](https://www.radix-ui.com/primitives/docs/components/tooltip)과 [Playwright Clock](https://playwright.dev/docs/clock)을 대조했고 dependency 버전은 변경하지 않았습니다.

이미 측정한 CSS를 SHA256 `b66166f52e29a2ffd85e4eaf679cc6ba3bc72b6580ae57f5ef8485edff53a177`로 재사용합니다. plain/controlled 두 fresh page의 Focus first→second→first와 후속 hover를 한 번씩 기록했습니다. source fixture의 일반 Tooltip은 uncontrolled이며 observer가 callback을 기록할 뿐 상태를 제어하지 않습니다. controlled fixture는 최초 오류 input의 blur에 전체 input/Tooltip을 제거합니다. 이는 original Explorer의 blur 취소 경계만 재현한 fixture이며 actual FileTreeDraftRowItem 전체/IME/키 처리 검증은 아닙니다.

CSS animation을 snapshot에서 pause해 닫힘 Content의 listener를 유지했습니다. 따라서 최초 세 Focus 전환의 닫힘 Presence 중 재진입은 확인하되, 가상350ms+400ms 후 hover 결과를 일반 CSS 시간 경과/150ms 제거의 근거로 사용하지 않습니다. source JSON의 `cssPresencePaused: true`와 전체 원자료를 보존합니다. 후속 hover의 self-close는 animation을 정지한 조건의 결과입니다. timer400/skip300·motion150ms의 기존 독립 성공 증거를 대신하지 않습니다.

- [x] 선택 fixture/tool 포맷과 `bunx --no-install tsc --noEmit --project experiments/native-tooltip-reference/tsconfig.json` exit0(직전 실행 묶음0.627초)를 재사용했습니다. `bun tools/m8-tooltip-events-source-measure.ts /private/tmp/taide-tooltip-events.lDAI8X /private/tmp/taide-tooltip-source.J9DufD/built/assets/index-DEfYI-e9.css --build-only`: exit0, Vite167ms/명령0.271초입니다. 같은 명령의 `--reuse-build`는 정확한 isolated Chrome 실행만 승격해 exit0(0.627초)이며 재빌드하지 않았습니다. fake origin/local-only route·fresh profile·service worker 차단을 유지했고 기존 사용자 브라우저를 조작하지 않았습니다.
- [x] [source event JSON](assets/2026-10-04-tooltip-events-source.json): 첫 focus는 first open, 다음 focus는 first closed/second open, 빠른 first 재진입은 first open/second closed/first closed입니다. 이미 mounted된 닫힘 Content가 새 document open event를 받아 자기 Tooltip을 다시 닫았습니다. controlled 최초 blur는 `input-blur:closed`/`controlled:closed` 후 input 제거/first open이며 input을 계속 남겨 두는 가정은 사용하지 않습니다.
- [x] `cargo test`의 정확한 필터 `tooltips::motion_tests::tooltip_motion은_source의_닫힘_presence_중_focus_재진입을_다시_닫는다 -- --exact`: source JSON의 최초3상태와 actual native Provider/AX를 대조해 third focus의 open Some(first)/기대None RED(compile7.56초/suite0.02초)였습니다. 일반 open request가 아직 present인 own closing Content의 listener를 반영하도록 연결한 뒤1 PASS(6.14초/0.02초)입니다. 기한 뒤 Content가 제거된 다음 focus에서는 정상 열림도 확인합니다. 모든 mixed event 순서를 해결했다는 뜻은 아닙니다.
- [x] compiled libtest의 exact2필터 closed150ms Role/즉시 described_by 해제와 오류 없는 controlled input focus/global close는 단일 실행2 PASS(suite0.01초)입니다. source32·engine/source16·unchanged consumer 성공은 재사용했습니다. native app `cargo clippy --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml -p taide-native-app --lib --tests -- -D warnings` exit0(17.24초)이며 기존 Wry17 dependency warning은 억제하지 않았습니다.

현재 수정은 일반 Tooltip의 닫힘 Presence 중 focus 재진입 경계입니다. 전체 ordered AX/pointer/touch/key·controlled top-modal Escape/조상 scroll·source unpaused 사건 전체·DPI/GUI와 후속46/M8 N1~N8은 미완료입니다. 제품TS/engine/dependency/manifest/lock/MSRV/보호bundle/OS/Git은 변경하지 않았고 전체완료 전 commit/push는 하지 않습니다. 문서 묶음 patch의 마지막 anchor 불일치는 적용 전에 거절됐고 실제 anchor를 확인해 재적용했습니다.

## Tooltip 등록된 Trigger의 ordered AX/key·controlled 입력 Escape 후속

대상은 `native/taide-native-app/src/{tooltips.rs,tooltip-key-tests.rs,explorer.rs,explorer-tooltip-tests.rs}`, `tools/m8-tooltip-{key,events}-source-measure.ts`, 격리 reference tsconfig입니다. source Tooltip의 focus/blur·Content dismiss와 실제 Explorer `handleKeyDown`의 stopPropagation/취소·pinned egui의 raw AX Focus 배정을 확인했습니다. [Playwright Keyboard](https://playwright.dev/docs/api/class-keyboard)·[Locator focus](https://playwright.dev/docs/api/class-locator#locator-focus) 공식 계약을 확인하고 기존 성공 event build를 그대로 사용했습니다. 새 source build나 dependency 추가는 없습니다.

`m8-tooltip-key-source-measure.ts`의 첫 실행은 same context의 병렬 Clock 제어 중 `runFor: Target page, context or browser has been closed` exit1(0.459초)이었습니다. 설치된 Playwright `coreBundle.js`의 Clock은 BrowserContext 소유이며 Page.clock도 같은 context.clock입니다. 따라서 각 사례의 context를 분리했습니다. 수정 뒤 `/private/tmp/taide-tooltip-keys.bTYyeL` 측정은1회 exit0(0.683초)입니다. [source key JSON](assets/2026-10-04-tooltip-keys-source.json)은 CSS animation pause 없이 세 가지 서로 다른 사건 순서의 Trigger data-state/aria-describedby/callback을 기록합니다. 가상32ms는 JS timer의 step이며 실제 CSS150ms 픽셀 시각의 근거는 아닙니다.

source에서 기존 first open 뒤 focus second→Escape는 second closed, Escape→focus second는 second open, 초기 아무 Tooltip 없음→focus second→Escape는 second closed입니다. native는 begin_frame에서 모든 Escape를 먼저 처리한 뒤 각 consumer의 focus를 적용해 첫/초기 두 경우에 second open과 stale AX가 남았습니다. 초기 경우는 Escape까지 하위 consumer로 전달됐습니다. 이 실제 RED는 UI 그리기 순서를 반대로 한 경우에도 동일했습니다.

Provider는 등록된 Widget의 trigger rect/focus target/enabled/actual controlled prop을 소유하고, raw AX Focus/Click·Enter/Space/Escape를 사건 순서대로 적용합니다. pointer down ref도 앞선 사건 위치에서 갱신해 focus 억제를 유지합니다. 처리한 raw action index는 같은 pass consumer에서 재적용하지 않으며 실제 Tooltip이 소유한 Escape index만 normalized events에서 소비합니다. focus로 잠깐 열렸다 닫힌 normal Content도 Motion/closed Presence를 보존합니다. closed→다음 focus의 skip/is_delayed와 같은 pass described_by는 최종 렌더에서 결정합니다. 등록 전 새 Trigger나 같은 pass의 제거/재배치·top-modal 전체 ordering까지 완료한 것이 아닙니다.

- [x] 신규 `tooltips::key_tests::tooltip_key는_source_focus_escape_순서와_같은_pass의_ax_timer_소비를_보존한다 --exact`: source3순서×native UI2순서의 open/AX/is_delayed/skip/Escape 소비 RED(compile7.69초/suite0.04초)→ordered replay1 PASS(8.07초/0.04초)입니다. 값만 닫는 우회가 아니라 open 후 close의 Provider timer와 normalized Escape owner를 함께 확인합니다.
- [x] 변경된 공용 입력/수명 영향은 compiled libtest의 `tooltips::tests::`8·controlled3·source focus Presence/closed AX2·actual Explorer/Keybinding modal2의 정확한 필터 묶음15 PASS(suite0.27초)입니다. 순수 motion32 수식/engine/source16/다른 unchanged 성공은 재사용합니다. 이 단계 app lib/tests strict exit0(17.06초)입니다.
- [x] 신규 actual Explorer `explorer_입력의_escape는_일반_hover_tooltip이_열려있어도_오류를_취소한다 --exact`: 오류 input과 일반 hover Tooltip의 실제 AX role2를 만든 뒤 Escape는 input owner에 남겨 취소합니다. 최초 실행은 draft 모델 취소 뒤 Output.input 잔존 RED(7.29초/0.03초)였습니다. 취소 분기에서 metadata를 회수하고 repaint를 요청해1 PASS(8.53초/0.03초)입니다. 오류 role 제거와 일반 hover role/설명 유지도 확인합니다. 이미 그려진 input의 같은 pass 픽셀 삭제까지 주장하지 않습니다.
- [x] Explorer 취소 수정 뒤 actual create/rename validation/Escape의 기존1건과 AX focus target alias/disabled span의 IconButton 영향1건은 exact2필터 단일 실행2 PASS(suite0.08초)입니다. 앞선15건 중 Explorer1은 변경된 취소 코드의 새 영향 검사이고 같은 입력·같은 코드의 반복 실행이 아닙니다. `cargo test ... --test explorer 탐색기_이름입력은_선택_키보드_검증_중복확정_취소와_ime를_보존한다 -- --exact`의 actual rename/blur/취소/합성 IME1 PASS(compile15.30초/suite0.02초)이며 real CJK 입력기 실기 검사는 아닙니다.
- [x] 최종 app `cargo clippy --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml -p taide-native-app --lib --tests -- -D warnings` exit0(17.05초)입니다. Wry17 dependency warning은 억제하지 않았습니다. selective source-tool typecheck/포맷은 환경 수정 뒤 exit0(0.503초)입니다.

앞선 event tool도 동일 context Clock 공유를 제거했으나 이미 성공한 측정을 반복하지 않았습니다. 이전 source-focus 원자료의 callback/Presence pause 상태는 보존하고 사례별 독립32ms라는 시간 주장에는 사용하지 않습니다. 존재하지 않는 `app.rs`/`problems-input.rs`/`explorer-tests.rs` 및 이전 Playwright 비번들 파일 조회 실패는 실제 `application.rs`/`problems.rs`/integration `tests/explorer.rs`/`coreBundle.js`로 정정했고 성공 증거가 아닙니다.

남은 경계는 등록 전/동적/disabled 현재 pass의 사건, 여러 pointer/touch/viewport·controlled top-modal Escape, 실제 조상 scroll·full source/GUI/DPI입니다. 제품TS/engine/dependency/manifest/lock/MSRV/보호bundle/OS/Git은 이번 수정에서 불변이며 후속46/M8 N1~N8 0/8·목표active·전체완료 전 commit/push 없음·live command 없음입니다.

## Tooltip 실제 조상 scroll·프로그램 offset 후속

대상은 `native/taide-native-app/src/{tooltips.rs,tooltip-scroll-tests.rs}`와 native 한정 pinned egui의 `src/{context.rs,pass_state.rs,ui.rs,containers/scroll_area.rs}`입니다. 설치된 Radix Tooltip Content의 capture scroll listener는 `event.target.contains(context.trigger)`인 경우만 `onClose()`를 요청합니다. [Radix Tooltip](https://www.radix-ui.com/primitives/docs/components/tooltip)과 [egui ScrollAreaOutput](https://docs.rs/egui/0.36.2/egui/containers/scroll_area/struct.ScrollAreaOutput.html), pinned Ui/ScrollArea 소스를 확인했습니다. 이번에는 source DOM scroll 브라우저 실측을 추가하지 않았으며 원본 코드 계약과 native 실제 UI 검사로 범위를 한정합니다.

egui의 `WidgetRect.parent_id`는 stable Ui ID로 정확한 물리 부모 그래프가 아니고, AccessKit parent는 접근성 재배치를 허용합니다. 따라서 Ui child/interact에서 unique ID 기반 물리 부모를 별도로 등록합니다. viewport별 current/previous PassState가 부모 관계·변경된 content ID·최종 offset만 소유하며 다음 pass에서 지우므로 영구 큐/Context 캡처/AX 활성화 의존성이 없습니다. cycle은 visited ID로 종료합니다. 기존 사각/rounded hit-test와 AccessKit parent map은 그대로 둡니다.

ScrollArea의 clamp·wheel·drag·scroll target 처리가 끝난 실제 offset을 이전 pass의 최종 offset과 비교합니다. 이전 pass에 없으면 begin에서 읽은 persisted offset을 기준으로 씁니다. 이에 따라 공개 `State.store`로 프레임 사이에 변경한 offset도 감지하고, 효과 없는 wheel 또는 음수 setter의 clamp 결과가 원래 값이면 scroll로 간주하지 않습니다. 공용 Provider finish에서 현재 등록된 Content만 조상 변경을 확인한 뒤 닫기 요청과 최종 렌더를 적용합니다. normal Tooltip의 설명은 즉시 제거하고 closed Presence는 유지하며 controlled 오류는 실제 prop/AX를 유지한 채 skip timer만 갱신합니다. 오류 없는 controlled false는 Content가 없어 요청하지 않습니다.

- [x] `tooltips::scroll_tests::tooltip_scroll은_실제_조상만_닫고_controlled_prop과_무변화를_보존한다 --exact`: 중첩 inner/outer·sibling·descendant·unchanged·clamped × normal/controlled true/false × AccessKit on/off를 actual ScrollArea에서 확인했습니다. 처음 테스트의 WidgetInfo 인자 순서 컴파일 오류는 fixture 오류입니다. 이후 실제 조상 scroll 뒤 open/AX/skip 잔존 RED(compile7.34초/suite0.14초)→1 PASS(15.06초/0.14초)입니다.
- [x] `tooltips::scroll_tests::scroll_조상은_ax_재배치와_무관하며_viewport와_pass_재배치로_회수된다 --exact`: AX 재배치·두 viewport 같은 Trigger ID·unchanged 다음 pass·물리 재배치·제거/재등장·AX on/off의 엔진 소유권1 PASS(7.12초/0.01초)입니다.
- [x] `tooltips::scroll_tests::tooltip_scroll은_직접_저장된_offset과_실제_wheel_변화를_감지한다 --exact`: wheel의 phase 누락 컴파일 오류와 존재하지 않는 input.rs 조회는 fixture/조회 오류입니다. 실제 공개 State.store 변경 offset12인데 Tooltip open 잔존 RED(6.90초/0.02초)를 이전 최종 offset 추적으로 수정했습니다. 이 엔진 변경에 영향받은 앞선2검사만 함께 재실행해2 PASS(14.32초/suite합계0.13초)입니다. 새 wheel 사례의 pointer가 auto-shrink 전 폭의 중앙으로 놓여 actual 영역 밖인 실패는 fixture 폭 제한과 Start/Move로 정정했습니다. 실패한 이 신규 검사만 다시 실행해 stored/wheel1 PASS(7.25초/0.02초)이며 성공한2검사는 반복하지 않았습니다.
- [x] 공용 입력/엔진 영향은 compiled libtest의 exact6필터 단일 실행으로 확인했습니다. ordered AX/key·Keybinding modal Escape·source16 rounded/arrow 뒤 Button hit·controlled 초기 오류 timer·actual Explorer create/rename invalid·Problems viewport scroll 회수6 PASS(suite0.28초)입니다. 기존 source CSS/DOM/motion/key/event 원자료와 변경 없는 성공은 재사용했습니다.
- [x] engine/app 정적 검사는 `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml -p egui -p taide-native-app --lib --tests -- -D warnings` exit0(20.61초)입니다. Wry17 dependency warning은 억제하지 않았습니다. authored2/vendor4 exact rustfmt와 diff 확인을 수행했습니다. cache 원본과 vendor 비교는 현재109파일 동일·변경7파일(Cargo.toml 포함)·추가 MIT1파일이며 앞선111 동일은 이번 ui/scroll_area 수정 이전의 기록입니다. vendored Rust는 authored99%에 포함하지 않습니다.

후속46/전체 graph 완료가 아닙니다. 등록 전/동적/disabled 현재 pass·mixed pointer/touch·controlled top-modal Escape·모든 소비자의 실제 App GUI/DPI/원본 scroll DOM 실측은 남습니다. 제품TS·dependency/manifest/lock/MSRV·보호bundle·OS·Git은 이번 작업에서 불변입니다. 전체 M8 N1~N8 0/8·목표active·전체완료 전 commit/push 없음입니다.

## source document capture Escape와 dismissal 마운트 순서 (2026-10-05)

대상은 `tooltips.rs`, `tooltip-modal-tests.rs`, `explorer-tooltip-tests.rs`, focused input을 명시한 `tooltip-motion-tests.rs`, native 한정 egui `context.rs`·`pass_state.rs`·`containers/modal.rs`입니다. 원본 `FileTreeDraftRowItem`과 shared Dialog/Tooltip을 직접 import한 격리 reference `escape.html`·`app/escape.tsx`, `tools/m8-tooltip-escape-source-measure.ts`와 reference tsconfig를 추가했습니다. 제품 TS view는 변경하지 않았습니다.

설치된 Radix DismissableLayer의 document keydown capture는 React input의 stopPropagation보다 먼저 실행됩니다. 따라서 이전 ordered AX/key 절의 “focused draft Escape는 normal hover 설명을 보존한다”는 기대는 잘못됐습니다. 취소 metadata 회수는 유효하지만 native의 focused controlled input 예외로 normal close까지 막는 계약은 이 절의 source 결과로 정정합니다. 마운트가 늦은 Dialog가 이미 열린 Tooltip보다 먼저 Escape를 받으며 paint Order만으로 이를 판단할 수 없습니다. [Radix Dialog](https://www.radix-ui.com/primitives/docs/components/dialog), [Playwright Keyboard](https://playwright.dev/docs/api/class-keyboard), [i18next createInstance](https://www.i18next.com/overview/api#createinstance)와 설치된 원본을 확인했습니다.

- [x] source valid/invalid draft와 later modal 3사례를 독립 BrowserContext에서 한 번 성공 측정했습니다. valid/invalid 모두 `normal:closed` 후 `draft:cancel`, input 제거·normal description null입니다. hover 뒤 Dialog를 마운트한 사례는 `dialog:closed`만 기록하고 normal은 delayed-open/설명을 유지합니다. source JSON은 [escape-source](assets/2026-10-05-tooltip-escape-source.json)이며 raw 결과와 깊은 동등성 비교 exit0입니다. 가상 clock·CSS pause 없이 상태/콜백을 확인했으며 픽셀·정밀 애니메이션 시각의 증거는 아닙니다.
- [x] actual Explorer 검사에서 취소 뒤 normal described_by 잔존 RED(compile6.39초/suite0.03초), actual egui Modal 검사에서 modal Escape 미전달 RED(7.31초/0.02초)를 재현했습니다. document capture에 해당하는 Tooltip 닫기 요청과 target input의 취소를 분리했습니다. native egui의 viewport/pass 마운트 순서 registry에 Modal과 Tooltip Content를 연결하고 closed Presence의 순서를 유지합니다. 두 수정 검사는 함께 2 PASS(16.07초/0.04초)입니다.
- [x] registry의 중복/역순 draw·root/child 같은 ID·pass 회수·즉시 unregister/remount·child 제거/새 Context 고유1 PASS(7.05초/0.01초)입니다. 첫 실패(7.36초/0.01초)는 child frame 밖의 current viewport를 child라고 간주한 fixture 오류였으며 frame 안에서 읽도록 수정했습니다. 엔진을 바꿔 기대를 맞추지 않았습니다.
- [x] ordered key·Keybinding modal·controlled3·motion4·actual Explorer validation의 영향10 PASS(suite0.28초)입니다. `cargo clippy --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml -p egui -p taide-native-app --lib --tests -- -D warnings`는 전용 CARGO_HOME에서 exit0(20.72초)입니다. authored4/vendor3 exact rustfmt, selective TS typecheck/format·JSON 비교·diff exit0(마지막0.7986초)이며 unchanged 성공을 반복하지 않았습니다. Wry dependency warning17은 억제하지 않았습니다.

source build는 `/private/tmp/taide-tooltip-escape.aWvZH4/built`에 한 번 생성했습니다(Vite312ms/명령0.4295초). 원래 source CSS의 SHA256 `b66166f52e29a2ffd85e4eaf679cc6ba3bc72b6580ae57f5ef8485edff53a177`을 재사용했습니다. 기본 sandbox Chrome launch의 SIGABRT/EPERM은 제품 RED가 아니며 합성 fixture·외부 요청 차단·임시 profile에 한정한 승격 실행을 사용했습니다. 첫 승격 측정은 valid input focus 이후 instant-open을 delayed-open으로만 기다려 timeout이었고, open selector를 정정한 뒤 같은 build를 재사용해 exit0입니다. 성공 측정 전체 소요시간은 따로 기록되지 않아 추정하지 않습니다. 없는 `features/file-tree`/`vite.events.config.ts` 조회는 실제 explorer/inline tool config로 정정했습니다.

현재 vendored egui는 원본과108파일 동일·8파일 변경·추가 MIT1이며 변경8은 Context/ScrollArea/Modal/hit-test/PassState/Memory/Ui/Cargo.toml입니다. vendored Rust는 authored99%에서 제외합니다. Modal/Tooltip의 dismissal registry만 구현했으며 다른 Popup/menu 전체 graph·Dialog 닫힘 Presence·동일 batch 중 새 modal topology·등록 전/동적/disabled 현재 pass·mixed pointer/touch/IME·전체 GUI/DPI는 미완료입니다. source3+native 고유3/영향10을 전체 후속46이나 N1~N8 완료로 세지 않습니다. 제품TS·dependency/manifest/lock/MSRV·보호bundle·OS·Git은 불변이며 live command는 없습니다.

## 등록된 Trigger의 pointer 클릭·AX focus 사건 순서 (2026-10-05)

대상은 `tooltips.rs`, `tooltip-key-tests.rs`와 native 한정 egui `context.rs`입니다. 설치된 Radix Tooltip의 pointerdown 시 open close·document pointerup ref 해제·그 뒤 focus open·click close와 [공식 Tooltip 계약](https://www.radix-ui.com/primitives/docs/components/tooltip)을 확인했습니다. 이번 순서의 원본 DOM은 새로 측정하지 않았으며 source 코드 계약을 native raw AX Focus로 대조한 범위입니다. 기존 source Escape/keys 자료는 재사용합니다.

- [x] 실제 두 Trigger의 pointerdown/up→AX Focus와 역순, trigger 내부/외부 위치, UI draw 정순/역순 8조합을 신규1검사로 재현했습니다. 처음은 focus-last4조합 모두 open/AX 설명이 None인 RED(compile7.12초/suite0.06초)였습니다. begin raw pointer ref만 처리하고 per-widget의 any-down/aggregate click이 더 늦은 focus까지 닫는 것이 원인입니다.
- [x] 등록된 Trigger의 실제 pointerdown close/paired primary click을 raw 사건 순서에 연결하고 replay된 클릭을 per-widget에서 다시 적용하지 않도록 수정했습니다. actual egui interaction snapshot의 clicked ID를 사용하며 자체 click distance/time 판정을 만들지 않았습니다. 기존 widget만 pointer_replayed로 표시해 신규 Trigger의 기존 fallback을 임의로 건너뛰지 않습니다. 첫 수정 뒤 trigger 내부는 맞지만 외부2조합이 실패했습니다(7.60초/0.06초). native egui의 aggregate 외부 클릭 focus 해제가 raw batch의 더 늦은 AX Focus를 지운 별개 원인이었습니다.
- [x] 엔진 외부 해제를 실제 pointer press 또는 click release의 raw index 뒤 Focus 요청과 비교하도록 수정했습니다. Clicks는 실제 PointerEvent::Released의 click 여부를 사용하며 no-click release를 click으로 바꾸지 않습니다. 수정 검사1 PASS(11.55초/0.06초)이며 이 성공은 재실행하지 않았습니다. 엔진의 Presses/Clicks/Never×Focus 배정3위치 독립 신규1 PASS(7.28초/0.03초)입니다.
- [x] 짝없는 up/본문·외부 down/지속 down-ref/Space/Touch·재마운트/ordered key/later Modal/Keybinding Modal/source16 actual hit/Explorer Escape 영향10을 정확한 필터로 한 번 실행해10 PASS(suite0.28초)입니다. `CARGO_HOME=/Users/hyunseokbyun/development/rust/cargo cargo clippy --locked --offline --target-dir experiments/native-shell-spike/target --manifest-path native/taide-native-app/Cargo.toml -p egui -p taide-native-app --lib --tests -- -D warnings` exit0(20.08초)이며 authored2/vendor1 exact rustfmt·diff exit0입니다. 이전 strict20.72초는 이전 capture 변경 상태의 기록이며 새 상태의 검증을 대신하지 않습니다.

엔진 Event 이름의 qualification 누락 E0433과 fixture의 private input_state import E0603은 제품 동작 RED가 아닌 compile 오류이며 crate::Event/공개 SurrenderFocusOn으로 정정했습니다. apply_patch 문맥 실패는 atomic 적용되지 않았음을 확인한 뒤 실제 파일 문맥으로 재적용했습니다. 처음 조회한 루트/native vendor 경로는 없었고 실제 vendor는 `native/taide-native-app/vendor/egui-input`입니다. 같은 성공 검사를 반복하지 않았으며 source browser 재실측·추가 dependency·제품TS·manifest/lock/MSRV·OS/보호bundle/Git 변경은 없습니다.

이 검사는 등록된 Trigger의 한 primary click batch와 명시적 AX Focus 범위입니다. engine의 aggregate interaction snapshot을 썼으므로 여러 release의 각 clicked target, 이동/삭제/disabled current-pass metadata, 실제 mouse/touch pointerType 혼합, 일반 Tab/IME 및 전체 GUI를 완료로 세지 않습니다. vendor는 여전히108동일/8변경/MIT이며 authored99%에서 제외합니다. 기존 후속46·N1~N8 0/8·목표active·전체완료 전 commit/push 없음입니다.

## 후속46 완료 전 남은 구현·검증

실제 bootstrap/terminal Loader에 UI regular/medium을 함께 설치하고 Problems title/파일 이름에 medium을 연결했습니다. macOS 실제 OS-selected face·variation과 원본 fallback stack·안전한 read·공유 budget을 유지했습니다. 최종 UI2/terminal1 PASS·생산 코드 strict17.62초와 dependency/fixture 실패 구분은 [native-ui-fonts QA](2026-10-04-m8-native-ui-fonts.md)가 정본입니다. `tnum`, CJK/script cascade·전체 font/GUI parity는 아직 미완료이며 medium 성공으로 대신하지 않습니다.

- [ ] 원본 tabular digits·전체 medium/font stack·hover/focus/corner·narrow window/빈 상태 레이아웃과 실제 GUI 픽셀입니다. SVG source·분류·DPI cache와 macOS medium 연결 성공은 전체 시각 parity가 아닙니다.
- [ ] 원본 DOM의 실제 low-window 픽셀·전체 viewport DPI/glyph/분할 cache입니다. source/표준 기반 low-window 제약과 actual headless 두 viewport scroll/회수는 통과했으며 전체 시각/OS 검증으로 세지 않습니다.
- [ ] Tab/여러 pointer와 AX Focus·동적 행 재등장·상태바/여러 slot/viewport가 섞인 전체 사건 적용 순서·전체 AX입니다. 명시적 AX Focus/Enter 배정과 filter/collapse/Close의6조합은 통과했지만 전체 keyboard parity와 사용자 담당 VoiceOver 실기는 완료로 보지 않습니다.
- [ ] 전체 provider/dynamic root 등록·raw/marker/catalogue/pending journal 누적 quota/RSS·전체 진단 consumer·실제 App GUI/OS 수명과 전체 M8 gates입니다. 같은 generation 재시도/소진 후 실제 worker 회수·다음 열기, root 합류/격리, 두 provider의 부분 URI 전환/폐기와 기본 inactive 진단·5초 grace·재사용/명시 stop·stale revision 경계는 통과했으며 full App/provider/OS 수명을 대신하지 않습니다.

보호 bundle·사용자 앱·OS·IME/VoiceOver·clipboard·Keychain·실제 네트워크 서비스·Git은 조작하지 않았습니다. 합성 fixture만 사용했고 live Cargo handle은 없습니다. 전체 M8 N1~N8은 0/8·목표 active이며 전체 완료 전 commit/push하지 않습니다.
